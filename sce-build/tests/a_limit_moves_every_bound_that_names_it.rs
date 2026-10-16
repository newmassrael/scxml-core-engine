// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `sce:limit` (docs/SCE_ACCEPTED_SUBSET.md §2.15): the bound a deployment may move.
//
// A `sce-static` list and the algorithm that returns it are held to one bound: a
// list may not be smaller than what the algorithm may return, because a machine holds
// the same list wherever it runs. A build that wants a larger one (a team calendar on a
// server, where a room on a microcontroller holds a few) cannot edit the number in one
// file and leave the other behind. `sce:limit="name"` beside a `capacity` or a
// `returns-max-size` is the name that ties them, and `sce-codegen --limit name=N` is the
// number a build gives it; a document built without one is the document as written.
//
// What is asked here is that one name moves the machine's list and the algorithm's
// buffer and result together, so that what was refused when only one moved is built when
// both do; that a bound that is NOT named does not move, and the build says so the way it
// always has (the algorithm holds more than the list); that the numbers a build was given
// are in the `source-hash` of what it wrote, so that a tree built larger is not taken for
// the tree built as written; that what is not a limit is refused with its reason; and,
// of the repository's own documents, that every bound that shares a name writes the same
// number, which is what makes the name one bound.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The algorithm: `count` copies of `value`, the most `bound` of them. `limit` is what
/// its `capacity` and `returns-max-size` carry, `" sce:limit=\"room\""` or nothing.
fn algorithm(bound: u32, limit: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="copies" version="1.0">
  <sce:signature>
    <sce:param name="value" type="uint8"/>
    <sce:param name="count" type="uint8"/>
    <sce:return type="list&lt;uint8&gt;" returns-max-size="{bound}"{limit} may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;uint8&gt;" capacity="{bound}"{limit}/>
    <sce:var name="i" type="uint8" init="0"/>
    <sce:while cond="i &lt; count" max-iter="8">
      <sce:append target="out" expr="value"/>
      <sce:assign target="i" expr="i + 1"/>
    </sce:while>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"##
    )
}

/// The statechart that assigns the algorithm's list to `shown`, whose own bound is
/// `bound` and carries `limit`.
fn machine(bound: u32, limit: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="algorithm" src="copies.scxml" as="Copies"/>
  <datamodel>
    <data id="shown" sce:type="list&lt;uint8&gt;" sce:capacity="{bound}"{limit}/>
    <data id="n" sce:type="uint8" expr="3"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="shown" expr="Copies(n, 2)"/>
    </transition>
  </state>
</scxml>
"##
    )
}

const ROOM: &str = " sce:limit=\"room\"";

/// The two documents in a directory of their own.
fn documents(algorithm_text: &str, machine_text: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("copies.scxml"), algorithm_text).expect("write algorithm");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, machine_text).expect("write probe");
    (dir, path)
}

fn codegen(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `check` of the statechart under the limits `given`.
fn check(machine_text: &str, algorithm_text: &str, given: &[&str]) -> (bool, String) {
    let (_dir, path) = documents(algorithm_text, machine_text);
    let mut args: Vec<&str> = Vec::new();
    for limit in given {
        args.push("--limit");
        args.push(limit);
    }
    args.extend(["check", "-l", "rust"]);
    codegen(&args, &path)
}

#[test]
fn a_document_without_a_deployment_is_built_as_written() {
    // The list holds eight, the algorithm four: it is taken, as it is without a name.
    let (ok, out) = check(&machine(8, ROOM), &algorithm(4, ROOM), &[]);
    assert!(
        ok,
        "a list that can hold what the algorithm returns:\n{out}"
    );
    // And a list of three cannot, named or not.
    let (ok, out) = check(&machine(3, ROOM), &algorithm(4, ROOM), &[]);
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(out.contains("at most 4"), "the refusal must say 4:\n{out}");
}

#[test]
fn one_name_moves_the_list_and_the_algorithm_together() {
    // Written, the list holds three and the algorithm may return four: refused, as above.
    // Both under `room=2` hold two, and a list that holds all the algorithm may return
    // is taken: the name moved the two together, which editing one file could not.
    let (ok, out) = check(&machine(3, ROOM), &algorithm(4, ROOM), &["room=2"]);
    assert!(ok, "both moved to two:\n{out}");
    let (ok, out) = check(&machine(3, ROOM), &algorithm(4, ROOM), &["room=4096"]);
    assert!(ok, "both moved to 4096:\n{out}");
}

#[test]
fn a_bound_that_is_not_named_does_not_move() {
    // The algorithm names its bound and the list does not: `room=9` moves the algorithm to
    // nine, the list stays at four, and the list can no longer hold what the algorithm
    // may return. The build says so as it always has.
    let (ok, out) = check(&machine(4, ""), &algorithm(4, ROOM), &["room=9"]);
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(out.contains("at most 9"), "the refusal must say 9:\n{out}");
}

#[test]
fn what_a_build_gives_the_name_is_what_the_files_it_writes_hold() {
    let (dir, path) = documents(&algorithm(4, ROOM), &machine(4, ROOM));
    let algorithm_path = dir.path().join("copies.scxml");
    // The statechart is generated without the algorithm it imports, so each is
    // generated by name, under the same limits.
    let produced = |given: &[&str]| -> (String, String) {
        let out = tempdir().expect("tempdir");
        for document in [&path, &algorithm_path] {
            let mut args: Vec<&str> = Vec::new();
            for limit in given {
                args.push("--limit");
                args.push(limit);
            }
            args.extend([
                "generate",
                "-l",
                "c",
                "-o",
                out.path().to_str().expect("utf-8"),
            ]);
            let (ok, text) = codegen(&args, document);
            assert!(ok, "{given:?} {}:\n{text}", document.display());
        }
        let machine = std::fs::read_to_string(out.path().join("probe_sm.h")).expect("the machine");
        let algorithm =
            std::fs::read_to_string(out.path().join("copies.h")).expect("the algorithm");
        (machine, algorithm)
    };
    let (machine_as_written, algorithm_as_written) = produced(&[]);
    assert!(
        algorithm_as_written.contains("items[4]"),
        "as written the algorithm holds four"
    );
    let (machine_moved, algorithm_moved) = produced(&["room=7"]);
    assert!(
        algorithm_moved.contains("items[7]") && !algorithm_moved.contains("items[4]"),
        "the algorithm's result holds seven:\n{algorithm_moved}"
    );
    assert!(
        machine_moved.contains("data[7]") && !machine_moved.contains("data[4]"),
        "the machine's list holds seven:\n{machine_moved}"
    );
    // A tree built larger is not the tree built as written: its `source-hash` is its own.
    let hash = |text: &str| -> String {
        text.lines()
            .find(|line| line.contains("source-hash:"))
            .expect("a header")
            .to_string()
    };
    assert_ne!(hash(&machine_as_written), hash(&machine_moved));
    assert_ne!(hash(&algorithm_as_written), hash(&algorithm_moved));
    // And the same numbers in another order are the same tree.
    let (again, _) = produced(&["room=7"]);
    assert_eq!(hash(&machine_moved), hash(&again));
}

#[test]
fn what_is_not_a_limit_is_refused_with_its_reason() {
    for (given, says) in [
        ("room", "name=N"),
        ("Room=4", "lower-case"),
        ("room=0", "positive"),
        ("room=4294967296", "32 bits"),
    ] {
        let (ok, out) = check(&machine(4, ROOM), &algorithm(4, ROOM), &[given]);
        assert!(!ok, "--limit {given} was taken:\n{out}");
        assert!(
            out.contains(says),
            "--limit {given} must say `{says}`:\n{out}"
        );
    }
    let (ok, out) = check(
        &machine(4, ROOM),
        &algorithm(4, ROOM),
        &["room=4", "room=5"],
    );
    assert!(!ok, "a name given twice was taken:\n{out}");
    assert!(out.contains("twice"), "{out}");
}

/// Every `sce:limit` the repository's own documents write, with the bound beside it.
fn limits_the_repository_writes() -> BTreeMap<String, Vec<(PathBuf, u32)>> {
    let mut found: BTreeMap<String, Vec<(PathBuf, u32)>> = BTreeMap::new();
    for root in ["stdlib", "sce-build/tests/fixtures/static_datamodel"] {
        let mut pending = vec![repo_root().join(root)];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("a directory") {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "scxml") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).expect("a document");
                if !text.contains("sce:limit") {
                    continue;
                }
                let doc = roxmltree::Document::parse(&text).expect("a document that parses");
                for node in doc.descendants().filter(|n| n.is_element()) {
                    let Some(name) = node.attribute(("http://sce.dev/ext", "limit")) else {
                        continue;
                    };
                    let bound = node
                        .attribute("capacity")
                        .or_else(|| node.attribute(("http://sce.dev/ext", "capacity")))
                        .or_else(|| node.attribute("returns-max-size"))
                        .unwrap_or_else(|| {
                            panic!(
                                "{}: sce:limit=\"{name}\" is beside no capacity or returns-max-size",
                                path.display()
                            )
                        })
                        .parse::<u32>()
                        .expect("a bound is a number");
                    found
                        .entry(name.to_string())
                        .or_default()
                        .push((path.clone(), bound));
                }
            }
        }
    }
    found
}

#[test]
fn the_bounds_of_the_repository_that_share_a_name_write_one_number() {
    let found = limits_the_repository_writes();
    assert!(!found.is_empty(), "the repository names no bound");
    for (name, sites) in &found {
        assert!(
            name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "`{name}` is not a name `--limit` can give a number"
        );
        let first = sites[0].1;
        for (path, bound) in sites {
            assert_eq!(
                *bound,
                first,
                "sce:limit=\"{name}\" is {first} in {} and {bound} in {}: one name is one bound",
                sites[0].0.display(),
                path.display()
            );
        }
    }
}
