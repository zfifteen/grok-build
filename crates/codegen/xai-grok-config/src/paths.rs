//! Filesystem locations for grok config files and binaries.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static GROK_HOME: OnceLock<PathBuf> = OnceLock::new();
static PROJECT_CONFIG_DIRNAME: OnceLock<&'static str> = OnceLock::new();

/// Test-only override for [`project_config_dirname`]. Production never reads this.
static PROJECT_CONFIG_DIRNAME_TEST_OVERRIDE: Mutex<Option<&'static str>> = Mutex::new(None);

#[cfg(target_os = "macos")]
const CLAUDE_MANAGED_SETTINGS_PATH: &str =
    "/Library/Application Support/ClaudeCode/managed-settings.json";
#[cfg(target_os = "linux")]
const CLAUDE_MANAGED_SETTINGS_PATH: &str = "/etc/claude-code/managed-settings.json";

/// The default user grok directory (`~/.grok`, canonicalized) used when
/// `GROK_HOME` is unset. Exposed so callers (e.g. display helpers) can detect
/// whether [`grok_home()`] is the default without duplicating the computation.
///
/// Uses [`dunce::canonicalize`] instead of [`std::fs::canonicalize`]: on
/// Windows, std returns a verbatim path (`\\?\C:\Users\...`) which external
/// tools choke on — e.g. `git clone` rejects `\\?\` destinations with
/// "Invalid argument", breaking marketplace cache clones under
/// `~/.grok/marketplace-cache`. `dunce` strips the prefix whenever the path
/// is safely representable in legacy form; on non-Windows it is identical to
/// `std::fs::canonicalize`.
///
/// Keep the dunce canonicalization in sync with the hand-rolled duplicate in
/// `xai_fast_worktree::db::resolve_grok_home` (deliberately standalone crate).
pub fn default_grok_home() -> PathBuf {
    #[allow(deprecated)]
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    dunce::canonicalize(&home).unwrap_or(home).join(".grok")
}

/// Per-user config directory: `$GROK_HOME` or `~/.grok`. Created if needed.
pub fn grok_home() -> PathBuf {
    GROK_HOME
        .get_or_init(|| {
            let grok_home = if let Ok(v) = std::env::var("GROK_HOME") {
                PathBuf::from(v)
            } else {
                default_grok_home()
            };
            let _ = std::fs::create_dir_all(&grok_home);
            grok_home
        })
        .clone()
}

/// The user-global grok home, but only when one genuinely resolves: `Some` when
/// `$GROK_HOME` is set or a home directory is found, `None` otherwise. Unlike
/// [`grok_home()`], this never falls back to a cwd-relative `.grok`, so callers
/// that *scan* user-global grok resources (hooks, marketplace sources, ...) don't
/// mistake a project's `.grok` tree for the user-global one when no home resolves.
pub fn user_grok_home() -> Option<PathBuf> {
    #[allow(deprecated)]
    let resolvable = std::env::var_os("GROK_HOME").is_some() || std::env::home_dir().is_some();
    resolvable.then(grok_home)
}

/// Basename of the per-workspace project config directory (`.grok` or `.powergrok`).
///
/// Resolution (BUILD_PLAN §7.1 / issue #4):
/// 1. Test override if set (unit tests only; never production).
/// 2. Else argv0 basename `powergrok` / `powergrok.exe` → `.powergrok`.
/// 3. Else → `.grok`.
///
/// Cached for process lifetime after the first non-override observation (mirrors
/// [`grok_home`] OnceLock discipline). Fail-closed: powergrok never falls back
/// to reading project `.grok/` (D7).
pub fn project_config_dirname() -> &'static str {
    if let Ok(guard) = PROJECT_CONFIG_DIRNAME_TEST_OVERRIDE.lock() {
        if let Some(name) = *guard {
            return name;
        }
    }
    PROJECT_CONFIG_DIRNAME.get_or_init(resolve_project_config_dirname_from_argv0)
}

/// `<workspace_root>/<project_config_dirname()>`.
pub fn project_config_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(project_config_dirname())
}

/// True when this process uses the Power Grok project tree name (`.powergrok`).
pub fn is_powergrok_project_tree() -> bool {
    project_config_dirname() == ".powergrok"
}

/// G4 predicate (pure): powergrok process, workspace has `.grok/`, lacks `.powergrok/`.
/// Independent of the one-shot latch so tests can assert conditions without
/// process-order dependence.
pub fn should_emit_empty_project_layer_warning(workspace_root: &Path) -> bool {
    is_powergrok_project_tree()
        && workspace_root.join(".grok").is_dir()
        && !workspace_root.join(".powergrok").is_dir()
}

/// G4: one-shot empty project-layer warning when running as powergrok, the
/// workspace has `.grok/` but no `.powergrok/`. Returns the message if it should
/// be shown **now** (first call only for this process when conditions hold).
///
/// Callers choose the channel (stderr, toast, status). Never auto-copies.
pub fn take_empty_project_layer_warning(workspace_root: &Path) -> Option<&'static str> {
    static WARNED: OnceLock<()> = OnceLock::new();
    if !should_emit_empty_project_layer_warning(workspace_root) {
        return None;
    }
    if WARNED.set(()).is_err() {
        return None;
    }
    Some(
        "Project config for powergrok uses `.powergrok/` (isolated from `.grok/`). \
         No `.powergrok/` found; project MCP/skills/hooks are empty until you create it. \
         In-product: `/bootstrap-project` (opt-in copy or --empty). \
         Manual: `cp -R .grok .powergrok`.",
    )
}

/// Set project config dirname for tests. Pass `None` to clear the override.
///
/// Does **not** clear the production OnceLock cache; prefer setting the override
/// before the first production resolve in a test process, or only use override
/// (override is checked first).
pub fn set_project_config_dirname_for_test(name: Option<&'static str>) {
    if let Ok(mut guard) = PROJECT_CONFIG_DIRNAME_TEST_OVERRIDE.lock() {
        *guard = name;
    }
}

fn resolve_project_config_dirname_from_argv0() -> &'static str {
    let argv0 = std::env::args_os().next();
    let Some(argv0) = argv0 else {
        return ".grok";
    };
    let path = PathBuf::from(argv0);
    let base = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if base == "powergrok" || base == "powergrok.exe" {
        ".powergrok"
    } else {
        ".grok"
    }
}

/// Canonical grok application path: `$GROK_HOME/bin/grok` (Unix) or `grok.exe` (Windows).
pub fn grok_application() -> PathBuf {
    grok_application_in(&grok_home())
}

/// [`grok_application`] under an explicit home instead of `$GROK_HOME`.
pub fn grok_application_in(home: &std::path::Path) -> PathBuf {
    let name = if cfg!(windows) { "grok.exe" } else { "grok" };
    home.join("bin").join(name)
}

/// System-wide config directory: `/etc/grok/` on Unix, `None` on Windows.
pub fn system_config_dir() -> Option<PathBuf> {
    if cfg!(unix) {
        Some(PathBuf::from("/etc/grok"))
    } else {
        None
    }
}

/// System path for the managed-settings.json used for settings compat, if it exists.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn claude_managed_settings_path() -> Option<PathBuf> {
    let path = PathBuf::from(CLAUDE_MANAGED_SETTINGS_PATH);
    path.exists().then_some(path)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn claude_managed_settings_path() -> Option<PathBuf> {
    None
}

/// The platform path where managed-settings.json would live for settings
/// compat, whether or not it exists. `None` on unsupported platforms.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn claude_managed_settings_probe_path() -> Option<PathBuf> {
    Some(PathBuf::from(CLAUDE_MANAGED_SETTINGS_PATH))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn claude_managed_settings_probe_path() -> Option<PathBuf> {
    None
}

/// Max bytes for a single directory name component (macOS APFS, Linux ext4,
/// NTFS all enforce 255 bytes).
const MAX_DIRNAME_BYTES: usize = 255;

/// Encode a CWD string into a filesystem-safe directory name component.
///
/// Short CWDs (URL-encoded form <= 255 bytes) use URL-encoding for backward
/// compatibility and human readability on disk.
///
/// Long CWDs (> 255 bytes encoded) use a compact `{slug}-{blake3_hex16}`
/// form that is always <= 57 bytes. Callers must write a `.cwd` metadata
/// file via [`ensure_sessions_cwd_dir`] so the original CWD can be
/// recovered by [`decode_cwd_from_dirname`].
pub fn encode_cwd_dirname(cwd: &str) -> String {
    let url_encoded = urlencoding::encode(cwd);
    if url_encoded.len() <= MAX_DIRNAME_BYTES {
        return url_encoded.into_owned();
    }
    let hash = blake3::hash(cwd.as_bytes());
    let hash16 = &hash.to_hex()[..16];
    let leaf = std::path::Path::new(cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace");
    let slug = slugify(leaf, 40);
    let slug = if slug.is_empty() { "workspace" } else { &slug };
    format!("{slug}-{hash16}")
}

/// Recover the original CWD from a sessions CWD directory.
///
/// Tries URL-decoding the directory name first (works for short/legacy dirs).
/// Falls back to reading a `.cwd` metadata file inside the directory (written
/// by [`ensure_sessions_cwd_dir`] for hash-based dirs).
pub fn decode_cwd_from_dirname(dir: &std::path::Path) -> Option<String> {
    let name = dir.file_name()?.to_str()?;
    if let Ok(decoded) = urlencoding::decode(name) {
        let s = decoded.into_owned();
        // URL-decoded absolute CWDs always start with `/` (Unix) or a drive
        // letter (Windows).  The slug-hash form never does, so this
        // distinguishes the two encodings unambiguously.
        if s.starts_with('/') || (cfg!(windows) && s.chars().nth(1) == Some(':')) {
            return Some(s);
        }
    }
    std::fs::read_to_string(dir.join(".cwd"))
        .ok()
        .map(|s| s.trim().to_string())
}

/// Build the CWD-level session directory path:
/// `grok_home()/sessions/{encode_cwd_dirname(cwd)}`.
///
/// Does **not** create the directory on disk — use [`ensure_sessions_cwd_dir`]
/// when the directory must exist.
pub fn sessions_cwd_dir(cwd: &str) -> PathBuf {
    grok_home().join("sessions").join(encode_cwd_dirname(cwd))
}

/// Create the CWD-level session directory and write a `.cwd` metadata file
/// when hash-based encoding is used (long paths).
///
/// For short paths the `.cwd` file is not written because the directory name
/// itself is reversible via URL-decoding.
pub fn ensure_sessions_cwd_dir(cwd: &str) -> std::io::Result<PathBuf> {
    let encoded_name = encode_cwd_dirname(cwd);
    let dir = grok_home().join("sessions").join(&encoded_name);
    std::fs::create_dir_all(&dir)?;
    // Hash-based encoding is in use when the dirname differs from the
    // plain URL-encoded form.  Write a `.cwd` file so decode can recover
    // the original path.  O_CREAT|O_EXCL via create_new avoids TOCTOU
    // races with parallel session starts.
    if encoded_name != urlencoding::encode(cwd).as_ref() {
        let cwd_file = dir.join(".cwd");
        match std::fs::File::create_new(&cwd_file) {
            Ok(mut f) => {
                std::io::Write::write_all(&mut f, cwd.as_bytes())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Ok(dir)
}

/// Generate a URL-safe slug from a string.
///
/// Lowercases, replaces non-alphanumeric chars with `-`, collapses
/// consecutive dashes, and truncates to `max_len` characters.
fn slugify(input: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(input.len());
    let mut prev_dash = false;
    for c in input.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }
    let trimmed = result.trim_matches('-');
    trimmed.chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Realistic CWDs that trigger the bug (URL-encoded > 255 bytes).
    const LONG_CWDS: &[&str] = &[
        "/Users/dev/Documents/開発プロジェクト/機能追加/テスト環境/ソースコード/main-branch",
        "/Users/user/Library/Mobile Documents/com~apple~CloudDocs/项目文件/深层嵌套目录/更深层次的/工作区域/project",
        "/Users/user/Library/CloudStorage/OneDrive-대한민국회사/프로젝트/개발환경/소스코드/백엔드/서비스/my-app",
        "/Users/user/Documents/工作文件夹/二零二六年项目/子目录一/子目录二/子目录三/源代码/code",
    ];

    #[test]
    fn long_cwd_uses_hash_fallback_within_name_max() {
        let long_cwd = format!("/Users/test/{}", "中".repeat(30));
        let encoded = encode_cwd_dirname(&long_cwd);
        assert!(encoded.len() <= MAX_DIRNAME_BYTES);
        assert!(!encoded.starts_with("%2F"));
    }

    #[test]
    fn different_long_paths_produce_different_hashes() {
        let a = format!("/Users/test/{}", "中".repeat(30));
        let b = format!("/Users/test/{}", "日".repeat(30));
        assert_ne!(encode_cwd_dirname(&a), encode_cwd_dirname(&b));
    }

    #[test]
    fn decode_reads_cwd_file_for_hash_dirs() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("some-slug-abcdef0123456789");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".cwd"), "/original/long/path").unwrap();
        assert_eq!(
            decode_cwd_from_dirname(&dir),
            Some("/original/long/path".to_string())
        );
    }

    #[test]
    fn decode_returns_none_without_cwd_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("some-slug-abcdef0123456789");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(decode_cwd_from_dirname(&dir), None);
    }

    #[test]
    fn cwd_file_write_is_idempotent_via_excl() {
        let tmp = TempDir::new().unwrap();
        let long_cwd = format!("/Users/test/{}", "中".repeat(30));
        let dir = tmp.path().join(encode_cwd_dirname(&long_cwd));
        std::fs::create_dir_all(&dir).unwrap();
        let cwd_file = dir.join(".cwd");
        std::fs::write(&cwd_file, &long_cwd).unwrap();
        match std::fs::File::create_new(&cwd_file) {
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            other => panic!("expected AlreadyExists, got: {other:?}"),
        }
        assert_eq!(std::fs::read_to_string(&cwd_file).unwrap(), long_cwd);
    }

    #[test]
    fn url_encoded_long_cwd_fails_on_real_filesystem() {
        let tmp = TempDir::new().unwrap();
        let url_encoded = urlencoding::encode(LONG_CWDS[0]).into_owned();
        let result = std::fs::create_dir_all(tmp.path().join(&url_encoded));
        assert!(result.is_err());
    }

    #[test]
    fn full_roundtrip_on_real_filesystem_for_long_cwds() {
        let tmp = TempDir::new().unwrap();
        for cwd in LONG_CWDS {
            let encoded = encode_cwd_dirname(cwd);
            let dir = tmp.path().join(&encoded);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(".cwd"), cwd).unwrap();
            assert_eq!(decode_cwd_from_dirname(&dir).as_deref(), Some(*cwd));
        }
    }

    #[test]
    fn short_cwds_use_url_encoding_and_roundtrip_on_real_filesystem() {
        let tmp = TempDir::new().unwrap();
        for cwd in [
            "/Users/foo/project",
            "/tmp",
            "/Users/user/Documents/project-名前",
        ] {
            let encoded = encode_cwd_dirname(cwd);
            assert_eq!(encoded, urlencoding::encode(cwd).into_owned());
            let dir = tmp.path().join(&encoded);
            std::fs::create_dir_all(&dir).unwrap();
            assert_eq!(decode_cwd_from_dirname(&dir).as_deref(), Some(cwd));
        }
    }

    #[test]
    fn default_grok_home_has_no_verbatim_prefix() {
        // On Windows, std::fs::canonicalize returns `\\?\C:\...` verbatim
        // paths that external tools (notably `git clone`) reject. The dunce
        // canonicalization must yield a plain path. No-op assertion on Unix.
        let home = default_grok_home();
        assert!(!home.to_string_lossy().starts_with(r"\\?\"));
        assert!(home.ends_with(".grok"));
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World!", 40), "hello-world");
    }

    #[test]
    fn slugify_cjk_produces_empty() {
        assert_eq!(slugify("深层目录", 40), "");
    }

    #[test]
    fn slugify_truncates() {
        assert_eq!(slugify(&"a".repeat(100), 10).len(), 10);
    }

    #[test]
    #[serial_test::serial]
    fn project_config_dirname_default_is_grok_for_test_binary() {
        set_project_config_dirname_for_test(None);
        // Test harness argv0 is not "powergrok" → official tree name.
        assert_eq!(project_config_dirname(), ".grok");
    }

    #[test]
    #[serial_test::serial]
    fn project_config_dirname_override_powergrok() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        assert_eq!(project_config_dirname(), ".powergrok");
        assert!(is_powergrok_project_tree());
        assert_eq!(
            project_config_dir(Path::new("/ws")),
            PathBuf::from("/ws/.powergrok")
        );
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn empty_project_layer_warning_under_powergrok_when_only_official_tree() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".grok")).unwrap();
        assert!(
            should_emit_empty_project_layer_warning(root),
            "pure predicate must hold regardless of latch"
        );
        // One-shot take: first Some (if latch free) must match G4 copy.
        if let Some(msg) = take_empty_project_layer_warning(root) {
            assert!(msg.contains(".powergrok"), "{msg}");
            assert!(
                msg.contains("/bootstrap-project") || msg.contains("cp -R .grok .powergrok"),
                "{msg}"
            );
            assert!(
                take_empty_project_layer_warning(root).is_none(),
                "G4 must fire at most once per process"
            );
        }
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn empty_project_layer_no_warn_when_powergrok_dir_exists() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".grok")).unwrap();
        std::fs::create_dir_all(root.join(".powergrok")).unwrap();
        assert!(!should_emit_empty_project_layer_warning(root));
        assert!(take_empty_project_layer_warning(root).is_none());
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn empty_project_layer_no_warn_under_official_tree() {
        set_project_config_dirname_for_test(Some(".grok"));
        assert!(
            !is_powergrok_project_tree(),
            "override must force official tree name"
        );
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".grok")).unwrap();
        assert!(!should_emit_empty_project_layer_warning(tmp.path()));
        assert!(take_empty_project_layer_warning(tmp.path()).is_none());
        set_project_config_dirname_for_test(None);
    }
}
