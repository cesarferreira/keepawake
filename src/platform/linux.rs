use super::controller::Backend;
use std::process::{Command, Stdio};

pub(crate) struct PlatformBackend;

impl PlatformBackend {
    pub(crate) fn new() -> Self {
        Self
    }

    fn reset_screensaver() -> Result<(), String> {
        let status = Command::new("xdg-screensaver")
            .arg("reset")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|err| format!("failed to run xdg-screensaver reset: {err}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("xdg-screensaver reset exited with {status}"))
        }
    }
}

impl Backend for PlatformBackend {
    fn activate(&mut self) -> Result<(), String> {
        Self::reset_screensaver()
    }

    fn deactivate(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), String> {
        Self::reset_screensaver()
    }

    fn requires_periodic_refresh(&self) -> bool {
        true
    }
}
