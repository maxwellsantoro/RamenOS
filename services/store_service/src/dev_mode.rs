//! Development-mode policy for the store service.
//!
//! Production builds are fail-closed. Development workflows may opt in via:
//! - `dev_insecure` compile-time feature: trusted-key fallback for signing tests
//! - `RAMEN_STORE_DEV_MODE` runtime env: unsigned artifacts + synthetic capabilities
//!   for local Foundry gates (S0/S1 store flows)

fn parse_env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

/// Returns true when development-only store relaxations are enabled.
///
/// Compile-time `dev_insecure` always enables dev mode. Otherwise the runtime
/// `RAMEN_STORE_DEV_MODE` flag is parsed with boolish semantics (`1/true/yes/on`).
pub fn is_dev_mode_enabled() -> bool {
    #[cfg(feature = "dev_insecure")]
    {
        return true;
    }

    #[cfg(not(feature = "dev_insecure"))]
    {
        parse_env_flag("RAMEN_STORE_DEV_MODE")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_mode_disabled_by_default() {
        std::env::remove_var("RAMEN_STORE_DEV_MODE");
        assert!(!is_dev_mode_enabled());
    }

    #[test]
    fn dev_mode_enabled_with_boolish_env() {
        std::env::set_var("RAMEN_STORE_DEV_MODE", "1");
        assert!(is_dev_mode_enabled());
        std::env::remove_var("RAMEN_STORE_DEV_MODE");
    }
}
