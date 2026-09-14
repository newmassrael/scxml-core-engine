// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What `sce-codegen acceptance-report` PRINTS is what the renderer
//! produced — checked against the published bytes.
//!
//! # The gap this fills, and why a strong test did not close it
//!
//! `an_acceptance_report_moves_when_a_requirement_is_violated` is a
//! detector test of the kind this repository prefers: it breaks an
//! element a requirement's evidence depends on and refuses a report
//! that comes back byte-identical. It is the right test for the
//! RENDERER — and it never spawns the binary. So it proves the page
//! moves when the document does, and says nothing about whether the
//! subcommand publishes that page.
//!
//! Between `acceptance_report::render` and a consumer's stdout sits
//! `cmd_acceptance_report`, which could truncate, re-wrap, swallow a
//! trailing block or route to stderr, and every in-crate gate would
//! stay green — because each asks the renderer what the renderer
//! answers. Measured 2026-09-14: of the twelve `sce-codegen`
//! subcommands that print a surface a consumer parses, ten carried a
//! binary-spawning gate and two did not. The other one was
//! `provenance-roster`, where exactly this blindness let the wire
//! publish 312 unmeasured codes as `never` — a claim about the codes
//! that nobody had evidence for — through a full suite, clippy,
//! tree-hygiene and a push.
//!
//! This matters beyond tidiness: the acceptance report is the artefact
//! ledger row G1 turns on, *"an acceptance record can be taken and
//! re-checked"*. A record whose published form nothing asserts on is
//! G1's own defect one layer out.

use std::path::PathBuf;
use std::process::Command;

/// The generator binary. `env!` here rather than in a helper: it
/// expands at compile time, and `cli_feature_gating` derives which
/// targets reach the binary by finding this expansion in the target's
/// own source.
const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/requirement_closure")
}

/// Run the subcommand over the committed (document, manifest) pair.
fn published() -> String {
    let dir = fixtures();
    let out = Command::new(CODEGEN)
        .arg("acceptance-report")
        .arg(dir.join("doip_nl_connection_states.scxml"))
        .arg("--manifest")
        .arg(dir.join("iso13400_2_nl_socket_handling.manifest.json"))
        .output()
        .expect("spawn sce-codegen");
    assert!(
        out.status.success(),
        "`acceptance-report` exited {:?} on the committed fixture pair\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("report is utf-8")
}

/// The subcommand publishes exactly what the renderer produced.
///
/// ⭐ The load-bearing assertion, and the only one that can see the
/// layer the in-crate tests cannot. Equality against
/// `acceptance_report::render` called on the same inputs: anything the
/// command does to the page between producing and printing it shows up
/// here and nowhere else.
#[test]
fn the_wire_publishes_exactly_what_the_renderer_produced() {
    let dir = fixtures();
    let scxml = dir.join("doip_nl_connection_states.scxml");
    let manifest_path = dir.join("iso13400_2_nl_socket_handling.manifest.json");

    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(scxml.to_str().expect("utf-8 path"))
        .expect("the committed fixture parses");
    let manifest = sce_build::requirement_manifest::RequirementManifest::load(&manifest_path)
        .expect("the committed manifest loads");
    let rendered = sce_build::acceptance_report::render(&model, &manifest, None);

    assert_eq!(
        published().trim_end(),
        rendered.trim_end(),
        "the subcommand's stdout differs from what `acceptance_report::render` \
         produced for the same inputs. Everything between the renderer and a \
         consumer is invisible to every other gate on this axis, because each \
         asks the renderer what the renderer answers.",
    );
}

/// The published page is not empty, and carries the report's own
/// structure rather than a stub.
///
/// A floor beside the equality: two empty strings compare equal, so an
/// equality alone would pass for a command that printed nothing and a
/// renderer that returned nothing.
#[test]
fn the_published_report_is_not_vacuously_equal() {
    let page = published();
    assert!(
        page.len() > 200,
        "the published report is {} byte(s) — too short to be the page \
         the renderer builds, and short enough that an equality against \
         it would prove nothing",
        page.len(),
    );
    assert!(
        page.lines().count() > 5,
        "the published report is {} line(s); a one-line page would make \
         the equality above vacuous",
        page.lines().count(),
    );
}
