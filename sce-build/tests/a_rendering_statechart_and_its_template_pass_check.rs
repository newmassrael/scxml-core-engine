// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The committed text template and the statechart that renders it
//! (docs/adr/0087, section 3) pass `sce-codegen check` on every language, the
//! command each regeneration script asks first: a template whose trusted and
//! untrusted holes write `sce:trust`, read through the vocabulary every SCE
//! attribute of a document is held to, and a `<sce:render>` of it.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// `sce-codegen check -l <lang> <document>`, as the regeneration scripts run it
/// (Go with the module its packages live under): its exit status, and what it
/// said. A refusal by name is status 7, which a regeneration script reads as
/// "not lowered for this language" and skips, so only 0 passes.
fn check(lang: Language, document: &str) -> (Option<i32>, String) {
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("check")
        .args(["-l", lang.canonical_name()]);
    if lang == Language::Go {
        command.args(["--go-module-prefix", "example.com/generated"]);
    }
    let out = command
        .arg(
            repo_root()
                .join("sce-build/tests/fixtures/static_datamodel")
                .join(document),
        )
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

#[test]
fn the_template_and_the_statechart_that_renders_it_pass_check_on_every_language() {
    for &lang in Language::ALL {
        for document in ["text_notice.scxml", "static_render.scxml"] {
            let (status, said) = check(lang, document);
            assert_eq!(
                status,
                Some(0),
                "{lang:?}: check {document} exited {status:?} (7 is a refusal by name, \
                 which a regeneration script skips):\n{said}"
            );
            assert!(!said.contains("unknown-sce-attribute"), "{lang:?}: {said}");
        }
    }
}
