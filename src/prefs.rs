/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;

use servo::{Opts, PrefValue, Preferences};

pub(crate) static EXPERIMENTAL_PREFS: &[&str] = &[
    "dom_async_clipboard_enabled",
    "dom_fontface_enabled",
    "dom_intersection_observer_enabled",
    "dom_navigator_protocol_handlers_enabled",
    "dom_navigator_sendbeacon_enabled",
    "dom_notification_enabled",
    "dom_offscreen_canvas_enabled",
    "dom_permissions_enabled",
    "dom_webgl2_enabled",
    "dom_webgpu_enabled",
    "layout_columns_enabled",
    "layout_container_queries_enabled",
    "layout_grid_enabled",
    "layout_variable_fonts_enabled",
];

#[derive(Clone)]
pub(crate) struct ServoShellPreferences {
    /// URL string of the search engine page with '%s' standing in for the search term.
    /// For example <https://duckduckgo.com/html/?q=%s>.
    pub searchpage: String,
}

impl Default for ServoShellPreferences {
    fn default() -> Self {
        Self {
            searchpage: "https://duckduckgo.com/html/?q=%s".into(),
        }
    }
}

pub fn default_config_dir() -> PathBuf {
    let mut config_dir = dirs::config_dir().unwrap();

    #[cfg(target_os = "linux")]
    config_dir.push("servo");

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    config_dir.push("Servo");

    config_dir
}

/// Get a Servo [`Preferences`] to use when initializing Servo by first reading the user
/// preferences file
pub(crate) fn get_preferences() -> Preferences {
    let mut preferences = Preferences::default();
    for pref in EXPERIMENTAL_PREFS {
        preferences.set_value(pref, PrefValue::Bool(true));
    }

    preferences
}

pub(crate) fn get_opts() -> Opts {
    Opts {
        config_dir: Some(default_config_dir()),
        ..Default::default()
    }
}
