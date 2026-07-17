use std::process::Command;

#[test]
#[cfg(feature = "powergrok")]
fn test_powergrok_branding_help() {
    let bin_path = env!("CARGO_BIN_EXE_xai-grok-pager");

    // Model A: product feature always brands as Power Grok.
    let output = Command::new(bin_path)
        .arg("--help")
        .env_remove("POWERGROK_BRANDING")
        .output()
        .expect("Failed to execute binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Power Grok"),
        "Expected 'Power Grok' in help output under powergrok feature, got: {stdout}"
    );
}

#[test]
#[cfg(not(feature = "powergrok"))]
fn test_default_branding_help() {
    let bin_path = env!("CARGO_BIN_EXE_xai-grok-pager");

    // Non-product build keeps upstream branding when env override is unset.
    let output = Command::new(bin_path)
        .arg("--help")
        .env_remove("POWERGROK_BRANDING")
        .output()
        .expect("Failed to execute binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Grok Build"),
        "Expected 'Grok Build' in help output without powergrok feature, got: {stdout}"
    );
}
