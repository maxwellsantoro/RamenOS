//! Shared model-facing syntax/serializer. This library mints no service authority.
pub mod protocol;
#[cfg(feature = "agent_task_v1_dev")]
pub mod rt;

pub mod stdio;
