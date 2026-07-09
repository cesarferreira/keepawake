mod controller;
use crate::runtime::AwakeControl;
use controller::StatefulController;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::PlatformBackend;
#[cfg(target_os = "macos")]
use macos::PlatformBackend;
#[cfg(target_os = "windows")]
use windows::PlatformBackend;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
struct PlatformBackend;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl PlatformBackend {
    fn new() -> Self {
        Self
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl controller::Backend for PlatformBackend {
    fn activate(&mut self) -> Result<(), String> {
        Err("unsupported platform".to_string())
    }

    fn deactivate(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn requires_periodic_refresh(&self) -> bool {
        false
    }
}

pub(crate) fn new_controller() -> impl AwakeControl {
    StatefulController::new(PlatformBackend::new())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::new_controller;
    use crate::runtime::AwakeControl;

    #[test]
    fn native_controller_can_activate_and_release() {
        let mut controller = new_controller();

        controller.set_active(true).unwrap();
        assert!(controller.is_active());
        controller.set_active(false).unwrap();
        assert!(!controller.is_active());
    }
}
