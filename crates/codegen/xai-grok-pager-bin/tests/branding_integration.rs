use std::process::Command;

#[test]
#[cfg(feature = "powergrok")]
fn test_powergrok_branding_help() {
    let bin_path = env!("CARGO_BIN_EXE_xai-grok-pager");
    
    // Under powergrok feature we always brand (Model A). The env var is
    // still respected for non-feature builds.
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
#[cfg(not(feature = "powergrok"))]
fn test_default_branding_help() {
    let bin_path = env!("CARGO_BIN_EXE_xai-grok-pager");
    
    // Only runs in non-powergrok builds. Under the product profile
    // (--features powergrok) we always expect "Power Grok" (Model A).
    let output = Command::new(bin_path)
        .arg("--help")
        .env_remove("POWERGROK_BRANDING")
        .output()
        .expect("Failed to execute binary");
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    assert!(
        stdout.contains("Grok Build"),
        "Expected 'Grok Build' in help output, but got: {}",
        stdout
    );
}
