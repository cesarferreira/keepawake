use std::{
    ffi::CString,
    os::raw::{c_char, c_void},
    process::{Child, Command, Stdio},
    ptr,
    sync::{Mutex, OnceLock},
};

type CFStringRef = *const c_void;
type CFStringEncoding = u32;
type IOPMAssertionLevel = u32;
type IOPMAssertionID = u32;
type IOReturn = i32;

const K_CFSTRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;
const K_IOPMASSERTION_LEVEL_ON: IOPMAssertionLevel = 255;

static ASSERTION_SLOT: OnceLock<Mutex<Option<IOPMAssertionID>>> = OnceLock::new();
static CAFFEINATE_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        assertion_level: IOPMAssertionLevel,
        assertion_name: CFStringRef,
        assertion_id: *mut IOPMAssertionID,
    ) -> IOReturn;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const c_char,
        encoding: CFStringEncoding,
    ) -> CFStringRef;
    fn CFRelease(cf: CFStringRef);
}

fn assertion_storage() -> &'static Mutex<Option<IOPMAssertionID>> {
    ASSERTION_SLOT.get_or_init(|| Mutex::new(None))
}

fn caffeinate_storage() -> &'static Mutex<Option<Child>> {
    CAFFEINATE_CHILD.get_or_init(|| Mutex::new(None))
}

pub fn keep_awake() -> Result<(), String> {
    {
        let mut guard = assertion_storage()
            .lock()
            .map_err(|_| "failed to lock assertion storage".to_string())?;

        if guard.is_none() {
            match create_assertion() {
                Ok(id) => {
                    *guard = Some(id);
                    return Ok(());
                }
                Err(err) => {
                    drop(guard);
                    return ensure_caffeinate_running().map_err(|fallback| {
                        format!("{err}; fallback caffeinate failed: {fallback}")
                    });
                }
            }
        }
    }

    Ok(())
}

fn create_assertion() -> Result<IOPMAssertionID, String> {
    let assertion_type = cfstring("NoDisplaySleepAssertion")?;
    let assertion_name = cfstring("keepawake")?;

    let mut id: IOPMAssertionID = 0;
    let result = unsafe {
        IOPMAssertionCreateWithName(
            assertion_type,
            K_IOPMASSERTION_LEVEL_ON,
            assertion_name,
            &mut id,
        )
    };

    unsafe {
        CFRelease(assertion_type);
        CFRelease(assertion_name);
    }

    if result == 0 {
        Ok(id)
    } else {
        Err(format!("IOPMAssertionCreateWithName returned {result}"))
    }
}

fn ensure_caffeinate_running() -> Result<(), String> {
    let mut slot = caffeinate_storage()
        .lock()
        .map_err(|_| "failed to lock caffeinate storage".to_string())?;

    let mut should_spawn = true;
    if let Some(child) = slot.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                *slot = None;
            }
            Ok(None) => {
                should_spawn = false;
            }
            Err(err) => {
                return Err(format!("caffeinate status check failed: {err}"));
            }
        }
    }

    if should_spawn {
        let child = Command::new("caffeinate")
            .args(["-du", "-t", "60"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| format!("failed to spawn caffeinate: {err}"))?;
        *slot = Some(child);
    }

    Ok(())
}

fn cfstring(value: &str) -> Result<CFStringRef, String> {
    let cstring =
        CString::new(value).map_err(|_| "value contained interior null byte".to_string())?;
    let cfstr = unsafe {
        CFStringCreateWithCString(ptr::null(), cstring.as_ptr(), K_CFSTRING_ENCODING_UTF8)
    };
    if cfstr.is_null() {
        Err("CFStringCreateWithCString returned null".to_string())
    } else {
        Ok(cfstr)
    }
}
