use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use owo_colors::OwoColorize;
use semver::Version;
use serde::Deserialize;

const CACHE_MAX_AGE_SECS: u64 = 24 * 60 * 60;
const UPDATE_URL: &str = "https://api.github.com/repos/herotomg/toms-tools/releases/latest";
const UPDATE_DISABLE_ENV: &str = "TT_NO_UPDATE_CHECK";
const UPDATE_COMMAND: &str =
    "curl -fsSL https://raw.githubusercontent.com/herotomg/toms-tools/main/install.sh | bash";

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateCache {
    checked_at: u64,
    latest: String,
}

#[derive(Debug, Deserialize)]
struct LatestRelease {
    tag_name: String,
}

pub fn maybe_check(disabled_by_flag: bool) {
    if update_check_disabled(
        disabled_by_flag,
        env::var(UPDATE_DISABLE_ENV).ok().as_deref(),
    ) {
        return;
    }

    let _ = check_for_update();
}

fn check_for_update() -> Result<(), ()> {
    let current_version = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| ())?;
    let cache_path = cache_file_path().ok_or(())?;
    let now = now_secs().ok_or(())?;

    if let Some(cache) = read_cache_if_fresh(&cache_path, now) {
        print_update_notice_if_newer(&current_version, &cache.latest);
        return Ok(());
    }

    let latest = fetch_latest_release_tag().unwrap_or_else(|| current_version.to_string());
    let cache = UpdateCache {
        checked_at: now,
        latest,
    };

    let _ = write_cache(&cache_path, &cache);
    print_update_notice_if_newer(&current_version, &cache.latest);
    Ok(())
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
    fn update_notice_is_omitted_when_not_newer() {
        let current = Version::parse("0.1.6").unwrap();

        assert_eq!(update_notice(&current, "0.1.6"), None);
        assert_eq!(update_notice(&current, "0.1.5"), None);
    }
}
