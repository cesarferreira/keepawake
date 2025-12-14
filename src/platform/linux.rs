use std::process::{Command, Stdio};

pub fn keep_awake() -> Result<(), String> {
    Command::new("xdg-screensaver")
        .arg("reset")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("failed to run xdg-screensaver reset: {err}"))
}
