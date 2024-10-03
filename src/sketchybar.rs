// modified from sketchybar-rs crate
use std::os::raw::c_char;
use std::{
    error::Error,
    ffi::{CStr, CString},
    fmt,
};

#[derive(Debug)]
pub enum SketchybarError {
    MessageConversionError,
    NullPointerError,
    Other(String),
}

impl fmt::Display for SketchybarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SketchybarError::MessageConversionError => {
                write!(f, "Failed to convert message to CString")
            }
            SketchybarError::Other(description) => {
                write!(f, "Sketchybar error: {description}")
            }
            SketchybarError::NullPointerError => write!(f, "Received null pointer from sketchybar"),
        }
    }
}

impl Error for SketchybarError {}

#[link(name = "sketchybar", kind = "static")]
extern "C" {
    fn sketchybar(message: *const c_char, bar_name: *const c_char) -> *mut c_char;
    fn free_sketchybar_response(response: *mut c_char);
}

/// Sends a message to `SketchyBar` and returns the response.
///
/// # Arguments
///
/// * `message` - A string slice containing the message to be sent to
///   `SketchyBar`.
/// * `bar_name` - An optional string slice containing the name of the process
///   of the target bar. This defaults to `sketchybar` however, if you're using a
///   secondary bar (eg. a `bottombar`) you can override the default there to pass
///   a message to this other bar.
///
/// # Returns
///
/// * `Ok(String)` - A `Result` containing a `String` with the response from
///   `SketchyBar` upon success.
/// * `Err(Box<dyn std::error::Error>)` - A `Result` containing an error if any
///   error occurs during the operation.
///
/// # Errors
///
/// This function will return an error if:
/// * The provided message cannot be converted to a `CString`.
/// * Any other unexpected condition occurs.
///
/// # Safety
///
/// This function contains unsafe code that calls into a C library (sketchybar).
/// Ensure the C library is correctly implemented to avoid undefined behavior.
///
/// # Examples
///
/// ```no-run
/// use sketchybar_rs::message;
///
/// fn main() {
///     let response = message("--query bar").unwrap();
///
///     println!("Response from SketchyBar: {}", response);
/// }
/// ```
fn message(message: &str, bar_name: Option<&str>) -> Result<String, SketchybarError> {
    let command = CString::new(message).map_err(|_| SketchybarError::MessageConversionError)?;
    let bar_name = CString::new(bar_name.unwrap_or("sketchybar"))
        .map_err(|_| SketchybarError::MessageConversionError)?;

    let result = unsafe {
        let result_ptr = sketchybar(command.as_ptr(), bar_name.as_ptr());

        if result_ptr.is_null() {
            return Err(SketchybarError::NullPointerError);
        }

        let result = CStr::from_ptr(result_ptr)
            .to_str()
            .map_err(|e| {
                SketchybarError::Other(format!("Failed to convert result to string: {}", e))
            })?
            .to_string();

        free_sketchybar_response(result_ptr as *mut _);

        result
    };

    Ok(result)
}

/// Sends a command to `SketchyBar` with the specified parameters.
///
/// # Parameters
/// - `flag`: The flag to send.
/// - `event`: The event to trigger.
/// - `vars`: Optional variables to include in the command.
/// - `bar`: Optional name of the `SketchyBar` instance.
/// - `verbose`: If true, print success messages.
pub fn send_to_sketchybar(
    flag: &str,
    event: &str,
    vars: Option<String>,
    bar: Option<&String>,
    verbose: bool,
) {
    let vars = vars.unwrap_or_default();
    let command = format!("--{flag} {event} {vars}");

    match message(&command, bar.map(std::string::String::as_str)) {
        Err(e) => {
            eprintln!("Failed to send to SketchyBar: {e}");
        }
        _ if verbose => {
            let msg = if let Some(bar_name) = bar {
                format!("Successfully sent to SketchyBar (Bar: {bar_name}): {command}")
            } else {
                format!("Successfully sent to SketchyBar: {command}")
            };
            println!("{msg}");
        }
        _ => (),
    }
}
