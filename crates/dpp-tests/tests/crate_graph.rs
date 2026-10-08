//! Drift tripwire: the crate dependency graph in `CONTRIBUTING.md` must match
//! the workspace manifests, and no crate may depend on an async runtime or a
//! network client.
//!
//! # Why this needs a test rather than care
//!
//! The graph is one of the few things `CONTRIBUTING.md` states as a rule rather
//! than as advice, and a rule nobody can check silently stops being true. Deriving
//! it from `cargo metadata` once found six wrong entries, five of them long-standing,
//! and a prose rule forbidding `tokio` that four crates already broke in
//! `[dev-dependencies]`. Nothing had noticed for five releases. Correcting it
//! once does not stop it drifting again.
//!
//! # What is held
//!
//! 1. **The documented graph equals the manifests'**, edge by edge, with
//!    `[dependencies]` and `[dev-dependencies]` kept apart. A dev edge to a crate
//!    already reached through `[dependencies]` is not a second edge and is not
//!    listed: `dpp-domain` takes `dpp-rules` both ways to switch on a feature for
//!    its own tests.
//! 2. **No crate's `[dependencies]` names `axum`, `tokio`, `tower`, `sqlx`,
//!    `redis` or `reqwest`.** `tokio` is allowed in `[dev-dependencies]` as a
//!    test runtime and nowhere else; the other five are allowed in neither table.
//!    This is the half with teeth beyond documentation: it is what keeps the
//!    published crates free of an async runtime and network I/O.
//! 3. **The same holds for every product group plugin, transitively.** A plugin
//!    is contractually deterministic and free of I/O (`DppProductGroupPlugin`),
//!    and it is compiled to Wasm and loaded by a host, so a runtime or network
//!    client in its tree is worse than in a library. The plugins are a separate
//!    workspace with a small lockfile, so this checks the lockfile as well as the
//!    manifests: a forbidden crate arriving two levels down fails here too. Today
//!    each plugin depends directly on `dpp-plugin-sdk` and `serde_json` only.
//!
//! # Why the manifests are read as text
//!
//! `dpp-tests` has no TOML parser, and a dependency added solely to police
//! dependencies is a poor trade. These manifests are uniform enough to read line
//! by line. The reader **fails loudly** on a form it does not understand rather
//! than skipping it, because a tripwire that ignores what it cannot parse stops
//! checking the day someone writes a new form. Its own behaviour is pinned by the
//! tests at the bottom, which feed it a drifted graph and a forbidden dependency
//! and require both to be caught.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Crates no workspace crate may depend on in `[dependencies]`.
const INFRASTRUCTURE: &[&str] = &["axum", "tokio", "tower", "sqlx", "redis", "reqwest"];

/// The one of them allowed in `[dev-dependencies]`, as a test runtime.
const TEST_RUNTIME: &str = "tokio";

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

/// What a member's `Cargo.toml` says about its dependencies, by crate name.
#[derive(Debug, Default)]
struct Manifest {
    name: String,
    /// `[dependencies]` and `[build-dependencies]`, `target.*` variants included.
    normal: BTreeSet<String>,
    /// `[dev-dependencies]`, `target.*` variants included.
    dev: BTreeSet<String>,
}

#[derive(Clone, Copy)]
enum Kind {
    Normal,
    Dev,
}

enum Header {
    Package,
    /// `[dependencies]`, `[dev-dependencies]`, `[target.….dependencies]`, …
    Section(Kind),
    /// `[dependencies.serde]`: one dependency written as its own table.
    Single(Kind, String),
    Other,
}

/// One line with its comment removed, and the net change in bracket depth.
///
/// String literals are skipped so a `#` or `[` inside one is not mistaken for
/// syntax. Depth tracks `{` and `[` so a multi-line inline table or array is read
/// as the one value it is, and its inner `key = value` lines are not taken for
/// dependencies.
fn scan(line: &str) -> (String, i32) {
    let mut out = String::new();
    let mut depth = 0;
    let mut quote: Option<char> = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' && q == '"' {
                out.extend(chars.next());
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '#' => break,
            '"' | '\'' => quote = Some(c),
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            _ => {}
        }
        out.push(c);
    }
    (out, depth)
}

/// Whether `name` occurs in the dotted header as a whole segment, and what follows.
fn segment_after<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.match_indices(name).find_map(|(at, _)| {
        let starts = at == 0 || header[..at].ends_with('.');
        let rest = &header[at + name.len()..];
        (starts && (rest.is_empty() || rest.starts_with('.'))).then_some(rest)
    })
}

fn classify(header: &str) -> Header {
    let h = header.trim().trim_matches(['[', ']']).trim();
    if h == "package" {
        return Header::Package;
    }
    for (name, kind) in [
        ("dev-dependencies", Kind::Dev),
        ("build-dependencies", Kind::Normal),
        ("dependencies", Kind::Normal),
    ] {
        match segment_after(h, name) {
            Some("") => return Header::Section(kind),
            Some(rest) => {
                let dep = rest.trim_start_matches('.').trim_matches(['"', '\'']);
                return Header::Single(kind, dep.to_owned());
            }
            None => {}
        }
    }
    Header::Other
}

fn record(kind: Kind, dep: &str, m: &mut Manifest) {
    let set = match kind {
        Kind::Normal => &mut m.normal,
        Kind::Dev => &mut m.dev,
    };
    set.insert(dep.to_owned());
}

fn parse_manifest(text: &str) -> Manifest {
    let mut m = Manifest::default();
    let mut header = Header::Other;
    let mut depth = 0;

    for raw in text.lines() {
        assert!(
            !raw.contains("\"\"\"") && !raw.contains("'''"),
            "multi-line strings are not read by this tripwire: `{raw}`"
        );
        let (code, delta) = scan(raw);
        let line = code.trim();

        if depth == 0 && line.starts_with('[') {
            header = classify(line);
            if let Header::Single(kind, dep) = &header {
                record(*kind, dep, &mut m);
            }
            continue;
        }
        if depth == 0 && !line.is_empty() {
            match &header {
                Header::Section(kind) => {
                    let Some((key, _)) = line.split_once('=') else {
                        panic!(
                            "dependency line with no `=`, which this tripwire cannot read: `{raw}`"
                        );
                    };
                    let dep = key.trim().split('.').next().unwrap_or_default();
                    record(*kind, dep.trim_matches(['"', '\'']), &mut m);
                }
                Header::Package => {
                    if let Some(value) = line
                        .strip_prefix("name")
                        .and_then(|r| r.trim_start().strip_prefix('='))
                    {
                        m.name = value.trim().trim_matches('"').to_owned();
                    }
                }
                Header::Single(..) | Header::Other => {}
            }
        }
        depth += delta;
    }
    assert!(!m.name.is_empty(), "manifest has no `[package] name`");
    m
}

/// The `members = [...]` paths of the root manifest.
fn workspace_members(root_manifest: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut inside = false;
    for raw in root_manifest.lines() {
        let (code, _) = scan(raw);
        let line = code.trim();
        if !inside {
            if !(line.starts_with("members") && line.contains('[')) {
                continue;
            }
            inside = true;
        }
        paths.extend(line.split('"').skip(1).step_by(2).map(str::to_owned));
        if line.contains(']') {
            break;
        }
    }
    paths
}

fn member_manifests() -> Vec<Manifest> {
    let root = workspace_root();
    let members = workspace_members(&read(&root.join("Cargo.toml")));
    assert!(
        members.len() > 10,
        "expected to discover the workspace members, found {} — has the root manifest moved?",
        members.len()
    );
    members
        .iter()
        .map(|dir| parse_manifest(&read(&root.join(dir).join("Cargo.toml"))))
        .collect()
}

/// Every product group plugin's manifest. Discovered, not listed.
fn plugin_manifests() -> Vec<Manifest> {
    let dir = workspace_root().join("plugins");
    let found: Vec<Manifest> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|entry| entry.path().join("Cargo.toml"))
        .filter(|manifest| manifest.is_file())
        .map(|manifest| parse_manifest(&read(&manifest)))
        .collect();
    assert!(
        found.len() >= 10,
        "expected to discover the plugins, found {} — has plugins/ moved?",
        found.len()
    );
    found
}

/// Every package name a `Cargo.lock` locks, direct or transitive.
fn locked_package_names(lock: &str) -> BTreeSet<String> {
    lock.lines()
        .filter_map(|line| line.strip_prefix("name = \""))
        .filter_map(|rest| rest.strip_suffix('"'))
        .map(str::to_owned)
        .collect()
}

/// One crate's documented edges.
#[derive(Debug, Default)]
struct Documented {
    normal: BTreeSet<String>,
    dev: BTreeSet<String>,
}

fn names(list: &str) -> impl Iterator<Item = String> + '_ {
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// The fenced graph under `### Dependency Rules` in `CONTRIBUTING.md`.
///
/// Three line shapes: `crate -> a, b`, `crate -> (parenthetical for none)`,
/// `crate -> dev only: a, b`, and an indented `+ dev: a, b` continuing the crate
/// above it. Anything else panics, so a new shape cannot be skipped unnoticed.
fn parse_documented(contributing: &str) -> BTreeMap<String, Documented> {
    let section = contributing
        .find("### Dependency Rules")
        .expect("CONTRIBUTING.md has no `### Dependency Rules` section");
    let after = &contributing[section..];
    let fence = after
        .find("```")
        .expect("no fenced block under `### Dependency Rules`");
    let body = &after[fence..];
    let body = &body[body.find('\n').expect("fence with no body") + 1..];
    let block = &body[..body.find("```").expect("unterminated fenced block")];

    let mut graph: BTreeMap<String, Documented> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in block.lines().filter(|l| !l.trim().is_empty()) {
        if line.starts_with(char::is_whitespace) {
            let dev = line
                .trim()
                .strip_prefix("+ dev:")
                .unwrap_or_else(|| panic!("unrecognised continuation line in the graph: `{line}`"));
            let owner = current.as_ref().expect("continuation before any crate");
            graph
                .entry(owner.clone())
                .or_default()
                .dev
                .extend(names(dev));
            continue;
        }
        let (name, rhs) = line
            .split_once("->")
            .unwrap_or_else(|| panic!("unrecognised line in the graph: `{line}`"));
        let (name, rhs) = (name.trim().to_owned(), rhs.trim());
        let mut entry = Documented::default();
        if rhs.starts_with('(') {
            // "(none)", "(standalone, …)": no internal edges.
        } else if let Some(dev) = rhs.strip_prefix("dev only:") {
            entry.dev.extend(names(dev));
        } else {
            entry.normal.extend(names(rhs));
        }
        assert!(
            graph.insert(name.clone(), entry).is_none(),
            "{name} appears twice in the graph"
        );
        current = Some(name);
    }
    graph
}

fn compare(
    out: &mut Vec<String>,
    krate: &str,
    table: &str,
    documented: &BTreeSet<&str>,
    actual: &BTreeSet<&str>,
) {
    for dep in actual.difference(documented) {
        out.push(format!(
            "{krate}: {table} names {dep}; CONTRIBUTING.md does not list it"
        ));
    }
    for dep in documented.difference(actual) {
        out.push(format!(
            "{krate}: CONTRIBUTING.md lists {dep} under {table}; the manifest does not name it"
        ));
    }
}

/// Every way the documented graph and the manifests disagree, one line each.
fn graph_differences(
    documented: &BTreeMap<String, Documented>,
    manifests: &[Manifest],
) -> Vec<String> {
    let members: BTreeSet<&str> = manifests.iter().map(|m| m.name.as_str()).collect();
    let mut out = Vec::new();
    for m in manifests {
        let Some(doc) = documented.get(&m.name) else {
            out.push(format!(
                "{} is a workspace member and is not in the graph",
                m.name
            ));
            continue;
        };
        let normal: BTreeSet<&str> = m
            .normal
            .iter()
            .map(String::as_str)
            .filter(|d| members.contains(d))
            .collect();
        // A dev edge to something already a normal dependency adds nothing.
        let dev: BTreeSet<&str> = m
            .dev
            .iter()
            .map(String::as_str)
            .filter(|d| members.contains(d) && !normal.contains(d))
            .collect();
        let doc_normal: BTreeSet<&str> = doc.normal.iter().map(String::as_str).collect();
        let doc_dev: BTreeSet<&str> = doc.dev.iter().map(String::as_str).collect();
        compare(&mut out, &m.name, "[dependencies]", &doc_normal, &normal);
        compare(&mut out, &m.name, "[dev-dependencies]", &doc_dev, &dev);
    }
    for name in documented.keys().filter(|n| !members.contains(n.as_str())) {
        out.push(format!(
            "{name} is in the graph and is not a workspace member"
        ));
    }
    out
}

/// Every runtime or network client a manifest names where it may not.
fn forbidden_dependencies(manifests: &[Manifest]) -> Vec<String> {
    let mut out = Vec::new();
    for m in manifests {
        for dep in INFRASTRUCTURE {
            if m.normal.contains(*dep) {
                out.push(format!("{}: [dependencies] names {dep}", m.name));
            }
            if *dep != TEST_RUNTIME && m.dev.contains(*dep) {
                out.push(format!(
                    "{}: [dev-dependencies] names {dep}; only {TEST_RUNTIME} is allowed there, as a test runtime",
                    m.name
                ));
            }
        }
    }
    out
}

#[test]
fn contributing_graph_matches_the_manifests() {
    let documented = parse_documented(&read(&workspace_root().join("CONTRIBUTING.md")));
    let differences = graph_differences(&documented, &member_manifests());
    assert!(
        differences.is_empty(),
        "the crate graph in CONTRIBUTING.md (### Dependency Rules) has drifted from \
         the manifests. Correct whichever is wrong:\n{}",
        differences.join("\n")
    );
}

#[test]
fn no_crate_depends_on_a_runtime_or_network_client() {
    let found = forbidden_dependencies(&member_manifests());
    assert!(
        found.is_empty(),
        "a workspace crate depends on an async runtime or network client, which the \
         published crates must stay free of:\n{}",
        found.join("\n")
    );
}

#[test]
fn no_plugin_depends_on_a_runtime_or_network_client_directly_or_transitively() {
    let mut found = forbidden_dependencies(&plugin_manifests());
    let locked = locked_package_names(&read(&workspace_root().join("plugins/Cargo.lock")));
    found.extend(
        INFRASTRUCTURE
            .iter()
            .filter(|dep| locked.contains(**dep))
            .map(|dep| format!("plugins/Cargo.lock locks {dep}, directly or transitively")),
    );
    assert!(
        found.is_empty(),
        "a product group plugin has an async runtime or network client in its tree, and a \
         plugin must be deterministic and free of I/O:\n{}",
        found.join("\n")
    );
}

// --- the tripwire's own tests -------------------------------------------------

fn manifest(name: &str, normal: &[&str], dev: &[&str]) -> Manifest {
    Manifest {
        name: name.to_owned(),
        normal: normal.iter().map(|s| (*s).to_owned()).collect(),
        dev: dev.iter().map(|s| (*s).to_owned()).collect(),
    }
}

#[test]
fn a_drifted_graph_is_caught_in_every_direction() {
    let manifests = member_manifests();
    let real = parse_documented(&read(&workspace_root().join("CONTRIBUTING.md")));
    assert!(
        graph_differences(&real, &manifests).is_empty(),
        "premise: the real graph agrees"
    );

    let mutate = |change: &dyn Fn(&mut BTreeMap<String, Documented>)| {
        let mut doc = parse_documented(&read(&workspace_root().join("CONTRIBUTING.md")));
        change(&mut doc);
        graph_differences(&doc, &manifests)
    };

    let dropped = mutate(&|d| {
        d.get_mut("dpp-vc").unwrap().normal.remove("dpp-vocab");
    });
    assert!(
        dropped
            .iter()
            .any(|l| l.contains("dpp-vc") && l.contains("dpp-vocab")),
        "{dropped:?}"
    );

    let invented = mutate(&|d| {
        d.get_mut("dpp-calc")
            .unwrap()
            .normal
            .insert("dpp-domain".to_owned());
    });
    assert!(
        invented
            .iter()
            .any(|l| l.contains("dpp-calc") && l.contains("dpp-domain")),
        "{invented:?}"
    );

    let demoted = mutate(&|d| {
        let tests = d.get_mut("dpp-tests").unwrap();
        tests.normal.remove("dpp-vc");
        tests.dev.insert("dpp-vc".to_owned());
    });
    assert!(
        demoted.len() >= 2,
        "a real edge listed as dev must fail both tables: {demoted:?}"
    );

    let forgotten = mutate(&|d| {
        d.remove("dpp-benches");
    });
    assert!(
        forgotten.iter().any(|l| l.contains("dpp-benches")),
        "{forgotten:?}"
    );

    let ghost = mutate(&|d| {
        d.insert("dpp-ghost".to_owned(), Documented::default());
    });
    assert!(ghost.iter().any(|l| l.contains("dpp-ghost")), "{ghost:?}");
}

#[test]
fn a_dev_edge_to_a_normal_dependency_is_not_a_second_edge() {
    let manifests = [manifest("a", &["b"], &["b"]), manifest("b", &[], &[])];
    let documented = BTreeMap::from([
        (
            "a".to_owned(),
            Documented {
                normal: ["b".to_owned()].into(),
                dev: BTreeSet::new(),
            },
        ),
        ("b".to_owned(), Documented::default()),
    ]);
    assert!(graph_differences(&documented, &manifests).is_empty());
}

#[test]
fn a_runtime_in_dependencies_is_caught_and_tokio_is_allowed_only_as_a_test_runtime() {
    let manifests = [
        manifest("ok", &["serde"], &["tokio"]),
        manifest("runtime", &["tokio"], &[]),
        manifest("client", &["serde"], &["reqwest"]),
        manifest("server", &["axum"], &[]),
    ];
    let found = forbidden_dependencies(&manifests);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found
            .iter()
            .any(|l| l.starts_with("runtime:") && l.contains("[dependencies]"))
    );
    assert!(
        found
            .iter()
            .any(|l| l.starts_with("client:") && l.contains("[dev-dependencies]"))
    );
    assert!(found.iter().any(|l| l.starts_with("server:")));
    assert!(!found.iter().any(|l| l.starts_with("ok:")));
}

#[test]
fn a_runtime_two_levels_down_a_lockfile_is_found() {
    let lock = "# generated\nversion = 4\n\n[[package]]\nname = \"product-group-x\"\nversion = \"1.0.0\"\n\
                dependencies = [\n \"helper\",\n]\n\n[[package]]\nname = \"helper\"\nversion = \"1.0.0\"\n\
                dependencies = [\n \"tokio\",\n]\n\n[[package]]\nname = \"tokio\"\nversion = \"1.0.0\"\n";
    let locked = locked_package_names(lock);
    assert!(
        locked.contains("tokio") && locked.contains("helper"),
        "{locked:?}"
    );
    assert!(!locked.contains("axum"));
}

#[test]
fn the_manifest_reader_handles_every_form_in_use_and_ignores_what_is_not_a_dependency() {
    let text = r#"
[package]
name        = "demo"   # a comment with [brackets] and "quotes"
version.workspace = true

[dependencies]
plain = { workspace = true }
dotted.workspace = true
multi = { version = "1",
          features = ["a", "b"] }
list = [
    "x",
]

[dev-dependencies]
tokio = { version = "1", features = ["rt"] }

[build-dependencies]
cc = "1"

[target.'cfg(unix)'.dependencies]
unixy = "1"

[target.'cfg(windows)'.dev-dependencies]
winy = "1"

[dependencies.tabled]
version = "1"

[dev-dependencies.tabled-dev]
version = "1"

[features]
default = ["plain"]

[[bin]]
name = "not-the-package"
"#;
    let m = parse_manifest(text);
    assert_eq!(m.name, "demo");
    assert_eq!(
        m.normal,
        ["plain", "dotted", "multi", "list", "cc", "unixy", "tabled"]
            .map(String::from)
            .into()
    );
    assert_eq!(
        m.dev,
        ["tokio", "winy", "tabled-dev"].map(String::from).into()
    );
}

#[test]
#[should_panic(expected = "multi-line strings")]
fn a_form_the_reader_cannot_follow_fails_loudly_instead_of_being_skipped() {
    parse_manifest("[package]\nname = \"x\"\ndescription = \"\"\"\nmore\n\"\"\"\n");
}

#[test]
fn the_documented_graph_reader_accepts_each_line_shape() {
    let doc = r#"
### Dependency Rules

```
a -> (none)
b -> a
c -> a, b
     + dev: d, e
f -> dev only: a
```
"#;
    let g = parse_documented(doc);
    assert!(g["a"].normal.is_empty() && g["a"].dev.is_empty());
    assert_eq!(g["b"].normal, ["a"].map(String::from).into());
    assert_eq!(g["c"].normal, ["a", "b"].map(String::from).into());
    assert_eq!(g["c"].dev, ["d", "e"].map(String::from).into());
    assert!(g["f"].normal.is_empty());
    assert_eq!(g["f"].dev, ["a"].map(String::from).into());
}
