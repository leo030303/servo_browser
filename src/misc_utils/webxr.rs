/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use euclid::{Angle, Rotation3D, Size2D, UnknownUnit, Vector3D};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use servo::webxr::{GlWindowDiscovery, WebXrRegistry};
#[cfg(target_os = "windows")]
use servo::webxr::{OpenXrAppInfo, OpenXrDiscovery};
use servo::{Key, KeyState, KeyboardEvent, Preferences, pref, prefs};
use winit::event::{ElementState, KeyEvent};
use winit::event_loop::ActiveEventLoop;

use crate::headed_window::BrowserWindow;
use winit::keyboard::{Key as LogicalKey, ModifiersState, NamedKey as WinitNamedKey};

enum XrDiscovery {
    GlWindow(GlWindowDiscovery),
    #[cfg(target_os = "windows")]
    OpenXr(OpenXrDiscovery),
}

pub(crate) struct XrDiscoveryWebXrRegistry {
    xr_discovery: RefCell<Option<XrDiscovery>>,
}

impl XrDiscoveryWebXrRegistry {
    pub(crate) fn new_boxed(
        window: Rc<BrowserWindow>,
        event_loop: &ActiveEventLoop,
        preferences: &Preferences,
    ) -> Box<Self> {
        let xr_discovery = if preferences.dom_webxr_openxr_enabled {
            #[cfg(target_os = "windows")]
            {
                let app_info = OpenXrAppInfo::new("Servoshell", 0, "Servo", 0);
                Some(XrDiscovery::OpenXr(OpenXrDiscovery::new(None, app_info)))
            }
            #[cfg(not(target_os = "windows"))]
            None
        } else if preferences.dom_webxr_glwindow_enabled {
            let window = window.new_glwindow(event_loop);
            Some(XrDiscovery::GlWindow(GlWindowDiscovery::new(window)))
        } else {
            None
        };

        Box::new(Self {
            xr_discovery: RefCell::new(xr_discovery),
        })
    }
}

struct XrPrefObserver(Arc<AtomicBool>);

impl prefs::PreferencesObserver for XrPrefObserver {
    fn prefs_changed(&self, changes: &[(&'static str, prefs::PrefValue)]) {
        if let Some((_, value)) = changes.iter().find(|(name, _)| *name == "dom_webxr_test") {
            let prefs::PrefValue::Bool(value) = value else {
                return;
            };
            self.0.store(*value, Ordering::Relaxed);
        }
    }
}

impl WebXrRegistry for XrDiscoveryWebXrRegistry {
    fn register(&self, xr: &mut servo::webxr::MainThreadRegistry) {
        use servo::webxr::HeadlessMockDiscovery;

        let mock_enabled = Arc::new(AtomicBool::new(pref!(dom_webxr_test)));
        xr.register_mock(HeadlessMockDiscovery::new(mock_enabled.clone()));
        prefs::add_observer(Box::new(XrPrefObserver(mock_enabled)));

        if let Some(xr_discovery) = self.xr_discovery.take() {
            match xr_discovery {
                XrDiscovery::GlWindow(discovery) => xr.register(discovery),
                #[cfg(target_os = "windows")]
                XrDiscovery::OpenXr(discovery) => xr.register(discovery),
            }
        }
    }
}

pub struct XRWindow {
    winit_window: winit::window::Window,
    pose: Rc<XRWindowPose>,
}

impl XRWindow {
    pub fn new(winit_window: winit::window::Window, pose: Rc<XRWindowPose>) -> Self {
        Self { winit_window, pose }
    }
}

pub struct XRWindowPose {
    xr_rotation: Cell<Rotation3D<f32, UnknownUnit, UnknownUnit>>,
    xr_translation: Cell<Vector3D<f32, UnknownUnit>>,
}

impl servo::webxr::GlWindow for XRWindow {
    fn get_render_target(
        &self,
        device: &mut surfman::Device,
        _context: &mut surfman::Context,
    ) -> servo::webxr::GlWindowRenderTarget {
        self.winit_window.set_visible(true);
        let window_handle = self
            .winit_window
            .window_handle()
            .expect("could not get window handle from window");
        let size = self.winit_window.inner_size();
        let size = Size2D::new(size.width as i32, size.height as i32);
        let native_widget = device
            .connection()
            .create_native_widget_from_window_handle(window_handle, size)
            .expect("Failed to create native widget");
        servo::webxr::GlWindowRenderTarget::NativeWidget(native_widget)
    }

    fn get_rotation(&self) -> Rotation3D<f32, UnknownUnit, UnknownUnit> {
        self.pose.xr_rotation.get()
    }

    fn get_translation(&self) -> Vector3D<f32, UnknownUnit> {
        self.pose.xr_translation.get()
    }

    fn get_mode(&self) -> servo::webxr::GlWindowMode {
        use servo::pref;
        if pref!(dom_webxr_glwindow_red_cyan) {
            servo::webxr::GlWindowMode::StereoRedCyan
        } else if pref!(dom_webxr_glwindow_left_right) {
            servo::webxr::GlWindowMode::StereoLeftRight
        } else if pref!(dom_webxr_glwindow_spherical) {
            servo::webxr::GlWindowMode::Spherical
        } else if pref!(dom_webxr_glwindow_cubemap) {
            servo::webxr::GlWindowMode::Cubemap
        } else {
            servo::webxr::GlWindowMode::Blit
        }
    }

    fn display_handle(&self) -> raw_window_handle::DisplayHandle<'_> {
        self.winit_window
            .display_handle()
            .expect("Every window should have a display handle")
    }
}

impl XRWindowPose {
    pub fn new(
        xr_rotation: Cell<Rotation3D<f32, UnknownUnit, UnknownUnit>>,
        xr_translation: Cell<Vector3D<f32, UnknownUnit>>,
    ) -> Self {
        Self {
            xr_rotation,
            xr_translation,
        }
    }
    pub fn handle_xr_translation(&self, input: &KeyboardEvent) {
        if input.event.state != KeyState::Down {
            return;
        }
        const NORMAL_TRANSLATE: f32 = 0.1;
        const QUICK_TRANSLATE: f32 = 1.0;
        let mut x = 0.0;
        let mut z = 0.0;
        match input.event.key {
            Key::Character(ref k) => match &**k {
                "w" => z = -NORMAL_TRANSLATE,
                "W" => z = -QUICK_TRANSLATE,
                "s" => z = NORMAL_TRANSLATE,
                "S" => z = QUICK_TRANSLATE,
                "a" => x = -NORMAL_TRANSLATE,
                "A" => x = -QUICK_TRANSLATE,
                "d" => x = NORMAL_TRANSLATE,
                "D" => x = QUICK_TRANSLATE,
                _ => return,
            },
            _ => return,
        };
        let (old_x, old_y, old_z) = self.xr_translation.get().to_tuple();
        let vec = Vector3D::new(x + old_x, old_y, z + old_z);
        self.xr_translation.set(vec);
    }

    pub fn handle_xr_rotation(&self, input: &KeyEvent, modifiers: ModifiersState) {
        if input.state != ElementState::Pressed {
            return;
        }
        let mut x = 0.0;
        let mut y = 0.0;
        match input.logical_key {
            LogicalKey::Named(WinitNamedKey::ArrowUp) => x = 1.0,
            LogicalKey::Named(WinitNamedKey::ArrowDown) => x = -1.0,
            LogicalKey::Named(WinitNamedKey::ArrowLeft) => y = 1.0,
            LogicalKey::Named(WinitNamedKey::ArrowRight) => y = -1.0,
            _ => return,
        };
        if modifiers.shift_key() {
            x *= 10.0;
            y *= 10.0;
        }
        let x: Rotation3D<_, UnknownUnit, UnknownUnit> = Rotation3D::around_x(Angle::degrees(x));
        let y: Rotation3D<_, UnknownUnit, UnknownUnit> = Rotation3D::around_y(Angle::degrees(y));
        let rotation = self.xr_rotation.get().then(&x).then(&y);
        self.xr_rotation.set(rotation);
    }
}
