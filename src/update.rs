use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use semver::Version;
use serde::Deserialize;

const CACHE_MAX_AGE_SECS: u64 = 24 * 60 * 60;
const UPDATE_URL: &str = "https://api.github.com/repos/herotomg/toms-tools/releases/latest";
const UPDATE_DISABLE_ENV: &str = "TT_NO_UPDATE_CHECK";
const UPDATE_COMMAND: &str = "tt update";
const INSTALL_ACTION_ENV: &str = "TT_INSTALL_ACTION";
const EMBEDDED_INSTALLER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"));

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateCache {
    checked_at: u64,
    latest: String,
}

#[derive(Debug, Deserialize)]
struct LatestRelease {
    tag_name: String,
}

pub fn maybe_check(disabled_by_flag: bool, force_refresh: bool) {
    if update_check_disabled(
        disabled_by_flag,
        env::var(UPDATE_DISABLE_ENV).ok().as_deref(),
    ) {
        return;
    }

    let _ = check_for_update(force_refresh);
}

pub fn run() -> Result<()> {
    println!("Updating tt from the latest release...");
    run_embedded_installer(EMBEDDED_INSTALLER)
}

fn check_for_update(force_refresh: bool) -> std::result::Result<(), ()> {
    let current_version = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| ())?;
    let cache_path = cache_file_path().ok_or(())?;
    let now = now_secs().ok_or(())?;

    let latest =
        latest_release_for_update(&cache_path, now, force_refresh, fetch_latest_release_tag);

    if let Some(latest) = latest {
        print_update_notice_if_newer(&current_version, &latest);
    }

    Ok(())
}

fn latest_release_for_update(
    cache_path: &Path,
    now: u64,
    force_refresh: bool,
    fetch_latest: impl FnOnce() -> Option<String>,
) -> Option<String> {
    if !force_refresh {
        if let Some(cache) = read_cache_if_fresh(cache_path, now) {
            return Some(cache.latest);
        }
    }

    let latest = fetch_latest()?;
    let cache = UpdateCache {
        checked_at: now,
        latest: latest.clone(),
    };

    let _ = write_cache(cache_path, &cache);
    Some(latest)
}

fn read_cache_if_fresh(path: &Path, now: u64) -> Option<UpdateCache> {
    let cache = read_cache(path)?;
    is_cache_fresh(now, cache.checked_at).then_some(cache)
}

fn read_cache(path: &Path) -> Option<UpdateCache> {
    let content = fs::read_to_string(path).ok()?;
    parse_cache(&content)
}

fn write_cache(path: &Path, cache: &UpdateCache) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = format!(
        "{{\"checked_at\":{},\"latest\":\"{}\"}}\n",
        cache.checked_at,
        escape_json_string(&cache.latest)
    );
    fs::write(path, content)
}

fn fetch_latest_release_tag() -> Option<String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let mut response = agent
        .get(UPDATE_URL)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tt")
        .call()
        .ok()?;
    let release: LatestRelease = response.body_mut().read_json().ok()?;

    normalize_version(&release.tag_name)
        .and_then(|version| Version::parse(version).ok())
        .map(|version| version.to_string())
}

fn print_update_notice_if_newer(current: &Version, latest: &str) {
    let Some(notice) = update_notice(current, latest) else {
        return;
    };

    eprintln!("{}", notice.yellow());
}

fn update_notice(current: &Version, latest: &str) -> Option<String> {
    let latest = Version::parse(latest).ok()?;

    if latest > *current {
        return Some(format!(
            "tt v{} → v{} available.\nRun to update:\n{}",
            current, latest, UPDATE_COMMAND
        ));
    }

    None
}

fn update_check_disabled(disabled_by_flag: bool, env_value: Option<&str>) -> bool {
    disabled_by_flag || matches!(env_value, Some("1"))
}

fn run_embedded_installer(script_contents: &str) -> Result<()> {
    let temp_dir = temp_update_dir()?;
    let result = run_embedded_installer_inner(script_contents, &temp_dir, run_installer_script);
    let cleanup = fs::remove_dir_all(&temp_dir);

    if let Err(err) = result {
        cleanup.ok();
        return Err(err);
    }

    cleanup.with_context(|| format!("failed to clean up {temp_dir:?}"))?;
    Ok(())
}

fn run_embedded_installer_inner(
    script_contents: &str,
    temp_dir: &Path,
    runner: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let script_path = write_installer_script(temp_dir, script_contents)?;
    runner(&script_path)
}

fn write_installer_script(temp_dir: &Path, script_contents: &str) -> Result<PathBuf> {
    fs::create_dir_all(temp_dir).with_context(|| format!("failed to create {temp_dir:?}"))?;
    let script_path = temp_dir.join("install.sh");
    fs::write(&script_path, script_contents)
        .with_context(|| format!("failed to write {script_path:?}"))?;
    set_script_permissions(&script_path)?;
    Ok(script_path)
}

#[cfg(unix)]
fn set_script_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .with_context(|| format!("failed to read metadata for {path:?}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("failed to set executable permissions on {path:?}"))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_script_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn run_installer_script(script_path: &Path) -> Result<()> {
    let bash = which::which("bash").context("bash is required to update tt")?;
    let status = Command::new(&bash)
        .arg(script_path)
        .env(INSTALL_ACTION_ENV, "Updated")
        .status()
        .with_context(|| format!("failed to run {script_path:?}"))?;

    if !status.success() {
        bail!("tt update failed")
    }

    Ok(())
}

fn cache_file_path() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    Some(
        Path::new(&home)
            .join(".cache")
            .join("toms-tools")
            .join("update_check.json"),
    )
}

fn now_secs() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn temp_update_dir() -> Result<PathBuf> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before unix epoch")?
        .as_nanos();
    Ok(env::temp_dir().join(format!("tt-update-{}-{unique}", std::process::id())))
}

fn is_cache_fresh(now: u64, checked_at: u64) -> bool {
    now.saturating_sub(checked_at) < CACHE_MAX_AGE_SECS
}

fn normalize_version(version: &str) -> Option<&str> {
    let trimmed = version.trim();
    (!trimmed.is_empty()).then_some(trimmed.strip_prefix('v').unwrap_or(trimmed))
}

fn parse_cache(content: &str) -> Option<UpdateCache> {
    Some(UpdateCache {
        checked_at: parse_json_u64(content, "checked_at")?,
        latest: parse_json_string(content, "latest")?,
    })
}

fn parse_json_u64(content: &str, key: &str) -> Option<u64> {
    let rest = content.split_once(&format!("\"{key}\""))?.1;
    let rest = rest.split_once(':')?.1.trim_start();
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

fn parse_json_string(content: &str, key: &str) -> Option<String> {
    let rest = content.split_once(&format!("\"{key}\""))?.1;
    let mut chars = rest.split_once(':')?.1.trim_start().chars();
    if chars.next()? != '"' {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            match ch {
                '"' | '\\' | '/' => value.push(ch),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                _ => return None,
            }
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Some(value),
            _ => value.push(ch),
        }
    }

    None
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_round_trips() {
        let cache = UpdateCache {
            checked_at: 1_700_000_000,
            latest: "1.2.3-beta.1".to_owned(),
        };
        let content = format!(
            "{{\"checked_at\":{},\"latest\":\"{}\"}}",
            cache.checked_at, cache.latest
        );

        assert_eq!(parse_cache(&content), Some(cache));
    }

    #[test]
    fn cache_age_is_checked_against_24_hours() {
        assert!(is_cache_fresh(CACHE_MAX_AGE_SECS - 1, 0));
        assert!(!is_cache_fresh(CACHE_MAX_AGE_SECS, 0));
    }

    #[test]
    fn update_check_can_be_disabled_by_flag_or_env() {
        assert!(update_check_disabled(true, None));
        assert!(update_check_disabled(false, Some("1")));
        assert!(!update_check_disabled(false, Some("0")));
    }

    #[test]
    fn leading_v_is_stripped_from_release_tags() {
        assert_eq!(normalize_version("v1.2.3"), Some("1.2.3"));
        assert_eq!(normalize_version("1.2.3"), Some("1.2.3"));
        assert_eq!(normalize_version(""), None);
    }

    #[test]
    fn update_notice_includes_version_line_and_install_command() {
        let current = Version::parse("0.1.6").unwrap();

        let notice = update_notice(&current, "9.9.9").unwrap();
        assert!(notice.contains("tt v0.1.6 → v9.9.9 available."));
        assert!(notice.contains("Run to update:"));
        assert!(notice.contains(UPDATE_COMMAND));
    }

    #[test]
    fn run_embedded_installer_inner_writes_script_before_invoking_runner() {
        let temp_dir =
            test_temp_dir("run_embedded_installer_inner_writes_script_before_invoking_runner");
        let mut captured = None;
        let script = "#!/usr/bin/env bash\necho hi\n";

        run_embedded_installer_inner(script, &temp_dir, |path| {
            captured = Some((path.to_path_buf(), fs::read_to_string(path).unwrap()));
            Ok(())
        })
        .unwrap();

        let (path, content) = captured.unwrap();
        assert_eq!(path, temp_dir.join("install.sh"));
        assert_eq!(content, script);
        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn update_notice_is_omitted_when_not_newer() {
        let current = Version::parse("0.1.6").unwrap();

        assert_eq!(update_notice(&current, "0.1.6"), None);
        assert_eq!(update_notice(&current, "0.1.5"), None);
    }

    #[test]
    fn fetch_failure_leaves_existing_cache_alone() {
        let path = test_cache_path("fetch_failure_leaves_existing_cache_alone");
        let original = "{\"checked_at\":1,\"latest\":\"8.8.8\"}\n";
        fs::write(&path, original).unwrap();

        let latest = latest_release_for_update(&path, CACHE_MAX_AGE_SECS + 1, false, || None);

        assert_eq!(latest, None);
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        cleanup_test_cache(&path);
    }

    #[test]
    fn successful_fetch_writes_cache() {
        let path = test_cache_path("successful_fetch_writes_cache");

        let latest =
            latest_release_for_update(&path, 1_700_000_123, false, || Some("9.9.9".to_owned()));

        assert_eq!(latest, Some("9.9.9".to_owned()));
        assert_eq!(
            read_cache(&path),
            Some(UpdateCache {
                checked_at: 1_700_000_123,
                latest: "9.9.9".to_owned(),
            })
        );
        cleanup_test_cache(&path);
    }

    #[test]
    fn fetch_failure_does_not_poison_empty_cache_with_current_version() {
        let path =
            test_cache_path("fetch_failure_does_not_poison_empty_cache_with_current_version");

        let latest = latest_release_for_update(&path, 1_700_000_123, false, || None);

        assert_eq!(latest, None);
        assert!(!path.exists());
        cleanup_test_cache(&path);
    }

    #[test]
    fn force_refresh_skips_fresh_cache() {
        let path = test_cache_path("force_refresh_skips_fresh_cache");
        fs::write(&path, "{\"checked_at\":100,\"latest\":\"1.0.0\"}\n").unwrap();

        let latest = latest_release_for_update(&path, 101, true, || Some("2.0.0".to_owned()));

        assert_eq!(latest, Some("2.0.0".to_owned()));
        assert_eq!(read_cache(&path).unwrap().latest, "2.0.0");
        cleanup_test_cache(&path);
    }

    fn test_cache_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("tt-update-test-{}-{name}.json", std::process::id()))
    }

    fn test_temp_dir(name: &str) -> PathBuf {
        env::temp_dir().join(format!("tt-update-test-dir-{}-{name}", std::process::id()))
    }

    fn cleanup_test_cache(path: &Path) {
        let _ = fs::remove_file(path);
    }

    fn cleanup_test_dir(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}
