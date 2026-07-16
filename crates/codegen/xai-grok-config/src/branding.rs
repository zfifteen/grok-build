//! Power Grok branding adapter (behind the `powergrok` feature flag).
//!
//! This module provides the canonical source of truth for product name and
//! branding state. It respects the `POWERGROK_BRANDING=1` environment variable
//! set by the install wrapper (primary signal) and falls back to compile-time
//! feature detection. No inline `is_powergrok()` or primary `argv0` checks are
//! allowed in core TUI/CLI paths — all branding decisions must route through
//! this adapter (per BRANDING_PLAN.md Hard rule and B8).

use std::sync::OnceLock;

/// Returns the user-facing product name.
///
/// - When Power Grok branding is active: `"Power Grok"`
/// - Otherwise: falls back to the upstream name `"Grok Build"`.
pub fn product_name() -> &'static str {
    if is_powergrok_branding() {
        "Power Grok"
    } else {
        "Grok Build"
    }
}

/// Returns whether Power Grok branding is active for this process.
///
/// Primary signal: `POWERGROK_BRANDING=1` environment variable (set by wrapper).
/// Secondary: compile-time `powergrok` feature flag (for builds that always
/// want Power Grok behavior).
pub fn is_powergrok_branding() -> bool {
    static POWERGROK_BRANDING: OnceLock<bool> = OnceLock::new();

    *POWERGROK_BRANDING.get_or_init(|| {
        // Primary: env var set by install wrapper (shell-agnostic, reliable).
        if std::env::var_os("POWERGROK_BRANDING").is_some_and(|v| v == "1") {
            return true;
        }

        // Secondary: compile-time feature (for powergrok feature-enabled builds).
        cfg!(feature = "powergrok")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_product_name_fallback() {
        // When neither env var nor feature is active, we must fall back to upstream name.
        // This test is skipped under the powergrok feature (the other test covers it).
        #[cfg(not(feature = "powergrok"))]
        {
            let name = product_name();
            assert_eq!(name, "Grok Build");
        }
        #[cfg(feature = "powergrok")]
        {
            // Under powergrok feature the fallback test is superseded by test_product_name_with_feature_flag.
        }
    }

    #[test]
    #[serial] // Env var test must not run concurrently with other tests that set it.
    fn test_product_name_with_env_var() {
        // Primary signal from install wrapper.
        unsafe {
            std::env::set_var("POWERGROK_BRANDING", "1");
        }
        let name = product_name();
        unsafe {
            std::env::remove_var("POWERGROK_BRANDING");
        }
        assert_eq!(name, "Power Grok");
    }

    #[test]
    #[cfg(feature = "powergrok")]
    fn test_product_name_with_feature_flag() {
        // Secondary compile-time path (used by powergrok builds).
        let name = product_name();
        assert_eq!(name, "Power Grok");
    }
}
