/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! State and methods for desktop implementations.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use log::{error, info};
use servo::{
    AllowOrDenyRequest, AuthenticationRequest, DeviceIntPoint, DeviceIntSize, EmbedderControl,
    EmbedderControlId, EventLoopWaker, GamepadHapticEffectType, GenericSender, InputEventId,
    InputEventResult, IpcSender, LoadStatus, MediaSessionEvent, PermissionRequest, Servo,
    ServoDelegate, ServoError, WebView, WebViewDelegate, WebViewId, pref,
};
use url::Url;

use crate::GamepadSupport;
use crate::browser_window::{BrowserWindow, BrowserWindowId};
use crate::prefs::ServoShellPreferences;

#[derive(Default)]
pub struct WebViewCollection {
    /// List of top-level browsing contexts.
    /// Modified by EmbedderMsg::WebViewOpened and EmbedderMsg::WebViewClosed,
    /// and we exit if it ever becomes empty.
    webviews: HashMap<WebViewId, WebView>,

    /// The order in which the webviews were created.
    pub(crate) creation_order: Vec<WebViewId>,

    /// The [`WebView`] that is currently active. This is the [`WebView`] that is shown and has
    /// input focus.
    active_webview_id: Option<WebViewId>,
}

impl WebViewCollection {
    pub fn add(&mut self, webview: WebView) {
        let id = webview.id();
        self.creation_order.push(id);
        self.webviews.insert(id, webview);
    }

    /// Removes a webview from the collection by [`WebViewId`]. If the removed [`WebView`] was the active
    /// [`WebView`] then the next newest [`WebView`] will be activated.
    pub fn remove(&mut self, id: WebViewId) -> Option<WebView> {
        self.creation_order.retain(|&webview_id| webview_id != id);
        let removed_webview = self.webviews.remove(&id);

        if self.active_webview_id == Some(id) {
            self.active_webview_id = None;
            if let Some(newest) = self.creation_order.last() {
                self.activate_webview(*newest);
            }
        }

        removed_webview
    }

    pub fn get(&self, id: WebViewId) -> Option<&WebView> {
        self.webviews.get(&id)
    }

    pub fn contains(&self, id: WebViewId) -> bool {
        self.webviews.contains_key(&id)
    }

    pub fn active(&self) -> Option<&WebView> {
        self.active_webview_id.and_then(|id| self.webviews.get(&id))
    }

    pub fn active_id(&self) -> Option<WebViewId> {
        self.active_webview_id
    }

    pub fn all_in_creation_order(&self) -> impl Iterator<Item = (WebViewId, &WebView)> {
        self.creation_order
            .iter()
            .filter_map(move |id| self.webviews.get(id).map(|webview| (*id, webview)))
    }

    /// Returns an iterator over all webview references (in arbitrary order).
    pub fn values(&self) -> impl Iterator<Item = &WebView> {
        self.webviews.values()
    }

    /// Returns true if the collection contains no webviews.
    pub fn is_empty(&self) -> bool {
        self.webviews.is_empty()
    }

    pub(crate) fn activate_webview(&mut self, id_to_activate: WebViewId) {
        assert!(self.creation_order.contains(&id_to_activate));

        self.active_webview_id = Some(id_to_activate);
        for (webview_id, webview) in self.all_in_creation_order() {
            if id_to_activate == webview_id {
                webview.show();
                webview.focus();
            } else {
                webview.hide();
                webview.blur();
            }
        }
    }

    pub(crate) fn activate_webview_by_index(&mut self, index: usize) {
        self.activate_webview(
            *self
                .creation_order
                .get(index)
                .expect("Tried to activate an unknown WebView"),
        );
    }
}

/// A command received via the user interacting with the user interface.
pub enum UserInterfaceCommand {
    Go(String),
    Back,
    Forward,
    Reload,
    NewWebView,
    CloseWebView(WebViewId),
}

pub(crate) struct RunningAppState {
    /// Gamepad support, which may be `None` if it failed to initialize.
    gamepad_support: RefCell<Option<GamepadSupport>>,

    /// servoshell specific preferences created during startup of the application.
    pub(crate) servoshell_preferences: ServoShellPreferences,

    /// A handle to the Servo instance.
    pub(crate) servo: Servo,

    /// Whether or not program exit has been triggered. This means that all windows
    /// will be destroyed and shutdown will start at the end of the current event loop.
    exit_scheduled: Cell<bool>,

    /// The set of [`BrowserWindow`]s that currently exist for this instance of servoshell.
    // This is the last field of the struct to ensure that windows are dropped *after* all
    // other references to the relevant rendering contexts have been destroyed.
    // See https://github.com/servo/servo/issues/36711.
    windows: RefCell<HashMap<BrowserWindowId, Rc<BrowserWindow>>>,
}

impl RunningAppState {
    pub(crate) fn new(
        servo: Servo,
        servoshell_preferences: ServoShellPreferences,
        _event_loop_waker: Box<dyn EventLoopWaker>,
    ) -> Self {
        servo.set_delegate(Rc::new(ServoShellServoDelegate));

        let gamepad_support = if pref!(dom_gamepad_enabled) {
            GamepadSupport::maybe_new()
        } else {
            None
        };

        Self {
            windows: Default::default(),
            gamepad_support: RefCell::new(gamepad_support),
            servoshell_preferences,
            servo,
            exit_scheduled: Default::default(),
        }
    }

    pub(crate) fn open_window(self: &Rc<Self>, window: Rc<BrowserWindow>, initial_url: Url) {
        window.create_and_activate_toplevel_webview(self.clone(), initial_url);
        self.windows.borrow_mut().insert(window.id(), window);
    }

    pub(crate) fn focused_window(&self) -> Option<Rc<BrowserWindow>> {
        self.windows
            .borrow()
            .values()
            .find(|window| window.focused())
            .cloned()
    }

    pub(crate) fn window(&self, id: BrowserWindowId) -> Option<Rc<BrowserWindow>> {
        self.windows.borrow().get(&id).cloned()
    }

    pub(crate) fn servo(&self) -> &Servo {
        &self.servo
    }

    pub(crate) fn schedule_exit(&self) {
        self.exit_scheduled.set(true);
    }

    /// Spins the internal application event loop.
    ///
    /// - Notifies Servo about incoming gamepad events
    /// - Spin the Servo event loop, which will run the compositor and trigger delegate methods.
    ///
    /// Returns true if the event loop should continue spinning and false if it should exit.
    pub(crate) fn spin_event_loop(self: &Rc<Self>) -> bool {
        if pref!(dom_gamepad_enabled) {
            self.handle_gamepad_events();
        }

        self.servo.spin_event_loop();

        for window in self.windows.borrow().values() {
            window.update_and_request_repaint_if_necessary(self);
        }

        // When a BrowserWindow has no more WebViews, close it. When no more windows are open, exit
        // the application.
        self.windows
            .borrow_mut()
            .retain(|_, window| !self.exit_scheduled.get() && !window.should_close());
        if self.windows.borrow().is_empty() {
            self.schedule_exit()
        }

        !self.exit_scheduled.get()
    }

    pub(crate) fn foreach_window_and_interface_commands(
        self: &Rc<Self>,
        callback: impl Fn(&BrowserWindow, Vec<UserInterfaceCommand>),
    ) {
        // We clone here to avoid a double borrow. User interface commands can update the list of windows.
        let windows: Vec<_> = self.windows.borrow().values().cloned().collect();
        for window in windows {
            callback(&window, window.take_user_interface_commands())
        }
    }

    pub(crate) fn maybe_window_for_webview_id(
        &self,
        webview_id: WebViewId,
    ) -> Option<Rc<BrowserWindow>> {
        for window in self.windows.borrow().values() {
            if window.contains_webview(webview_id) {
                return Some(window.clone());
            }
        }
        None
    }

    pub(crate) fn window_for_webview_id(&self, webview_id: WebViewId) -> Rc<BrowserWindow> {
        self.maybe_window_for_webview_id(webview_id)
            .expect("Looking for unexpected WebView: {webview_id:?}")
    }

    pub(crate) fn platform_window_for_webview_id(
        &self,
        webview_id: WebViewId,
    ) -> Rc<BrowserWindow> {
        self.window_for_webview_id(webview_id)
    }

    pub(crate) fn handle_gamepad_events(&self) {
        let Some(active_webview) = self
            .focused_window()
            .and_then(|window| window.active_webview())
        else {
            return;
        };
        if let Some(gamepad_support) = self.gamepad_support.borrow_mut().as_mut() {
            gamepad_support.handle_gamepad_events(active_webview);
        }
    }
}

impl WebViewDelegate for RunningAppState {
    fn screen_geometry(&self, webview: WebView) -> Option<servo::ScreenGeometry> {
        Some(
            self.platform_window_for_webview_id(webview.id())
                .screen_geometry(),
        )
    }

    fn notify_status_text_changed(&self, webview: WebView, _status: Option<String>) {
        self.window_for_webview_id(webview.id()).set_needs_update();
    }

    fn notify_history_changed(&self, webview: WebView, _entries: Vec<Url>, _current: usize) {
        self.window_for_webview_id(webview.id()).set_needs_update();
    }

    fn notify_page_title_changed(&self, webview: WebView, _: Option<String>) {
        self.window_for_webview_id(webview.id()).set_needs_update();
    }

    fn request_move_to(&self, webview: WebView, new_position: DeviceIntPoint) {
        self.platform_window_for_webview_id(webview.id())
            .set_position(new_position);
    }

    fn request_resize_to(&self, webview: WebView, requested_outer_size: DeviceIntSize) {
        self.platform_window_for_webview_id(webview.id())
            .request_resize(&webview, requested_outer_size);
    }

    fn request_authentication(
        &self,
        webview: WebView,
        authentication_request: AuthenticationRequest,
    ) {
        self.platform_window_for_webview_id(webview.id())
            .show_http_authentication_dialog(webview.id(), authentication_request);
    }

    fn notify_closed(&self, webview: WebView) {
        self.window_for_webview_id(webview.id())
            .close_webview(webview.id())
    }

    fn notify_input_event_handled(
        &self,
        webview: WebView,
        id: InputEventId,
        result: InputEventResult,
    ) {
        self.platform_window_for_webview_id(webview.id())
            .notify_input_event_handled(&webview, id, result);
    }

    fn notify_cursor_changed(&self, webview: WebView, cursor: servo::Cursor) {
        self.platform_window_for_webview_id(webview.id())
            .set_cursor(cursor);
    }

    fn notify_load_status_changed(&self, webview: WebView, _status: LoadStatus) {
        self.window_for_webview_id(webview.id()).set_needs_update();
    }

    fn notify_fullscreen_state_changed(&self, webview: WebView, fullscreen_state: bool) {
        self.platform_window_for_webview_id(webview.id())
            .set_fullscreen(fullscreen_state);
    }

    fn show_bluetooth_device_dialog(
        &self,
        webview: WebView,
        devices: Vec<String>,
        response_sender: GenericSender<Option<String>>,
    ) {
        self.platform_window_for_webview_id(webview.id())
            .show_bluetooth_device_dialog(webview.id(), devices, response_sender);
    }

    fn request_permission(&self, webview: WebView, permission_request: PermissionRequest) {
        self.platform_window_for_webview_id(webview.id())
            .show_permission_dialog(webview.id(), permission_request);
    }

    fn notify_new_frame_ready(&self, webview: WebView) {
        self.window_for_webview_id(webview.id()).set_needs_repaint();
    }

    fn play_gamepad_haptic_effect(
        &self,
        _webview: WebView,
        index: usize,
        effect_type: GamepadHapticEffectType,
        effect_complete_sender: IpcSender<bool>,
    ) {
        match self.gamepad_support.borrow_mut().as_mut() {
            Some(gamepad_support) => {
                gamepad_support.play_haptic_effect(index, effect_type, effect_complete_sender);
            }
            None => {
                let _ = effect_complete_sender.send(false);
            }
        }
    }

    fn stop_gamepad_haptic_effect(
        &self,
        _webview: WebView,
        index: usize,
        haptic_stop_sender: IpcSender<bool>,
    ) {
        let stopped = match self.gamepad_support.borrow_mut().as_mut() {
            Some(gamepad_support) => gamepad_support.stop_haptic_effect(index),
            None => false,
        };
        let _ = haptic_stop_sender.send(stopped);
    }

    fn show_embedder_control(&self, webview: WebView, embedder_control: EmbedderControl) {
        self.window_for_webview_id(webview.id())
            .show_embedder_control(webview.id(), embedder_control);
    }

    fn hide_embedder_control(&self, webview: WebView, embedder_control_id: EmbedderControlId) {
        self.window_for_webview_id(webview.id())
            .hide_embedder_control(webview.id(), embedder_control_id);
    }

    fn notify_favicon_changed(&self, webview: WebView) {
        self.window_for_webview_id(webview.id())
            .notify_favicon_changed(webview);
    }

    fn notify_media_session_event(&self, webview: WebView, event: MediaSessionEvent) {
        self.platform_window_for_webview_id(webview.id())
            .notify_media_session_event(event);
    }

    fn notify_crashed(&self, webview: WebView, reason: String, backtrace: Option<String>) {
        self.platform_window_for_webview_id(webview.id())
            .notify_crashed(webview, reason, backtrace);
    }
}

struct ServoShellServoDelegate;
impl ServoDelegate for ServoShellServoDelegate {
    fn notify_devtools_server_started(&self, port: u16, _token: String) {
        info!("Devtools Server running on port {port}");
    }

    fn request_devtools_connection(&self, request: AllowOrDenyRequest) {
        request.allow();
    }

    fn notify_error(&self, error: ServoError) {
        error!("Saw Servo error: {error:?}!");
    }
}
