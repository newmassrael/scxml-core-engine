// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Item C1 Path A — per-backend EventSchema codegen
// smoke gate.
//
// Generates each of the 2 standalone EventSchema fixtures (no enum
// imports — those are exercised at the integration-test layer via
// `positive_cross_doc_enum_import_compiles`) on all 6 backends and
// feeds the output to a per-backend syntax checker (g++, syn::parse_file,
// gofmt -e, kotlinc, py_compile, clang -fsyntax-only). The contract
// is structural — no runtime correctness assertion — but every
// backend's generated payload-struct header MUST compile per
// `feedback_byte_goldens_not_compile` + the design RFC §5.2 acceptance
// gate ("6/6 EventSchema backends produce compilable code per
// per-backend harness").
//
// Toolchain-missing behaviour mirrors `forge_codegen_enum_smoke.rs`
// and `codegen_smoke.rs`: detect each compiler with `which` and skip
// with `eprintln!` if absent. CI is expected to install every
// backend's toolchain; local dev machines without one toolchain
// should not be blocked.

use sce_build::toolchain;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Standalone EventSchema fixtures from `tests/fixtures/event_schema/`.
/// Each varies one axis per `feedback_codegen_fixture_orthogonal_axes`:
///   * `schema_job_completed_minimal` — one uint32 field
///   * `schema_job_completed_multi`   — uint32 + bool + string axes
///
/// Cross-doc enum-import fixtures (`schema_job_completed_with_enum.scxml`)
/// require multi-file orchestrator invocation — exercised by the
/// integration test `positive_cross_doc_enum_import_compiles` in
/// `event_schema_kind.rs`, not by this single-file smoke gate.
const FIXTURES: &[&str] = &["schema_job_completed_minimal", "schema_job_completed_multi"];

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build manifest dir has parent (workspace root)")
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/event_schema")
}

fn scratch_for(lang: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge_event_schema_smoke");
    base.join(lang)
}

/// Run `sce-codegen generate -l <lang> -o <out> <fixture>.scxml`. Adds
/// `--go-module-prefix` when targeting Go (resolve_imports rejects bare
/// Go imports; standalone EventSchema fixtures import nothing, but the
/// flag must still be supplied because the CLI validates it up-front).
///
/// C++ is generated with `--no-format`, so the text the assertions read is
/// the generator's own. A formatter is an input to a run only when the run
/// asks for one, and clang-format's output is fixed by no contract: pc2's
/// 18.1.3 wrote `template <typename M>::std::string` where the generator
/// writes `template <typename M> ::std::string`, and pc3, which has no
/// clang-format, handed the generator's text through — so the same tree was
/// green on one host and red on the next (measured 2026-09-24).
fn run_generate(lang: &str, out_dir: &Path, fixture: &str) {
    std::fs::create_dir_all(out_dir).expect("create out dir");
    let fx_path = fixtures_dir().join(format!("{fixture}.scxml"));

    let mut cmd = Command::new(sce_codegen_bin());
    cmd.args(["generate", "-l", lang, "-o"]).arg(out_dir);
    if lang == "go" {
        cmd.args([
            "--go-module-prefix",
            "example.com/sce-forge/event-schema-smoke",
        ]);
    }
    if lang == "cpp" {
        cmd.arg("--no-format");
    }
    cmd.arg(&fx_path);

    let output = cmd.output().expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "sce-codegen generate -l {lang} {fixture} failed (exit {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn find_by_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some(ext) {
            out.push(p);
        }
    }
    out.sort();
    out
}

fn reset_scratch(lang: &str) -> PathBuf {
    let scratch = scratch_for(lang);
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch");
    scratch
}

#[test]
fn smoke_cpp() {
    let Some(gpp) = toolchain::locate("g++") else {
        toolchain::skipped("smoke_cpp: g++ not on PATH");
        return;
    };
    let scratch = reset_scratch("cpp");

    let mut headers: Vec<String> = Vec::new();
    let mut include_dirs: Vec<PathBuf> = Vec::new();
    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("cpp", &out_dir, fixture);
        include_dirs.push(out_dir.clone());
        for hdr in find_by_ext(&out_dir, "h") {
            if let Some(name) = hdr.file_name().and_then(|s| s.to_str()) {
                headers.push(name.to_string());
            }
        }
    }
    assert!(!headers.is_empty(), "no .h artefacts emitted");

    let stub = scratch.join("stub.cpp");
    let mut src = String::new();
    for h in &headers {
        src.push_str(&format!("#include \"{h}\"\n"));
    }
    std::fs::write(&stub, &src).expect("write stub");

    let mut cmd = Command::new(&gpp);
    cmd.args(["-std=c++20", "-fsyntax-only", "-Wall", "-Wextra", "-Werror"]);
    for dir in &include_dirs {
        cmd.arg("-I").arg(dir);
    }
    cmd.arg(&stub);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run g++");

    assert!(
        output.status.success(),
        "g++ -fsyntax-only failed\nheaders: {headers:?}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn smoke_rust() {
    let scratch = reset_scratch("rust");

    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("rust", &out_dir, fixture);
        let rs_files = find_by_ext(&out_dir, "rs");
        assert!(!rs_files.is_empty(), "no .rs emitted for {fixture}");
        for rs in rs_files {
            let src = std::fs::read_to_string(&rs).expect("read rs");
            if let Err(e) = syn::parse_file(&src) {
                panic!(
                    "syn::parse_file failed for {fixture} at {}: {e}",
                    rs.display(),
                );
            }
        }
    }
}

// NL→IR Item C1 Path A (EventSchema MCU native lowering, step 2) — the
// importing-statechart side of the contract. `statechart_minimal`
// imports the minimal schema and reads `_event.data.elapsed_ms` in a
// transition guard. The Rust backend must lower it to a script-engine-
// free native typed-payload comparison: `needs_script_engine = false`,
// a `<Machine>Payload` sum, and a `matches!(&self.pending_payload, …)`
// guard — never the raw ECMAScript cond (which would not even parse:
// `===` is not Rust) and never the `safe_evaluate_guard` script path.
// `syn::parse_file` is the syntactic gate; the substring assertions pin
// the lowering shape so a regression to the raw-cond branch fails loudly.
#[test]
fn statechart_native_lowering_emits_engine_free_typed_guard() {
    let scratch = reset_scratch("rust_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("rust", &out_dir, "statechart_minimal");

    let rs_files = find_by_ext(&out_dir, "rs");
    assert_eq!(rs_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&rs_files[0]).expect("read rs");

    if let Err(e) = syn::parse_file(&src) {
        panic!("generated native-lowered SM does not parse as Rust: {e}");
    }

    assert!(
        src.contains("const NEEDS_SCRIPT_ENGINE: bool = false;"),
        "typed-guard statechart must not require a script engine"
    );
    assert!(
        src.contains("pub enum StatechartMinimalPayload"),
        "expected the per-document typed payload sum"
    );
    assert!(
        src.contains(
            "matches!(&self.pending_payload, StatechartMinimalPayload::JobCompleted(ev) if ev.elapsed_ms == 0)"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: an EXTENSION TRAIT (orphan-rule-clean —
    // `Engine` is foreign, so an inherent impl would be E0116) whose impl
    // binds the event name + payload variant in one call so the pairing
    // cannot be constructed inconsistently (the generic
    // `raise_external_typed_with_data` is the underlying primitive).
    assert!(
        src.contains("pub trait StatechartMinimalInject")
            && src.contains(
                "impl StatechartMinimalInject for ::sce_rust_runtime::Engine<StatechartMinimalPolicy>"
            )
            && src.contains(
                "fn raise_job_completed(&mut self, payload: StatechartMinimalJobCompletedPayload)"
            )
            && src.contains("self.raise_external_typed_with_data(")
            && src.contains("StatechartMinimalPayload::JobCompleted(payload),"),
        "expected the per-event typed inject extension trait (orphan-rule-clean); got:\n{src}"
    );
    // ⚠ `…_with_data`, not `raise_external_typed`: the seam fills BOTH
    // carriers. The typed payload is what a native guard reads; the wire is
    // what the script engine binds `_event.data` from, and a document with an
    // `<assign expr="_event.data.x">` on this same event read nothing from it
    // while only the first was filled (measured 2026-09-22).
    assert!(
        src.contains("::sce_rust_runtime::payload_wire(format_args!(")
            && src.contains("\\\"elapsed_ms\\\":{}"),
        "expected the inject seam to fill the `data` wire too; got:\n{src}"
    );
    // The other direction: an event carrying only that wire — which is every
    // producer except the seam above — has its typed view lifted back out.
    assert!(
        src.contains("fn lift_event_payload(")
            && src.contains("PayloadFields::decode(data)?")
            && src.contains("fields.unsigned::<u32>(\"elapsed_ms\")?"),
        "expected the lift that reads the typed view out of `data`; got:\n{src}"
    );
    assert!(
        !src.contains("safe_evaluate_guard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The raw cond fragment `elapsed_ms ===` would only appear if the
    // pre-lowering ECMAScript guard leaked into the emitted code (`===`
    // is not valid Rust). A precise sentinel — decorative `// ====`
    // separators and `_event.data` prose comments do not match it.
    assert!(
        !src.contains("elapsed_ms ==="),
        "the raw ECMAScript guard must be fully lowered away"
    );
}

// NL→IR Item C1 Path A (EventSchema native lowering) — Go parity for
// `statechart_native_lowering_emits_engine_free_typed_guard`. The Go backend
// must lower the typed `_event.data.elapsed_ms` guard to a script-engine-free
// native comparison: a `<Machine>PayloadTag` enum, a `pendingPayloadTag` +
// `pending<Event>Payload` policy channel, a per-event `Raise<Event>` inject
// seam, and a `p.pendingPayloadTag == … && (…)` guard — never the raw
// ECMAScript cond (`===` is not Go) and never the `p.evaluateGuard(...)`
// script path. `gofmt -e` is the syntactic gate; the substring assertions pin
// the lowering shape. The real compile+run is the committed integration gate
// `backends/go/tests/integration/event_schema_native` (regen:
// scripts/regen_event_schema_native_go.sh).
#[test]
fn statechart_native_lowering_go_emits_engine_free_typed_guard() {
    let Some(gofmt) = toolchain::locate("gofmt") else {
        toolchain::skipped("statechart_native_lowering_go: gofmt not on PATH");
        return;
    };
    let scratch = reset_scratch("go_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("go", &out_dir, "statechart_minimal");

    let go_files = find_by_ext(&out_dir, "go");
    assert_eq!(go_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&go_files[0]).expect("read go");

    let gofmt_ok = Command::new(&gofmt)
        .arg("-e")
        .arg(&go_files[0])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("run gofmt");
    assert!(
        gofmt_ok.status.success(),
        "gofmt -e failed for the native-lowered Go SM\nstderr: {}",
        String::from_utf8_lossy(&gofmt_ok.stderr),
    );

    assert!(
        src.contains("type StatechartMinimalPayloadTag int"),
        "expected the per-document typed payload tag enum; got:\n{src}"
    );
    assert!(
        src.contains(
            "p.pendingPayloadTag == StatechartMinimalPayloadTagJobCompleted && (p.pendingJobCompletedPayload.elapsed_ms == 0)"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: a free function (the Go twin of Rust's
    // extension trait — `Engine` is foreign, so no method can be added) that
    // binds the event name + payload field values in one call, packing them
    // into the type-erased `TypedPayload` carrier AND onto the `Data` wire.
    // Named by its machine, as a package may hold more than one.
    assert!(
        src.contains(
            "func StatechartMinimalRaiseJobCompleted(e *sce.Engine[StatechartMinimalState, StatechartMinimalEvent], elapsed_ms uint32)"
        ) && src.contains("TypedPayload: StatechartMinimalJobCompletedPayload{"),
        "expected the per-event typed inject seam; got:\n{src}"
    );
    // ⚠ Both carriers. The typed payload is what a native guard reads; `Data`
    // is what the script engine binds `_event.data` from, and a document with
    // an `<assign expr="_event.data.x">` on this same event read nothing from
    // it while only the first was filled (measured 2026-09-22).
    assert!(
        src.contains("Data: sce.PayloadJSON(sce.Pair(\"elapsed_ms\", elapsed_ms)),"),
        "expected the inject seam to fill the `Data` wire too; got:\n{src}"
    );
    // The other direction: an event carrying only that wire — which is every
    // producer except the seam above — has its typed view lifted back out.
    assert!(
        src.contains("func (p *StatechartMinimalPolicy) LiftTypedPayload(")
            && src.contains("sce.LiftPayload(meta.Data)")
            && src.contains("sce.PayloadUnsigned[uint32](fields, \"elapsed_ms\")"),
        "expected the lift that reads the typed view out of `Data`; got:\n{src}"
    );
    assert!(
        !src.contains("evaluateGuard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The `// "…" lowered to …` traceability comment echoes the raw cond
    // (`elapsed_ms === 0`), so a bare `elapsed_ms ===` sentinel would false-
    // positive on it (the same choice the C11 smoke makes). A precise sentinel
    // instead: the raw ECMAScript cond would only reach emitted code via the
    // `{% else %} if {{ trans.cond }}` arm, i.e. `if _event.data.elapsed_ms === 0 {`.
    assert!(
        !src.contains("if _event.data.elapsed_ms === 0 {"),
        "the raw ECMAScript guard must be fully lowered away (not leaked into the else arm)"
    );
}

/// The names a Go source declares at package level: each function, type,
/// variable and constant, in the single and the parenthesised form, and each
/// method as `Receiver.Method`. `_` and `init` may be declared any number of
/// times in a package and are left out.
fn go_package_level_names(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_block = false;
    for line in src.lines() {
        if in_block {
            if line.starts_with(')') {
                in_block = false;
            } else if let Some(entry) = line.strip_prefix('\t') {
                names.extend(go_leading_identifier(entry));
            }
            continue;
        }
        if matches!(line, "const (" | "var (" | "type (") {
            in_block = true;
        } else if let Some(rest) = line.strip_prefix("func (") {
            let Some((receiver, after)) = rest.split_once(')') else {
                continue;
            };
            let receiver = receiver
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .trim_start_matches('*');
            let receiver = receiver.split('[').next().unwrap_or_default();
            if let Some(method) = go_leading_identifier(after.trim_start()) {
                names.push(format!("{receiver}.{method}"));
            }
        } else if let Some(rest) = line.strip_prefix("func ") {
            names.extend(go_leading_identifier(rest).filter(|name| name != "init"));
        } else if let Some(rest) = ["type ", "var ", "const "]
            .into_iter()
            .find_map(|keyword| line.strip_prefix(keyword))
        {
            names.extend(go_leading_identifier(rest));
        }
    }
    names
}

/// The identifier `text` begins with, unless it is the blank one.
fn go_leading_identifier(text: &str) -> Option<String> {
    let name: String = text
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let first = name.chars().next()?;
    (name != "_" && !first.is_ascii_digit()).then_some(name)
}

/// Each machine file of one Go package, by file name, with the package-level
/// names it declares ([`go_package_level_names`]).
type GoPackageMachines = Vec<(String, Vec<String>)>;

/// Generate `document` for `lang` into `out_dir`, with `extra` arguments. Go is
/// given the module prefix its imports are written under.
fn generate_into(lang: &str, document: &Path, out_dir: &Path, extra: &[&str]) {
    std::fs::create_dir_all(out_dir).expect("create out dir");
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.args(["generate", "-l", lang, "-o"]).arg(out_dir);
    if lang == "go" {
        cmd.args([
            "--go-module-prefix",
            "example.com/sce-forge/event-schema-smoke",
        ]);
    }
    let output = cmd
        .args(extra)
        .arg(document)
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "sce-codegen generate -l {lang} {extra:?} {} failed (exit {:?})\nstderr: {}",
        document.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Generate `document` for Go into `out_dir`, with `extra` arguments.
fn generate_go_into(document: &Path, out_dir: &Path, extra: &[&str]) {
    generate_into("go", document, out_dir, extra);
}

/// The package a Go or Kotlin source declares in its `package` clause.
fn package_clause(src: &str) -> Option<&str> {
    src.lines()
        .find_map(|line| line.strip_prefix("package "))
        .map(str::trim)
}

/// Generate `document` for Go into `out_dir`, as a parent and every child it
/// invokes are generated: into one package. Answers each machine file written,
/// by name and in name order, with the names it declares at package level.
fn generate_go_package(document: &Path, out_dir: &Path) -> GoPackageMachines {
    generate_go_into(document, out_dir, &[]);
    go_package_machines(out_dir)
}

/// Each machine file in `out_dir`, by name and in name order, with the names it
/// declares at package level.
fn go_package_machines(out_dir: &Path) -> GoPackageMachines {
    find_by_ext(out_dir, "go")
        .into_iter()
        .filter(|path| path.to_string_lossy().ends_with("_sm.go"))
        .map(|path| {
            let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&path).expect("read go");
            (file_name, go_package_level_names(&src))
        })
        .collect()
}

/// Every name more than one declaration in `machines` gives, with the files
/// that declare it — a package that does not compile.
fn declared_more_than_once(machines: &GoPackageMachines) -> Vec<(String, Vec<String>)> {
    let mut declared: std::collections::BTreeMap<&str, Vec<String>> =
        std::collections::BTreeMap::new();
    for (file_name, names) in machines {
        for name in names {
            declared
                .entry(name.as_str())
                .or_default()
                .push(file_name.clone());
        }
    }
    declared
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(name, files)| (name.to_string(), files))
        .collect()
}

/// A parent and every child it invokes are generated into one Go package, and
/// the two may import the same EventSchema: `record_list_parent` guards on four
/// events of the standard record list it invokes, whose machine guards on them
/// too. Each machine has a typed inject seam for each of them, and a seam named
/// by its event alone (`RaiseRecordListEdit`) was declared once by each machine,
/// so the package did not compile. Every package-level name the package's
/// machines declare is declared once.
#[test]
fn a_go_package_declares_each_name_once_where_a_parent_and_its_child_share_a_schema() {
    let out_dir = reset_scratch("go_shared_package");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/static_datamodel/record_list_parent.scxml");
    let machines = generate_go_package(&fixture, &out_dir);
    let file_names: Vec<&str> = machines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        file_names,
        [
            "record_list_parent__sce_synth_invoke__list_sm.go",
            "record_list_parent_sm.go"
        ],
        "the parent and its in-line child are generated into one package"
    );
    for (file_name, names) in &machines {
        // The shared event's seam is there in each machine, so the check below
        // reads the declarations that collided rather than a file it misread.
        assert!(
            names
                .iter()
                .any(|name| name.ends_with("RaiseRecordListEdit")),
            "{file_name} declares no inject seam for `record_list.edit`: {names:?}"
        );
    }
    let twice = declared_more_than_once(&machines);
    assert!(
        twice.is_empty(),
        "names declared more than once in the package: {twice:?}"
    );
}

/// What `shared_state` and the child it invokes both hold — a record of the
/// standard clock and a value of the standard `rpc_status` enum — and the call
/// of the standard `hlc_send` both make with the record, the same way.
const SHARED_STATE_IMPORTS: &str = r#"
  <sce:import kind="event-schema" src="sce:std/merge/hlc_timestamp.scxml" as="Hlc"/>
  <sce:import kind="algorithm" src="sce:std/merge/hlc_send.scxml" as="HlcSend"/>
  <sce:import kind="enum" src="sce:std/mesh/rpc_status.scxml" as="Status"/>"#;
const SHARED_STATE_DATA: &str = r#"
    <data id="status" sce:type="enum:Status" expr="Status.ok"/>
    <data id="now" sce:type="int64" expr="0"/>
    <data id="node" sce:type="uint64" expr="0"/>
    <data id="clock" sce:type="record:Hlc">
      <sce:set name="wallTime" expr="0"/>
      <sce:set name="counter" expr="0"/>
      <sce:set name="nodeId" expr="0"/>
    </data>"#;
const SHARED_STATE_TICK: &str = r#"
    <transition event="tick" type="internal">
      <assign location="clock" expr="HlcSend(clock, now, node)"/>
    </transition>"#;

/// `shared_state` and `shared_state_kid`, written into `dir`. The parent holds
/// [`SHARED_STATE_DATA`] and invokes the child as a candidate of a hybrid
/// `<invoke>` (`sce:candidates`), which is generated into the parent's package;
/// the child holds the same and makes the same call. The child written in-line
/// instead is [`a_go_package_declares_each_name_once_where_an_in_line_child_holds_the_same_state`]'s.
fn shared_state_documents(dir: &Path) -> (PathBuf, PathBuf) {
    let parent = dir.join("shared_state.scxml");
    std::fs::write(
        &parent,
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="shared_state">{SHARED_STATE_IMPORTS}
  <datamodel>
    <data id="pick" sce:type="string" sce:capacity="48" expr="'file:shared_state_kid.scxml'"/>{SHARED_STATE_DATA}
  </datamodel>
  <state id="running">
    <invoke type="scxml" id="kid" srcexpr="pick" sce:candidates="shared_state_kid.scxml"/>{SHARED_STATE_TICK}
  </state>
</scxml>"#
        ),
    )
    .expect("write shared_state");
    let child = dir.join("shared_state_kid.scxml");
    std::fs::write(
        &child,
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="shared_state_kid">{SHARED_STATE_IMPORTS}
  <datamodel>{SHARED_STATE_DATA}
  </datamodel>
  <state id="running">{SHARED_STATE_TICK}
  </state>
</scxml>"#
        ),
    )
    .expect("write shared_state_kid");
    (parent, child)
}

/// A parent and the child it invokes, in one Go package, that hold the same
/// state and call one algorithm the same way. Each has an adapter for the call,
/// which takes and gives back the machine's own record, and an adapter named by
/// the call alone (`sceAdapt_HlcSend__v`) was declared once by each machine;
/// each holds the enum in a type of its own, and a constant named by the enum
/// alone (`RpcStatusOk`) was declared once by each machine. Every package-level
/// name the package's machines declare is declared once.
///
/// The child is generated into the parent's package as the committed Go trees
/// generate a candidate (`--as-child --parent-stem`).
#[test]
fn a_go_package_declares_each_name_once_where_a_parent_and_its_child_hold_the_same_state() {
    let scratch = reset_scratch("go_shared_state");
    let (parent, child) = shared_state_documents(&scratch);
    let out_dir = scratch.join("out");
    generate_go_into(&parent, &out_dir, &[]);
    generate_go_into(
        &child,
        &out_dir,
        &["--as-child", "--parent-stem", "shared_state"],
    );
    let machines = go_package_machines(&out_dir);
    let file_names: Vec<&str> = machines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        file_names,
        ["shared_state_kid_sm.go", "shared_state_sm.go"],
        "the parent and its candidate are generated into one package"
    );
    assert_each_machine_holds_the_shared_state_once(&machines);
}

/// Each machine of `machines` holds [`SHARED_STATE_DATA`] and makes the call of
/// [`SHARED_STATE_TICK`], and no package-level name is declared twice.
fn assert_each_machine_holds_the_shared_state_once(machines: &GoPackageMachines) {
    for (file_name, names) in machines {
        // Each machine has its adapter of the call, so the check below reads the
        // declarations that collided rather than a file it misread.
        assert!(
            names
                .iter()
                .any(|name| name.starts_with("sceAdapt_") && name.ends_with("HlcSend__v")),
            "{file_name} declares no adapter of `HlcSend(clock, now, node)`: {names:?}"
        );
        assert!(
            names
                .iter()
                .any(|name| name.contains("Status") && name.ends_with("Ok")),
            "{file_name} declares no constant for `Status.ok`: {names:?}"
        );
    }
    let twice = declared_more_than_once(machines);
    assert!(
        twice.is_empty(),
        "names declared more than once in the package: {twice:?}"
    );
}

/// `in_line_shared_state`, written into `dir`: a parent that holds
/// [`SHARED_STATE_DATA`] and makes the call of [`SHARED_STATE_TICK`], and starts,
/// written in-line under the id `kid`, a child that imports, holds and calls the
/// same.
fn in_line_shared_state_document(dir: &Path) -> PathBuf {
    let parent = dir.join("in_line_shared_state.scxml");
    std::fs::write(
        &parent,
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="in_line_shared_state">{SHARED_STATE_IMPORTS}
  <datamodel>{SHARED_STATE_DATA}
  </datamodel>
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="running" datamodel="sce-static">{SHARED_STATE_IMPORTS}
          <datamodel>{SHARED_STATE_DATA}
          </datamodel>
          <state id="running">{SHARED_STATE_TICK}
          </state>
        </scxml>
      </content>
    </invoke>{SHARED_STATE_TICK}
  </state>
</scxml>"#
        ),
    )
    .expect("write in_line_shared_state");
    parent
}

/// [`a_go_package_declares_each_name_once_where_a_parent_and_its_child_hold_the_same_state`]
/// with the child written in-line. An in-line child was parsed with no
/// directory, so none of its `<sce:import>`s resolved and `Status.ok` in it was
/// an unknown identifier: the document was refused. Its imports resolve as its
/// parent's do, and the two machines, written into one package, declare every
/// package-level name once.
#[test]
fn a_go_package_declares_each_name_once_where_an_in_line_child_holds_the_same_state() {
    let scratch = reset_scratch("go_in_line_shared_state");
    let parent = in_line_shared_state_document(&scratch);
    let machines = generate_go_package(&parent, &scratch.join("out"));
    let file_names: Vec<&str> = machines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        file_names,
        [
            "in_line_shared_state__sce_synth_invoke__kid_sm.go",
            "in_line_shared_state_sm.go"
        ],
        "the parent and its in-line child are generated into one package"
    );
    assert_each_machine_holds_the_shared_state_once(&machines);
}

/// The backends that lower `datamodel="sce-static"`, by the name `-l` takes.
const STATIC_DATAMODEL_BACKENDS: [&str; 6] = ["rust", "cpp", "c11", "go", "kotlin", "python"];

/// A parent whose in-line child imports an enum of the standard library by its
/// library name and an algorithm beside the parent by a relative `src`, and
/// uses both: `Status.ok` in a guard, `DaysInMonth(year, month)` in an
/// `<assign>`.
const IN_LINE_IMPORTS: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="in_line_imports">
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="counting" datamodel="sce-static">
          <sce:import kind="enum" src="sce:std/mesh/rpc_status.scxml" as="Status"/>
          <sce:import kind="algorithm" src="algorithm_days_in_month.scxml" as="DaysInMonth"/>
          <datamodel>
            <data id="status" sce:type="enum:Status" expr="Status.ok"/>
            <data id="year" sce:type="uint16" expr="2024"/>
            <data id="month" sce:type="uint8" expr="2"/>
            <data id="days" sce:type="uint8" expr="0"/>
          </datamodel>
          <state id="counting">
            <onentry>
              <assign location="days" expr="DaysInMonth(year, month)"/>
            </onentry>
            <transition event="tick" cond="status === Status.ok" target="done"/>
          </state>
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"#;

/// An in-line child's `<sce:import>`s resolve as the enclosing document's do,
/// on every backend that lowers `sce-static`: a library name from the library,
/// a relative `src` against the directory of the document whose `<content>`
/// holds the child. The child was parsed with no directory, so neither
/// resolved and the run was refused (`expression/unknown-identifier` for
/// `Status.ok`). Each run succeeds and writes the child's machine, which names
/// the algorithm it calls.
#[test]
fn an_in_line_childs_imports_resolve_as_its_parents_do_on_every_backend() {
    let scratch = reset_scratch("in_line_imports");
    let parent = in_line_imports_document(&scratch);
    for lang in STATIC_DATAMODEL_BACKENDS {
        let out_dir = scratch.join("out").join(lang);
        let extra: &[&str] = if lang == "cpp" { &["--no-format"] } else { &[] };
        generate_into(lang, &parent, &out_dir, extra);
        // The child's machine files, and not the copy of its document the run
        // writes beside them, which holds the call as its author wrote it.
        let child_files: Vec<PathBuf> =
            machine_files_of(&out_dir, "in_line_imports__sce_synth_invoke__kid");
        assert!(
            !child_files.is_empty(),
            "{lang}: the in-line child's machine was not written into {}",
            out_dir.display()
        );
        assert!(
            spelling_free(&read_all(&child_files)).contains("daysinmonth"),
            "{lang}: the in-line child's machine does not name the algorithm it calls"
        );
    }
}

/// [`IN_LINE_IMPORTS`], written into `dir` as `in_line_imports.scxml`, with the
/// algorithm its in-line child imports by a relative `src` beside it.
fn in_line_imports_document(dir: &Path) -> PathBuf {
    let parent = dir.join("in_line_imports.scxml");
    std::fs::write(&parent, IN_LINE_IMPORTS).expect("write in_line_imports");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/static_datamodel/algorithm_days_in_month.scxml"),
        dir.join("algorithm_days_in_month.scxml"),
    )
    .expect("copy the algorithm beside the document");
    parent
}

/// The staged flow of the integration regeneration scripts: the document is
/// copied into a stage directory and generated there with `--input-root`
/// naming the directory it was copied from, and the in-line child the run
/// writes beside it (`<parent>__sce_synth_invoke__<id>.scxml`) is generated
/// again from there `--as-child --parent-stem`. Neither copy has the algorithm
/// its child imports beside it, and both runs read the import from the stage
/// directory and were refused. A staged document's relative imports that are
/// not beside it are read from the input root, the directory its author wrote
/// them from and the one the `source-hash` covers; both runs succeed, and the
/// child's machine is in the parent's package.
#[test]
fn a_staged_copy_reads_its_relative_imports_from_the_input_root() {
    let scratch = reset_scratch("in_line_imports_staged");
    let source_dir = scratch.join("source");
    std::fs::create_dir_all(&source_dir).expect("create source dir");
    let source = in_line_imports_document(&source_dir);
    let stage = scratch.join("stage");
    std::fs::create_dir_all(&stage).expect("create stage dir");
    let staged = stage.join("in_line_imports.scxml");
    std::fs::copy(&source, &staged).expect("stage the document");
    let input_root = source_dir.to_string_lossy().into_owned();
    generate_go_into(&staged, &stage, &["--input-root", &input_root]);
    let child = stage.join("in_line_imports__sce_synth_invoke__kid.scxml");
    assert!(
        child.is_file(),
        "the run did not write its in-line child's document beside its output"
    );
    generate_go_into(
        &child,
        &stage,
        &[
            "--as-child",
            "--parent-stem",
            "in_line_imports",
            "--input-root",
            &input_root,
        ],
    );
    let machines = go_package_machines(&stage);
    let file_names: Vec<&str> = machines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        file_names,
        [
            "in_line_imports__sce_synth_invoke__kid_sm.go",
            "in_line_imports_sm.go"
        ],
        "the staged parent and its in-line child are generated into the stage"
    );
    for file_name in file_names {
        let src = std::fs::read_to_string(stage.join(file_name)).expect("read go");
        assert_eq!(
            package_clause(&src),
            Some("in_line_imports"),
            "{file_name} is not in the parent's package"
        );
    }
}

/// The text of every file of `files`, one after another.
fn read_all(files: &[PathBuf]) -> String {
    files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("read a generated file"))
        .collect()
}

/// `text` with the spellings the backends derive from one name
/// (`in_line_chain`, `InLineChain`, `IN_LINE_CHAIN`, `inLineChain`) made one:
/// lowercased, with no underscore.
fn spelling_free(text: &str) -> String {
    text.to_lowercase().replace('_', "")
}

/// The files in `dir` a machine named `machine` was generated into: every file
/// whose name is the machine's followed by what a backend appends to it
/// (`_sm.rs`, `Sm.kt`, `_sm.h`, ...). Neither the copy of the machine's
/// document a run writes beside it (`<machine>.scxml`) nor the machine of a
/// child it starts (`<machine>__sce_synth_invoke__<id>_sm.rs`) is one.
fn machine_files_of(dir: &Path, machine: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("read the output directory")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix(machine))
                .is_some_and(|rest| rest.starts_with("_sm") || rest.starts_with("Sm"))
        })
        .collect();
    files.sort();
    files
}

/// A parent that invokes a child in-line under the id `kid`, whose child invokes
/// its own in-line under the same id.
const NESTED_INVOKE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="nested_invoke">
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting">
          <state id="waiting">
            <invoke type="scxml" id="kid">
              <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done">
                  <final id="done"/>
                </scxml>
              </content>
            </invoke>
          </state>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"#;

/// A parent and the child it invokes, in one Go package, that each invoke a
/// child under the same id. Each has a wrapper of the engine it starts, and a
/// wrapper named by the invoke id alone (`childEngineWrapperKid`) was declared
/// once by each machine. Every package-level name the parent and its child
/// declare is declared once.
#[test]
fn a_go_package_declares_each_name_once_where_a_child_invokes_under_its_parents_id() {
    let scratch = reset_scratch("go_nested_invoke");
    let parent = scratch.join("nested_invoke.scxml");
    std::fs::write(&parent, NESTED_INVOKE).expect("write nested_invoke");
    let machines = generate_go_package(&parent, &scratch.join("out"));
    for file_name in [
        "nested_invoke_sm.go",
        "nested_invoke__sce_synth_invoke__kid_sm.go",
    ] {
        let names = &machines
            .iter()
            .find(|(name, _)| name == file_name)
            .unwrap_or_else(|| panic!("{file_name} was not generated: {machines:?}"))
            .1;
        // Each of the two starts a child, so the check below reads the
        // declarations that collided rather than a file it misread.
        assert!(
            names
                .iter()
                .any(|name| name.starts_with("childEngineWrapper") && !name.contains('.')),
            "{file_name} declares no wrapper of the child it invokes: {names:?}"
        );
    }
    let twice = declared_more_than_once(&machines);
    assert!(
        twice.is_empty(),
        "names declared more than once in the package: {twice:?}"
    );
}

/// Three machines in a chain, each written in-line in the one before it:
/// `in_line_chain` starts `kid`, which starts a `kid` of its own. The second id
/// is the first's, so the grandchild's name differs from the child's by its
/// depth alone.
const IN_LINE_CHAIN: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="in_line_chain">
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting"
               datamodel="sce-static">
          <state id="waiting">
            <invoke type="scxml" id="kid">
              <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle"
                       datamodel="sce-static">
                  <state id="idle">
                    <transition event="stop" target="done"/>
                  </state>
                  <final id="done"/>
                </scxml>
              </content>
            </invoke>
          </state>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"#;

/// The machines of [`IN_LINE_CHAIN`], each followed by the one it starts.
const IN_LINE_CHAIN_MACHINES: [&str; 3] = [
    "in_line_chain",
    "in_line_chain__sce_synth_invoke__kid",
    "in_line_chain__sce_synth_invoke__kid__sce_synth_invoke__kid",
];

/// [`IN_LINE_CHAIN`], written into `dir`.
fn in_line_chain_document(dir: &Path) -> PathBuf {
    let document = dir.join("in_line_chain.scxml");
    std::fs::write(&document, IN_LINE_CHAIN).expect("write in_line_chain");
    document
}

/// A plain `sce-codegen generate` writes every machine of an in-line chain, on
/// every backend that writes a child as a machine of its own: the document's
/// in-line children were written and their own in-line children were not, so
/// the child named a machine that no run wrote. Each machine is written, and
/// each names the machine it starts.
#[test]
fn every_machine_of_an_in_line_chain_is_generated_on_every_backend() {
    let scratch = reset_scratch("in_line_chain");
    let document = in_line_chain_document(&scratch);
    for lang in STATIC_DATAMODEL_BACKENDS {
        let out_dir = scratch.join("out").join(lang);
        let extra: &[&str] = if lang == "cpp" { &["--no-format"] } else { &[] };
        generate_into(lang, &document, &out_dir, extra);
        let texts: Vec<String> = IN_LINE_CHAIN_MACHINES
            .iter()
            .map(|machine| {
                let files = machine_files_of(&out_dir, machine);
                assert!(
                    !files.is_empty(),
                    "{lang}: {machine} was not written into {}",
                    out_dir.display()
                );
                spelling_free(&read_all(&files))
            })
            .collect();
        for (text, (machine, started)) in texts.iter().zip(
            IN_LINE_CHAIN_MACHINES
                .iter()
                .zip(IN_LINE_CHAIN_MACHINES.iter().skip(1)),
        ) {
            assert!(
                text.contains(&spelling_free(started)),
                "{lang}: {machine} does not name {started}, the machine it starts"
            );
        }
    }
}

/// The Go package of an in-line chain: the three machines are written into the
/// document's package, and the names they declare at package level are
/// declared once — the grandchild is named after the child that starts it, so
/// its declarations stand apart from the child's though its invoke id is the
/// child's own.
#[test]
fn a_go_package_holds_every_machine_of_an_in_line_chain_once() {
    let scratch = reset_scratch("go_in_line_chain");
    let document = in_line_chain_document(&scratch);
    let out_dir = scratch.join("out");
    let machines = generate_go_package(&document, &out_dir);
    let file_names: Vec<&str> = machines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        file_names,
        [
            "in_line_chain__sce_synth_invoke__kid__sce_synth_invoke__kid_sm.go",
            "in_line_chain__sce_synth_invoke__kid_sm.go",
            "in_line_chain_sm.go",
        ],
        "the document, its in-line child and that child's in-line child are \
         generated into one directory"
    );
    for path in find_by_ext(&out_dir, "go") {
        let src = std::fs::read_to_string(&path).expect("read go");
        assert_eq!(
            package_clause(&src),
            Some("in_line_chain"),
            "{} is not in the document's package",
            path.display()
        );
    }
    let twice = declared_more_than_once(&machines);
    assert!(
        twice.is_empty(),
        "names declared more than once in the package: {twice:?}"
    );
}

/// Under `--as-child --parent-stem`, every machine of the chain the document
/// starts is placed in the package of the machine that starts the document, as
/// the document is: each is written beside it and named unqualified by the
/// machine that starts it. Go and Kotlin, the two backends whose machines
/// declare a package.
#[test]
fn every_machine_of_an_in_line_chain_joins_the_package_its_parent_stem_names() {
    let scratch = reset_scratch("in_line_chain_as_child");
    let document = in_line_chain_document(&scratch);
    for (lang, extra, package) in [
        ("go", &[][..], "holder"),
        (
            "kotlin",
            &["--kotlin-package-prefix", "com.sce.integration"][..],
            "com.sce.integration.holder",
        ),
    ] {
        let out_dir = scratch.join("out").join(lang);
        let args: Vec<&str> = ["--as-child", "--parent-stem", "holder"]
            .into_iter()
            .chain(extra.iter().copied())
            .collect();
        generate_into(lang, &document, &out_dir, &args);
        for machine in IN_LINE_CHAIN_MACHINES {
            let files = machine_files_of(&out_dir, machine);
            assert!(
                !files.is_empty(),
                "{lang}: {machine} was not written into {}",
                out_dir.display()
            );
            for path in &files {
                let src = std::fs::read_to_string(path).expect("read the machine");
                assert_eq!(
                    package_clause(&src),
                    Some(package),
                    "{lang}: {} is not in the package --parent-stem names",
                    path.display()
                );
            }
        }
    }
}

/// Two in-line children of one document that the infix alone would give one
/// name: `a`'s own child `b` is `<doc>__sce_synth_invoke__a__sce_synth_invoke__b`,
/// and so is a child beside `a` whose id is `a__sce_synth_invoke__b`.
const IN_LINE_NAME_CLASH: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="in_line_name_clash">
  <state id="running">
    <invoke type="scxml" id="a">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting"
               datamodel="sce-static">
          <state id="waiting">
            <invoke type="scxml" id="b">
              <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done"
                       datamodel="sce-static">
                  <final id="done"/>
                </scxml>
              </content>
            </invoke>
          </state>
        </scxml>
      </content>
    </invoke>
    <invoke type="scxml" id="a__sce_synth_invoke__b">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="done"
               datamodel="sce-static">
          <final id="done"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"#;

/// An `<invoke>` whose child is carried in the document may not have an id
/// holding `sce_synth_invoke`, the word the name of each such child is joined
/// with: with it, a child beside another could take the name of that one's
/// child, and the two would be written to one file. The run is refused at the
/// id, and writes no machine.
#[test]
fn an_in_line_invoke_id_holding_the_synthesised_name_word_is_refused() {
    let scratch = reset_scratch("in_line_name_clash");
    let document = scratch.join("in_line_name_clash.scxml");
    std::fs::write(&document, IN_LINE_NAME_CLASH).expect("write in_line_name_clash");
    let out_dir = scratch.join("out");
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let output = Command::new(sce_codegen_bin())
        .args(["generate", "-l", "rust", "-o"])
        .arg(&out_dir)
        .arg(&document)
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "a document whose two in-line children share a name was generated\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("invalid id value 'a__sce_synth_invoke__b'"),
        "the refusal does not stand at the id that holds the word\nstderr: {stderr}"
    );
    assert!(
        find_by_ext(&out_dir, "rs").is_empty(),
        "a refused run wrote a machine"
    );
}

/// Two in-line children at one level: the first names no id, the second is
/// `invoke_0`. The first was given `_invoke_0`, whose leading underscore was
/// trimmed into the same machine name as the second's.
const IN_LINE_GENERATED_BESIDE_AUTHORED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="in_line_ids">
  <state id="running">
    <invoke type="scxml">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle"
               datamodel="sce-static">
          <state id="idle"/>
        </scxml>
      </content>
    </invoke>
    <invoke type="scxml" id="invoke_0">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle"
               datamodel="sce-static">
          <state id="idle"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"#;

/// An in-line child that names no id and one whose author named it `invoke_0`
/// are two machines in one Go package: the id generated for the first is
/// numbered past the author's, so the two have two names, and every
/// package-level name of the package is declared once. One file was written
/// for both, and the parent declared what it holds of each child twice.
#[test]
fn a_generated_and_an_authored_invoke_id_name_two_in_line_children() {
    let scratch = reset_scratch("go_in_line_ids");
    let document = scratch.join("in_line_ids.scxml");
    std::fs::write(&document, IN_LINE_GENERATED_BESIDE_AUTHORED).expect("write in_line_ids");
    let machines = generate_go_package(&document, &scratch.join("out"));
    let file_names: Vec<&str> = machines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        file_names,
        [
            "in_line_ids__sce_synth_invoke__invoke_0_sm.go",
            "in_line_ids__sce_synth_invoke__invoke_1_sm.go",
            "in_line_ids_sm.go",
        ],
        "each in-line child is a machine of its own"
    );
    let twice = declared_more_than_once(&machines);
    assert!(
        twice.is_empty(),
        "names declared more than once in the package: {twice:?}"
    );
}

/// A document whose in-line child `kid` starts an in-line `kid` of its own and,
/// by `src`, the document [`IN_LINE_LISTING_LEAF`] beside the parent.
const IN_LINE_LISTING: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="running" name="in_line_listing">
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting">
          <state id="waiting">
            <invoke type="scxml" id="kid">
              <content>
                <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle">
                  <state id="idle"/>
                </scxml>
              </content>
            </invoke>
            <invoke type="scxml" id="beside" src="in_line_listing_leaf.scxml"/>
          </state>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>"#;

/// The document [`IN_LINE_LISTING`]'s in-line child starts by `src`.
const IN_LINE_LISTING_LEAF: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="idle" name="in_line_listing_leaf">
  <state id="idle"/>
</scxml>"#;

/// The C++ children listing (`<stem>_children.txt`) names the children of
/// every machine the run writes, the in-line child's as well as the
/// document's, and the document that child starts by `src` is staged into the
/// output directory as one the document starts is. The listing named the
/// document's own children only, and nothing staged the in-line child's
/// `src` child, whose machine the in-line child's machine includes.
#[test]
fn a_cpp_children_listing_names_the_children_of_every_machine_the_run_writes() {
    let scratch = reset_scratch("cpp_in_line_listing");
    let document = scratch.join("in_line_listing.scxml");
    std::fs::write(&document, IN_LINE_LISTING).expect("write in_line_listing");
    std::fs::write(
        scratch.join("in_line_listing_leaf.scxml"),
        IN_LINE_LISTING_LEAF,
    )
    .expect("write in_line_listing_leaf");
    let out_dir = scratch.join("out");
    generate_into("cpp", &document, &out_dir, &["--no-format"]);
    let listing = std::fs::read_to_string(out_dir.join("in_line_listing_children.txt"))
        .expect("read the children listing");
    let listed: Vec<&str> = listing.lines().collect();
    assert_eq!(
        listed,
        [
            "in_line_listing__sce_synth_invoke__kid",
            "in_line_listing__sce_synth_invoke__kid__sce_synth_invoke__kid",
            "in_line_listing_leaf",
        ],
        "the listing names each child of each machine the run writes, once"
    );
    assert!(
        out_dir.join("in_line_listing_leaf.scxml").is_file(),
        "the document the in-line child starts by `src` was not staged beside the output"
    );
}

/// The machine `static_child_mesh` and the candidate its hybrid `<invoke>`
/// names, `static_mesh_relay`, generated for `lang` into `out_dir` as the
/// committed trees generate them: the candidate `--as-child --parent-stem
/// static_child_mesh`, with `extra` arguments to both. The candidate starts an
/// in-line child of its own, `inner`, which is written beside it.
fn generate_child_mesh_package(lang: &str, out_dir: &Path, extra: &[&str]) {
    let fixtures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel");
    generate_into(
        lang,
        &fixtures.join("static_child_mesh.scxml"),
        out_dir,
        extra,
    );
    let as_child: Vec<&str> = ["--as-child", "--parent-stem", "static_child_mesh"]
        .into_iter()
        .chain(extra.iter().copied())
        .collect();
    generate_into(
        lang,
        &fixtures.join("static_mesh_relay.scxml"),
        out_dir,
        &as_child,
    );
}

/// A candidate generated into its parent's package (`--as-child --parent-stem`)
/// that starts an in-line child of its own. The candidate refers to the child's
/// machine unqualified, so the child belongs to the same package, the parent's;
/// it was given one named after the candidate (`package static_mesh_relay`), a
/// second package in one directory, which Go refuses. Every Go file written
/// declares the parent's package, and every package-level name the four machines
/// declare is declared once: the candidate's child is named after the
/// candidate, apart from the parent's own child.
#[test]
fn a_go_candidates_in_line_child_is_generated_into_its_parents_package() {
    let out_dir = reset_scratch("go_candidate_in_line_child");
    generate_child_mesh_package("go", &out_dir, &[]);
    let machines = go_package_machines(&out_dir);
    let file_names: Vec<&str> = machines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        file_names,
        [
            "static_child_mesh__sce_synth_invoke__list_sm.go",
            "static_child_mesh_sm.go",
            "static_mesh_relay__sce_synth_invoke__inner_sm.go",
            "static_mesh_relay_sm.go",
        ],
        "the parent, its in-line child, the candidate and the candidate's in-line \
         child are generated into one directory"
    );
    for path in find_by_ext(&out_dir, "go") {
        let src = std::fs::read_to_string(&path).expect("read go");
        assert_eq!(
            package_clause(&src),
            Some("static_child_mesh"),
            "{} is not in the package of the machine that starts the candidate",
            path.display()
        );
    }
    for (file_name, names) in &machines {
        // Each machine declares its policy, so the check below reads the
        // declarations of four machines rather than files it misread.
        assert!(
            names.iter().any(|name| name.ends_with("Policy")),
            "{file_name} declares no policy of its machine: {names:?}"
        );
    }
    let twice = declared_more_than_once(&machines);
    assert!(
        twice.is_empty(),
        "names declared more than once in the package: {twice:?}"
    );
}

/// The Kotlin side of
/// [`a_go_candidates_in_line_child_is_generated_into_its_parents_package`]: the
/// candidate's in-line child was given a package named after the candidate
/// (`static_mesh_relay` under the prefix), where the candidate, in the parent's
/// package, cannot name it unqualified. Every machine file declares the
/// parent's package.
#[test]
fn a_kotlin_candidates_in_line_child_is_generated_into_its_parents_package() {
    let out_dir = reset_scratch("kotlin_candidate_in_line_child");
    generate_child_mesh_package(
        "kotlin",
        &out_dir,
        &["--kotlin-package-prefix", "com.sce.integration"],
    );
    let machine_files: Vec<PathBuf> = find_by_ext(&out_dir, "kt")
        .into_iter()
        .filter(|path| path.to_string_lossy().ends_with("Sm.kt"))
        .collect();
    let mut file_names: Vec<String> = machine_files
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    file_names.sort();
    let mut expected = [
        "static_child_meshSm.kt",
        "static_child_mesh__sce_synth_invoke__listSm.kt",
        "static_mesh_relaySm.kt",
        "static_mesh_relay__sce_synth_invoke__innerSm.kt",
    ];
    expected.sort_unstable();
    assert_eq!(
        file_names, expected,
        "the parent, its in-line child, the candidate and the candidate's in-line \
         child are generated into one directory"
    );
    for path in &machine_files {
        let src = std::fs::read_to_string(path).expect("read kotlin");
        assert_eq!(
            package_clause(&src),
            Some("com.sce.integration.static_child_mesh"),
            "{} is not in the package of the machine that starts the candidate",
            path.display()
        );
    }
}

// NL→IR Item C1 Path A (EventSchema native lowering) — Kotlin parity for
// `statechart_native_lowering_emits_engine_free_typed_guard`. The Kotlin
// backend lowers the typed `_event.data.elapsed_ms` guard to a script-engine-
// free native comparison over the type-erased `EventMetadata.typedPayload`
// carrier: a per-event payload data class, a nullable `pending<Event>Payload`
// field, a `populateTypedPayload` override that `is`-checks the carrier, a
// per-event `raise<Event>` inject seam, and a `pending<Event>Payload != null &&
// (…)` guard — never the raw ECMAScript cond (`===` is not Kotlin) and never
// the `safeEvaluateGuard(...)` script path. The substring assertions pin the
// lowering shape; a kotlinc compile against the runtime jar is the syntactic
// gate (skipped when kotlinc or the jar is absent — the same toolchain-missing
// behaviour as `smoke_kotlin`). The real compile+run is the committed
// integration gate `sce-kotlin-tests` `EventSchemaNativeTest` (regen:
// scripts/regen_event_schema_native_kotlin.sh).
#[test]
fn statechart_native_lowering_kotlin_emits_engine_free_typed_guard() {
    let scratch = reset_scratch("kotlin_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("kotlin", &out_dir, "statechart_minimal");

    let kt_files = find_by_ext(&out_dir, "kt");
    assert_eq!(kt_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&kt_files[0]).expect("read kt");

    assert!(
        src.contains("data class StatechartMinimalJobCompletedPayload(val elapsed_ms: UInt)"),
        "expected the per-event typed payload data class; got:\n{src}"
    );
    // Nullable carrier field replaces Go's tag enum — `null` is the "no payload"
    // sentinel, and the guard's `… != null` prefix is the tag check.
    assert!(
        src.contains(
            "private var pendingJobCompletedPayload: StatechartMinimalJobCompletedPayload? = null"
        ),
        "expected the nullable typed-payload policy field; got:\n{src}"
    );
    assert!(
        src.contains(
            "override fun populateTypedPayload(event: StatechartMinimalEvent, metadata: EventMetadata)"
        ) && src.contains(
            "is StatechartMinimalJobCompletedPayload -> pendingJobCompletedPayload = tp"
        ),
        "expected the populateTypedPayload carrier bind; got:\n{src}"
    );
    // ⚠ The hook takes the EVENT as well as the metadata, because binding the
    // typed view has a second source: an event that arrived with only the
    // `data` wire — every producer except the inject seam — has its fields
    // read out of that, and only the event's name says which schema to read.
    assert!(
        src.contains("EventPayload.decode(metadata.data)")
            && src.contains("fields.uint32(\"elapsed_ms\")"),
        "expected the lift that reads the typed view out of `data`; got:\n{src}"
    );
    assert!(
        src.contains(
            "pendingJobCompletedPayload != null && (pendingJobCompletedPayload!!.elapsed_ms == 0.toUInt())"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: a member of the generated machine (which IS the
    // engine subclass — no orphan-rule barrier as in Rust/Go) binding the event
    // name + payload field values in one call.
    assert!(
        src.contains("fun raiseJobCompleted(elapsed_ms: UInt)")
            && src.contains("typedPayload = StatechartMinimalJobCompletedPayload(elapsed_ms),")
            // ⚠ Both carriers: the typed payload a native guard reads, and the
            // `data` wire the script engine binds `_event.data` from. A
            // document with an `<assign expr="_event.data.x">` on this same
            // event read nothing from it while only the first was filled.
            && src.contains("data = EventPayload.encode(mapOf(\"elapsed_ms\" to elapsed_ms))"),
        "expected the per-event typed inject seam; got:\n{src}"
    );
    assert!(
        !src.contains("safeEvaluateGuard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The raw ECMAScript cond would only reach emitted code via the
    // `{% else %}` arm (`… && _event.data.elapsed_ms === 0 ->`); a bare
    // `elapsed_ms ===` sentinel would false-positive on the traceability
    // comment, so pin the leaked-guard shape precisely.
    assert!(
        !src.contains("&& _event.data.elapsed_ms === 0 ->"),
        "the raw ECMAScript guard must be fully lowered away (not leaked into the else arm)"
    );

    // Syntactic gate: compile the native-lowered SM against the runtime jar.
    let Some(kotlinc) = toolchain::locate("kotlinc") else {
        toolchain::skipped("statechart_native_lowering_kotlin compile: kotlinc not on PATH");
        return;
    };
    let Some(jar) = find_kotlin_runtime_jar() else {
        toolchain::skipped(
            "statechart_native_lowering_kotlin compile: runtime jar missing under \
             backends/kotlin/runtime/build/libs (run `./gradlew :sce-kotlin-runtime:assemble`)",
        );
        return;
    };
    let classes = scratch.join("classes");
    std::fs::create_dir_all(&classes).expect("create classes dir");
    let output = Command::new(&kotlinc)
        .args(["-cp"])
        .arg(&jar)
        .args(["-d"])
        .arg(&classes)
        .arg("-nowarn")
        .arg(&kt_files[0])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run kotlinc");
    assert!(
        output.status.success(),
        "kotlinc failed for the native-lowered Kotlin SM\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// NL→IR Item C1 Path A (EventSchema native lowering) — Python parity for
// `statechart_native_lowering_emits_engine_free_typed_guard`. The Python
// backend lowers the typed `_event.data.elapsed_ms` guard to a script-engine-
// free native comparison over the type-erased `EventMetadata.typed_payload`
// carrier: a per-event payload dataclass, a `_pending_<event>_payload` instance
// field, a `set_current_event` lift that `isinstance`-checks the carrier, a
// per-event module-level `raise_<event>` inject seam, and a
// `self._pending_<event>_payload is not None and (…)` guard — never the
// `self._guard(engine, …)` script path. `py_compile` is the syntactic gate; the
// substring assertions pin the lowering shape. The real run is the committed
// integration gate `backends/python/tests/integration/event_schema_native`
// (regen: scripts/regen_event_schema_native_python.sh).
#[test]
fn statechart_native_lowering_python_emits_engine_free_typed_guard() {
    let scratch = reset_scratch("python_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("python", &out_dir, "statechart_minimal");

    let py_files = find_by_ext(&out_dir, "py");
    assert_eq!(py_files.len(), 1, "expected exactly one generated SM");
    let src = std::fs::read_to_string(&py_files[0]).expect("read py");

    assert!(
        src.contains("class StatechartMinimalJobCompletedPayload:")
            && src.contains("    elapsed_ms: int"),
        "expected the per-event typed payload dataclass; got:\n{src}"
    );
    assert!(
        src.contains("self._pending_job_completed_payload = None"),
        "expected the instance-field reset (init + set_current_event); got:\n{src}"
    );
    assert!(
        src.contains("if isinstance(_tp, StatechartMinimalJobCompletedPayload):"),
        "expected the populate isinstance lift in set_current_event; got:\n{src}"
    );
    assert!(
        src.contains(
            "self._pending_job_completed_payload is not None and \
             (self._pending_job_completed_payload.elapsed_ms == 0)"
        ),
        "expected the native typed-payload guard; got:\n{src}"
    );
    // Per-event typed inject: a module-level free function (the Python twin of
    // the Go `Raise<Event>` func — the runtime `Engine` is foreign) binding the
    // event name + payload field values in one call.
    assert!(
        src.contains("def raise_job_completed(engine, elapsed_ms: int) -> None:")
            && src.contains("typed_payload=StatechartMinimalJobCompletedPayload(elapsed_ms)"),
        "expected the per-event typed inject seam; got:\n{src}"
    );
    assert!(
        !src.contains("self._guard(engine,"),
        "native lowering must not fall back to the script-engine guard"
    );

    // Syntactic gate: py_compile the native-lowered SM.
    let Some(python) = toolchain::locate_any(&["python3", "python"]) else {
        toolchain::skipped("statechart_native_lowering_python compile: python3/python not on PATH");
        return;
    };
    let output = Command::new(&python)
        .args(["-m", "py_compile"])
        .arg(&py_files[0])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run python -m py_compile");
    assert!(
        output.status.success(),
        "py_compile failed for the native-lowered Python SM\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

// NL→IR Item C1 Path A (EventSchema native lowering) — C++ parity +
// real compile+run gate. The C++ backend lowers the typed
// `_event.data.elapsed_ms` guard to a script-engine-free native comparison
// over an `std::any`-carried typed payload: a `<Machine>PayloadTag` enum, a
// `pendingPayloadTag_` + `pending<Event>Payload_` policy channel, a
// `populateTypedPayload` any_cast hook, a per-event `raise<Event>` inject seam,
// and a `pendingPayloadTag_ == … && (…)` guard. Beyond the shape assertions
// this generates the SM and really compiles + runs it (header-only, NO runtime
// lib — the no-script-engine value path links zero, the C++ analogue of the
// C11 freestanding gate) to prove the typed inject → native guard → transition
// actually fires.
#[test]
fn statechart_native_lowering_cpp_compiles_and_runs() {
    let scratch = reset_scratch("cpp_native");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("cpp", &out_dir, "statechart_minimal");

    let h_files = find_by_ext(&out_dir, "h");
    assert_eq!(h_files.len(), 1, "expected exactly one generated header");
    let header = std::fs::read_to_string(&h_files[0]).expect("read h");
    let inl = std::fs::read_to_string(&find_by_ext(&out_dir, "inl")[0]).expect("read inl");

    assert!(
        header.contains("enum class StatechartMinimalPayloadTag"),
        "expected the per-document typed payload tag enum; got:\n{header}"
    );
    assert!(
        header.contains("template <typename M> ::std::string populateTypedPayload(const M &meta)")
            && header.contains(
                "::std::any_cast<StatechartMinimalJobCompletedPayload>(&meta.typedPayload)"
            ),
        "expected the any_cast populate hook; got:\n{header}"
    );
    // ⚠ The hook takes the WHOLE dequeued event and answers with a sentence,
    // because binding the typed view has a second source: an event that
    // arrived with only the `data` wire — every producer except the inject
    // seam — has its fields read out of that, and a payload that does not read
    // as its schema is reported for the engine to raise as error.execution.
    assert!(
        header.contains("::SCE::Common::EventPayloadFields::decode(meta.data, fields)")
            && header.contains("fields.readInteger(\"elapsed_ms\", payload.elapsed_ms)"),
        "expected the lift that reads the typed view out of `data`; got:\n{header}"
    );
    assert!(
        header.contains("void raiseJobCompleted(uint32_t elapsed_ms)")
            && header.contains("m.typedPayload = StatechartMinimalJobCompletedPayload{")
            // Both carriers, and the wire built by the same header the lift
            // reads it with: a generated C++ machine LINKS against
            // `sce/include` alone, so a call into the runtime library here
            // would take that property away (measured 2026-09-22).
            //
            // One anchor for the whole statement. It was two fragments while
            // the text read here was clang-format's, which wraps the call at
            // its own column; the generator writes it on one line.
            && header.contains(
                "m.data = ::SCE::Common::EventPayloadFields::wire(\
                 {::SCE::Common::EventPayloadFields::field(\"elapsed_ms\", elapsed_ms)});"
            ),
        "expected the per-event typed inject seam; got:\n{header}"
    );
    assert!(
        inl.contains(
            "pendingPayloadTag_ == StatechartMinimalPayloadTag::JobCompleted && \
             (pendingJobCompletedPayload_.elapsed_ms == 0)"
        ),
        "expected the native typed-payload guard; got:\n{inl}"
    );
    assert!(
        !inl.contains("safeEvaluateGuard"),
        "native lowering must not fall back to the script-engine guard"
    );
    // The `// "…" lowered to …` comment echoes the raw cond, so use a precise
    // sentinel: the raw cond would only reach emitted code via the `{% else %}`
    // arm, i.e. `if (_event.data.elapsed_ms === 0) SCE_LIKELY {`.
    assert!(
        !inl.contains("if (_event.data.elapsed_ms === 0)"),
        "the raw ECMAScript guard must be fully lowered away"
    );

    // Real compile + run. The generated SM is header-only template code; a
    // typed-guard machine has needs_script_engine == false, so it links no
    // runtime (the MCU-relevant property, proven here for C++ too).
    let Some(gpp) = toolchain::locate("g++") else {
        toolchain::skipped("statechart_native_lowering_cpp compile: g++ not on PATH");
        return;
    };
    let driver = out_dir.join("driver.cpp");
    std::fs::write(
        &driver,
        r#"#include "statechart_minimal_sm.h"
#include <cstdio>
using SM = ::SCE::Generated::statechart_minimal::statechart_minimal;
int main() {
    SM sm; sm.initialize();
    if (sm.getCurrentState() != SM::State::Waiting) { std::puts("FAIL initial"); return 1; }
    sm.raiseJobCompleted(0);   // elapsed_ms == 0 satisfies the native typed guard
    sm.step();
    if (sm.getCurrentState() != SM::State::Done) { std::puts("FAIL fire"); return 2; }
    SM sm2; sm2.initialize();
    sm2.raiseJobCompleted(5);  // guard rejects -> stays put
    sm2.step();
    if (sm2.getCurrentState() != SM::State::Waiting) { std::puts("FAIL miss"); return 3; }
    std::puts("OK"); return 0;
}
"#,
    )
    .expect("write driver");
    let bin = out_dir.join("driver");
    let inc = repo_root().join("sce/include");
    let compile = Command::new(&gpp)
        .arg("-std=c++17")
        .arg("-I")
        .arg(&inc)
        .arg("-I")
        .arg(&out_dir)
        .arg(&driver)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run g++");
    assert!(
        compile.status.success(),
        "header-only compile of the native-lowered C++ SM failed\nstderr: {}",
        String::from_utf8_lossy(&compile.stderr),
    );
    let run = Command::new(&bin).output().expect("run driver");
    assert!(
        run.status.success(),
        "native-lowered C++ SM run failed (exit {:?})\nstdout: {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
    );
}

// NL→IR Item C1 Path A (EventSchema MCU native lowering) — C11 parity
// for `statechart_native_lowering_emits_engine_free_typed_guard`.
// The C11 backend must lower the typed `_event.data.elapsed_ms` guard to a
// native tagged-union comparison: a `<name>_payload_t` union, a
// `pending_payload` copy in the pop loop, a `raise_external_typed` inject
// seam, and a `tag ==` + field-comparison guard — never the Lua
// `lua_eval_guard` path (there is no script engine on an MCU). Beyond the
// shape assertions the generated translation unit must COMPILE under the
// strict MCU profile (`-std=c11 -Wall -Wextra -Wpedantic -Werror`) and,
// critically, under `-ffreestanding` — the value path's whole point is that
// it carries no hosted-libc / Lua dependency, so it builds for a bare-metal
// target. That freestanding compile is the A5 MCU acceptance gate.
#[test]
fn statechart_native_lowering_c11_compiles_freestanding() {
    let Some(cc) = toolchain::locate_any(&["clang", "gcc"]) else {
        toolchain::skipped("statechart_native_lowering_c11: clang/gcc not on PATH");
        return;
    };
    let scratch = reset_scratch("c11_statechart");
    let out_dir = scratch.join("statechart_minimal");
    run_generate("c11", &out_dir, "statechart_minimal");

    let c_files = find_by_ext(&out_dir, "c");
    assert_eq!(c_files.len(), 1, "expected exactly one generated C11 TU");
    let src = std::fs::read_to_string(&c_files[0]).expect("read .c");

    assert!(
        src.contains(
            "sm->pending_payload.tag == STATECHART_MINIMAL_PAYLOAD_JOB_COMPLETED \
             && (sm->pending_payload.as.job_completed.elapsed_ms == 0)"
        ),
        "expected the native tagged-union typed-payload guard; got:\n{src}"
    );
    assert!(
        src.contains(
            "statechart_minimal_raise_job_completed_typed(statechart_minimal_t *sm, \
             const statechart_minimal_job_completed_payload_t *payload)"
        ),
        "expected the per-event typed-payload inject seam (binds event+tag+member)"
    );
    assert!(
        !src.contains("lua_eval_guard"),
        "native lowering must not fall back to the Lua script-engine guard"
    );
    // `elapsed_ms ===` would only survive if the raw ECMAScript cond leaked
    // (`===` is not valid C). A precise sentinel — the `cond="…"` echo
    // comment uses `escape_c` and reads `elapsed_ms === 0` inside a quoted
    // string, so match the bare operator form the guard would emit.
    assert!(
        !src.contains("if (sm->pending_payload.as.job_completed.elapsed_ms === 0)")
            && !src.contains("lua_eval"),
        "the raw ECMAScript guard must be fully lowered away"
    );

    let runtime_inc = repo_root().join("backends/c/runtime/include");
    // Strict MCU profile + freestanding: the no-script-engine value path
    // must build with no hosted-libc assumption (A5 MCU gate).
    let mut cmd = Command::new(&cc);
    cmd.args([
        "-std=c11",
        "-ffreestanding",
        "-c",
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        "-Werror",
    ]);
    cmd.arg("-I").arg(&runtime_inc);
    cmd.arg("-I").arg(&out_dir);
    cmd.arg("-o").arg(out_dir.join("sm.o"));
    cmd.arg(&c_files[0]);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c11 compiler");
    assert!(
        output.status.success(),
        "freestanding -std=c11 compile of the native-lowered TU failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    // A5 north-star gate: a REAL bare-metal ARM cross-compile (Cortex-M4,
    // Thumb). The freestanding host compile above proves "no hosted-libc
    // calls"; this proves the value path actually builds for the MCU ISA
    // the downstream consumer targets. Skipped (not failed) when the
    // cross toolchain is absent, mirroring the host-compiler skips above —
    // CI installs `gcc-arm-none-eabi`.
    let Some(arm_cc) = toolchain::locate("arm-none-eabi-gcc") else {
        toolchain::skipped("arm-none-eabi cross-compile: arm-none-eabi-gcc not on PATH");
        return;
    };
    let mut arm = Command::new(&arm_cc);
    arm.args([
        "-mcpu=cortex-m4",
        "-mthumb",
        "-std=c11",
        "-ffreestanding",
        "-c",
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        "-Werror",
    ]);
    arm.arg("-I").arg(&runtime_inc);
    arm.arg("-I").arg(&out_dir);
    arm.arg("-o").arg(out_dir.join("sm.arm.o"));
    arm.arg(&c_files[0]);
    let arm_out = arm
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run arm-none-eabi-gcc");
    assert!(
        arm_out.status.success(),
        "arm-none-eabi (cortex-m4) cross-compile of the native-lowered TU failed\nstderr: {}",
        String::from_utf8_lossy(&arm_out.stderr),
    );
}

#[test]
fn smoke_go() {
    let Some(gofmt) = toolchain::locate("gofmt") else {
        toolchain::skipped("smoke_go: gofmt not on PATH");
        return;
    };
    let scratch = reset_scratch("go");

    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("go", &out_dir, fixture);
        let go_files = find_by_ext(&out_dir, "go");
        assert!(!go_files.is_empty(), "no .go emitted for {fixture}");
        for go in go_files {
            let output = Command::new(&gofmt)
                .arg("-e")
                .arg(&go)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .expect("run gofmt");
            assert!(
                output.status.success(),
                "gofmt -e failed for {fixture} at {}\nstderr: {}",
                go.display(),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}

fn find_kotlin_runtime_jar() -> Option<PathBuf> {
    let libs = repo_root().join("backends/kotlin/runtime/build/libs");
    let entries = std::fs::read_dir(&libs).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("sce-kotlin-runtime-jvm-")
                && name.ends_with(".jar")
                && !name.ends_with("-sources.jar")
        })
        .collect();
    candidates.sort();
    candidates.pop()
}

#[test]
fn smoke_kotlin() {
    let Some(kotlinc) = toolchain::locate("kotlinc") else {
        toolchain::skipped("smoke_kotlin: kotlinc not on PATH");
        return;
    };
    let Some(jar) = find_kotlin_runtime_jar() else {
        toolchain::skipped(
            "smoke_kotlin: runtime jar missing under backends/kotlin/runtime/build/libs \
             (run `./gradlew :sce-kotlin-runtime:assemble`)",
        );
        return;
    };
    let scratch = reset_scratch("kotlin");

    let mut kt_paths: Vec<PathBuf> = Vec::new();
    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("kotlin", &out_dir, fixture);
        kt_paths.extend(find_by_ext(&out_dir, "kt"));
    }
    assert!(!kt_paths.is_empty(), "no .kt files emitted");

    let classes = scratch.join("classes");
    std::fs::create_dir_all(&classes).expect("create classes dir");

    let mut cmd = Command::new(&kotlinc);
    cmd.args(["-cp"]).arg(&jar);
    cmd.args(["-d"]).arg(&classes);
    cmd.arg("-nowarn");
    for p in &kt_paths {
        cmd.arg(p);
    }
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run kotlinc");
    assert!(
        output.status.success(),
        "kotlinc failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn smoke_python() {
    let Some(python) = toolchain::locate_any(&["python3", "python"]) else {
        toolchain::skipped("smoke_python: python3/python not on PATH");
        return;
    };
    let scratch = reset_scratch("python");

    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("python", &out_dir, fixture);
        let py_files = find_by_ext(&out_dir, "py");
        assert!(!py_files.is_empty(), "no .py emitted for {fixture}");
        for py in py_files {
            let output = Command::new(&python)
                .args(["-m", "py_compile"])
                .arg(&py)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run python -m py_compile");
            assert!(
                output.status.success(),
                "py_compile failed for {fixture} at {}\nstdout: {}\nstderr: {}",
                py.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}

#[test]
fn smoke_c11() {
    let Some(cc) = toolchain::locate_any(&["clang", "gcc"]) else {
        toolchain::skipped("smoke_c11: clang/gcc not on PATH");
        return;
    };
    let scratch = reset_scratch("c11");

    let mut headers: Vec<String> = Vec::new();
    let mut include_dirs: Vec<PathBuf> = Vec::new();
    for fixture in FIXTURES {
        let out_dir = scratch.join(fixture);
        run_generate("c11", &out_dir, fixture);
        include_dirs.push(out_dir.clone());
        for hdr in find_by_ext(&out_dir, "h") {
            if let Some(name) = hdr.file_name().and_then(|s| s.to_str()) {
                headers.push(name.to_string());
            }
        }
    }
    assert!(!headers.is_empty(), "no .h artefacts emitted");

    let stub = scratch.join("stub.c");
    let mut src = String::new();
    for h in &headers {
        src.push_str(&format!("#include \"{h}\"\n"));
    }
    std::fs::write(&stub, &src).expect("write stub");

    let mut cmd = Command::new(&cc);
    cmd.args(["-std=c11", "-fsyntax-only", "-Wall", "-Wextra", "-Werror"]);
    for dir in &include_dirs {
        cmd.arg("-I").arg(dir);
    }
    cmd.arg(&stub);
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run c11 compiler");

    assert!(
        output.status.success(),
        "c11 compiler -fsyntax-only failed\nheaders: {headers:?}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}
