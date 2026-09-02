//! Whether a tool is behind something we do not ship.
//!
//! Most tools are current when their recorded version matches the bundled one:
//! the payload is in the binary, so the manifest is the whole truth. A tool
//! that installs a *release of someone else's repository* is different — the
//! version that matters lives on GitHub, and neither `tool.toml` nor
//! `installed.toml` can see it.
//!
//! Such a tool ships an `update-check.sh` that prints one line when an update
//! is waiting and prints nothing when it is current. The catch is that
//! answering costs a network round trip, and `tt` and `tt list` are the two
//! commands people run constantly. So the script never runs on the hot path:
//! [`refresh`] runs it at most once a day and writes the answer down, and
//! everything else reads the cache.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::{installer, EmbeddedTool, Registry};

pub const HOOK: &str = "update-check.sh";
const MAX_AGE_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Cache {
    #[serde(default)]
    tools: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    checked_at: u64,
    /// The one-line reason an update is wanted. `None` means "checked, and it
    /// is current" — which is why the field is optional rather than a bool: the
    /// text is what we show the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl Cache {
    /// The reason this tool is behind, if we know of one.
    pub fn detail(&self, id: &str) -> Option<&str> {
        self.tools.get(id)?.detail.as_deref()
    }

    pub fn has_update(&self, id: &str) -> bool {
        self.detail(id).is_some()
    }
}

/// Read what we already know. Never runs anything, never touches the network.
pub fn cached() -> Cache {
    cache_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

/// Run the checks that have gone stale, and write the answers down.
///
/// Checks run concurrently: they are independent network calls, and doing them
/// in series would make one command a day as slow as the sum of every tool.
pub fn refresh(registry: &Registry, force: bool) {
    let Some(path) = cache_path() else {
        return;
    };
    let Some(now) = now_secs() else {
        return;
    };

    let mut cache = cached();

    // Pair each due tool with what we last knew, so a check that cannot run
    // can fall back to it instead of erasing it.
    let due: Vec<(&EmbeddedTool, Option<String>)> = registry
        .tools()
        .filter(|tool| installer::has_hook(tool, HOOK))
        .filter(|tool| force || is_stale(&cache, &tool.definition.id, now))
        .map(|tool| {
            let previous = cache.detail(&tool.definition.id).map(str::to_owned);
            (tool, previous)
        })
        .collect();

    if due.is_empty() {
        return;
    }

    let results: Vec<(String, Option<String>)> = thread::scope(|scope| {
        let handles: Vec<_> = due
            .iter()
            .map(|(tool, previous)| {
                scope.spawn(move || {
                    let outcome = installer::capture_hook(tool, HOOK);
                    (
                        tool.definition.id.clone(),
                        resolve_detail(outcome, previous.as_deref()),
                    )
                })
            })
            .collect();

        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .collect()
    });

    for (id, detail) in results {
        cache.tools.insert(
            id,
            Entry {
                checked_at: now,
                detail,
            },
        );
    }

    let _ = write(&path, &cache);
}

/// Re-run one tool's check now and record the answer.
///
/// Called straight after installing, because the daily refresh runs *before*
/// the install: without this, a tool you just updated goes on claiming it is
/// behind until the cache expires tomorrow.
pub fn recheck(tool: &EmbeddedTool) {
    if !installer::has_hook(tool, HOOK) {
        return;
    }

    let Some(path) = cache_path() else {
        return;
    };
    let Some(now) = now_secs() else {
        return;
    };

    let id = &tool.definition.id;
    let mut cache = cached();
    let previous = cache.detail(id).map(str::to_owned);
    let detail = resolve_detail(installer::capture_hook(tool, HOOK), previous.as_deref());

    cache.tools.insert(
        id.clone(),
        Entry {
            checked_at: now,
            detail,
        },
    );
    let _ = write(&path, &cache);
}

/// Forget what we recorded for a tool, so it is asked again next time. Used on
/// removal, where a stale "update available" would outlive the tool itself.
pub fn forget(id: &str) {
    let Some(path) = cache_path() else {
        return;
    };

    let mut cache = cached();
    if cache.tools.remove(id).is_some() {
        let _ = write(&path, &cache);
    }
}

fn write(path: &Path, cache: &Cache) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(cache)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(path, content)
}

fn is_stale(cache: &Cache, id: &str, now: u64) -> bool {
    match cache.tools.get(id) {
        Some(entry) => now.saturating_sub(entry.checked_at) >= MAX_AGE_SECS,
        None => true,
    }
}

/// What to record, given how the check went.
///
/// `None` from the hook means it could not run — no network, a daemon down.
/// That is not evidence the tool is current, so the previous answer stands:
/// it was true yesterday, and all a failed probe tells us is that we could not
/// look. The attempt is still recorded by the caller, so we back off for a day
/// rather than paying the timeout on every command.
fn resolve_detail(outcome: Option<String>, previous: Option<&str>) -> Option<String> {
    match outcome {
        Some(output) => first_line(output),
        None => previous.map(str::to_owned),
    }
}

/// The hook's contract is one line. Trimming to it means a chatty script cannot
/// spill a paragraph into a status listing.
fn first_line(output: String) -> Option<String> {
    let line = output.lines().next()?.trim().to_owned();
    (!line.is_empty()).then_some(line)
}

fn cache_path() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    Some(
        Path::new(&home)
            .join(".cache")
            .join("toms-tools")
            .join("tool_updates.toml"),
    )
}

fn now_secs() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::{first_line, is_stale, resolve_detail, Cache, Entry, MAX_AGE_SECS};

    fn cache_with(id: &str, checked_at: u64, detail: Option<&str>) -> Cache {
        let mut cache = Cache::default();
        cache.tools.insert(
            id.to_owned(),
            Entry {
                checked_at,
                detail: detail.map(str::to_owned),
            },
        );
        cache
    }

    #[test]
    fn an_unknown_tool_is_always_stale() {
        assert!(is_stale(&Cache::default(), "nope", 0));
    }

    #[test]
    fn a_check_goes_stale_after_a_day() {
        let cache = cache_with("t", 0, None);
        assert!(!is_stale(&cache, "t", MAX_AGE_SECS - 1));
        assert!(is_stale(&cache, "t", MAX_AGE_SECS));
    }

    #[test]
    fn a_recorded_detail_means_an_update_is_waiting() {
        let cache = cache_with("t", 0, Some("v1 -> v2"));
        assert!(cache.has_update("t"));
        assert_eq!(cache.detail("t"), Some("v1 -> v2"));
    }

    /// "Checked, and current" must be distinguishable from "never checked",
    /// or a current tool would be re-probed on every command.
    #[test]
    fn a_checked_and_current_tool_is_recorded_without_a_detail() {
        let cache = cache_with("t", 100, None);
        assert!(!cache.has_update("t"));
        assert!(!is_stale(&cache, "t", 101));
    }

    #[test]
    fn cache_round_trips_through_toml() {
        let cache = cache_with("t", 42, Some("v1 -> v2"));
        let text = toml::to_string_pretty(&cache).unwrap();
        let parsed: Cache = toml::from_str(&text).unwrap();

        assert_eq!(parsed.detail("t"), Some("v1 -> v2"));
    }

    /// A check that cannot run must not quietly clear a real update.
    #[test]
    fn an_unrunnable_check_keeps_the_previous_answer() {
        assert_eq!(
            resolve_detail(None, Some("v1 -> v2")),
            Some("v1 -> v2".to_owned())
        );
        assert_eq!(resolve_detail(None, None), None);
    }

    /// But a check that *did* run and said nothing means current, and must
    /// clear a detail that is no longer true.
    #[test]
    fn a_successful_silent_check_clears_a_stale_detail() {
        assert_eq!(resolve_detail(Some(String::new()), Some("v1 -> v2")), None);
        assert_eq!(
            resolve_detail(Some("v2 -> v3\n".to_owned()), Some("v1 -> v2")),
            Some("v2 -> v3".to_owned())
        );
    }

    #[test]
    fn only_the_first_non_empty_line_is_kept() {
        assert_eq!(
            first_line("v1 -> v2\nand a paragraph\n".to_owned()),
            Some("v1 -> v2".to_owned())
        );
        assert_eq!(first_line(String::new()), None);
        assert_eq!(first_line("   \n".to_owned()), None);
    }
}
