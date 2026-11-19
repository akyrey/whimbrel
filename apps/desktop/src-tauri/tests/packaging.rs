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
