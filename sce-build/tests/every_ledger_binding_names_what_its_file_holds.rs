// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every binding a ledger keeps names a file that exists and, when it names a
// symbol, a symbol that file still holds.
//
// A ledger's bindings are (section, file, symbol, kind) rows in its atomic
// store, and `mnemosyne-cli validate-code-refs` is what checks them. It checks
// them from the citation side: every cite in a scanned file must be bound
// (`citation_unbound`), a binding's file must carry a cite of its section
// (`binding_unbacked`), and a cite must lie inside a symbol bound to its section
// (`symbol_mismatch`). It does not check that a bound symbol still exists. A
// function renamed or folded into another one, with its cite moved along, leaves
// the old row behind: the file still cites the section, the new symbol is bound
// as well, and every count reads zero.
//
// That is how they piled up. On 2026-11-07 the SCXML ledger held 19 bindings
// whose symbol its file no longer held, and the synthesis ledger 3 more, with
// `symbol_mismatch_count: 0` and `binding_unbacked_count: 0` over the same
// stores. Six of the 19 named one function of the Rust engine that a refactor
// had split in two; the refactor bound the two halves and left the old name.
//
// The population is derived: every directory directly under `docs/spec` and
// `docs/sce-ledger` that holds a `mnemosyne.toml` is a ledger, so a new ledger
// is read the day it is created. Every failure is reported, naming the ledger,
// the section, the file and the symbol, so one run gives the whole repair list.
// A floor on the bindings read keeps a store that parsed into nothing from
// passing.
//
// The check is textual on purpose: the symbol has to appear in its file as a
// whole word. That is the known limit. A symbol that survives only in a comment,
// or as another item of the same name, passes here. Resolving a symbol to the
// item that defines it is Mnemosyne's job, and its resolvers are where that
// check belongs; this one only refuses the rows nothing could resolve.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The directories whose children may be ledgers.
const LEDGER_PARENTS: &[&str] = &["docs/spec", "docs/sce-ledger"];

/// The ledgers there were on 2026-11-07: `docs/spec/{scxml,synth}` and
/// `docs/sce-ledger/{mesh,wire,bytesguard}`. Fewer means the walk stopped
/// finding them, not that they went away.
const MIN_LEDGERS: usize = 5;

/// Measured 2026-11-07: 5247 bindings across the five stores. The floor sits
/// well below that so a cleanup does not trip it; what it refuses is a walk or a
/// parse that read next to nothing.
const MIN_BINDINGS: usize = 4000;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every ledger directory, repository-relative and sorted.
fn ledgers(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for parent in LEDGER_PARENTS {
        let entries =
            fs::read_dir(root.join(parent)).unwrap_or_else(|e| panic!("read {parent}: {e}"));
        for entry in entries {
            let entry = entry.unwrap_or_else(|e| panic!("read an entry of {parent}: {e}"));
            if entry.path().join("mnemosyne.toml").is_file() {
                found.push(format!("{parent}/{}", entry.file_name().to_string_lossy()));
            }
        }
    }
    found.sort();
    found
}

/// One binding as a store keeps it.
#[derive(Debug)]
struct Binding {
    section: String,
    file: String,
    symbol: Option<String>,
}

/// Every binding of every section in `store`. A store of another shape is a
/// failure, never an empty answer.
fn bindings_of(ledger: &str, store: &Value) -> Vec<Binding> {
    let sections = store
        .get("sections")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{ledger}: the store has no `sections` object"));
    let mut found = Vec::new();
    for (section, body) in sections {
        let Some(bindings) = body.get("bindings") else {
            continue;
        };
        let bindings = bindings
            .as_array()
            .unwrap_or_else(|| panic!("{ledger}: the bindings of {section} are not a list"));
        for binding in bindings {
            let file = binding
                .get("file")
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    panic!("{ledger}: a binding of {section} names no file: {binding}")
                });
            let symbol = binding.get("symbol").map(|symbol| {
                symbol
                    .as_str()
                    .unwrap_or_else(|| {
                        panic!("{ledger}: a symbol of {section} is not text: {binding}")
                    })
                    .to_string()
            });
            found.push(Binding {
                section: section.clone(),
                file: file.to_string(),
                symbol,
            });
        }
    }
    found
}

/// Whether `word` appears in `text` with no identifier character on either
/// side. An empty `word` never does, so an empty symbol is reported.
fn names_as_a_word(text: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let continues = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    text.match_indices(word).any(|(at, _)| {
        !continues(text[..at].chars().next_back())
            && !continues(text[at + word.len()..].chars().next())
    })
}

/// The bound files' text, each read once: a file bound hundreds of times is
/// not read hundreds of times.
struct Files {
    root: PathBuf,
    read: BTreeMap<String, Option<String>>,
}

impl Files {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            read: BTreeMap::new(),
        }
    }

    /// The text of `file`, relative to the root; `None` when it is not a file.
    fn text(&mut self, file: &str) -> Option<&str> {
        let root = &self.root;
        self.read
            .entry(file.to_string())
            .or_insert_with(|| {
                let path = root.join(file);
                if !path.is_file() {
                    return None;
                }
                let bytes =
                    fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
                Some(String::from_utf8_lossy(&bytes).into_owned())
            })
            .as_deref()
    }
}

/// Each binding of `bindings` whose file does not exist, or whose symbol that
/// file does not hold as a word, as one line naming all of it.
fn dead_bindings(ledger: &str, bindings: &[Binding], files: &mut Files) -> Vec<String> {
    let mut dead = Vec::new();
    for binding in bindings {
        let Some(text) = files.text(&binding.file) else {
            dead.push(format!(
                "{ledger}: {} binds {}, which is not a file",
                binding.section, binding.file
            ));
            continue;
        };
        if let Some(symbol) = &binding.symbol {
            if !names_as_a_word(text, symbol) {
                dead.push(format!(
                    "{ledger}: {} binds {} `{symbol}`, which the file does not hold as a word",
                    binding.section, binding.file
                ));
            }
        }
    }
    dead
}

#[test]
fn every_ledger_binding_names_what_its_file_holds() {
    let root = repo_root();
    let ledgers = ledgers(&root);
    assert!(
        ledgers.len() >= MIN_LEDGERS,
        "found {} ledger(s) under {LEDGER_PARENTS:?}, at least {MIN_LEDGERS} expected: {ledgers:?}",
        ledgers.len()
    );

    let mut files = Files::new(root.clone());
    let mut read = 0;
    let mut dead = Vec::new();
    for ledger in &ledgers {
        // A binding's file is relative to the workspace root, which every
        // ledger here declares as the repository root, and the store is where
        // the ledger says it is. A ledger laid out otherwise is refused rather
        // than read against the wrong root.
        let store = format!("{ledger}/.atomic/workspace.atomic.json");
        let config_path = root.join(ledger).join("mnemosyne.toml");
        let config = fs::read_to_string(&config_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", config_path.display()));
        let declares = |line: &str| config.lines().any(|l| l.trim() == line);
        assert!(
            declares(r#"root = "../../..""#),
            "{ledger}: mnemosyne.toml does not declare the repository root as the \
             workspace root, so its bindings' paths are not repository-relative"
        );
        assert!(
            declares(format!(r#"sidecar_path = "{store}""#).as_str()),
            "{ledger}: mnemosyne.toml does not keep its store at {store}"
        );

        let text =
            fs::read_to_string(root.join(&store)).unwrap_or_else(|e| panic!("read {store}: {e}"));
        let parsed: Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {store}: {e}"));
        let bindings = bindings_of(ledger, &parsed);
        assert!(
            !bindings.is_empty(),
            "{ledger}: the store parsed into no binding at all"
        );
        read += bindings.len();
        dead.extend(dead_bindings(ledger, &bindings, &mut files));
    }

    assert!(
        read >= MIN_BINDINGS,
        "read {read} binding(s) across {} ledger(s), at least {MIN_BINDINGS} expected",
        ledgers.len()
    );
    assert!(
        dead.is_empty(),
        "{} of {read} binding(s) name what their file does not hold. Re-point each to \
         the symbol that implements the section now, or remove it with a reason, through \
         mnemosyne-cli remove-section-binding and add-section-binding:\n{}",
        dead.len(),
        dead.join("\n")
    );
}

/// The word test the sweep rests on: a prefix, a suffix or a longer name is not
/// the symbol, and a qualified name is matched whole.
#[test]
fn a_symbol_is_held_only_as_a_whole_word() {
    assert!(names_as_a_word("fn kept() {}", "kept"));
    assert!(names_as_a_word("kept", "kept"));
    assert!(names_as_a_word("fn kept_longer() {} fn kept() {}", "kept"));
    assert!(names_as_a_word("void Foo::bar(int x);", "Foo::bar"));
    assert!(names_as_a_word("Foo::~Foo() {}", "Foo::~Foo"));
    assert!(!names_as_a_word("fn kept_longer() {}", "kept"));
    assert!(!names_as_a_word("fn unkept() {}", "kept"));
    assert!(!names_as_a_word("let x = Foo::barrel;", "Foo::bar"));
    assert!(!names_as_a_word("", "kept"));
    assert!(!names_as_a_word("fn kept() {}", ""));
}

/// The sweep reports every dead binding, not the first: a missing file, a
/// symbol gone, and a symbol present only as part of a longer name, while a
/// live symbol and a file-level binding pass.
#[test]
fn every_dead_binding_is_reported_not_the_first() {
    let dir = tempfile::tempdir().expect("make a temporary directory");
    fs::write(
        dir.path().join("a.rs"),
        "fn kept() {}\nfn kept_longer() {}\n",
    )
    .expect("write a.rs");
    let store: Value = serde_json::json!({
        "sections": {
            "s-1": { "bindings": [
                { "file": "a.rs", "symbol": "kept", "kind": "implements" },
                { "file": "a.rs", "symbol": "gone", "kind": "implements" }
            ]},
            "s-2": { "bindings": [
                { "file": "a.rs", "symbol": "kept_long", "kind": "references" },
                { "file": "a.rs", "kind": "references" },
                { "file": "missing.rs", "kind": "references" }
            ]},
            "s-3": { "title": "no bindings" }
        }
    });
    let bindings = bindings_of("fixture", &store);
    assert_eq!(bindings.len(), 5, "every binding of every section is read");

    let mut files = Files::new(dir.path().to_path_buf());
    let dead = dead_bindings("fixture", &bindings, &mut files);
    assert_eq!(
        dead,
        vec![
            "fixture: s-1 binds a.rs `gone`, which the file does not hold as a word".to_string(),
            "fixture: s-2 binds a.rs `kept_long`, which the file does not hold as a word"
                .to_string(),
            "fixture: s-2 binds missing.rs, which is not a file".to_string(),
        ]
    );
}
