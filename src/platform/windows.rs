use super::controller::Backend;
use windows::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, EXECUTION_STATE,
    SetThreadExecutionState,
};

pub(crate) struct PlatformBackend;

impl PlatformBackend {
    pub(crate) fn new() -> Self {
        Self
    }

    fn set_execution_state(flags: EXECUTION_STATE) -> Result<(), String> {
        let result = unsafe { SetThreadExecutionState(flags) };
        if result == EXECUTION_STATE(0) {
            Err("SetThreadExecutionState failed".to_string())
        } else {
            Ok(())
        }
    }
}

impl Backend for PlatformBackend {
    fn activate(&mut self) -> Result<(), String> {
        Self::set_execution_state(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)
    }

    fn deactivate(&mut self) -> Result<(), String> {
        Self::set_execution_state(ES_CONTINUOUS)
    }

    fn refresh(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn requires_periodic_refresh(&self) -> bool {
        false
    }
}
