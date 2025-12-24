/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Application entry point, runs the event loop.

use log::warn;
use servo::protocol_handler::ProtocolRegistry;
use servo::{EventLoopWaker, Preferences, ServoBuilder, ServoUrl};
use std::rc::Rc;
use std::time::Instant;
use url::Url;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;

use super::browser_window::{self, BrowserWindow};
use super::event_loop::{AppEvent, HeadedEventLoopWaker};
use super::resource_protocol::ResourceProtocolHandler;
use crate::panic_utils::tracing::trace_winit_event;
use crate::parser::location_bar_input_to_url;
use crate::prefs::ServoShellPreferences;
use crate::running_app_state::{RunningAppState, UserInterfaceCommand};
use crate::{NEW_TAB_PAGE_URL, prefs};

pub(crate) enum AppState {
    Initializing,
    Running(Rc<RunningAppState>),
    ShuttingDown,
}

pub struct App {
    preferences: Preferences,
    servoshell_preferences: ServoShellPreferences,
    waker: Box<dyn EventLoopWaker>,
    event_loop_proxy: EventLoopProxy<AppEvent>,
    initial_url: ServoUrl,
    t_start: Instant,
    t: Instant,
    state: AppState,
}

impl App {
    pub fn new(
        preferences: Preferences,
        servo_shell_preferences: ServoShellPreferences,
        event_loop: &EventLoop<AppEvent>,
    ) -> Self {
        let t = Instant::now();
        App {
            preferences,
            servoshell_preferences: servo_shell_preferences,
            waker: Box::new(HeadedEventLoopWaker::new(event_loop)),
            event_loop_proxy: event_loop.create_proxy(),
            initial_url: ServoUrl::parse(NEW_TAB_PAGE_URL).expect("Coming from const"),
            t_start: t,
            t,
            state: AppState::Initializing,
        }
    }

    /// Initialize Application once event loop start running.
    pub fn init(&mut self, active_event_loop: &ActiveEventLoop) {
        let mut protocol_registry = ProtocolRegistry::default();
        let _ = protocol_registry.register("resource", ResourceProtocolHandler::default());

        let servo_builder = ServoBuilder::default()
            .opts(prefs::get_opts())
            .preferences(self.preferences.clone())
            .protocol_registry(protocol_registry)
            .event_loop_waker(self.waker.clone());

        let url = self.initial_url.as_url().clone();
        let platform_window = self.create_platform_window(url, active_event_loop);

        #[cfg(feature = "webxr")]
        let servo_builder = servo_builder.webxr_registry(
            super::misc_utils::webxr::XrDiscoveryWebXrRegistry::new_boxed(
                platform_window.clone(),
                active_event_loop,
                &self.preferences,
            ),
        );

        let servo = servo_builder.build();
        servo.setup_logging();

        let running_state = Rc::new(RunningAppState::new(
            servo,
            self.servoshell_preferences.clone(),
            self.waker.clone(),
        ));
        running_state.open_window(platform_window, self.initial_url.as_url().clone());

        self.state = AppState::Running(running_state);
    }

    fn create_platform_window(
        &self,
        url: Url,
        active_event_loop: &ActiveEventLoop,
    ) -> Rc<BrowserWindow> {
        browser_window::BrowserWindow::new(active_event_loop, self.event_loop_proxy.clone(), url)
    }

    pub fn pump_servo_event_loop(&mut self, active_event_loop: Option<&ActiveEventLoop>) -> bool {
        let AppState::Running(state) = &self.state else {
            return false;
        };

        state.foreach_window_and_interface_commands(|window, commands| {
            self.handle_interface_commands_for_window(active_event_loop, state, window, commands);
        });

        if !state.spin_event_loop() {
            self.state = AppState::ShuttingDown;
            return false;
        }
        true
    }

    /// Takes any events generated during `egui` updates and performs their actions.
    fn handle_interface_commands_for_window(
        &self,
        _active_event_loop: Option<&ActiveEventLoop>,
        state: &Rc<RunningAppState>,
        window: &BrowserWindow,
        commands: Vec<UserInterfaceCommand>,
    ) {
        for event in commands {
            match event {
                UserInterfaceCommand::Go(location) => {
                    window.set_needs_update();
                    let Some(url) = location_bar_input_to_url(
                        &location.clone(),
                        &state.servoshell_preferences.searchpage,
                    ) else {
                        warn!("failed to parse location");
                        break;
                    };
                    if let Some(active_webview) = window.active_webview() {
                        active_webview.load(url.into_url());
                    }
                }
                UserInterfaceCommand::Back => {
                    if let Some(active_webview) = window.active_webview() {
                        active_webview.go_back(1);
                    }
                }
                UserInterfaceCommand::Forward => {
                    if let Some(active_webview) = window.active_webview() {
                        active_webview.go_forward(1);
                    }
                }
                UserInterfaceCommand::Reload => {
                    window.set_needs_update();
                    if let Some(active_webview) = window.active_webview() {
                        active_webview.reload();
                    }
                }
                UserInterfaceCommand::NewWebView => {
                    window.set_needs_update();
                    let url = Url::parse("resource:///newtab.html")
                        .expect("Should always be able to parse");
                    window.create_and_activate_toplevel_webview(state.clone(), url);
                }
                UserInterfaceCommand::CloseWebView(id) => {
                    window.set_needs_update();
                    window.close_webview(id);
                }
            }
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.init(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        window_event: WindowEvent,
    ) {
        let now = Instant::now();
        trace_winit_event!(
            window_event,
            "@{:?} (+{:?}) {window_event:?}",
            now - self.t_start,
            now - self.t
        );
        self.t = now;

        {
            let AppState::Running(state) = &self.state else {
                return;
            };
            let window_id: u64 = window_id.into();
            if let Some(window) = state.window(window_id.into()) {
                window.handle_winit_window_event(state.clone(), window_event);
            }
        }

        if !self.pump_servo_event_loop(event_loop.into()) {
            event_loop.exit();
        }
        // Block until the window gets an event
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, app_event: AppEvent) {
        {
            let AppState::Running(state) = &self.state else {
                return;
            };
            if let Some(window_id) = app_event.window_id() {
                let window_id: u64 = window_id.into();
                if let Some(window) = state.window(window_id.into()) {
                    window.handle_winit_app_event(app_event);
                }
            }
        }

        if !self.pump_servo_event_loop(event_loop.into()) {
            event_loop.exit();
        }

        // Block until the window gets an event
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}
