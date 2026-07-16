//! Power Grok branding adapter (behind the `powergrok` feature flag).
//!
//! When the `powergrok` feature is enabled, `product_name()` returns "Power Grok"
//! and `is_powergrok_branding()` returns `true`. This matches the product build
//! used by the Power Grok wrapper and CI (Model A from Hermes review).
//!
//! The `POWERGROK_BRANDING=1` environment variable is still supported for
//! testing and non-feature builds. `OnceLock` means the decision is frozen for
//! the lifetime of the process (first observation wins).
//!
//! No inline `is_powergrok()` or primary `argv0` checks are allowed in core
//! TUI/CLI paths — all branding must route through this adapter (per
//! BRANDING_PLAN.md Hard rule and B8).

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
/// When the `powergrok` feature is enabled this returns `true` (Model A).
/// `POWERGROK_BRANDING=1` can still force branding in non-feature builds.
pub fn is_powergrok_branding() -> bool {
    static POWERGROK_BRANDING: OnceLock<bool> = OnceLock::new();

    *POWERGROK_BRANDING.get_or_init(|| {
        // POWERGROK_BRANDING=1 can force branding (useful for testing).
        if std::env::var_os("POWERGROK_BRANDING").is_some_and(|v| v == "1") {
            return true;
        }

        // When the powergrok feature is enabled, we always brand as Power Grok.
        // This is the model chosen after Hermes review (simpler, matches product
        // build profile, removes contradictory fallback path).
        cfg!(feature = "powergrok")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[cfg(not(feature = "powergrok"))]
    fn test_product_name_fallback() {
        // Only compiled in non-powergrok builds. When the feature is enabled we
        // always return "Power Grok" (Model A).
        let name = product_name();
        assert_eq!(name, "Grok Build");
    }

    #[test]
    #[serial] // Env var test must not run concurrently with other tests that set it.
    fn test_product_name_with_env_var() {
        // POWERGROK_BRANDING=1 forces branding even in non-feature builds.
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
