//! Tripwire: the product group plugins carry the workspace version.
//!
//! # Why this needs a test rather than care
//!
//! The Wasm plugins are excluded from the workspace, so `cargo` never moves their
//! version when the workspace's moves. For nine releases nothing did: it sat at
//! 0.2.0 from core 0.11.0 to 0.20.0 while the artifacts changed underneath it,
//! and nothing could notice, because a host negotiates on `AbiVersion` and
//! `SchemaVersionRange` and never reads the number. It is an *identity*: whatever
//! caches, pins or reports a plugin by version was describing nine releases' worth
//! of different artifacts with one string.
//!
//! # Why lockstep, and not "bump it when a plugin changes"
//!
//! The obvious tripwire is "fail when `plugins/` changed since the last tag and
//! the version did not". It is blind to most of what changes a plugin. Every
//! artifact compiles `dpp-plugin-sdk`, `dpp-plugin-traits` and `dpp-rules`, which
//! live under `crates/`, and across the eleven release intervals from 0.11.0 to
//! 0.21.0 `plugins/` changed in four while that dependency closure changed in
//! seven. A change to a validation rule in `dpp-rules` changes what a plugin
//! accepts and touches nothing under `plugins/`.
//!
//! So the plugins follow the workspace version, which every release already
//! moves, and this test holds the two equal. The price is a plugin version that
//! moves in a release where no plugin changed. Nothing reads it to decide
//! anything, so that costs nothing; the opposite error cost nine releases.
//!
//! # What is held
//!
//! 1. `plugins/Cargo.toml` and the root `Cargo.toml` declare the same
//!    `[workspace.package] version`.
//! 2. Every plugin inherits it (`version.workspace = true`) and sets no version of
//!    its own.
//! 3. Every plugin reports it, by passing `env!("CARGO_PKG_VERSION")` as its
//!    identity's version, rather than a literal that would stop following it.
//!
//! Like the other manifest tripwires here, the manifests are read as text.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/dpp-tests.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/dpp-tests sits two levels below the workspace root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The `version = "…"` under `[workspace.package]`.
fn workspace_version(manifest: &str) -> Option<String> {
    let mut inside = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[workspace.package]";
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "version" {
            return Some(value.split('#').next()?.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// Whether a plugin crate's `[package]` takes its version from the workspace and
/// sets none of its own.
fn inherits_version(manifest: &str) -> bool {
    let mut inside = false;
    let (mut inherits, mut literal) = (false, false);
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[package]";
            continue;
        }
        if !inside {
            continue;
        }
        let compact: String = line
            .split('#')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        inherits |= compact == "version.workspace=true";
        literal |= compact.starts_with("version=");
    }
    inherits && !literal
}

/// Everything that stops the plugins following the workspace version.
fn failures(
    root_manifest: &str,
    plugins_manifest: &str,
    plugins: &[(String, String, String)],
) -> Vec<String> {
    let mut out = Vec::new();
    match (
        workspace_version(root_manifest),
        workspace_version(plugins_manifest),
    ) {
        (Some(core), Some(plugin)) if core != plugin => out.push(format!(
            "plugins/Cargo.toml declares {plugin} and the workspace declares {core}; set the \
             plugins' [workspace.package] version to {core} and run `cargo update -w` in plugins/"
        )),
        (Some(_), Some(_)) => {}
        _ => out.push("a [workspace.package] version could not be read".to_owned()),
    }
    for (name, manifest, lib) in plugins {
        if !inherits_version(manifest) {
            out.push(format!(
                "{name}: [package] must say `version.workspace = true` and no other version"
            ));
        }
        if !lib.contains("version: env!(\"CARGO_PKG_VERSION\")") {
            out.push(format!(
                "{name}: its identity must report `env!(\"CARGO_PKG_VERSION\")`"
            ));
        }
    }
    out
}

fn real_plugins() -> Vec<(String, String, String)> {
    let dir = workspace_root().join("plugins");
    let mut plugins: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("Cargo.toml").is_file() && p.join("src/lib.rs").is_file())
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (
                name,
                read(&p.join("Cargo.toml")),
                read(&p.join("src/lib.rs")),
            )
        })
        .collect();
    plugins.sort();
    assert!(
        plugins.len() >= 10,
        "expected to discover the plugins, found {}",
        plugins.len()
    );
    plugins
}

#[test]
fn the_plugins_carry_the_workspace_version() {
    let root = workspace_root();
    let found = failures(
        &read(&root.join("Cargo.toml")),
        &read(&root.join("plugins/Cargo.toml")),
        &real_plugins(),
    );
    assert!(
        found.is_empty(),
        "the product group plugins have stopped following the workspace version, so two \
         different builds can report the same one:\n{}",
        found.join("\n")
    );
}

// --- the tripwire's own tests -------------------------------------------------

const LIB: &str = "PluginIdentity { version: env!(\"CARGO_PKG_VERSION\"), name: \"x\" }";

fn root(version: &str) -> String {
    format!(
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion     = \"{version}\"\nedition = \"2024\"\n"
    )
}

fn plugin(manifest: &str, lib: &str) -> Vec<(String, String, String)> {
    vec![(
        "product-group-x".to_owned(),
        manifest.to_owned(),
        lib.to_owned(),
    )]
}

#[test]
fn a_plugin_version_that_falls_behind_the_workspace_is_caught() {
    let ok = plugin("[package]\nname = \"x\"\nversion.workspace = true\n", LIB);
    assert!(failures(&root("0.22.0"), &root("0.22.0"), &ok).is_empty());

    let behind = failures(&root("0.22.0"), &root("0.21.0"), &ok);
    assert_eq!(behind.len(), 1, "{behind:?}");
    assert!(
        behind[0].contains("0.21.0") && behind[0].contains("0.22.0"),
        "{behind:?}"
    );
}

#[test]
fn a_plugin_that_stops_inheriting_or_reporting_the_version_is_caught() {
    let own = plugin("[package]\nname = \"x\"\nversion = \"0.1.0\"\n", LIB);
    assert_eq!(failures(&root("1.0.0"), &root("1.0.0"), &own).len(), 1);

    let none = plugin("[package]\nname = \"x\"\n", LIB);
    assert_eq!(failures(&root("1.0.0"), &root("1.0.0"), &none).len(), 1);

    let hardcoded = plugin(
        "[package]\nname = \"x\"\nversion.workspace = true\n",
        "PluginIdentity { version: \"0.1.0\", name: \"x\" }",
    );
    assert_eq!(
        failures(&root("1.0.0"), &root("1.0.0"), &hardcoded).len(),
        1
    );
}

#[test]
fn the_version_readers_ignore_comments_and_other_tables() {
    let manifest = "# version = \"9.9.9\"\n[workspace.dependencies]\nversion = \"8.8.8\"\n\n\
                    [workspace.package]\nedition = \"2024\"\nversion = \"1.2.3\" # lockstep\n";
    assert_eq!(workspace_version(manifest).as_deref(), Some("1.2.3"));
    assert!(inherits_version(
        "[package]\nversion.workspace = true # inherited\n"
    ));
    assert!(!inherits_version(
        "[dependencies]\nversion.workspace = true\n"
    ));
}
