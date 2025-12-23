/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! An event loop implementation that works in headless mode.

use std::sync::{Arc, Mutex};

use log::warn;
use servo::EventLoopWaker;
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::WindowId;

#[derive(Debug)]
pub enum AppEvent {
    /// Another process or thread has kicked the OS event loop with EventLoopWaker.
    Waker,
    Accessibility(egui_winit::accesskit_winit::Event),
    UpdateTheme {
        theme: winit::window::Theme,
        window_id: winit::window::WindowId,
    },
}

impl From<egui_winit::accesskit_winit::Event> for AppEvent {
    fn from(event: egui_winit::accesskit_winit::Event) -> AppEvent {
        AppEvent::Accessibility(event)
    }
}

impl AppEvent {
    pub(crate) fn window_id(&self) -> Option<WindowId> {
        match self {
            AppEvent::Waker => None,
            AppEvent::Accessibility(event) => Some(event.window_id),
            AppEvent::UpdateTheme { window_id, .. } => Some(*window_id),
        }
    }
}

#[derive(Clone)]
pub struct HeadedEventLoopWaker {
    proxy: Arc<Mutex<EventLoopProxy<AppEvent>>>,
}

impl HeadedEventLoopWaker {
    pub fn new(event_loop: &EventLoop<AppEvent>) -> HeadedEventLoopWaker {
        let proxy = Arc::new(Mutex::new(event_loop.create_proxy()));
        HeadedEventLoopWaker { proxy }
    }
}

impl EventLoopWaker for HeadedEventLoopWaker {
    fn wake(&self) {
        // Kick the OS event loop awake.
        if let Err(err) = self.proxy.lock().unwrap().send_event(AppEvent::Waker) {
            warn!("Failed to wake up event loop ({err}).");
        }
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
}
