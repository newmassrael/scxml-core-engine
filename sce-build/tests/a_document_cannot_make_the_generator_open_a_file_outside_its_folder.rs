// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A document cannot make the generator open a file outside the folder it was confined to.
//!
//! # What was wrong
//!
//! A document names files: a template (`<sce:use template>`), an include, an import, a script, a
//! driver header, a child. The generator opens what it names and says what it found, and a
//! document handed over by someone who does not own the machine is then a way to ask the machine
//! about its files. Measured 2026-10-07 through the authoring server: `<sce:use
//! template="/abs/path">` was answered `template is malformed: root element must be <sce:template>,
//! got <state>` for a file that is there and `file not found` for one that is not, and a template
//! that is well-formed was spliced into the document and read back in the diagnostics. A reading
//! of the document's attributes before it is read had listed `src` and `href`, and `template` was
//! not one of them: a list of the attributes that name a file is a list of the places somebody
//! thought of.
//!
//! # What is held
//!
//! With `SCE_FILE_ROOT` naming a folder, every file a document names is opened inside it or not
//! at all, and a file outside is answered as a file that is not there. For each place that
//! `check` reaches this runs the generator on a document that names a file outside the folder,
//! with the file there and with it gone, and holds that
//!
//! - the two answers are the same, so the file's being there is not said;
//! - a word that is only in the file is in neither;
//! - and, as the control that this reaches the place at all, that without the folder the two
//!   answers are not the same.
//!
//! A file inside the folder is opened as it was: the folder is a boundary and not a refusal of
//! every file.
//!
//! The places the command line does not reach (a driver header, which only the library's compile
//! entry resolves, and a document or a child that the parser reads) are held by unit tests where
//! they are (`lib.rs`, `parser.rs`). That every place that opens a file goes through the rule at
//! all is held by `document_files_pass_through_confine.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

const SCXML: &str = r#"xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext""#;

fn codegen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// What a run printed and how it ended, with the folders it names made one word, so that two runs
/// can be compared whatever the folders are called.
struct Answer {
    code: Option<i32>,
    said: String,
}

/// `check --lint` of `document`, run in `work`, confined to `confined_to` when that is given.
fn check(document: &Path, work: &Path, confined_to: Option<&Path>, folders: &[&Path]) -> Answer {
    let mut command = Command::new(codegen());
    command
        .current_dir(work)
        .env_remove("SCE_FILE_ROOT")
        .args(["--error-format", "json", "check"])
        .arg(document)
        .arg("--lint");
    if let Some(root) = confined_to {
        command.env("SCE_FILE_ROOT", root);
    }
    let output = command.output().expect("the generator runs");
    let mut said = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for folder in folders {
        said = said.replace(&folder.display().to_string(), "<folder>");
    }
    Answer {
        code: output.status.code(),
        said,
    }
}

/// One place that opens a file: a document that names `file` (outside the folder), and the word
/// that is only in that file.
struct Place {
    name: &'static str,
    /// The name of the file outside.
    file: &'static str,
    /// What is in it.
    body: &'static str,
    /// The word that is in the file and nowhere else.
    marker: &'static str,
    /// The document, given the path of the file it names.
    document: fn(&str) -> String,
}

fn places() -> Vec<Place> {
    vec![
        Place {
            name: "a template",
            file: "outside.sce-template.xml",
            body: r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" name="outside">
  <state id="a"><transition event="CANARY-TEMPLATE-ZETA" target="b"/></state>
  <final id="b"/>
</sce:template>
"#,
            marker: "CANARY-TEMPLATE-ZETA",
            document: |path| {
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} version="1.0" name="host" initial="a"><sce:use template="{path}"/></scxml>
"#
                )
            },
        },
        Place {
            name: "an include",
            file: "outside.frag.xml",
            body: r#"<?xml version="1.0" encoding="UTF-8"?>
<fragment><transition event="CANARY-INCLUDE-ETA" target="b" xmlns="http://www.w3.org/2005/07/scxml"/></fragment>
"#,
            marker: "CANARY-INCLUDE-ETA",
            document: |path| {
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" name="host" initial="a">
  <state id="a"><xi:include href="{path}"/></state>
  <final id="b"/>
</scxml>
"#
                )
            },
        },
        Place {
            name: "an import",
            file: "outside.import.scxml",
            body: "CANARY-IMPORT-THETA is not a document",
            marker: "CANARY-IMPORT-THETA",
            document: |path| {
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} version="1.0" name="host" initial="a">
  <sce:import as="Level" src="{path}" kind="enum"/>
  <state id="a"/>
</scxml>
"#
                )
            },
        },
        Place {
            name: "a script",
            file: "outside.script.js",
            body: "var CANARY_SCRIPT_IOTA = 1;\n",
            marker: "CANARY_SCRIPT_IOTA",
            document: |path| {
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} version="1.0" name="host" initial="a" datamodel="ecmascript">
  <script src="{path}"/>
  <state id="a"/>
</scxml>
"#
                )
            },
        },
    ]
}

/// A folder to work in and one beside it that the generator is not to open anything in.
struct Folders {
    work: TempDir,
    outside: TempDir,
}

fn folders() -> Folders {
    Folders {
        work: TempDir::new().unwrap(),
        outside: TempDir::new().unwrap(),
    }
}

#[test]
fn a_file_outside_the_folder_is_answered_as_a_file_that_is_not_there_in_every_place_check_opens_one(
) {
    for place in places() {
        let Folders { work, outside } = folders();
        let named = outside.path().join(place.file);
        let document = work.path().join("host.scxml");
        fs::write(&document, (place.document)(&named.display().to_string())).unwrap();
        let both = [work.path(), outside.path()];
        let confined = Some(work.path());

        fs::write(&named, place.body).unwrap();
        let there_confined = check(&document, work.path(), confined, &both);
        let there_free = check(&document, work.path(), None, &both);
        fs::remove_file(&named).unwrap();
        let gone_confined = check(&document, work.path(), confined, &both);
        let gone_free = check(&document, work.path(), None, &both);

        // The control: without the folder the file's being there is said, so this document does
        // reach the place that opens the file.
        assert!(
            there_free.said != gone_free.said || there_free.code != gone_free.code,
            "{}: the document does not make the generator open the file, so this holds nothing:\n{}",
            place.name,
            there_free.said
        );
        // With the folder it is not.
        assert_eq!(
            there_confined.said, gone_confined.said,
            "{}: the file's being there is said",
            place.name
        );
        assert_eq!(there_confined.code, gone_confined.code, "{}", place.name);
        assert!(
            !there_confined.said.contains(place.marker),
            "{}: a word that is only in the file is in what is said:\n{}",
            place.name,
            there_confined.said
        );
    }
}

/// What `generate` wrote under `out`, as one text.
fn written_under(out: &Path) -> String {
    let mut all = String::new();
    let mut pending = vec![out.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(bytes) = fs::read(&path) {
                all.push_str(&String::from_utf8_lossy(&bytes));
            }
        }
    }
    all
}

/// `generate` of `document` for `language` into `out`, run in `work`, confined to `confined_to`
/// when that is given: what it said and what it wrote.
fn generate(
    document: &Path,
    language: &str,
    out: &Path,
    work: &Path,
    confined_to: Option<&Path>,
    folders: &[&Path],
) -> (Answer, String) {
    let _ = fs::remove_dir_all(out);
    let mut command = Command::new(codegen());
    command
        .current_dir(work)
        .env_remove("SCE_FILE_ROOT")
        .arg("generate")
        .arg(document)
        .args(["-o"])
        .arg(out)
        .args(["-l", language]);
    if let Some(root) = confined_to {
        command.env("SCE_FILE_ROOT", root);
    }
    let output = command.output().expect("the generator runs");
    let mut said = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for folder in folders {
        said = said.replace(&folder.display().to_string(), "<folder>");
    }
    (
        Answer {
            code: output.status.code(),
            said,
        },
        written_under(out),
    )
}

/// A place `generate` opens a file that `check` does not: what a document names to be copied into,
/// or read into, what is written.
fn written_places() -> Vec<(Place, &'static str)> {
    vec![
        (
            Place {
                name: "an invoke candidate",
                file: "outside.child.scxml",
                body: r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
  <state id="s"><transition event="CANARY-CANDIDATE-KAPPA" target="s"/></state>
</scxml>
"#,
                marker: "CANARY-CANDIDATE-KAPPA",
                document: |path| {
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} version="1.0" name="host" initial="a" datamodel="ecmascript">
  <datamodel><data id="which" expr="'x'"/></datamodel>
  <state id="a"><invoke type="http://www.w3.org/TR/scxml/" srcexpr="which" sce:candidates="{path}"/></state>
</scxml>
"#
                    )
                },
            },
            "cpp",
        ),
        (
            Place {
                name: "a data source",
                file: "outside.data.txt",
                body: "CANARY-DATA-LAMBDA",
                marker: "CANARY-DATA-LAMBDA",
                document: |path| {
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} version="1.0" name="host" initial="a" datamodel="ecmascript">
  <datamodel><data id="x" src="{path}"/></datamodel>
  <state id="a"/>
</scxml>
"#
                    )
                },
            },
            "python",
        ),
    ]
}

#[test]
fn a_file_outside_the_folder_is_not_carried_into_what_generate_writes() {
    for (place, language) in written_places() {
        let Folders { work, outside } = folders();
        let named = outside.path().join(place.file);
        let document = work.path().join("host.scxml");
        fs::write(&document, (place.document)(&named.display().to_string())).unwrap();
        let out = work.path().join("out");
        let both = [work.path(), outside.path()];
        let confined = Some(work.path());

        fs::write(&named, place.body).unwrap();
        let (_, free_there) = generate(&document, language, &out, work.path(), None, &both);
        let (there, written_there) =
            generate(&document, language, &out, work.path(), confined, &both);
        fs::remove_file(&named).unwrap();
        let (gone, written_gone) =
            generate(&document, language, &out, work.path(), confined, &both);

        // The control: without the folder the file reaches what is written, so this document does
        // make the generator open it.
        assert!(
            free_there.contains(place.marker),
            "{}: the document does not make `generate` carry the file, so this holds nothing",
            place.name
        );
        // With the folder it does not, and the file's being there is not said.
        assert!(
            !written_there.contains(place.marker) && !there.said.contains(place.marker),
            "{}: a word that is only in the file is in what was written or said:\n{}",
            place.name,
            there.said
        );
        assert_eq!(
            there.said, gone.said,
            "{}: the file's being there is said",
            place.name
        );
        assert_eq!(there.code, gone.code, "{}", place.name);
        assert_eq!(written_there, written_gone, "{}", place.name);
    }
}

#[test]
fn a_file_inside_the_folder_is_opened_as_it_was() {
    // The boundary is not a refusal of every file: what a document names beside it is read.
    let Folders { work, .. } = folders();
    fs::write(
        work.path().join("shared.sce-template.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" name="shared">
  <state id="a"><transition event="tick" target="b"/></state>
  <final id="b"/>
</sce:template>
"#,
    )
    .unwrap();
    fs::write(
        work.path().join("frag.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<fragment><transition event="tick" target="b" xmlns="http://www.w3.org/2005/07/scxml"/></fragment>
"#,
    )
    .unwrap();
    let with_template = work.path().join("with_template.scxml");
    fs::write(
        &with_template,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} version="1.0" name="host" initial="a"><sce:use template="shared.sce-template.xml"/></scxml>
"#
        ),
    )
    .unwrap();
    let with_include = work.path().join("with_include.scxml");
    fs::write(
        &with_include,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" name="host" initial="a">
  <state id="a"><xi:include href="frag.xml"/></state>
  <final id="b"/>
</scxml>
"#
        ),
    )
    .unwrap();

    for document in [&with_template, &with_include] {
        let free = check(document, work.path(), None, &[work.path()]);
        let confined = check(document, work.path(), Some(work.path()), &[work.path()]);

        assert_eq!(free.code, Some(0), "{}\n{}", document.display(), free.said);
        assert_eq!(confined.code, free.code, "{}", document.display());
        assert_eq!(confined.said, free.said, "{}", document.display());
    }
}

#[test]
fn a_folder_that_cannot_be_used_lets_nothing_through() {
    // A confinement that is lost when its folder is mistyped is none.
    let Folders { work, outside } = folders();
    let named = outside.path().join("outside.sce-template.xml");
    fs::write(&named, "not read").unwrap();
    let document = work.path().join("host.scxml");
    fs::write(
        &document,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml {SCXML} version="1.0" name="host" initial="a"><sce:use template="{}"/></scxml>
"#,
            named.display()
        ),
    )
    .unwrap();
    let nowhere = work.path().join("no-such-folder");

    let said = check(
        &document,
        work.path(),
        Some(&nowhere),
        &[work.path(), outside.path()],
    );

    assert!(said.said.contains("file not found"), "{}", said.said);
    assert!(!said.said.contains("not read"), "{}", said.said);
}
