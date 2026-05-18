//! C3 Atomic B-β + B-γ2c integration tests — Rust no_std variant rejection.
//!
//! Watching-zenoh RFC §5.J.2 author-side gate: when
//! `sce-codegen generate -l rust --no-std <doc>` is invoked, the
//! SCXML document must not exercise constructs that require a
//! std-coupled runtime. Four axes are checked in order:
//!
//! - `<script>` — Lua/QuickJS need `alloc`, no_std forbids it
//!   (spec line 1989 zero-alloc mandate). Fires
//!   `codegen/no-std-script-not-supported`.
//! - `<send type="BasicHTTPEventProcessor">` with http(s) target —
//!   tokio/reqwest are std-coupled. Fires
//!   `codegen/no-std-http-not-supported`.
//! - `<data src="...">` — filesystem load needs `PathBuf` plus
//!   `std::fs::read_to_string`, both alloc/OS-coupled per spec line
//!   1989-1994. Fires `codegen/no-std-fs-load-not-supported`.
//! - `<invoke>` — invoke machinery uses `Arc<Mutex<Vec<…>>>` plus
//!   `HashMap`, all alloc-coupled per the same RFC anchor. Fires
//!   `codegen/no-std-invoke-not-supported`.
//!
//! The validator (`sce_build::validate_no_std_compatibility`) is the
//! library entry point the CLI binary's `cmd_generate` calls when
//! `lang == Rust && no_std == true`. Tests drive the validator
//! directly with parsed SCXML models so no subprocess is needed.

use std::path::Path;

use sce_build::forge::error::{ForgeError, GenerateError};
use sce_build::parser::SCXMLParser;
use sce_build::validate_no_std_compatibility;

fn parse(content: &str, label: &str) -> sce_build::model::SCXMLModel {
    let mut parser = SCXMLParser::new();
    parser
        .parse_string(content, label)
        .unwrap_or_else(|e| panic!("parse failed for {label}: {:?}", e.error))
}

const PLAIN_FSM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <state id="s1">
    <transition event="back" target="s0"/>
  </state>
</scxml>
"#;

const FSM_WITH_INLINE_SCRIPT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0"
       datamodel="ecmascript">
  <datamodel><data id="x" expr="0"/></datamodel>
  <state id="s0">
    <onentry>
      <script>x = x + 1;</script>
    </onentry>
  </state>
</scxml>
"#;

const FSM_WITH_HTTP_SEND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0">
    <onentry>
      <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            target="http://localhost:8000/event"
            event="ping"/>
    </onentry>
    <transition event="done" target="s1"/>
  </state>
  <state id="s1"/>
</scxml>
"#;

#[test]
fn plain_fsm_passes_no_std_validation() {
    let model = parse(PLAIN_FSM, "plain");
    let result = validate_no_std_compatibility(&model, Path::new("plain.scxml"));
    assert!(
        result.is_ok(),
        "plain SCXML without scripts or HTTP must pass no_std gate, got: {result:?}"
    );
}

#[test]
fn fsm_with_inline_script_fires_no_std_script_diagnostic() {
    let model = parse(FSM_WITH_INLINE_SCRIPT, "script_fsm");
    let err = validate_no_std_compatibility(&model, Path::new("script_fsm.scxml"))
        .expect_err("script-bearing SCXML must reject under --no-std");

    match err {
        ForgeError::Generate(GenerateError::CodegenNoStdScriptNotSupported {
            document,
            locations,
        }) => {
            assert_eq!(document, "script_fsm");
            assert!(
                !locations.is_empty(),
                "locations summary must be non-empty for downstream agent dispatch"
            );
        }
        other => panic!("expected CodegenNoStdScriptNotSupported, got: {other:?}"),
    }
}

#[test]
fn fsm_with_http_send_fires_no_std_http_diagnostic() {
    let model = parse(FSM_WITH_HTTP_SEND, "http_fsm");
    let err = validate_no_std_compatibility(&model, Path::new("http_fsm.scxml"))
        .expect_err("HTTP-send SCXML must reject under --no-std");

    match err {
        ForgeError::Generate(GenerateError::CodegenNoStdHttpNotSupported {
            document,
            locations,
        }) => {
            assert_eq!(document, "http_fsm");
            assert!(locations.contains("BasicHTTPEventProcessor"));
        }
        other => panic!("expected CodegenNoStdHttpNotSupported, got: {other:?}"),
    }
}

#[test]
fn script_axis_fires_before_http_when_both_present() {
    // Spec line 1989 ⇒ both axes are mutually exclusive with no_std;
    // when an SCXML document tickles both, the validator must surface
    // one diagnostic per pass so the wire contract stays
    // single-record-per-rejection. Script-first matches the
    // C2-outbox precedent (suffix-before-owner) — the syntactic axis
    // (script) checks before the semantic axis (http send wire).
    let both = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0"
       datamodel="ecmascript">
  <datamodel><data id="x" expr="0"/></datamodel>
  <state id="s0">
    <onentry>
      <script>x = 1;</script>
      <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            target="http://localhost/x" event="ping"/>
    </onentry>
  </state>
</scxml>
"#;
    let model = parse(both, "both");
    let err = validate_no_std_compatibility(&model, Path::new("both.scxml"))
        .expect_err("doc with both axes must reject");
    assert!(
        matches!(
            err,
            ForgeError::Generate(GenerateError::CodegenNoStdScriptNotSupported { .. })
        ),
        "script axis must fire before http axis when both apply, got: {err:?}"
    );
}

// ───────────────────────────────────────────────────────────────────
// C3 Atomic B-γ2c: filesystem load axis
// ───────────────────────────────────────────────────────────────────

const FSM_WITH_DATA_SRC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <datamodel>
    <data id="cfg" src="file:cfg.json"/>
  </datamodel>
  <state id="s0"/>
</scxml>
"#;

const FSM_WITH_MULTIPLE_DATA_SRC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <datamodel>
    <data id="cfg" src="file:cfg.json"/>
    <data id="schema" src="file:schema.xml"/>
  </datamodel>
  <state id="s0">
    <datamodel>
      <data id="state_local" src="state.txt"/>
    </datamodel>
  </state>
</scxml>
"#;

#[test]
fn fsm_with_data_src_fires_no_std_fs_load_diagnostic() {
    let model = parse(FSM_WITH_DATA_SRC, "fs_fsm");
    let err = validate_no_std_compatibility(&model, Path::new("fs_fsm.scxml"))
        .expect_err("doc with <data src> must reject under --no-std");

    match err {
        ForgeError::Generate(GenerateError::CodegenNoStdFsLoadNotSupported {
            document,
            locations,
        }) => {
            assert_eq!(document, "fs_fsm");
            assert!(
                locations.contains("cfg.json"),
                "locations summary must name the offending src URL, got {locations:?}"
            );
        }
        other => panic!("expected CodegenNoStdFsLoadNotSupported, got: {other:?}"),
    }
}

#[test]
fn fsm_with_multiple_data_src_reports_all_sites() {
    // The validator's wire contract is single-record-per-pass, but the
    // human-readable `locations` summary must enumerate every offending
    // site so the author repairs all of them before re-running codegen
    // (otherwise the next pass fires the same diagnostic on the next
    // site, multiplying agent round-trips). Both top-level and
    // state-nested `<data src>` must surface in one summary.
    let model = parse(FSM_WITH_MULTIPLE_DATA_SRC, "multi_fs");
    let err = validate_no_std_compatibility(&model, Path::new("multi_fs.scxml"))
        .expect_err("multi-src doc must reject under --no-std");
    let locations = match err {
        ForgeError::Generate(GenerateError::CodegenNoStdFsLoadNotSupported {
            locations, ..
        }) => locations,
        other => panic!("expected CodegenNoStdFsLoadNotSupported, got: {other:?}"),
    };
    assert!(locations.contains("cfg.json"), "missing cfg.json site");
    assert!(locations.contains("schema.xml"), "missing schema.xml site");
    assert!(locations.contains("state.txt"), "missing state-nested site");
    assert!(
        locations.contains("'s0'"),
        "state-nested src must name its enclosing state, got {locations:?}"
    );
}

// ───────────────────────────────────────────────────────────────────
// C3 Atomic B-γ2c: invoke axis
// ───────────────────────────────────────────────────────────────────

const FSM_WITH_INVOKE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="parent">
  <state id="parent">
    <invoke id="child" type="http://www.w3.org/TR/scxml/" src="child.scxml"/>
    <transition event="done.invoke.child" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

#[test]
fn fsm_with_invoke_fires_no_std_invoke_diagnostic() {
    let model = parse(FSM_WITH_INVOKE, "invoke_fsm");
    let err = validate_no_std_compatibility(&model, Path::new("invoke_fsm.scxml"))
        .expect_err("doc with <invoke> must reject under --no-std");

    match err {
        ForgeError::Generate(GenerateError::CodegenNoStdInvokeNotSupported {
            document,
            locations,
        }) => {
            assert_eq!(document, "invoke_fsm");
            assert!(
                locations.contains("invoke"),
                "locations summary must mention `<invoke>`, got {locations:?}"
            );
        }
        other => panic!("expected CodegenNoStdInvokeNotSupported, got: {other:?}"),
    }
}

#[test]
fn axis_ordering_is_fs_then_invoke_then_script_then_http() {
    // Spec lines 1989-1994 ⇒ all four axes are mutually exclusive with
    // no_std. The validator's `Err` return is single-record-per-pass:
    // when a document tickles multiple axes the order must be stable
    // (fs-load → invoke → script → http, most-specific first) so the
    // author repair path names the offending construct directly
    // instead of bottoming out on the broad script catch-all.
    //
    // This test pairs `<data src>` (which the script-engine analyzer
    // *also* flags as a script cause) with `<invoke>` to confirm
    // fs-load wins the head of the chain. Without the fs-first
    // reorder, the broader script axis would mask both specific
    // diagnostics and the author would see only "ECMAScript executable
    // content (analyzer-detected)".
    let fs_and_invoke = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="parent">
  <datamodel>
    <data id="cfg" src="file:cfg.json"/>
  </datamodel>
  <state id="parent">
    <invoke id="child" type="http://www.w3.org/TR/scxml/" src="child.scxml"/>
    <transition event="done.invoke.child" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let model = parse(fs_and_invoke, "fs_and_invoke");
    let err = validate_no_std_compatibility(&model, Path::new("fs_and_invoke.scxml"))
        .expect_err("multi-axis doc must reject");
    assert!(
        matches!(
            err,
            ForgeError::Generate(GenerateError::CodegenNoStdFsLoadNotSupported { .. })
        ),
        "fs-load axis must fire before invoke axis when both apply, got: {err:?}"
    );
}

#[test]
fn document_basename_is_extracted_from_path() {
    let model = parse(FSM_WITH_INLINE_SCRIPT, "script_fsm");
    // Path basename without extension should populate `document` field
    // so downstream agents dispatching on `key_fragments` can match
    // the SCXML file independent of the calling CWD.
    let err = validate_no_std_compatibility(&model, Path::new("/abs/some/path/widget_fsm.scxml"))
        .expect_err("script-bearing SCXML must reject");
    if let ForgeError::Generate(GenerateError::CodegenNoStdScriptNotSupported {
        document, ..
    }) = err
    {
        assert_eq!(document, "widget_fsm");
    } else {
        panic!("unexpected error variant");
    }
}
