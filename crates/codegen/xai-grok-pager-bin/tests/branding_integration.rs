use std::process::Command;

#[test]
#[cfg(feature = "powergrok")]
fn test_powergrok_branding_help() {
    let bin_path = env!("CARGO_BIN_EXE_xai-grok-pager");
    
    // Set POWERGROK_BRANDING=1 as the primary signal
    let output = Command::new(bin_path)
        .arg("--help")
        .env("POWERGROK_BRANDING", "1")
        .output()
        .expect("Failed to execute binary");
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // This will fail initially until clap config is updated (TDD!)
    assert!(
        stdout.contains("Power Grok"),
        "Expected 'Power Grok' in help output, but got: {}",
        stdout
    );
}

#[test]
fn test_default_branding_help() {
    let bin_path = env!("CARGO_BIN_EXE_xai-grok-pager");
    
    // When POWERGROK_BRANDING is not set, we should see Grok Build (fallback)
    let output = Command::new(bin_path)
        .arg("--help")
        .env_remove("POWERGROK_BRANDING")
        .output()
        .expect("Failed to execute binary");
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Smoke test for fallback path (should pass immediately)
    assert!(
        stdout.contains("Grok Build"),
        "Expected 'Grok Build' in help output, but got: {}",
        stdout
    );
}
