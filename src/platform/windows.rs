use windows::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, EXECUTION_STATE,
    SetThreadExecutionState,
};

pub fn keep_awake() -> Result<(), String> {
    let flags = ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED;
    let result = unsafe { SetThreadExecutionState(flags) };

    if result == EXECUTION_STATE(0) {
        Err("SetThreadExecutionState failed".to_string())
    } else {
        Ok(())
    }
}
