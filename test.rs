/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::parser::location_bar_input_to_url;

// Helper function to test url
fn test_url(input: &str, location: &str) {
    assert_eq!(
        location_bar_input_to_url(input, "https://duckduckgo.com/html/?q=%s")
            .unwrap()
            .into_string(),
        location
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_cmdline_and_location_bar_url() {
    test_url("data:text/html,a", "data:text/html,a");
    test_url("README.md", "https://readme.md/");
    test_url("nic.md", "https://nic.md/");
    test_url("nic.md/ro", "https://nic.md/ro");
    test_url("foo.txt", "https://foo.txt/");
    test_url("foo.txt/ro", "https://foo.txt/ro");
    test_url(
        "resources/public_domains.txt",
        "https://resources/public_domains.txt",
    );
    test_url("dragonfruit", "https://duckduckgo.com/html/?q=dragonfruit");
}

#[test]
#[cfg(target_os = "windows")]
fn test_cmdline_and_location_bar_url() {
    test_url("data:text/html,a", "data:text/html,a");
    test_url("README.md", "https://readme.md/");
    test_url("nic.md", "https://nic.md/");
    test_url("nic.md/ro", "https://nic.md/ro");
    test_url("foo.txt", "https://foo.txt/");
    test_url("foo.txt/ro", "https://foo.txt/ro");
    test_url(
        "resources/public_domains.txt",
        "https://resources/public_domains.txt",
    );
    test_url("dragonfruit", "https://duckduckgo.com/html/?q=dragonfruit");
}

#[cfg(target_os = "linux")]
#[test]
fn test_cmd_and_location_bar_url() {
    test_url("/dev/null", "file:///dev/null");
}

/// Like [test_url] but will produce test for Windows or non Windows using `#[cfg(target_os)]` internally.
fn test_url_any_os(input: &str, location: &str) {
    #[cfg(not(target_os = "windows"))]
    test_url(input, location);

    #[cfg(target_os = "windows")]
    test_url(input, location);
}

// https://github.com/servo/servo/issues/35754
#[test]
fn test_issue_35754() {
    test_url_any_os("leah.chromebooks.lol", "https://leah.chromebooks.lol/");

    // ends with dot
    test_url_any_os("leah.chromebooks.lol.", "https://leah.chromebooks.lol./");

    // starts with dot
    test_url_any_os(
        ".leah.chromebooks.lol",
        "https://duckduckgo.com/html/?q=.leah.chromebooks.lol",
    );

    // contains spaces
    test_url_any_os(
        "3.5 kg in lb",
        "https://duckduckgo.com/html/?q=3.5%20kg%20in%20lb",
    );

    // user-local domain
    test_url_any_os("foo/bar", "https://foo/bar");
}
