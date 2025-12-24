/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::panic;

use app::App;
use prefs::{ServoShellPreferences, get_preferences};
use winit::event_loop::EventLoop;

#[cfg(test)]
mod test;

pub(crate) mod app;
pub(crate) mod dialog;
pub(crate) mod event_loop;
pub mod geometry;
pub mod headed_window;
mod keyutils;
pub mod misc_utils;
pub mod panic_utils;
mod parser;
mod prefs;
mod resource_protocol;
mod resources;
mod running_app_state;
pub mod user_interface;
mod window;

const NEW_TAB_PAGE_URL: &str = "resource:///newtab.html";

pub(crate) use crate::misc_utils::gamepad::GamepadSupport;

pub mod platform {
    #[cfg(target_os = "macos")]
    pub use crate::platform::macos::deinit;

    #[cfg(target_os = "macos")]
    pub mod macos;

    #[cfg(not(target_os = "macos"))]
    pub fn deinit(_clean_shutdown: bool) {}
}

pub fn main() {
    panic_utils::crash_handler::install();
    init_crypto();
    resources::init();

    // TODO: once log-panics is released, can this be replaced by
    // log_panics::init()?
    panic::set_hook(Box::new(panic_utils::panic_hook::panic_hook));

    let event_loop = EventLoop::with_user_event()
        .build()
        .expect("Could not start winit event loop");

    {
        let preferences = get_preferences();
        let servoshell_preferences = ServoShellPreferences::default();
        let mut app = App::new(preferences, servoshell_preferences, &event_loop);
        event_loop
            .run_app(&mut app)
            .expect("Failed while running events loop");
    }

    crate::platform::deinit(false)
}

pub fn init_crypto() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Error initializing crypto provider");
}

pub const VERSION: &str = concat!("Servo ", env!("CARGO_PKG_VERSION"), "-", env!("GIT_SHA"));
