use mach2::bootstrap::{bootstrap_look_up, bootstrap_register};
use mach2::kern_return::KERN_SUCCESS;
use mach2::mach_port::{mach_port_allocate, mach_port_deallocate, mach_port_insert_right};
use mach2::message::{
    mach_msg, mach_msg_header_t, mach_msg_ool_descriptor_t, mach_msg_trailer_t, MACH_MSG_SUCCESS,
    MACH_MSG_TIMEOUT_NONE, MACH_MSG_TYPE_MAKE_SEND, MACH_RCV_MSG, MACH_SEND_MSG,
};
use mach2::port::{mach_port_name_t, MACH_PORT_NULL, MACH_PORT_RIGHT_RECEIVE};
use mach2::task::{task_get_special_port, TASK_BOOTSTRAP_PORT};
use mach2::traps::mach_task_self;
use std::ffi::CString;
use std::sync::{Arc, Mutex};
use tokio::task;

const SERVICE_NAME_PREFIX: &str = "git.felix.";

pub struct MachBuffer {
    header: mach_msg_header_t,
    descriptor: mach_msg_ool_descriptor_t,
    trailer: mach_msg_trailer_t,
}

#[derive(Clone)]
pub struct MachServer {
    is_running: Arc<Mutex<bool>>,
    task: mach_port_name_t,
    port: Arc<Mutex<mach_port_name_t>>,
    bs_port: Arc<Mutex<mach_port_name_t>>,
    handler: Arc<dyn Fn(&str) + Send + Sync>,
}

impl MachServer {
    pub fn new(handler: Arc<dyn Fn(&str) + Send + Sync>) -> Self {
        Self {
            is_running: Arc::new(Mutex::new(false)),
            task: unsafe { mach_task_self() },
            port: Arc::new(Mutex::new(MACH_PORT_NULL)),
            bs_port: Arc::new(Mutex::new(MACH_PORT_NULL)),
            handler,
        }
    }

    pub async fn start(self: Arc<Self>, bootstrap_name: &str) -> Result<(), &'static str> {
        // Allocate a port for receiving messages
        let mut port = self.port.lock().unwrap();
        if unsafe { mach_port_allocate(self.task, MACH_PORT_RIGHT_RECEIVE, &mut *port) }
            != KERN_SUCCESS
        {
            return Err("Failed to allocate port");
        }

        // Insert the port into the task
        if unsafe { mach_port_insert_right(self.task, *port, *port, MACH_MSG_TYPE_MAKE_SEND) }
            != KERN_SUCCESS
        {
            return Err("Failed to insert port right");
        }

        // Get the special port
        let mut bs_port = self.bs_port.lock().unwrap();
        if unsafe { task_get_special_port(self.task, TASK_BOOTSTRAP_PORT, &mut *bs_port) }
            != KERN_SUCCESS
        {
            return Err("Failed to get special port");
        }

        // Register the bootstrap name
        let c_bootstrap_name =
            CString::new(bootstrap_name).map_err(|_| "Invalid bootstrap name")?;
        if unsafe { bootstrap_register(*bs_port, c_bootstrap_name.as_ptr() as *mut i8, *port) }
            != KERN_SUCCESS
        {
            return Err("Failed to register bootstrap");
        }

        // Mark server as running
        *self.is_running.lock().unwrap() = true;

        // Spawn an async task to handle incoming messages
        let self_clone = Arc::clone(&self);
        task::spawn(async move {
            Self::message_loop(self_clone).await;
        });

        Ok(())
    }

    async fn message_loop(self: Arc<Self>) {
        let mut buffer = MachBuffer {
            header: mach_msg_header_t::default(),
            descriptor: mach_msg_ool_descriptor_t {
                address: std::ptr::null_mut(),
                size: 0,
                deallocate: 0,
                copy: 0,
                pad1: 0,
                type_: 0,
            },
            trailer: mach_msg_trailer_t::default(),
        };

        while *self.is_running.lock().unwrap() {
            let port = *self.port.lock().unwrap();
            let msg_return = unsafe {
                mach_msg(
                    &mut buffer.header,
                    MACH_RCV_MSG,
                    0,
                    std::mem::size_of::<MachBuffer>() as u32,
                    port,
                    MACH_MSG_TIMEOUT_NONE,
                    MACH_PORT_NULL,
                )
            };

            if msg_return == MACH_MSG_SUCCESS {
                let message_ptr = buffer.descriptor.address as *mut i8;
                if !message_ptr.is_null() {
                    let message = unsafe { CString::from_raw(message_ptr) };
                    if let Ok(msg_str) = message.to_str() {
                        (self.handler)(msg_str);
                    }
                }
            }
        }
    }

    pub fn stop(&self) {
        *self.is_running.lock().unwrap() = false;
    }
}

pub fn mach_get_bs_port(bar_name: &str) -> Result<mach_port_name_t, &'static str> {
    let task = unsafe { mach_task_self() };
    let mut bs_port: mach_port_name_t = MACH_PORT_NULL;

    if unsafe { task_get_special_port(task, TASK_BOOTSTRAP_PORT, &mut bs_port) } != KERN_SUCCESS {
        return Err("Failed to get bootstrap port");
    }

    let service_name = format!("{}{}", SERVICE_NAME_PREFIX, bar_name);
    let c_service_name = CString::new(service_name).map_err(|_| "Invalid service name")?;
    let mut port: mach_port_name_t = MACH_PORT_NULL;

    let result = unsafe { bootstrap_look_up(bs_port, c_service_name.as_ptr(), &mut port) };
    unsafe { mach_port_deallocate(task, bs_port) };

    if result != KERN_SUCCESS {
        Err("Failed to look up bootstrap port")
    } else {
        Ok(port)
    }
}

pub fn mach_send_message(port: mach_port_name_t, message: &str) -> Result<String, &'static str> {
    let task = unsafe { mach_task_self() };
    let mut response_port: mach_port_name_t = MACH_PORT_NULL;

    if unsafe { mach_port_allocate(task, MACH_PORT_RIGHT_RECEIVE, &mut response_port) }
        != KERN_SUCCESS
    {
        return Err("Failed to allocate response port");
    }

    if unsafe {
        mach_port_insert_right(task, response_port, response_port, MACH_MSG_TYPE_MAKE_SEND)
    } != KERN_SUCCESS
    {
        unsafe { mach_port_deallocate(task, response_port) };
        return Err("Failed to insert response port right");
    }

    let c_message = CString::new(message).map_err(|_| "Invalid message")?;
    let mut msg_header = mach_msg_header_t {
        msgh_remote_port: port,
        msgh_local_port: response_port,
        msgh_id: 0,
        msgh_size: (std::mem::size_of::<mach_msg_header_t>() + c_message.as_bytes().len()) as u32,
        ..Default::default()
    };

    let send_result = unsafe {
        mach_msg(
            &mut msg_header,
            MACH_SEND_MSG,
            msg_header.msgh_size,
            0,
            MACH_PORT_NULL,
            MACH_MSG_TIMEOUT_NONE,
            MACH_PORT_NULL,
        )
    };

    if send_result != MACH_MSG_SUCCESS {
        unsafe { mach_port_deallocate(task, response_port) };
        return Err("Failed to send message");
    }

    let mut buffer = MachBuffer {
        header: mach_msg_header_t::default(),
        descriptor: mach_msg_ool_descriptor_t {
            address: c_message.as_ptr() as *mut _,
            size: c_message.as_bytes().len() as u32,
            deallocate: 0,
            copy: 0,
            pad1: 0,
            type_: 0,
        },
        trailer: mach_msg_trailer_t::default(),
    };

    let msg_return = unsafe {
        mach_msg(
            &mut buffer.header,
            MACH_RCV_MSG,
            0,
            std::mem::size_of::<MachBuffer>() as u32,
            response_port,
            MACH_MSG_TIMEOUT_NONE,
            MACH_PORT_NULL,
        )
    };

    unsafe { mach_port_deallocate(task, response_port) };

    if msg_return != MACH_MSG_SUCCESS {
        return Err("Failed to receive response");
    }

    let response = unsafe {
        let response_str = std::slice::from_raw_parts(
            buffer.descriptor.address as *const u8,
            buffer.descriptor.size as usize,
        );
        String::from_utf8_lossy(response_str).to_string()
    };

    Ok(response)
}

pub fn sketchybar(message: &str, bar_name: &str) -> Result<String, &'static str> {
    let g_mach_port = mach_get_bs_port(bar_name)?;
    mach_send_message(g_mach_port, message)
}
