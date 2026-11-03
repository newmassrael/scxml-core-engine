// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! §scxml-6.4 — the stub child of a hybrid `<invoke>` is a document the
//! generator writes, and only the generator: no copy of it is tracked, and
//! every reader takes it from a generation.
//!
//! The W3C corpus tracked the stub of each of its hybrid invokes beside its
//! parent (`resources/216/test216_hybrid0.scxml`,
//! `resources/530/test530_hybrid0.scxml`), because the conformance CMake route
//! planned each machine of a fixture by naming its document, and a stub is
//! written by its parent's generation, on no disk when the tree is
//! configured. A test held each copy to the text the generator writes: a
//! second copy of a generated artifact, kept true by hand. The route now asks
//! the generator for the generation of every child document a fixture's
//! generation writes (`sce_codegen_plan(... CHILD_GENERATIONS)`, which runs
//! `--plan-children`), as the integration route does, and the sourcemap drift
//! gate regenerates a stub through the parent whose generation writes it.
//!
//! Held here:
//!
//! * the corpus still has a hybrid invoke with a stub, so the rest asks about
//!   something;
//! * git tracks no stub under `resources/`;
//! * the plan of each such fixture's generation names the stub among the
//!   files it writes, and plans the stub's own generation, which writes the
//!   stub's machine: what the route declares, read from the generator.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::parser::SCXMLParser;

/// A tracked W3C fixture with a hybrid invoke: the fixture's path relative to
/// the repository, and the name of each stub its generation writes.
struct Hybrid {
    fixture: String,
    stubs: Vec<String>,
}

/// Every tracked `resources/<id>/test<id>.scxml` whose document has a hybrid
/// invoke with a stub.
fn hybrids() -> Vec<Hybrid> {
    let root = common::repository::root();
    let mut found = Vec::new();
    for rel in common::repository::paths_git_tracks(&["resources/*/*.scxml"]) {
        let path = root.join(&rel);
        let Some(path_str) = path.to_str() else {
            continue;
        };
        // A document the parser refuses has no invokes to ask about.
        let Ok(model) = SCXMLParser::new().parse_file(path_str) else {
            continue;
        };
        let stubs: Vec<String> = model
            .iter_hybrid_invokes()
            .filter(|invoke| invoke.stub_document().is_some())
            .map(|invoke| invoke.child_name.clone())
            .collect();
        if !stubs.is_empty() {
            found.push(Hybrid {
                fixture: rel,
                stubs,
            });
        }
    }
    found
}

#[test]
fn no_hybrid_stub_is_tracked() {
    let hybrids = hybrids();
    // The corpus holds two hybrid invokes today (test216, test530). A sweep
    // that found none would pass while asserting nothing.
    assert!(
        !hybrids.is_empty(),
        "no hybrid invoke found in resources/ — the sweep is broken, not the corpus"
    );
    let tracked = common::repository::paths_git_tracks(&["resources/*/*_hybrid*.scxml"]);
    assert!(
        tracked.is_empty(),
        "a hybrid invoke's stub is tracked beside its fixture: {tracked:?}. The generator \
         writes it, and every reader takes it from a generation; delete the copy."
    );
    for hybrid in &hybrids {
        let dir = Path::new(&hybrid.fixture)
            .parent()
            .expect("a fixture sits in a directory");
        for stub in &hybrid.stubs {
            let rel = dir.join(format!("{stub}.scxml"));
            let rel = rel.to_string_lossy();
            assert!(
                common::repository::paths_git_tracks(&[&*rel]).is_empty(),
                "{rel} is tracked"
            );
        }
    }
}

/// `sce-codegen --plan --plan-children generate <fixture> -l <lang> -o <out>
/// --input-root <its directory>`, as the conformance route asks it; its
/// stdout lines.
fn planned(fixture: &Path, language: &str, out: &Path) -> Vec<String> {
    let dir = fixture.parent().expect("a fixture sits in a directory");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--plan", "--plan-children", "generate"])
        .arg(fixture)
        .args(["-l", language, "-o"])
        .arg(out)
        .arg("--input-root")
        .arg(dir)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "the plan of {} ({language}) failed:\n{}",
        fixture.display(),
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// The conformance route declares a stub's generation from the plan of its
/// fixture, so the plan has to name the stub as a file of the fixture's
/// generation and plan the stub's own, which writes its machine. Asked of
/// both languages the route generates.
#[test]
fn the_plan_of_a_fixture_plans_the_generation_of_its_stub() {
    let root = common::repository::root();
    for hybrid in hybrids() {
        let fixture: PathBuf = root.join(&hybrid.fixture);
        for language in ["cpp", "c11"] {
            let out = tempfile::tempdir().expect("tempdir");
            let lines = planned(&fixture, language, out.path());
            for stub in &hybrid.stubs {
                let document = out
                    .path()
                    .join(format!("{stub}.scxml"))
                    .to_string_lossy()
                    .into_owned();
                assert!(
                    lines.contains(&document),
                    "[{language}] the plan of {} does not name {document} among the files \
                     its generation writes:\n{}",
                    hybrid.fixture,
                    lines.join("\n")
                );
                let machine = out.path().join(format!("{stub}_sm.h"));
                let generation = format!("{document}\t{}", machine.to_string_lossy());
                assert!(
                    lines.contains(&generation),
                    "[{language}] the plan of {} does not plan the generation of {stub}, \
                     whose machine the fixture's machine names:\n{}",
                    hybrid.fixture,
                    lines.join("\n")
                );
            }
        }
    }
}
