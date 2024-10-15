use anyhow::{Context, Result};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Once;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

// Modified from sketchybar-rs (https://github.com/johnallen3d/sketchybar-rs)
#[link(name = "sketchybar", kind = "static")]
extern "C" {
    fn sketchybar(message: *const c_char, bar_name: *const c_char) -> *mut c_char;
    fn free_sketchybar_response(response: *mut c_char);
    fn cleanup_sketchybar();
    fn refresh_sketchybar_port(bar_name: *const c_char) -> bool;
}

static CLEANUP: Once = Once::new();

struct PortState {
    last_refresh: Instant,
    refresh_interval: Duration,
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
                refresh_interval: Duration::from_secs(300), // 5 minutes
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
