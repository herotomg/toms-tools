//! The claude-artifacts hooks against a fake `paseo` that keeps its config in a
//! plain JSON file, so the merge logic is exercised without a real daemon.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const FAKE_PASEO: &str = r#"#!/usr/bin/env bash
# paseo daemon config get|set agents.providers [value] --home H --json
set -euo pipefail
cfg="$PASEO_HOME/config.json"
[ -f "$cfg" ] || echo '{}' > "$cfg"
case "$3" in
  get) jq '{set: (.agents.providers != null), value: .agents.providers}' "$cfg" ;;
  set)
    echo "set $5" >> "$TEST_PASEO_LOG"
    jq --argjson v "$5" '.agents.providers = $v' "$cfg" > "$cfg.tmp"
    mv "$cfg.tmp" "$cfg"
    echo '{"action":"saved"}' ;;
esac
"#;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(config: Option<&str>) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tt-claude-artifacts-{nonce}"));
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("paseo")).unwrap();
        let paseo = root.join("bin/paseo");
        fs::write(&paseo, FAKE_PASEO).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&paseo, fs::Permissions::from_mode(0o755)).unwrap();
        }
        if let Some(contents) = config {
            fs::write(root.join("paseo/config.json"), contents).unwrap();
        }
        Self { root }
    }

    fn run(&self, hook: &str) -> String {
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tools/claude-artifacts")
            .join(hook);
        let path = format!(
            "{}:{}",
            self.root.join("bin").display(),
            std::env::var("PATH").unwrap()
        );
        let output = Command::new("bash")
            .arg(script)
            .env("PATH", path)
            .env("HOME", &self.root)
            .env("PASEO_HOME", self.root.join("paseo"))
            .env("TEST_PASEO_LOG", self.root.join("paseo.log"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(output.status.success(), "{hook} failed: {stderr}");
        stderr
    }

    fn providers(&self) -> String {
        let raw = fs::read_to_string(self.root.join("paseo/config.json")).unwrap();
        jq(".agents.providers", &raw)
    }

    fn writes(&self) -> usize {
        fs::read_to_string(self.root.join("paseo.log"))
            .unwrap_or_default()
            .lines()
            .count()
    }

    fn owned(&self) -> bool {
        self.root
            .join(".local/share/toms-tools/claude-artifacts/enabled-flag")
            .exists()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Canonical, key-sorted JSON so comparisons ignore formatting and key order.
fn jq(filter: &str, input: &str) -> String {
    let mut child = Command::new("jq")
        .args(["-cS", filter])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("jq is installed");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn normalise(json: &str) -> String {
    jq(".", json)
}

const EXISTING: &str = r#"{"version":1,"agents":{"providers":{
  "claude":{"env":{"ANTHROPIC_LOG":"debug"},"additionalModels":[],"enabled":true},
  "codex":{"env":{"OPENAI_BASE_URL":"https://example.test"}},
  "pi":{"enabled":false}}}}"#;

#[test]
fn install_merges_the_flag_as_a_string_and_preserves_everything_else() {
    let fx = Fixture::new(Some(EXISTING));
    fx.run("install.sh");

    assert_eq!(
        fx.providers(),
        normalise(
            r#"{"claude":{"env":{"ANTHROPIC_LOG":"debug","CLAUDE_CODE_ARTIFACT":"1"},
               "additionalModels":[],"enabled":true},
               "codex":{"env":{"OPENAI_BASE_URL":"https://example.test"}},
               "pi":{"enabled":false}}"#
        )
    );
    assert!(fx.owned());
}

#[test]
fn install_is_idempotent() {
    let fx = Fixture::new(Some(EXISTING));
    fx.run("install.sh");
    let after_first = fx.providers();
    let stderr = fx.run("install.sh");

    assert_eq!(fx.providers(), after_first);
    assert_eq!(fx.writes(), 1, "a second install must not write the config");
    assert!(stderr.contains("nothing changed"));
}

#[test]
fn install_works_with_no_providers_configured() {
    let fx = Fixture::new(None);
    fx.run("install.sh");
    assert_eq!(
        fx.providers(),
        normalise(r#"{"claude":{"env":{"CLAUDE_CODE_ARTIFACT":"1"}}}"#)
    );
}

#[test]
fn remove_restores_the_original_providers() {
    let fx = Fixture::new(Some(EXISTING));
    let before = fx.providers();
    fx.run("install.sh");
    fx.run("uninstall.sh");
    assert_eq!(fx.providers(), before);
}

#[test]
fn a_flag_the_user_set_themselves_is_neither_claimed_nor_removed() {
    let fx = Fixture::new(Some(
        r#"{"agents":{"providers":{"claude":{"env":{"CLAUDE_CODE_ARTIFACT":"1"}}}}}"#,
    ));
    let before = fx.providers();
    fx.run("install.sh");
    assert!(!fx.owned());
    let stderr = fx.run("uninstall.sh");

    assert_eq!(fx.providers(), before);
    assert_eq!(fx.writes(), 0);
    assert!(stderr.contains("left alone"));
}
