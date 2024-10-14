use anyhow::{Context, Result};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Once;

// Modified from sketchybar-rs (https://github.com/johnallen3d/sketchybar-rs)
#[link(name = "sketchybar", kind = "static")]
extern "C" {
    fn sketchybar(message: *const c_char, bar_name: *const c_char) -> *mut c_char;
    fn free_sketchybar_response(response: *mut c_char);
    fn cleanup_sketchybar();
}

static CLEANUP: Once = Once::new();

pub struct Sketchybar {
    bar_name: CString,
}

impl Sketchybar {
    pub fn new(bar_name: Option<&str>) -> Result<Self> {
        let name = bar_name.unwrap_or("sketchybar");
        let c_string = CString::new(name).context("Failed to create CString for bar_name")?;
        Ok(Self { bar_name: c_string })
    }

    pub fn send_message(
        &self,
        flag: &str,
        event: &str,
        payload: Option<&str>,
        verbose: bool,
    ) -> Result<String> {
        let message = format!("--{} {} {}", flag, event, payload.unwrap_or_default());
        let c_message = CString::new(message).context("Failed to create CString for message")?;
        let response_ptr = unsafe { sketchybar(c_message.as_ptr(), self.bar_name.as_ptr()) };

        if response_ptr.is_null() {
            anyhow::bail!("Failed to get response from sketchybar");
        }

        let response = unsafe {
            let response = CStr::from_ptr(response_ptr)
                .to_str()
                .context("Failed to convert C string to Rust string")?
                .to_owned();
            free_sketchybar_response(response_ptr);
            response
        };

        if verbose {
            println!(
                "Successfully sent to SketchyBar: (Bar: {}): --{} {} {}",
                self.bar_name.to_str().unwrap(),
                flag,
                event,
                payload.unwrap_or_default()
            );
        }

        Ok(response)
    }
}

impl Drop for Sketchybar {
    fn drop(&mut self) {
        CLEANUP.call_once(|| unsafe {
            cleanup_sketchybar();
        });
    }
}
