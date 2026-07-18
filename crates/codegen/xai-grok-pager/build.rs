use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-env-changed=GROK_VERSION");
    println!("cargo:rerun-if-changed=docs/user-guide/");

    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let version = std::env::var("GROK_VERSION")
        .or_else(|_| std::env::var("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| "0.0.0".to_string());

    println!(
        "cargo:rustc-env=VERSION_WITH_COMMIT={} ({})",
        version, commit
    );

    // B9: Build-time structured documentation override for Power Grok.
    // Copy the upstream user-guide and apply targeted replacements only when
    // the powergrok feature is enabled. Avoids duplicating the entire guide.
    if std::env::var_os("CARGO_FEATURE_POWERGROK").is_some() {
        let src_dir = Path::new("docs/user-guide");
        let out_dir = Path::new(&std::env::var("OUT_DIR").unwrap()).join("user-guide");

        fs::create_dir_all(&out_dir).unwrap();

        for entry in fs::read_dir(src_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let content = fs::read_to_string(&path).unwrap();

                // Structured replacements for Power Grok branding (B9).
                // User-facing product names and command examples are updated.
                // Technical paths (.grok/, GROK_HOME, model names, etc.) stay.
                let branded = content
                    .replace("Grok Build", "Power Grok")
                    .replace("Grok CLI", "Power Grok")
                    .replace("the Grok TUI", "the Power Grok TUI")
                    .replace("Grok is", "Power Grok is")
                    .replace("\"grok\"", "\"powergrok\"")
                    .replace("`grok ", "`powergrok ");

                let dest = out_dir.join(path.file_name().unwrap());
                fs::write(&dest, branded).unwrap();
            }
        }

        println!(
            "cargo:rustc-env=POWERGROK_USER_GUIDE_DIR={}",
            out_dir.display()
        );

        // Real golden check (not cargo:warning): fail the build if branding
        // replacements did not land on the getting-started page.
        let golden = out_dir.join("01-getting-started.md");
        let golden_body = fs::read_to_string(&golden).unwrap_or_else(|e| {
            panic!(
                "B9 golden: failed to read branded {}: {e}",
                golden.display()
            )
        });
        assert!(
            golden_body.contains("Power Grok"),
            "B9 golden: branded {} must contain \"Power Grok\"",
            golden.display()
        );
    }
}
