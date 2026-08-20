#[cfg(target_os = "macos")]
pub fn paste() -> Result<(), String> {
    use std::process::Command;
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .output()
        .map_err(|error| format!("Could not start macOS paste automation: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(
        if detail.contains("not authorized") || detail.contains("-1743") {
            "macOS blocked Paste Next. Allow Accessibility access for Pasted (or the terminal/IDE running this development build), then try again.".to_string()
        } else if detail.is_empty() {
            "macOS rejected the simulated paste. Check Pasted's Accessibility permission."
                .to_string()
        } else {
            format!("macOS rejected the simulated paste: {detail}")
        },
    )
}

#[cfg(target_os = "windows")]
pub fn paste() -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("powershell")
        .arg("-Command")
        .arg("$wshell = New-Object -ComObject wscript.shell; $wshell.SendKeys('^v')")
        .status()
        .map_err(|error| format!("Could not start Windows paste automation: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "Windows rejected the simulated paste".to_string())
}

#[cfg(target_os = "linux")]
pub fn paste() -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("xdotool")
        .arg("key")
        .arg("ctrl+v")
        .status()
        .map_err(|error| format!("Could not start Linux paste automation: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "Linux rejected the simulated paste".to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn paste() -> Result<(), String> {
    Err("Paste automation is unavailable on this platform".to_string())
}
