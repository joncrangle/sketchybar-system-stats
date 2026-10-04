use anyhow::{Context, Result};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

// Modified from sketchybar-rs (https://github.com/johnallen3d/sketchybar-rs)
const PORT_REFRESH_INTERVAL_SECS: u64 = 300;

#[link(name = "sketchybar", kind = "static")]
unsafe extern "C" {
    fn sketchybar_send(
        message: *const c_char,
        bar_name: *const c_char,
        response: *mut *mut c_char,
    ) -> c_int;
    fn cleanup_sketchybar();
    fn refresh_sketchybar_port(bar_name: *const c_char) -> bool;
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeliveryStatus {
    Acknowledged,
    SentWithoutAcknowledgment,
}

struct PortState {
    last_refresh: Instant,
    refresh_interval: Duration,
}

fn format_sketchybar_message(flag: &str, event: &str, payload: Option<&str>) -> String {
    let operation = if flag == "trigger" {
        format!("--add event {event} --trigger {event}")
    } else {
        format!("--{flag} {event}")
    };
    match payload.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => format!("{operation} {p}"),
        None => operation,
    }
}

pub(crate) fn escape_quoted_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character == '\\' || character == '"' {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
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
            let bar_name = self.bar_name.clone();
            let refreshed = tokio::task::spawn_blocking(move || unsafe {
                refresh_sketchybar_port(bar_name.as_ptr())
            })
            .await
            .context("SketchyBar port refresh task failed")?;
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
    ) -> Result<DeliveryStatus> {
        self.maybe_refresh_port().await?;

        let message = format_sketchybar_message(flag, event, payload);
        let c_message =
            CString::new(message.as_str()).context("Failed to create CString for message")?;
        let bar_name = self.bar_name.clone();

        tokio::task::spawn_blocking(move || {
            // The C enum uses 0 for failed delivery, 1 for an acknowledgment,
            // and 2 for delivery without a reply. No response copy is needed.
            let status = match unsafe {
                sketchybar_send(c_message.as_ptr(), bar_name.as_ptr(), std::ptr::null_mut())
            } {
                0 => anyhow::bail!("Failed to deliver command to SketchyBar"),
                1 => DeliveryStatus::Acknowledged,
                2 => DeliveryStatus::SentWithoutAcknowledgment,
                other => anyhow::bail!("Unexpected SketchyBar delivery status: {other}"),
            };

            if verbose {
                let detail = match status {
                    DeliveryStatus::Acknowledged => "acknowledged",
                    DeliveryStatus::SentWithoutAcknowledgment => "no acknowledgment received",
                };
                println!(
                    "Sent to SketchyBar ({detail}): (Bar: {}): {}",
                    bar_name.to_str().unwrap_or("?"),
                    message
                );
            }

            Ok(status)
        })
        .await
        .context("SketchyBar IPC task failed")?
    }
}

impl Drop for Sketchybar {
    fn drop(&mut self) {
        // The C cleanup is mutex-protected and idempotent. Run it for every
        // instance so a later instance can also release a port it reacquired.
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
            "--add event system_stats --trigger system_stats CPU_USAGE=\"5%\""
        );
    }

    #[test]
    fn test_format_sketchybar_message_registers_event_before_trigger() {
        assert_eq!(
            format_sketchybar_message("trigger", "system_stats", None),
            "--add event system_stats --trigger system_stats"
        );
    }

    #[test]
    fn test_escape_quoted_value_escapes_quotes_and_backslashes() {
        assert_eq!(
            escape_quoted_value(r#"Alice's \"Mac\""#),
            r#"Alice's \\\"Mac\\\""#
        );
    }
}
