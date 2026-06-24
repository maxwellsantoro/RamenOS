//! Instance context for WASM execution.

use crate::kernel_bridge::KernelBridgeOps;

/// Context passed to WASM host functions.
///
/// This is the `Caller<T>` data for all host function invocations.
/// It provides access to the kernel bridge and captures stdout.
pub struct InstanceContext {
    /// Kernel bridge for IPC operations.
    pub kernel_bridge: Box<dyn KernelBridgeOps>,

    /// Captured stdout from WASM execution.
    pub stdout: Vec<u8>,
}

impl InstanceContext {
    /// Create a new context with the given kernel bridge.
    pub fn new(kernel_bridge: Box<dyn KernelBridgeOps>) -> Self {
        Self {
            kernel_bridge,
            stdout: Vec::new(),
        }
    }

    /// Create a context with mock kernel bridge for testing.
    pub fn with_mock() -> Self {
        Self::new(Box::new(crate::kernel_bridge::MockKernelBridge::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_bridge::MockKernelBridge;

    #[test]
    fn instance_context_holds_memory_and_bridge() {
        let bridge = MockKernelBridge::new();

        // Context can be created with bridge
        let _ctx = InstanceContext {
            kernel_bridge: Box::new(bridge),
            stdout: Vec::new(),
        };
    }
}
