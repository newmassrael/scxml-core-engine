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

/// The C11 backend returns an algorithm's list by value on the stack, a few copies of it at
/// a call, so a bound a stack cannot hold is a crash at the call and not a refusal the
/// machine could report. It is refused where the bound is built in, by the algorithm and by
/// the machine that calls it, and the backends whose lists live on the heap take it.
#[test]
fn a_bound_the_c_stack_cannot_hold_is_refused_where_it_is_built_in() {
    let (dir, path) = documents(&algorithm(4, ROOM), &machine(4, ROOM));
    let algorithm_path = dir.path().join("copies.scxml");
    let generate_c = |given: &str, document: &Path| -> (bool, String) {
        let out = tempdir().expect("tempdir");
        codegen(
            &[
                "--limit",
                given,
                "generate",
                "-l",
                "c",
                "-o",
                out.path().to_str().expect("utf-8"),
            ],
            document,
        )
    };
    // A list of bytes holds a byte an element: 200000 of them are 200 KB, which a stack
    // holds, and 300000 are more than the 256 KiB a list may take.
    for document in [&path, &algorithm_path] {
        let (ok, out) = generate_c("room=200000", document);
        assert!(ok, "{}: a bound a stack holds:\n{out}", document.display());
        let (ok, out) = generate_c("room=300000", document);
        assert!(
            !ok,
            "{}: expected a refusal, got:\n{out}",
            document.display()
        );
        assert!(
            out.contains("stack") && out.contains("--limit"),
            "{}: the refusal must say what to do:\n{out}",
            document.display()
        );
    }
    // The backends whose lists live on the heap take the same bound.
    for language in ["rust", "kotlin", "cpp", "python"] {
        let (_dir, path) = documents(&algorithm(4, ROOM), &machine(4, ROOM));
        let (ok, out) = codegen(&["--limit", "room=300000", "check", "-l", language], &path);
        assert!(ok, "{language} takes the bound:\n{out}");
    }
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

/// A schema whose text field holds `bound` bytes, its `sce:max-size` carrying `limit`.
fn labelled(bound: u32, limit: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="labelled" sce:event-name="labelled">
  <datamodel>
    <data id="text" sce:type="string" sce:max-size="{bound}"{limit} sce:direction="in"/>
  </datamodel>
</scxml>
"##
    )
}

/// The statechart whose record of `labelled` starts at six bytes of text.
const STARTS_AT_SIX: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="labelled.scxml" as="Labelled"/>
  <datamodel>
    <data id="shown" sce:type="record:Labelled">
      <sce:set name="text" expr="'abcdef'"/>
    </data>
  </datamodel>
  <state id="s"/>
</scxml>
"##;

/// `check` of [`STARTS_AT_SIX`] beside `schema`, under the limits `given`.
fn check_labelled(schema: &str, given: &[&str]) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("labelled.scxml"), schema).expect("write schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, STARTS_AT_SIX).expect("write probe");
    let mut args: Vec<&str> = Vec::new();
    for limit in given {
        args.push("--limit");
        args.push(limit);
    }
    args.extend(["check", "-l", "rust"]);
    codegen(&args, &path)
}

/// The text a schema's field holds is a bound a deployment moves as it moves a list: a
/// record of the schema holds it in a buffer of that size on every backend, and an event
/// of the schema carries it into one. A list of longer texts built for a server is the
/// same schema built with a larger number, not a copy of it.
#[test]
fn a_name_moves_the_text_a_schema_field_holds() {
    // Six bytes do not start in a field of four, named or not, when no number is given.
    let (ok, out) = check_labelled(&labelled(4, ROOM), &[]);
    assert!(!ok, "six bytes were taken into four:\n{out}");
    // Given eight, the field holds them; given five, it does not.
    let (ok, out) = check_labelled(&labelled(4, ROOM), &["room=8"]);
    assert!(ok, "the field moved to eight:\n{out}");
    let (ok, out) = check_labelled(&labelled(4, ROOM), &["room=5"]);
    assert!(!ok, "six bytes were taken into five:\n{out}");
    // A field that does not name the bound stays at four.
    let (ok, out) = check_labelled(&labelled(4, ""), &["room=8"]);
    assert!(!ok, "a field that names no bound moved:\n{out}");
}

/// The statechart of [`machine`] that also reads the bound its list names, as
/// `_x.limits.<read>` into `room` (docs/adr/0050).
fn reading_machine(bound: u32, read: &str) -> String {
    machine(bound, ROOM)
        .replace(
            r#"<data id="n" sce:type="uint8" expr="3"/>"#,
            r#"<data id="n" sce:type="uint8" expr="3"/>
    <data id="room" sce:type="uint32" expr="0" sce:direction="out"/>"#,
        )
        .replace(
            r#"<assign location="shown" expr="Copies(n, 2)"/>"#,
            &format!(
                r#"<assign location="shown" expr="Copies(n, 2)"/>
      <assign location="room" expr="_x.limits.{read}"/>"#
            ),
        )
}

/// What `generate -l <language>` writes for the statechart `text` beside the
/// algorithm, under the limits `given`, every file of it in one text.
fn generated_machine(text: &str, language: &str, given: &[&str]) -> String {
    let (dir, path) = documents(&algorithm(8, ROOM), text);
    let out = tempdir().expect("tempdir");
    let mut args: Vec<&str> = Vec::new();
    for limit in given {
        args.push("--limit");
        args.push(limit);
    }
    args.extend([
        "generate",
        "-l",
        language,
        "--go-module-prefix",
        "example.test/generated",
        "-o",
        out.path().to_str().expect("utf-8"),
    ]);
    let (ok, written) = codegen(&args, &path);
    assert!(ok, "{language} {given:?}:\n{written}");
    let mut text = String::new();
    let mut pending = vec![out.path().to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the output directory") {
            let file = entry.expect("an entry").path();
            if file.is_dir() {
                pending.push(file);
            } else {
                text.push_str(&std::fs::read_to_string(&file).unwrap_or_default());
            }
        }
    }
    drop(dir);
    text
}

/// A statechart reads a bound by the name `sce:limit` gives it, `_x.limits.<name>`,
/// and a machine declares each bound it names as a typed constant a host reads: one
/// number, the build's, for the read, the constant and the buffer, so `--limit`
/// moves the three together (docs/adr/0050).
#[test]
fn a_statechart_reads_a_bound_by_its_name_and_a_host_reads_its_constant() {
    let (ok, out) = check(&reading_machine(8, "room"), &algorithm(8, ROOM), &[]);
    assert!(ok, "a read of a bound the statechart names:\n{out}");
    for (language, written, moved) in [
        (
            "rust",
            "pub const ROOM: u32 = 8;",
            "pub const ROOM: u32 = 9;",
        ),
        (
            "kotlin",
            "const val ROOM: UInt = 8u",
            "const val ROOM: UInt = 9u",
        ),
        (
            "cpp",
            "static constexpr std::uint32_t ROOM = 8;",
            "static constexpr std::uint32_t ROOM = 9;",
        ),
        (
            "go",
            "const ProbeLimitRoom uint32 = 8",
            "const ProbeLimitRoom uint32 = 9",
        ),
        ("python", "ROOM: int = 8", "ROOM: int = 9"),
        (
            "c",
            "#define PROBE_LIMIT_ROOM UINT32_C(8)",
            "#define PROBE_LIMIT_ROOM UINT32_C(9)",
        ),
    ] {
        let as_written = generated_machine(&reading_machine(8, "room"), language, &[]);
        assert!(
            as_written.contains(written) && !as_written.contains("_x.limits"),
            "{language}: the machine declares `{written}` and reads the number:\n{as_written}"
        );
        let built = generated_machine(&reading_machine(8, "room"), language, &["room=9"]);
        assert!(
            built.contains(moved),
            "{language}: `--limit room=9` moves the constant to `{moved}`:\n{built}"
        );
    }
}

/// A read of a name no element of the statechart names is refused, naming the ones
/// it does, and so is a name two of its elements resolve to two numbers.
#[test]
fn a_read_of_a_bound_the_statechart_does_not_name_is_refused() {
    let (ok, out) = check(&reading_machine(8, "rooms"), &algorithm(8, ROOM), &[]);
    assert!(!ok, "a read of an unnamed bound was taken:\n{out}");
    assert!(
        out.contains("rooms") && out.contains("_x.limits"),
        "the refusal names the read:\n{out}"
    );
    let twice = reading_machine(8, "room").replace(
        r#"<data id="room" sce:type="uint32" expr="0" sce:direction="out"/>"#,
        r#"<data id="room" sce:type="uint32" expr="0" sce:direction="out"/>
    <data id="other" sce:type="list&lt;uint8&gt;" sce:capacity="16" sce:limit="room"/>"#,
    );
    let (ok, out) = check(&twice, &algorithm(8, ROOM), &[]);
    assert!(!ok, "a read of a name given two numbers was taken:\n{out}");
    assert!(out.contains("8 and 16"), "the refusal names both:\n{out}");
}

/// An algorithm reads the bounds it names the same way.
#[test]
fn an_algorithm_reads_a_bound_by_its_name() {
    let reading = algorithm(8, ROOM).replace(
        r#"<sce:while cond="i &lt; count" max-iter="8">"#,
        r#"<sce:while cond="i &lt; count &amp;&amp; i &lt; _x.limits.room" max-iter="8">"#,
    );
    assert_ne!(reading, algorithm(8, ROOM), "the algorithm reads the bound");
    let check_algorithm = |text: &str| -> (bool, String) {
        let (dir, _) = documents(text, &machine(8, ROOM));
        codegen(&["check", "-l", "rust"], &dir.path().join("copies.scxml"))
    };
    let (ok, out) = check_algorithm(&reading);
    assert!(ok, "an algorithm reads the bound it names:\n{out}");
    let (ok, out) = check_algorithm(&reading.replace("_x.limits.room", "_x.limits.rooms"));
    assert!(
        !ok,
        "a read of a bound the algorithm does not name was taken:\n{out}"
    );
    assert!(out.contains("rooms"), "the refusal names the read:\n{out}");
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
                        .or_else(|| node.attribute(("http://sce.dev/ext", "max-size")))
                        .unwrap_or_else(|| {
                            panic!(
                                "{}: sce:limit=\"{name}\" is beside no capacity, \
                                 returns-max-size or max-size",
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
