// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------

#[cfg(target_os = "windows")]
pub fn wait_for_debugger() {
    loop {
        if unsafe { windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent().as_bool() } {
            log::info!("Debugger found.");
            break;
        } else {
            log::info!("Waiting for debugger.");
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}

#[cfg(target_os = "linux")]
pub fn wait_for_debugger() {}
