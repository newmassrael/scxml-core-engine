// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! SCE Mesh §9.5 `<invoke type="sce:mesh-rpc">` — which route serves it on
//! each backend is a contract, and this file is what makes it one.
//!
//! Every backend's parser accepts the invoke, so a backend whose templates
//! neither lower it into a generated router nor receive it as a host-served
//! invoke would emit a machine whose `<onentry>` ignores a declaration the
//! author wrote. That used to be refused at build time on every backend
//! without `tools/codegen/templates/mesh/<dir>/`
//! (`reject_mesh_rpc_in_unsupported_lang`). The refusal was retired when
//! §mesh-19's host router began serving requests as well as sends: every
//! other backend now lowers the invoke to a host-served one of the same
//! type. Removing a refusal is the claim that a path exists, and this file
//! is where that claim is paid for, as `native_action_backend_parity.rs` pays
//! for `<sce:action>`'s.
//!
//! The roster lives in exactly one place — §9.5's table — and this file
//! binds it in every direction:
//!
//! 1. The table names every backend, once.
//! 2. Its `generated` rows are exactly the tracked template directories, and
//!    its `host` rows exactly the backends the lowering routes through the
//!    host. A backend on both routes, or on neither, is a red.
//! 3. The CLI agrees with the table on all six. Every backend generates the
//!    fixture; a `host` row must also tell its host to register a router
//!    (`needs_mesh_router`) and carry the type in its output, so a request
//!    the lowering lost cannot pass as one it served.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::host_processor_analyzer::routes_mesh_through_host;

/// The contract document, and the anchor that opens its table. Named rather
/// than searched for by shape: a second table in this file must not be able
/// to answer for this one.
const CONTRACT_DOC: &str = "SCE_MESH.md";
/// The slug, not the whole HTML comment: the retirement site cites it in
/// prose too, and one spelling has to reach both readers.
const ANCHOR: &str = "sce:mesh-rpc-backends";

/// Where a backend's generated-router templates live, relative to the repo
/// root.
const MESH_TEMPLATE_ROOT: &str = "tools/codegen/templates/mesh";

/// The §9.5 fixture every backend is asked to generate — a real mesh-rpc
/// document this repository already ships and compiles, not a fresh one
/// written to agree with this test.
const FIXTURE: &str = "tests/mesh/brake_invoke.scxml";

/// The type a host-route backend's output must carry: the invoke it
/// dispatches to the host's router by name.
const MESH_RPC_TYPE: &str = "sce:mesh-rpc";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// Resolved through `CARGO_BIN_EXE_*` so the emitter under test is a build
/// dependency of this test rather than whatever binary happens to be on disk.
fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// The two routes a §9.5 row can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// `templates/mesh/<dir>/` generates the machine's own router.
    Generated,
    /// The request is lowered to a host-served invoke the host's router runs.
    Host,
}

/// One row of the §9.5 table.
#[derive(Debug)]
struct Row {
    /// The `--lang` spelling, which is `Language::canonical_name`.
    lang: String,
    /// The subdirectory of `MESH_TEMPLATE_ROOT` this backend's templates
    /// would live in. Carried by the row because the mapping is not the
    /// identity — `c11` renders from `mesh/c/` — and a test that guessed it
    /// would bless the wrong spelling.
    dir: String,
    route: Route,
}

impl Row {
    fn language(&self) -> Language {
        *Language::ALL
            .iter()
            .find(|l| l.canonical_name() == self.lang)
            .unwrap_or_else(|| panic!("§9.5 row `{}` names no backend", self.lang))
    }
}

/// Cells of one markdown table row, trimmed, without the outer empties.
fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn unticked(cell: &str) -> String {
    cell.trim()
        .trim_matches('`')
        .trim_end_matches('/')
        .to_string()
}

/// The table as written, read from the anchor to the first line that is not
/// part of it.
fn contract_rows() -> Vec<Row> {
    let path = repo_root().join(CONTRACT_DOC);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{CONTRACT_DOC} is readable: {e}"));

    let anchor_at = text.find(ANCHOR).unwrap_or_else(|| {
        panic!(
            "{CONTRACT_DOC} carries no `{ANCHOR}` anchor. §9.5's backend table is the \
             single place the mesh-rpc roster is written down; without the anchor this \
             gate reads nothing and would pass by reading nothing."
        )
    });

    // The header carries a backticked first cell (`--lang`) just as the data
    // rows do, so it cannot be told apart by shape. It is matched by name and
    // skipped once: a reshaped table is then a red here rather than a row
    // parsed as data.
    const HEADER_FIRST_CELL: &str = "`--lang`";

    let mut rows = Vec::new();
    let mut header_seen = false;
    for line in text[anchor_at..].lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // Blank lines separate the anchor from its table; once rows have
            // been seen, a blank ends it.
            if rows.is_empty() {
                continue;
            }
            break;
        }
        if !trimmed.starts_with('|') {
            if rows.is_empty() {
                continue;
            }
            break;
        }
        let c = cells(trimmed);
        if c.len() < 3 {
            continue;
        }
        // The `|---|` separator carries no backticked lang.
        if !c[0].starts_with('`') {
            continue;
        }
        if !header_seen {
            assert_eq!(
                c[0], HEADER_FIRST_CELL,
                "§9.5's backend table opens with an unexpected header. This gate reads \
                 the table positionally — first cell `--lang`, second the template \
                 directory, third the route — so a reshaped table has to fail here \
                 rather than be read as one row short."
            );
            header_seen = true;
            continue;
        }
        let route = match c[2].trim() {
            "generated" => Route::Generated,
            "host" => Route::Host,
            other => panic!(
                "§9.5 row `{}` says '{other}'. The column has two values — `generated` \
                 or `host` — because it answers which router serves the request on \
                 that backend, and a third word is a claim nothing checks.",
                c[0]
            ),
        };
        rows.push(Row {
            lang: unticked(&c[0]),
            dir: unticked(&c[1]),
            route,
        });
    }
    rows
}

/// The mesh template directories git actually tracks.
///
/// Read from `git ls-files` rather than the filesystem: an untracked scratch
/// directory under the template root is not a backend anyone can generate
/// from, and a directory holding no tracked file renders nothing.
fn tracked_mesh_dirs() -> BTreeSet<String> {
    common::repository::paths_git_tracks(&[MESH_TEMPLATE_ROOT])
        .iter()
        .filter_map(|p| {
            p.strip_prefix(MESH_TEMPLATE_ROOT)?
                .trim_start_matches('/')
                .split('/')
                .next()
                .filter(|d| !d.is_empty())
                .map(str::to_string)
        })
        .collect()
}

/// Lower bound, asserted wherever the table is looped over. A sweep that lost
/// rows reports "every backend agrees" by asking fewer of them, and an empty
/// loop is indistinguishable from a pass.
fn rows_or_panic() -> Vec<Row> {
    let rows = contract_rows();
    assert_eq!(
        rows.len(),
        Language::ALL.len(),
        "§9.5's backend table has {} rows for {} backends. Every backend needs a row \
         — an absent row is how a backend ends up with no stated route at all.\n\
         rows: {rows:?}",
        rows.len(),
        Language::ALL.len()
    );
    rows
}

#[test]
fn the_contract_names_every_backend_exactly_once() {
    let rows = rows_or_panic();

    let named: BTreeSet<&str> = rows.iter().map(|r| r.lang.as_str()).collect();
    assert_eq!(
        named.len(),
        rows.len(),
        "§9.5's backend table names a backend twice. Two rows for one `--lang` can \
         disagree, and then the contract has no answer.\nrows: {rows:?}"
    );

    let expected: BTreeSet<&str> = Language::ALL.iter().map(|l| l.canonical_name()).collect();
    assert_eq!(
        named, expected,
        "§9.5's backend table and `Language::ALL` name different backends. A backend \
         this table forgot is one whose mesh-rpc route is written nowhere; a spelling \
         it invents is one no `--lang` accepts."
    );
}

#[test]
fn the_generated_rows_are_exactly_the_tracked_template_directories() {
    let rows = rows_or_panic();
    let tracked = tracked_mesh_dirs();

    assert!(
        !tracked.is_empty(),
        "no tracked file under {MESH_TEMPLATE_ROOT}. Mesh has a C++ arm, so an empty \
         read here is this gate having lost its subject rather than the tree having \
         lost its templates."
    );

    let claimed: BTreeSet<String> = rows
        .iter()
        .filter(|r| r.route == Route::Generated)
        .map(|r| r.dir.clone())
        .collect();

    assert_eq!(
        claimed, tracked,
        "§9.5's `generated` rows and {MESH_TEMPLATE_ROOT} disagree. The route is \
         DERIVED — `mesh_templates_exist_for` reads this tree at codegen time — so a \
         directory the table does not claim is a generated router the contract does \
         not name, and a claim with no directory behind it is a router nothing \
         generates.\nclaimed: {claimed:?}\ntracked: {tracked:?}"
    );
}

#[test]
fn the_host_rows_are_exactly_the_backends_the_lowering_routes_through_the_host() {
    let rows = rows_or_panic();
    let (mut generated, mut hosted) = (0usize, 0usize);
    for row in &rows {
        let host = routes_mesh_through_host(row.language());
        assert_eq!(
            row.route == Route::Host,
            host,
            "§9.5 says `{}` is served by the {:?} route, and \
             `routes_mesh_through_host` answers {host}. The lowering decides which \
             backends hand a request to the host's router; a row that disagrees tells \
             an author to register a router the machine never calls, or not to \
             register one it does.",
            row.lang,
            row.route
        );
        match row.route {
            Route::Generated => generated += 1,
            Route::Host => hosted += 1,
        }
    }
    // Both bounds, because either side alone makes the partition vacuous.
    assert!(
        generated >= 1,
        "§9.5 names no generated route — this case read nothing"
    );
    assert!(
        hosted >= 1,
        "§9.5 names no host route — this case read nothing"
    );
}

/// The one manifest line `sce-codegen generate` writes to stdout.
fn manifest(stdout: &str) -> serde_json::Value {
    let line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!("no manifest line on stdout:\n{stdout}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("manifest is JSON ({e}):\n{line}"))
}

/// Every artifact the manifest lists, read, as one string.
fn artifacts_text(manifest: &serde_json::Value) -> String {
    let artifacts = manifest["artifacts"]
        .as_array()
        .unwrap_or_else(|| panic!("manifest lists no artifacts: {manifest}"));
    assert!(
        !artifacts.is_empty(),
        "manifest lists no artifacts: {manifest}"
    );
    artifacts
        .iter()
        .map(|a| {
            let path = a["path"].as_str().expect("artifact has a path");
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} is readable: {e}"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_backend_serves_the_fixture_by_the_route_the_contract_names() {
    let rows = rows_or_panic();

    for row in &rows {
        let out_dir = repo_root()
            .join("target")
            .join("mesh_rpc_backend_contract")
            .join(&row.lang);
        let _ = std::fs::remove_dir_all(&out_dir);
        std::fs::create_dir_all(&out_dir).expect("scratch dir");

        let result = Command::new(codegen_bin())
            .args([
                "generate",
                FIXTURE,
                "-l",
                &row.lang,
                "-o",
                out_dir.to_str().expect("utf-8 path"),
                "--no-format",
            ])
            .current_dir(repo_root())
            .output()
            .expect("sce-codegen runs");
        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        assert!(
            result.status.success(),
            "§9.5 says `{}` serves mesh-rpc by the {:?} route, and `sce-codegen generate \
             -l {}` refused {FIXTURE}.\nstderr:\n{stderr}",
            row.lang,
            row.route,
            row.lang
        );
        let manifest = manifest(&stdout);
        assert_eq!(
            manifest["needs_mesh_router"],
            serde_json::json!(row.route == Route::Host),
            "`{}`'s manifest answers needs_mesh_router = {} for {FIXTURE}. A host-route \
             backend runs the request only once its host registers a router, and the \
             manifest is how the host learns that from the build rather than from the \
             first error.execution; a generated-route backend carries its own.",
            row.lang,
            manifest["needs_mesh_router"]
        );
        if row.route == Route::Host {
            let text = artifacts_text(&manifest);
            assert!(
                text.contains(MESH_RPC_TYPE),
                "`{}` generated {FIXTURE} without `{MESH_RPC_TYPE}` anywhere in its \
                 output. A host-route backend dispatches the request to the router by \
                 that type; output without it is a request the lowering dropped — the \
                 silent skip the retired refusal existed to prevent.",
                row.lang
            );
        }
    }
}

/// The retirement is recorded where the gate was, and it names its contract.
///
/// A refusal removed without a word at its site is one the next reader may
/// restore, and one whose record does not say where the roster lives sends
/// that reader to re-derive it from the code.
#[test]
fn the_retirement_site_points_at_its_contract() {
    const SITE: &str = "sce-build/src/generator.rs";
    const HEADER: &str = "// ── Mesh-rpc: who serves it";
    let source = std::fs::read_to_string(repo_root().join(SITE))
        .unwrap_or_else(|e| panic!("{SITE} is readable: {e}"));
    let at = source.find(HEADER).unwrap_or_else(|| {
        panic!(
            "{SITE} carries no `{HEADER}` header. That comment records why the mesh-rpc \
             refusal was retired and where its contract lives."
        )
    });
    let end = source[at..]
        .find("fn mesh_templates_exist_for")
        .expect("the route helper follows the retirement comment");
    let comment = &source[at..at + end];

    // Floor. A region that stopped being found would read as a comment
    // satisfying nothing, and an empty `contains` sweep passes.
    assert!(
        comment.len() > 400,
        "the retirement comment shrank to {} bytes; it carries why the gate went and \
         what pays for it, and a stub cannot.",
        comment.len()
    );
    for needle in ["SCE_MESH.md §9.5", ANCHOR, "mesh_rpc_backend_contract"] {
        assert!(
            comment.contains(needle),
            "the retirement comment does not name `{needle}`. The code has to say where \
             its roster lives, or the next reader takes the code for the roster."
        );
    }
}
