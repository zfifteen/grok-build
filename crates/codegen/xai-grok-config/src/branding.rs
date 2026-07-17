//! Power Grok branding adapter.
//!
//! # Model A (Hermes PR #5)
//! When the `powergrok` Cargo feature is enabled, branding is always active:
//! `product_name()` returns `"Power Grok"` and `is_powergrok_branding()` is
//! `true`. This matches the product build used by the Power Grok wrapper and CI.
//!
//! Without the feature, the upstream name `"Grok Build"` is used unless
//! `POWERGROK_BRANDING=1` is set (test / experiment override only).
//!
//! Call sites must use this adapter (or `xai_grok_config::product_name`) rather
//! than scattering `cfg!(feature = "powergrok")` forks. The module is always
//! compiled so non-feature builds stay green.

/// Returns the user-facing product name.
///
/// - Power Grok branding active: `"Power Grok"`
/// - Otherwise: upstream `"Grok Build"`
pub fn product_name() -> &'static str {
    if is_powergrok_branding() {
        "Power Grok"
    } else {
        "Grok Build"
    }
}

/// Returns whether Power Grok branding is active for this process.
///
/// Model A: the `powergrok` feature always enables branding. Without the
/// feature, `POWERGROK_BRANDING=1` can force branding for tests/experiments.
pub fn is_powergrok_branding() -> bool {
    // Product builds always brand (compile-time).
    if cfg!(feature = "powergrok") {
        return true;
    }
    // Non-feature: optional env override only.
    std::env::var_os("POWERGROK_BRANDING").is_some_and(|v| v == "1")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "powergrok"))]
    use serial_test::serial;

    #[test]
    #[cfg(not(feature = "powergrok"))]
    #[serial]
    fn test_product_name_fallback() {
        // Ensure env override is off for this assertion.
        unsafe {
            std::env::remove_var("POWERGROK_BRANDING");
        }
        assert_eq!(product_name(), "Grok Build");
        assert!(!is_powergrok_branding());
    }

    #[test]
    #[cfg(not(feature = "powergrok"))]
    #[serial]
    fn test_product_name_with_env_var() {
        unsafe {
            std::env::set_var("POWERGROK_BRANDING", "1");
        }
        let name = product_name();
        let active = is_powergrok_branding();
        unsafe {
            std::env::remove_var("POWERGROK_BRANDING");
        }
        assert!(active);
        assert_eq!(name, "Power Grok");
    }

    #[test]
    #[cfg(feature = "powergrok")]
    fn test_product_name_with_feature_flag() {
        assert!(is_powergrok_branding());
        assert_eq!(product_name(), "Power Grok");
    }
}
