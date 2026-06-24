use serde::Deserialize;

/// V-04: Unforgeable 128-bit token for display export capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayCapToken {
    pub high: u64,
    pub low: u64,
}

impl DisplayCapToken {
    /// Create a new token from high/low u64 parts.
    pub const fn new(high: u64, low: u64) -> Self {
        Self { high, low }
    }

    /// Create a zero/invalid token (test-only helper).
    #[cfg(test)]
    pub const fn invalid() -> Self {
        Self { high: 0, low: 0 }
    }

    /// Check if token is valid (non-zero, test-only helper).
    #[cfg(test)]
    pub fn is_valid(&self) -> bool {
        self.high != 0 || self.low != 0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GpuQuarantineConfig {
    pub domain_id: u64,
    pub display_cap_token_high: u64,
    pub display_cap_token_low: u64,
    pub width: u32,
    pub height: u32,
    pub gpu_profile: u32,
}

pub fn gpu_run_v1(
    config: &GpuQuarantineConfig,
    expected_token: DisplayCapToken,
) -> Result<(), String> {
    if config.domain_id == 0 {
        return Err("gpu_run_v1: domain_id must be non-zero".to_string());
    }

    // Reject all-zero expected token (configuration error)
    if expected_token.high == 0 && expected_token.low == 0 {
        return Err(
            "gpu_run_v1: expected_token must be non-zero (check plan configuration)".to_string(),
        );
    }

    let provided_token =
        DisplayCapToken::new(config.display_cap_token_high, config.display_cap_token_low);

    // Reject all-zero provided token
    if provided_token.high == 0 && provided_token.low == 0 {
        return Err("gpu_run_v1: provided display_cap_token must be non-zero".to_string());
    }

    // Token comparison
    if provided_token != expected_token {
        return Err(format!(
            "gpu_run_v1: invalid display_cap_token provided={:016x}{:016x} expected={:016x}{:016x}",
            provided_token.high, provided_token.low, expected_token.high, expected_token.low
        ));
    }
    if config.width == 0 || config.height == 0 {
        return Err("gpu_run_v1: width and height must be non-zero".to_string());
    }

    let surface_id = (config.domain_id << 32) | 1;
    println!(
        "gpu_runner_v1: start ok domain={} profile={}",
        config.domain_id, config.gpu_profile
    );
    println!(
        "gpu_runner_v1: export ok surface={} {}x{}",
        surface_id, config.width, config.height
    );
    println!("gpu_runner_v1: scanout ok frame_seq=1");
    println!("gpu_runner_v1: ok");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_token() {
        let cfg = GpuQuarantineConfig {
            domain_id: 700,
            display_cap_token_high: 0x0000_0000_0000_0000,
            display_cap_token_low: 0x0000_0000_0000_0002,
            width: 1280,
            height: 720,
            gpu_profile: 1,
        };
        let expected = DisplayCapToken::new(0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210);
        let err = gpu_run_v1(&cfg, expected).expect_err("invalid token should fail");
        assert!(err.contains("invalid display_cap_token"));
    }

    #[test]
    fn accepts_valid_token() {
        let cfg = GpuQuarantineConfig {
            domain_id: 700,
            display_cap_token_high: 0x1234_5678_9ABC_DEF0,
            display_cap_token_low: 0xFEDC_BA98_7654_3210,
            width: 1280,
            height: 720,
            gpu_profile: 1,
        };
        let expected = DisplayCapToken::new(0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210);
        gpu_run_v1(&cfg, expected).expect("valid token should succeed");
    }

    #[test]
    fn rejects_forged_token() {
        let cfg = GpuQuarantineConfig {
            domain_id: 700,
            display_cap_token_high: 0xDEAD_BEEF_CAFE_BABE,
            display_cap_token_low: 0x0BAD_F00D_1DEA_5555,
            width: 1280,
            height: 720,
            gpu_profile: 1,
        };
        let expected = DisplayCapToken::new(0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210);
        let err = gpu_run_v1(&cfg, expected).expect_err("forged token should fail");
        assert!(err.contains("invalid display_cap_token"));
    }

    #[test]
    fn token_is_valid_checks_non_zero() {
        assert!(!DisplayCapToken::invalid().is_valid());
        assert!(DisplayCapToken::new(1, 0).is_valid());
        assert!(DisplayCapToken::new(0, 1).is_valid());
    }

    fn test_config() -> GpuQuarantineConfig {
        GpuQuarantineConfig {
            domain_id: 700,
            display_cap_token_high: 1,
            display_cap_token_low: 1,
            width: 800,
            height: 600,
            gpu_profile: 1,
        }
    }

    #[test]
    fn rejects_zero_expected_token() {
        let cfg = test_config();
        let zero_expected = DisplayCapToken::new(0, 0);
        let err = gpu_run_v1(&cfg, zero_expected).expect_err("zero expected token should fail");
        assert!(err.contains("expected_token must be non-zero"));
    }

    #[test]
    fn rejects_zero_provided_token() {
        let mut cfg = test_config();
        cfg.display_cap_token_high = 0;
        cfg.display_cap_token_low = 0;
        let expected = DisplayCapToken::new(1, 1);
        let err = gpu_run_v1(&cfg, expected).expect_err("zero provided token should fail");
        assert!(err.contains("provided display_cap_token must be non-zero"));
    }

    #[test]
    fn rejects_mismatched_tokens() {
        let mut cfg = test_config();
        cfg.display_cap_token_high = 1;
        cfg.display_cap_token_low = 1;
        let expected = DisplayCapToken::new(2, 2);
        let err = gpu_run_v1(&cfg, expected).expect_err("mismatched tokens should fail");
        assert!(err.contains("invalid display_cap_token"));
    }
}
