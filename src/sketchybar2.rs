use std::ffi::{c_void, CString};
use std::mem;
use std::slice;
use std::str;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use mach2::bootstrap::bootstrap_look_up;
use mach2::kern_return::KERN_SUCCESS;
use mach2::mach_port::{mach_port_allocate, mach_port_deallocate, mach_port_insert_right};
use mach2::message::{
    mach_msg, mach_msg_destroy, mach_msg_header_t, mach_msg_ool_descriptor_t, mach_msg_size_t,
    mach_msg_trailer_t, MACH_MSGH_BITS_COMPLEX, MACH_MSGH_BITS_PORTS_MASK, MACH_MSG_OOL_DESCRIPTOR,
    MACH_MSG_SUCCESS, MACH_MSG_TIMEOUT_NONE, MACH_MSG_TYPE_COPY_SEND, MACH_MSG_TYPE_MAKE_SEND,
    MACH_MSG_VIRTUAL_COPY, MACH_RCV_MSG, MACH_RCV_TIMEOUT, MACH_SEND_MSG,
};
use mach2::port::{mach_port_t, MACH_PORT_NULL, MACH_PORT_RIGHT_RECEIVE};
use mach2::task::{task_get_special_port, TASK_BOOTSTRAP_PORT};
use mach2::traps::mach_task_self;

const SERVICE_NAME_PREFIX: &str = "git.felix.";
static G_MACH_PORT: AtomicU32 = AtomicU32::new(MACH_PORT_NULL);

#[repr(C)]
struct MachMessage {
    header: mach_msg_header_t,
    msgh_descriptor_count: mach_msg_size_t,
    descriptor: mach_msg_ool_descriptor_t,
}

#[repr(C)]
struct MachBuffer {
    message: MachMessage,
    trailer: mach_msg_trailer_t,
}

#[macro_export]
macro_rules! kern_try {
    ($expr:expr) => {
        match $expr {
            KERN_SUCCESS => Ok(()),
            err_code => Err(anyhow!("Mach error: {}", err_code)),
        }
    };
}

fn deallocate_mach_port(port: mach_port_t) {
    unsafe {
        mach_port_deallocate(mach_task_self(), port);
    }
}

fn mach_msgh_bits_set(remote: u32, local: u32, voucher: u32, other: u32) -> u32 {
    let remote_bits = remote & 0x0000001f; // Mask for remote
    let local_bits = (local << 8) & 0x00001f00; // Shift and mask for local
    let voucher_bits = (voucher << 16) & 0x001f0000; // Shift and mask for voucher
    let other_bits = other & !MACH_MSGH_BITS_PORTS_MASK; // Mask other to clear port bits

    remote_bits | local_bits | voucher_bits | other_bits | MACH_MSGH_BITS_COMPLEX
}

fn mach_get_bs_port(bar_name: &str) -> Result<mach_port_t> {
    let task = unsafe { mach_task_self() };
    let mut bs_port: mach_port_t = MACH_PORT_NULL;

    kern_try!(unsafe { task_get_special_port(task, TASK_BOOTSTRAP_PORT, &mut bs_port) })
        .context("Failed to get bootstrap port")?;

    let service_name = CString::new(format!("{}{}", SERVICE_NAME_PREFIX, bar_name))?;

    let mut port: mach_port_t = MACH_PORT_NULL;

    kern_try!(unsafe { bootstrap_look_up(bs_port, service_name.as_ptr(), &mut port) })
        .inspect_err(|_| deallocate_mach_port(bs_port))
        .context("Failed to look up bootstrap port")?;

    deallocate_mach_port(bs_port);

    Ok(port)
}

fn mach_receive_message(port: mach_port_t, timeout: bool) -> Result<MachBuffer> {
    let mut buffer: MachBuffer = unsafe { mem::zeroed() };

    let msg_option = if timeout {
        MACH_RCV_MSG | MACH_RCV_TIMEOUT
    } else {
        MACH_RCV_MSG
    };

    let timeout_ms = if timeout { 100 } else { MACH_MSG_TIMEOUT_NONE };

    let msg_return = unsafe {
        mach_msg(
            &mut buffer.message.header,
            msg_option,
            0,
            mem::size_of::<MachBuffer>() as mach_msg_size_t,
            port,
            timeout_ms,
            MACH_PORT_NULL,
        )
    };

    if msg_return != MACH_MSG_SUCCESS {
        eprintln!("Error receiving message: {}", msg_return);
        buffer.message.descriptor.address = std::ptr::null_mut();
    }

    Ok(buffer)
}

fn mach_send_message(port: mach_port_t, message: &[u8]) -> Result<String> {
    if message.is_empty() || port == MACH_PORT_NULL {
        return Err(anyhow!("Null message or port"));
    }

    let task = unsafe { mach_task_self() };
    let mut response_port = MACH_PORT_NULL;

    kern_try!(unsafe { mach_port_allocate(task, MACH_PORT_RIGHT_RECEIVE, &mut response_port) })
        .context("Failed to allocate port")?;

    kern_try!(unsafe {
        mach_port_insert_right(task, response_port, response_port, MACH_MSG_TYPE_MAKE_SEND)
    })
    .context("Failed to insert right")?;

    let mut msg: MachMessage = unsafe { mem::zeroed() };
    msg.header.msgh_remote_port = port;
    msg.header.msgh_local_port = response_port;
    msg.header.msgh_id = response_port as i32;
    msg.header.msgh_bits = mach_msgh_bits_set(
        MACH_MSG_TYPE_COPY_SEND,
        MACH_MSG_TYPE_MAKE_SEND,
        0,
        MACH_MSGH_BITS_COMPLEX,
    );
    msg.header.msgh_size = mem::size_of::<MachMessage>() as u32;
    msg.msgh_descriptor_count = 1;
    msg.descriptor.address = message.as_ptr() as *mut c_void;
    msg.descriptor.size = (message.len() + 1) as u32;
    msg.descriptor.copy = MACH_MSG_VIRTUAL_COPY as u8;
    msg.descriptor.deallocate = 0;
    msg.descriptor.type_ = MACH_MSG_OOL_DESCRIPTOR as u8;

    kern_try!(unsafe {
        mach_msg(
            &mut msg.header,
            MACH_SEND_MSG,
            mem::size_of::<MachMessage>() as mach_msg_size_t,
            0,
            MACH_PORT_NULL,
            MACH_MSG_TIMEOUT_NONE,
            MACH_PORT_NULL,
        )
    })
    .inspect_err(|_| deallocate_mach_port(response_port))
    .context("Failed to send message")?;

    let receive_result = mach_receive_message(response_port, true);

    match receive_result {
        Ok(mut buffer) => {
            let result = if !buffer.message.descriptor.address.is_null() {
                let slice = unsafe {
                    slice::from_raw_parts(
                        buffer.message.descriptor.address as *const u8,
                        buffer.message.descriptor.size as usize,
                    )
                };
                String::from_utf8_lossy(slice).into_owned()
            } else {
                String::new()
            };

            unsafe {
                mach_msg_destroy(&mut buffer.message.header);
            }

            deallocate_mach_port(response_port);

            Ok(result)
        }
        Err(e) => {
            deallocate_mach_port(response_port);
            Err(e).context("Failed to receive response")
        }
    }
}

//TODO: Replace spaces with null bytes and strip quotes, then update mach_send_message to use
//CString from vec with null bytes

//fn format_message(message: &str) -> Vec<u8> {
//    let mut formatted_message = Vec::new();
//    let mut in_quotes = false;
//    let mut current_arg = Vec::new();
//
//    for c in message.chars() {
//        match c {
//            '"' | '\'' => {
//                in_quotes = !in_quotes;
//                current_arg.push(c as u8);
//            }
//            ' ' if !in_quotes => {
//                if !current_arg.is_empty() {
//                    current_arg.push(0); // Null-terminate the argument
//                    formatted_message.extend_from_slice(&current_arg);
//                    current_arg.clear();
//                }
//            }
//            _ => {
//                current_arg.push(c as u8);
//            }
//        }
//    }
//
//    if !current_arg.is_empty() {
//        current_arg.push(0); // Null-terminate the last argument
//        formatted_message.extend_from_slice(&current_arg);
//    }
//
//    formatted_message
//}

fn format_message(message: &str) -> Vec<u8> {
    let mut formatted_message = Vec::new();
    let mut quote: Option<char> = None; // Tracks if we're inside quotes

    let message_bytes = message.as_bytes();

    for &c in message_bytes {
        let current_char = c as char;

        // Toggle quote mode if we encounter a quote character
        if current_char == '"' || current_char == '\'' {
            if quote == Some(current_char) {
                quote = None; // Close the quote
            } else {
                quote = Some(current_char); // Open a quote
            }
            continue; // Skip adding the quote to the output
        }

        // Add the current character to the formatted message
        formatted_message.push(c);

        // Replace spaces with null bytes if not inside quotes
        if current_char == ' ' && quote.is_none() {
            formatted_message.pop(); // Remove the space
            formatted_message.push(0); // Replace with null byte
        }
    }

    // If there's a trailing space followed by a null byte, remove the extra null
    if formatted_message.last() == Some(&0) && formatted_message.len() > 1 {
        formatted_message.pop();
    }

    // Ensure the message ends with a null byte, but only if it's not there already
    if formatted_message.is_empty() || formatted_message.last() != Some(&0) {
        formatted_message.push(0); // Add a null byte at the end if not present
    }

    formatted_message
}

pub fn send_to_sketchybar(
    flag: &str,
    event: &str,
    vars: Option<String>,
    bar_name: Option<&String>,
    verbose: bool,
) -> Result<String> {
    let message = format!("--{} {} {}", flag, event, vars.unwrap_or_default());
    let binding = "sketchybar".to_string();
    let bar_name = bar_name.unwrap_or(&binding);

    let formatted_message = format_message(&message);

    if verbose {
        println!("Sending message: {}", message);
        println!("Formatted message: {:?}", formatted_message);
        println!(
            "Formatted message as string: {:?}",
            String::from_utf8_lossy(&formatted_message)
        );
    }

    for attempt in 1..=3 {
        if verbose {
            println!("Attempt {}/3", attempt);
        }

        let port = G_MACH_PORT.load(Ordering::SeqCst);
        let port = if port == MACH_PORT_NULL {
            if verbose {
                println!("Getting new bs_port for {}", bar_name);
            }
            let new_port = mach_get_bs_port(bar_name)?;
            G_MACH_PORT.store(new_port, Ordering::SeqCst);
            new_port
        } else {
            port
        };

        if verbose {
            println!("Using port: {:?}", port);
        }

        match mach_send_message(port, &formatted_message) {
            Ok(response) => {
                if verbose {
                    println!(
                        "Successfully sent message to SketchyBar (Bar: {}): {}",
                        bar_name, message
                    );
                }
                return Ok(response);
            }
            Err(e) => {
                eprintln!(
                    "Failed to send message to SketchyBar (Bar: {}): {} (Port: {:?}), Error: {}",
                    bar_name, message, port, e
                );

                if attempt < 3 {
                    if verbose {
                        println!("Retrying with new port (Attempt {}/3)", attempt + 1);
                    }
                    let new_port = mach_get_bs_port(bar_name)?;
                    println!("New port: {:?}", new_port);
                    G_MACH_PORT.store(new_port, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(100 * attempt as u64));
                }
            }
        }
    }

    Err(anyhow!(
        "Max retries reached. Failed to send message to SketchyBar."
    ))
}
