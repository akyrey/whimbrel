//! Task 8.8: the app's name, identifier and version each have one source, and the release
//! configuration is what the shared workflow expects.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use serde_json::Value;

fn json(path: &str) -> Value {
    let text = std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn the_version_comes_only_from_cargo() {
    let conf = json("tauri.conf.json");
    let package = json("../package.json");

    assert!(
        conf.get("version").is_none(),
        "set the version in the workspace Cargo.toml"
    );
    assert_eq!(
        package["version"],
        env!("CARGO_PKG_VERSION"),
        "package.json version"
    );
}

#[test]
fn the_bundle_is_named_and_identified_once() {
    let conf = json("tauri.conf.json");

    assert_eq!(conf["productName"], "Whimbrel");
    assert_eq!(conf["identifier"], "dev.whimbrel.desktop");
    assert_eq!(conf["app"]["windows"][0]["title"], "Whimbrel");
    for icon in conf["bundle"]["icon"].as_array().unwrap() {
        let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), icon.as_str().unwrap());
        assert!(std::path::Path::new(&path).is_file(), "{path}");
    }
}

#[test]
fn linux_builds_deb_rpm_and_appimage_without_overriding_the_identity() {
    let linux = json("tauri.linux.conf.json");

    let targets: Vec<&str> = linux["bundle"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert_eq!(targets, ["deb", "rpm", "appimage"]);
    for key in ["productName", "identifier", "version"] {
        assert!(linux.get(key).is_none(), "{key} comes from tauri.conf.json");
    }
}

#[test]
fn updates_come_signed_from_whimbrels_own_releases() {
    let conf = json("tauri.conf.json");
    let updater = &conf["plugins"]["updater"];

    assert_eq!(
        updater["endpoints"][0],
        "https://github.com/akyrey/whimbrel/releases/latest/download/latest.json"
    );
    let key = updater["pubkey"].as_str().unwrap();
    assert!(!key.is_empty());
    assert_eq!(
        json("tauri.updater.conf.json")["bundle"]["createUpdaterArtifacts"],
        true
    );
}

/// The lines under the `## [version]` heading of CHANGELOG.md, or `None` without one.
fn changelog_section(changelog: &str, version: &str) -> Option<String> {
    let heading = format!("## [{version}]");
    let mut lines = changelog
        .lines()
        .skip_while(|line| !line.starts_with(&heading));
    lines.next()?;
    let body: Vec<&str> = lines.take_while(|line| !line.starts_with("## [")).collect();
    Some(body.join("\n").trim().to_owned())
}

#[test]
fn the_changelog_has_notes_for_the_version() {
    // The release workflow publishes this section as the release notes and fails without it.
    let path = format!("{}/../../../CHANGELOG.md", env!("CARGO_MANIFEST_DIR"));
    let changelog = std::fs::read_to_string(path).unwrap();
    let version = env!("CARGO_PKG_VERSION");

    let notes = changelog_section(&changelog, version)
        .unwrap_or_else(|| panic!("CHANGELOG.md has no section for {version}"));
    assert!(
        !notes.is_empty(),
        "CHANGELOG.md's section for {version} is empty"
    );
}
