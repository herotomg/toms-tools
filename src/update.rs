use std::{
    env, fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use dialoguer::Confirm;
use owo_colors::{OwoColorize, Stream, Style};
use semver::Version;

const CACHE_MAX_AGE_SECS: u64 = 24 * 60 * 60;
/// Deliberately github.com, not api.github.com. The API caps unauthenticated
/// callers at 60 requests an hour *per IP* — shared by everyone behind one
/// office or VPN address — and when that runs out the probe comes back empty
/// and `tt` silently concludes there is nothing newer. This URL answers with a
/// 302 to the release tag and is not subject to that limit.
const UPDATE_URL: &str = "https://github.com/herotomg/toms-tools/releases/latest";
const UPDATE_DISABLE_ENV: &str = "TT_NO_UPDATE_CHECK";
const UPDATE_COMMAND: &str = "tt update";
const INSTALL_ACTION_ENV: &str = "TT_INSTALL_ACTION";
/// Set on the process we hand over to after replacing the binary, so the new
/// one does not check for an update again and hand over once more.
const SELF_UPDATED_ENV: &str = "TT_SELF_UPDATED";
const EMBEDDED_INSTALLER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"));

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateCache {
    checked_at: u64,
    /// `None` means "we checked and came back empty" — an unreachable network,
    /// usually. It is still a real result: it stops us probing again for a day.
    latest: Option<String>,
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

/// What `tt update` managed to do about the binary itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfUpdate {
    /// Already current, or the user declined.
    NothingToDo,
    /// A newer `tt` is now on disk. This process is still the old one.
    Installed,
    /// We could not find out what the newest release is.
    CouldNotCheck,
}

/// True in the process we handed over to, which must not check again.
pub fn self_update_already_handled() -> bool {
    env::var_os(SELF_UPDATED_ENV).is_some()
}

/// Update the binary when there is something newer. `tt update` calls this
/// before updating tools, so one command makes the whole installation current.
///
/// Forces a fresh check: someone typing `tt update` is asking us to look now,
/// not to consult a cache that may be up to a day old. And unlike the passive
/// nudge, a check that *fails* says so — staying quiet there reads as "you are
/// up to date", which is the one thing it does not mean.
pub fn update_self_if_newer() -> SelfUpdate {
    let Ok(current) = Version::parse(env!("CARGO_PKG_VERSION")) else {
        return SelfUpdate::CouldNotCheck;
    };
    let Some(cache_path) = cache_file_path() else {
        return SelfUpdate::CouldNotCheck;
    };
    let Some(now) = now_secs() else {
        return SelfUpdate::CouldNotCheck;
    };

    match latest_release_for_update(&cache_path, now, true, fetch_latest_release_tag) {
        Some(latest) => offer_update_if_newer(&current, &latest),
        None => {
            eprintln!("{}", could_not_check_note());
            SelfUpdate::CouldNotCheck
        }
    }
}

/// Hand this command over to the `tt` we just installed.
///
/// The registry is compiled into the binary, so the rest of this process would
/// go on updating tools from the *old* list — and miss any tool the new
/// version adds. That is exactly the confusing case: `tt update` installs a
/// release that adds a tool, then reports every tool current.
#[cfg(unix)]
pub fn reexec_updated_binary() -> Result<()> {
    use std::os::unix::process::CommandExt;

    let exe = env::current_exe().context("cannot find the tt that was just installed")?;
    // `exec` replaces this process, so it only returns on failure.
    let error = Command::new(exe)
        .args(env::args_os().skip(1))
        .env(SELF_UPDATED_ENV, "1")
        .exec();

    Err(anyhow::Error::new(error).context("failed to hand over to the updated tt"))
}

#[cfg(not(unix))]
pub fn reexec_updated_binary() -> Result<()> {
    eprintln!("{}", update_hint());
    Ok(())
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
        offer_update_if_newer(&current_version, &latest);
    }

    Ok(())
}

fn latest_release_for_update(
    cache_path: &Path,
    now: u64,
    force_refresh: bool,
    fetch_latest: impl FnOnce() -> Option<String>,
) -> Option<String> {
    let cached = read_cache(cache_path);

    if !force_refresh {
        if let Some(cache) = cached
            .as_ref()
            .filter(|cache| is_cache_fresh(now, cache.checked_at))
        {
            return cache.latest.clone();
        }
    }

    // Record the attempt whether or not it succeeded. Without this an offline
    // machine re-probes the network — and pays the timeout — on every single
    // command, forever. A previously-known version survives a failed probe: it
    // was true yesterday, and all the failure tells us is that GitHub was
    // unreachable just now.
    let latest = fetch_latest().or_else(|| cached.and_then(|cache| cache.latest));
    let _ = write_cache(
        cache_path,
        &UpdateCache {
            checked_at: now,
            latest: latest.clone(),
        },
    );

    latest
}

fn read_cache(path: &Path) -> Option<UpdateCache> {
    let content = fs::read_to_string(path).ok()?;
    parse_cache(&content)
}

fn write_cache(path: &Path, cache: &UpdateCache) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let latest = match &cache.latest {
        Some(latest) => format!("\"{}\"", escape_json_string(latest)),
        None => "null".to_owned(),
    };
    let content = format!(
        "{{\"checked_at\":{},\"latest\":{latest}}}\n",
        cache.checked_at
    );
    fs::write(path, content)
}

fn fetch_latest_release_tag() -> Option<String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        // Do not follow it: the redirect *is* the answer, and the page it
        // points at is a few hundred KB of HTML we would only throw away.
        .max_redirects(0)
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let response = agent
        .get(UPDATE_URL)
        .header("User-Agent", "tt")
        .call()
        .ok()?;
    let location = response.headers().get("location")?.to_str().ok()?;

    tag_from_release_url(location)
}

/// Pull the version out of `…/releases/tag/v1.2.3`.
fn tag_from_release_url(url: &str) -> Option<String> {
    let tag = url.rsplit_once("/tag/")?.1.trim_end_matches('/');

    normalize_version(tag)
        .and_then(|version| Version::parse(version).ok())
        .map(|version| version.to_string())
}

fn offer_update_if_newer(current: &Version, latest: &str) -> SelfUpdate {
    let Some(latest) = newer_version(current, latest) else {
        return SelfUpdate::NothingToDo;
    };

    eprintln!("{}", update_headline(current, &latest));

    if !prompt_is_interactive() {
        eprintln!("{}", update_hint());
        return SelfUpdate::NothingToDo;
    }

    match confirm_update() {
        Ok(true) if install_update_inline(&latest) => SelfUpdate::Installed,
        Ok(true) => SelfUpdate::NothingToDo,
        Ok(false) => {
            eprintln!("{}", update_hint());
            SelfUpdate::NothingToDo
        }
        // Ctrl-C / closed terminal: stay out of the way.
        Err(_) => SelfUpdate::NothingToDo,
    }
}

/// Returns whether a newer binary is now on disk.
fn install_update_inline(latest: &Version) -> bool {
    let downloading = format!("  Downloading tt v{latest}…");
    eprintln!(
        "{}",
        downloading.if_supports_color(Stream::Stderr, |text| text.dimmed())
    );

    match run_embedded_installer_quietly(EMBEDDED_INSTALLER) {
        Ok(()) => {
            eprintln!("{} tt v{latest} installed\n", success_mark());
            true
        }
        Err(err) => {
            eprintln!(
                "{} update failed: {err:#}\n{}\n",
                failure_mark(),
                update_hint()
            );
            false
        }
    }
}

fn confirm_update() -> Result<bool> {
    Ok(Confirm::new()
        .with_prompt("Install it now?")
        .default(true)
        .interact()?)
}

fn prompt_is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

fn newer_version(current: &Version, latest: &str) -> Option<Version> {
    Version::parse(latest)
        .ok()
        .filter(|latest| latest > current)
}

fn update_headline(current: &Version, latest: &Version) -> String {
    let latest = format!("v{latest}");
    format!(
        "{} tt v{} → {}",
        "↑".if_supports_color(Stream::Stderr, |text| text
            .style(Style::new().yellow().bold())),
        current,
        latest.if_supports_color(Stream::Stderr, |text| text
            .style(Style::new().green().bold()))
    )
}

/// Said out loud, because `tt update` was an explicit request to look.
fn could_not_check_note() -> String {
    let note = "  Could not reach GitHub, so tt's own version was not checked.";
    format!(
        "{}",
        note.if_supports_color(Stream::Stderr, |text| text.dimmed())
    )
}

fn update_hint() -> String {
    let hint = format!("  Run `{UPDATE_COMMAND}` whenever you want it.");
    format!(
        "{}",
        hint.if_supports_color(Stream::Stderr, |text| text.dimmed())
    )
}

fn success_mark() -> String {
    format!(
        "{}",
        "✓".if_supports_color(Stream::Stderr, |text| text.green())
    )
}

fn failure_mark() -> String {
    format!(
        "{}",
        "✗".if_supports_color(Stream::Stderr, |text| text.red())
    )
}

fn update_check_disabled(disabled_by_flag: bool, env_value: Option<&str>) -> bool {
    disabled_by_flag || matches!(env_value, Some("1"))
}

fn run_embedded_installer(script_contents: &str) -> Result<()> {
    run_embedded_installer_with(script_contents, run_installer_script)
}

fn run_embedded_installer_quietly(script_contents: &str) -> Result<()> {
    run_embedded_installer_with(script_contents, run_installer_script_quietly)
}

fn run_embedded_installer_with(
    script_contents: &str,
    runner: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let temp_dir = temp_update_dir()?;
    let result = run_embedded_installer_inner(script_contents, &temp_dir, runner);
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
    let status = installer_command(script_path)?
        .status()
        .with_context(|| format!("failed to run {script_path:?}"))?;

    if !status.success() {
        bail!("tt update failed")
    }

    Ok(())
}

/// Same install, but the download chatter is swallowed so an auto-update stays
/// a single line unless something goes wrong.
fn run_installer_script_quietly(script_path: &Path) -> Result<()> {
    let output = installer_command(script_path)?
        .output()
        .with_context(|| format!("failed to run {script_path:?}"))?;

    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        let details = details.trim();
        bail!(if details.is_empty() {
            "tt update failed".to_owned()
        } else {
            details.to_owned()
        })
    }

    Ok(())
}

fn installer_command(script_path: &Path) -> Result<Command> {
    let bash = which::which("bash").context("bash is required to update tt")?;
    let mut command = Command::new(bash);
    command.arg(script_path).env(INSTALL_ACTION_ENV, "Updated");
    Ok(command)
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
        latest: parse_json_string(content, "latest"),
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
    fn cache_round_trips_through_the_file() {
        let path = test_cache_path("cache_round_trips");

        for cache in [
            UpdateCache {
                checked_at: 1_700_000_000,
                latest: Some("1.2.3-beta.1".to_owned()),
            },
            UpdateCache {
                checked_at: 1_700_000_000,
                latest: None,
            },
        ] {
            write_cache(&path, &cache).unwrap();
            assert_eq!(read_cache(&path), Some(cache));
        }

        cleanup_test_cache(&path);
    }

    #[test]
    fn a_cache_written_before_null_support_still_parses() {
        assert_eq!(
            parse_cache("{\"checked_at\":7,\"latest\":\"1.2.3\"}"),
            Some(UpdateCache {
                checked_at: 7,
                latest: Some("1.2.3".to_owned()),
            })
        );
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

    /// The release check reads a redirect rather than the GitHub API, because
    /// the API's 60-per-hour unauthenticated limit is per IP and silently
    /// turned "there is a new tt" into "you are up to date".
    #[test]
    fn the_tag_is_read_from_a_release_redirect() {
        assert_eq!(
            tag_from_release_url("https://github.com/herotomg/toms-tools/releases/tag/v0.1.24"),
            Some("0.1.24".to_owned())
        );
        // Trailing slash, and a tag without the conventional `v`.
        assert_eq!(
            tag_from_release_url("https://github.com/o/r/releases/tag/1.2.3/"),
            Some("1.2.3".to_owned())
        );
    }

    #[test]
    fn a_redirect_that_names_no_tag_yields_nothing() {
        for url in [
            "https://github.com/herotomg/toms-tools/releases",
            "https://github.com/o/r/releases/tag/not-a-version",
            "https://github.com/o/r/releases/tag/",
            "",
        ] {
            assert_eq!(tag_from_release_url(url), None, "{url} should not parse");
        }
    }

    #[test]
    fn nothing_to_do_when_the_release_is_not_newer() {
        let current = Version::parse("1.0.0").unwrap();

        assert_eq!(
            offer_update_if_newer(&current, "1.0.0"),
            SelfUpdate::NothingToDo
        );
        assert_eq!(
            offer_update_if_newer(&current, "0.9.0"),
            SelfUpdate::NothingToDo
        );
        assert_eq!(
            offer_update_if_newer(&current, "not-a-version"),
            SelfUpdate::NothingToDo
        );
    }

    #[test]
    fn update_url_is_not_the_rate_limited_api() {
        assert!(
            !UPDATE_URL.contains("api.github.com"),
            "the API rate-limits unauthenticated callers per IP: {UPDATE_URL}"
        );
        assert!(UPDATE_URL.ends_with("/releases/latest"));
    }

    #[test]
    fn leading_v_is_stripped_from_release_tags() {
        assert_eq!(normalize_version("v1.2.3"), Some("1.2.3"));
        assert_eq!(normalize_version("1.2.3"), Some("1.2.3"));
        assert_eq!(normalize_version(""), None);
    }

    #[test]
    fn update_headline_shows_both_versions() {
        let current = Version::parse("0.1.6").unwrap();
        let latest = newer_version(&current, "9.9.9").unwrap();

        let headline = update_headline(&current, &latest);
        assert!(headline.contains("tt v0.1.6"));
        assert!(headline.contains("v9.9.9"));
    }

    #[test]
    fn update_hint_points_at_the_update_command() {
        assert!(update_hint().contains(UPDATE_COMMAND));
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
    fn newer_version_is_none_when_not_newer() {
        let current = Version::parse("0.1.6").unwrap();

        assert_eq!(newer_version(&current, "0.1.6"), None);
        assert_eq!(newer_version(&current, "0.1.5"), None);
        assert_eq!(newer_version(&current, "not-a-version"), None);
        assert_eq!(
            newer_version(&current, "0.1.7"),
            Some(Version::parse("0.1.7").unwrap())
        );
    }

    #[test]
    fn fetch_failure_keeps_the_version_it_already_knew() {
        let path = test_cache_path("fetch_failure_keeps_the_version_it_already_knew");
        fs::write(&path, "{\"checked_at\":1,\"latest\":\"8.8.8\"}\n").unwrap();

        let now = CACHE_MAX_AGE_SECS + 1;
        let latest = latest_release_for_update(&path, now, false, || None);

        // The probe failed, but 8.8.8 was real yesterday and still is.
        assert_eq!(latest, Some("8.8.8".to_owned()));
        // And the attempt is recorded, so we back off instead of retrying.
        assert_eq!(
            read_cache(&path),
            Some(UpdateCache {
                checked_at: now,
                latest: Some("8.8.8".to_owned()),
            })
        );
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
                latest: Some("9.9.9".to_owned()),
            })
        );
        cleanup_test_cache(&path);
    }

    #[test]
    fn fetch_failure_records_the_attempt_without_inventing_a_version() {
        let path = test_cache_path("fetch_failure_records_the_attempt");

        let latest = latest_release_for_update(&path, 1_700_000_123, false, || None);

        assert_eq!(latest, None);
        assert_eq!(
            read_cache(&path),
            Some(UpdateCache {
                checked_at: 1_700_000_123,
                latest: None,
            })
        );
        cleanup_test_cache(&path);
    }

    /// The reason negative caching exists: an offline machine used to pay the
    /// network timeout on every command because a failed probe wrote nothing.
    #[test]
    fn a_failed_probe_is_not_retried_until_the_cache_expires() {
        let path = test_cache_path("a_failed_probe_is_not_retried");
        let calls = std::cell::Cell::new(0);
        let probe = || {
            calls.set(calls.get() + 1);
            None
        };

        assert_eq!(latest_release_for_update(&path, 1_000, false, probe), None);
        assert_eq!(calls.get(), 1);

        // Same day: no second probe.
        assert_eq!(latest_release_for_update(&path, 1_001, false, probe), None);
        assert_eq!(calls.get(), 1);

        // A day later it is allowed to try again.
        let tomorrow = 1_000 + CACHE_MAX_AGE_SECS;
        assert_eq!(
            latest_release_for_update(&path, tomorrow, false, probe),
            None
        );
        assert_eq!(calls.get(), 2);

        cleanup_test_cache(&path);
    }

    #[test]
    fn force_refresh_skips_fresh_cache() {
        let path = test_cache_path("force_refresh_skips_fresh_cache");
        fs::write(&path, "{\"checked_at\":100,\"latest\":\"1.0.0\"}\n").unwrap();

        let latest = latest_release_for_update(&path, 101, true, || Some("2.0.0".to_owned()));

        assert_eq!(latest, Some("2.0.0".to_owned()));
        assert_eq!(read_cache(&path).unwrap().latest, Some("2.0.0".to_owned()));
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
