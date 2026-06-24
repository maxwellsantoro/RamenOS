//! Hello World WASM example for RamenOS
//!
//! This example demonstrates using the RamenOS SDK to call a harness
//! endpoint from a WASM module.

#![cfg_attr(all(not(test), target_arch = "wasm32"), no_std)]

#[cfg(target_arch = "wasm32")]
use ramen_sdk::generated::harness_echo_v0::EchoRequestClient;
#[cfg(target_arch = "wasm32")]
use ramen_sdk::{CapabilityHandle, Status};

/// Capability handle for echo harness (injected by runner)
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub static mut RAMEN_CAP_ECHO_REQUEST: CapabilityHandle = CapabilityHandle::INVALID;

/// WASM entry point
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn _start() {
    // Get the capability handle (safely - this is set by runner before _start)
    let cap = unsafe { RAMEN_CAP_ECHO_REQUEST };

    // Check if we have a valid capability
    if cap.is_invalid() {
        // No capability - cannot proceed
        return;
    }

    // Create client from capability
    let client = EchoRequestClient::from_cap(cap.raw());

    // Prepare request buffer
    let mut reply_buf = [0u8; 256];

    // Call the echo harness
    // request_id=1, payload_len=5 (hello), reserved=0
    let (status, _reply_len) = client.call(1, 5, 0, &mut reply_buf);

    // Status handling (in a real module, you'd do something with the result)
    match status {
        Status::Ok => {
            // Success - reply is in reply_buf
        }
        _ => {
            // Error - handle appropriately
        }
    }
}

/// Panic handler for no_std
#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
