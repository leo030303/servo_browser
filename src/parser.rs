/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use servo::{ServoUrl, is_reg_domain};

/// Interpret an input URL.
///
/// If this is not a valid URL, try to "fix" it by adding a scheme or if all else fails,
/// interpret the string as a search term.
pub(crate) fn location_bar_input_to_url(request: &str, searchpage: &str) -> Option<ServoUrl> {
    let request = request.trim();
    ServoUrl::parse(request)
        .ok()
        .or_else(|| try_as_file(request))
        .or_else(|| try_as_domain(request))
        .or_else(|| try_as_search_page(request, searchpage))
}

fn try_as_file(request: &str) -> Option<ServoUrl> {
    if request.starts_with('/') {
        return ServoUrl::parse(&format!("file://{request}")).ok();
    }
    None
}

fn try_as_domain(request: &str) -> Option<ServoUrl> {
    fn is_domain_like(s: &str) -> bool {
        !s.starts_with('/') && s.contains('/')
            || (!s.contains(' ') && !s.starts_with('.') && s.split('.').count() > 1)
    }

    if !request.contains(' ') && is_reg_domain(request) || is_domain_like(request) {
        return ServoUrl::parse(&format!("https://{request}")).ok();
    }
    None
}

fn try_as_search_page(request: &str, searchpage: &str) -> Option<ServoUrl> {
    if request.is_empty() {
        return None;
    }
    ServoUrl::parse(&searchpage.replace("%s", request)).ok()
}
