use super::controller::Backend;
use std::{
    ffi::CString,
    os::raw::{c_char, c_void},
    process::{Child, Command, Stdio},
    ptr,
};

type CFStringRef = *const c_void;
type CFStringEncoding = u32;
type IOPMAssertionLevel = u32;
type IOPMAssertionID = u32;
type IOReturn = i32;

const K_CFSTRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;
const K_IOPMASSERTION_LEVEL_ON: IOPMAssertionLevel = 255;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        assertion_level: IOPMAssertionLevel,
        assertion_name: CFStringRef,
        assertion_id: *mut IOPMAssertionID,
    ) -> IOReturn;
    fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> IOReturn;
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

pub(crate) struct PlatformBackend {
    assertion: Option<IOPMAssertionID>,
    caffeinate: Option<Child>,
}

impl PlatformBackend {
    pub(crate) fn new() -> Self {
        Self {
            assertion: None,
            caffeinate: None,
        }
    }

    fn start_caffeinate(&mut self) -> Result<(), String> {
        let child = Command::new("caffeinate")
            .arg("-d")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| format!("failed to spawn caffeinate: {err}"))?;
        self.caffeinate = Some(child);
        Ok(())
    }

    fn stop_caffeinate(&mut self) -> Result<(), String> {
        let Some(mut child) = self.caffeinate.take() else {
            return Ok(());
        };

        if child
            .try_wait()
            .map_err(|err| format!("caffeinate status check failed: {err}"))?
            .is_none()
        {
            child
                .kill()
                .map_err(|err| format!("failed to stop caffeinate: {err}"))?;
        }
        child
            .wait()
            .map(|_| ())
            .map_err(|err| format!("failed to reap caffeinate: {err}"))
    }
}

impl Backend for PlatformBackend {
    fn activate(&mut self) -> Result<(), String> {
        match create_assertion() {
            Ok(id) => {
                self.assertion = Some(id);
                Ok(())
            }
            Err(assertion_error) => self.start_caffeinate().map_err(|fallback_error| {
                format!("{assertion_error}; fallback caffeinate failed: {fallback_error}")
            }),
        }
    }

    fn deactivate(&mut self) -> Result<(), String> {
        let assertion_error = self.assertion.take().and_then(|id| {
            let result = unsafe { IOPMAssertionRelease(id) };
            (result != 0).then(|| format!("IOPMAssertionRelease returned {result}"))
        });
        let caffeinate_error = self.stop_caffeinate().err();

        match (assertion_error, caffeinate_error) {
            (None, None) => Ok(()),
            (Some(err), None) | (None, Some(err)) => Err(err),
            (Some(assertion), Some(caffeinate)) => Err(format!("{assertion}; {caffeinate}")),
        }
    }

    fn refresh(&mut self) -> Result<(), String> {
        let Some(child) = self.caffeinate.as_mut() else {
            return Ok(());
        };

        match child.try_wait() {
            Ok(None) => Ok(()),
            Ok(Some(_)) => {
                self.caffeinate = None;
                self.start_caffeinate()
            }
            Err(err) => Err(format!("caffeinate status check failed: {err}")),
        }
    }

    fn requires_periodic_refresh(&self) -> bool {
        self.caffeinate.is_some()
    }
}

fn create_assertion() -> Result<IOPMAssertionID, String> {
    let assertion_type = cfstring("NoDisplaySleepAssertion")?;
    let assertion_name = match cfstring("keepawake") {
        Ok(value) => value,
        Err(err) => {
            unsafe { CFRelease(assertion_type) };
            return Err(err);
        }
    };

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
