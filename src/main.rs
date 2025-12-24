/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// Normally, rust uses the "Console" Windows subsystem, which pops up a console
// when running an application. Switching to the "Windows" subsystem prevents
// this, but also hides debugging output. We mitigate this by attempting to
// attach to the console of the parent process.
#![windows_subsystem = "windows"]

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Console;

fn main() {
    #[cfg(target_os = "windows")]
    // SAFETY: No safety related side effects or requirements.
    unsafe {
        // When servo is started from the commandline, we still want output
        // to be printed. Due to us using the `windows` subsystem, this doesn't
        // work out-of-the-box, and we need to manually attempt to attach to
        // the console of the parent process. If servo was not started from
        // the commandline, then the call will fail, which we can ignore.
        let _result = Console::AttachConsole(Console::ATTACH_PARENT_PROCESS);
    }
    servo_browser::main()
}
