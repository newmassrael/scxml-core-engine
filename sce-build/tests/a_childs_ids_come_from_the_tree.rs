// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The id a child session's start is made under comes from the tree alone
// (docs/adr/0045, "The ids inside a tree are the same on every backend"):
// `<state>.<start>.<invoke>`, `start` being which start of that `<invoke>` this
// is in its parent, counted by the parent. A saved tree holds these ids, and a
// saved state is one text whatever backend wrote it, so no backend may spell
// the platform part with something only its own process knows: the address of
// the parent, its identity hash, a process-wide counter.
//
// The six generated backends are asked for the same document, one whose
// `<invoke>` names no id and stores the generated one in an `idlocation`
// (the shape W3C test 224 reads), and each is held to counting the start in
// the parent and to not reaching for the spelling it used before.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// Everything `sce-codegen generate -l <language>` wrote for `document`,
/// joined; the run must succeed.
fn generated(language: &str, document: &str) -> String {
    let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("child-ids-{language}-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join("parent.scxml");
    std::fs::write(&path, document).expect("write document");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            language,
            "-o",
            dir.to_str().unwrap(),
            "--error-format=json",
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    let mut text = String::new();
    for entry in std::fs::read_dir(&dir).expect("read scratch dir") {
        let file = entry.expect("dir entry").path();
        if file
            .extension()
            .is_some_and(|e| e == "json" || e == "scxml")
        {
            continue;
        }
        if let Ok(written) = std::fs::read_to_string(&file) {
            text.push_str(&written);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        run.status.success(),
        "{language}: the parent does not generate:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    text
}

/// A parent whose `<invoke>` names no id and keeps the generated one.
const PARENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0" datamodel="ecmascript" name="parent">
  <datamodel>
    <data id="Var1"/>
  </datamodel>
  <state id="s0">
    <invoke type="scxml" idlocation="Var1">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="f" datamodel="ecmascript"><final id="f"/></scxml>
      </content>
    </invoke>
    <transition event="done.invoke" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#;

/// Per backend: what counts a start in the parent, and the spelling of the
/// platform part it used before, which no longer names a child's start.
const BACKENDS: [(&str, &str, &str); 6] = [
    ("rust", "self.invoke_starts_", "self as *const _ as usize"),
    (
        "cpp",
        "++invokeStarts_",
        "\"s0.\" + std::to_string(reinterpret_cast<uintptr_t>(this))",
    ),
    ("go", "p.invokeStarts", "sce.NextInvokeCounter()"),
    ("python", "self._invoke_starts_", "{id(self)}"),
    (
        "kotlin",
        "nextInvokeStart(\"",
        "System.identityHashCode(this)",
    ),
    (
        "c11",
        "sce_invoke_format_start_id(",
        "sce_invoke_format_id(",
    ),
];

#[test]
fn every_backend_numbers_a_childs_start_in_its_parent() {
    for (language, counted, before) in BACKENDS {
        let text = generated(language, PARENT);
        assert!(
            text.contains(counted),
            "{language}: the parent does not count the start of its child ({counted:?})"
        );
        assert!(
            !text.contains(before),
            "{language}: the child's id is still spelled with what only this process \
             knows ({before:?})"
        );
    }
}
