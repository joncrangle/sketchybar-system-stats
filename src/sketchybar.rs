use anyhow::{Context, Result};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr::NonNull;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

// Modified from sketchybar-rs (https://github.com/johnallen3d/sketchybar-rs)
const PORT_REFRESH_INTERVAL_SECS: u64 = 300;

#[link(name = "sketchybar", kind = "static")]
unsafe extern "C" {
    fn sketchybar(message: *const c_char, bar_name: *const c_char) -> *mut c_char;
    fn free_sketchybar_response(response: *mut c_char);
    fn cleanup_sketchybar();
    fn refresh_sketchybar_port(bar_name: *const c_char) -> bool;
}

struct SketchybarResponse {
    ptr: NonNull<c_char>,
}

impl SketchybarResponse {
    fn new(ptr: *mut c_char) -> Result<Self> {
        let ptr = NonNull::new(ptr).context("Failed to get response from sketchybar")?;
        Ok(Self { ptr })
    }

    unsafe fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.ptr.as_ptr()) }
    }
}

impl Drop for SketchybarResponse {
    fn drop(&mut self) {
        unsafe {
            free_sketchybar_response(self.ptr.as_ptr());
        }
    }
}

struct PortState {
    last_refresh: Instant,
    refresh_interval: Duration,
}

fn format_sketchybar_message(flag: &str, event: &str, payload: Option<&str>) -> String {
    match payload.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => format!("--{flag} {event} {p}"),
        None => format!("--{flag} {event}"),
    }
}

pub struct Sketchybar {
    bar_name: CString,
    port_state: Mutex<PortState>,
}

impl Sketchybar {
    pub fn new(bar_name: Option<&str>) -> Result<Self> {
        let name = bar_name.unwrap_or("sketchybar");
        let c_string = CString::new(name).context("Failed to create CString for bar_name")?;
        Ok(Self {
            bar_name: c_string,
            port_state: Mutex::new(PortState {
                last_refresh: Instant::now(),
                refresh_interval: Duration::from_secs(PORT_REFRESH_INTERVAL_SECS),
            }),
        })
    }

    async fn maybe_refresh_port(&self) -> Result<()> {
        let mut state = self.port_state.lock().await;
        if state.last_refresh.elapsed() >= state.refresh_interval {
            let refreshed = unsafe { refresh_sketchybar_port(self.bar_name.as_ptr()) };
            if !refreshed {
                anyhow::bail!("Failed to refresh sketchybar port");
            }
            state.last_refresh = Instant::now();
        }
        Ok(())
    }

    pub async fn send_message(
        &self,
        flag: &str,
        event: &str,
        payload: Option<&str>,
        verbose: bool,
    ) -> Result<String> {
        self.maybe_refresh_port().await?;

        let message = format_sketchybar_message(flag, event, payload);
        let c_message =
            CString::new(message.as_str()).context("Failed to create CString for message")?;

        let response = SketchybarResponse::new(unsafe {
            sketchybar(c_message.as_ptr(), self.bar_name.as_ptr())
        })?;

        let response = unsafe {
            response
                .as_c_str()
                .to_str()
                .context("Failed to convert C string to Rust string")?
                .to_owned()
        };

        if verbose {
            println!(
                "Successfully sent to SketchyBar: (Bar: {}): {}",
                self.bar_name.to_str().unwrap_or("?"),
                message
            );
        }

        Ok(response)
    }
}

impl Drop for Sketchybar {
    fn drop(&mut self) {
        unsafe {
            cleanup_sketchybar();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sketchybar_message_none_payload() {
        assert_eq!(
            format_sketchybar_message("add event", "system_stats", None),
            "--add event system_stats"
        );
    }

    #[test]
    fn test_format_sketchybar_message_empty_payload() {
        assert_eq!(
            format_sketchybar_message("add event", "system_stats", Some("")),
            "--add event system_stats"
        );
        assert_eq!(
            format_sketchybar_message("add event", "system_stats", Some("   ")),
            "--add event system_stats"
        );
    }

    #[test]
    fn test_format_sketchybar_message_with_payload_trims_trailing_space() {
        assert_eq!(
            format_sketchybar_message("trigger", "system_stats", Some("CPU_USAGE=\"5%\" ")),
            "--trigger system_stats CPU_USAGE=\"5%\""
        );
    }
}
