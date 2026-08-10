// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A forge document that still carries `<sce:use>` / `<xi:include>` has
// not been through `expand_preprocessors`, and must not compile.
//
// The defect this guards against already happened.
// `parse_sce_entries` matches `<sce:entry>` by tag name and has no
// else-branch, so any other element under `<data>` is skipped without a
// word. An unexpanded `<sce:use>` is therefore not a parse error — it is
// a row that never arrives. With `sce:default` present the missing row
// answers with the default, so the caller sees a well-formed lookup that
// is simply wrong, and nothing on the path — compiler, generated code,
// runtime — says anything.
//
// Two things kept this alive longer than it should have been:
//
//   * The XSD does not object. `<sce:use>` is a declared element
//     (`schemas/sce-forge-ext.xsd`) and the containers admitting it are
//     `xs:any processContents="lax"`, so schema validation calls the
//     unexpanded document valid. `xsd_validator.rs` even pins that with
//     a `VALID_USE` fixture. The schema is not the layer that can catch
//     this, which is why the check lives in the parser.
//
//   * No fixture crossed the two features. Every `.scxml` in the tree
//     using `<sce:use>` or `<xi:include>` is a statechart under
//     `tests/w3c_template_parity/`; every forge fixture is
//     preprocessor-free. The interaction had no coverage at all, so
//     both directions are asserted below rather than only the failing
//     one.
//
// The statechart route cannot reach this state through `parse_file`,
// which calls the expander itself — the asymmetry is the whole reason
// the forge route needed its own guard.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::SingleDiagnostic;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// The row the template carries, as it reaches generated Rust. The
/// fixtures below author it as `TEMPLATE_EXPANDED`; the lookup backend
/// folds that into an enum variant, so the authored spelling never
/// appears in the output — assert on what is actually emitted.
const TEMPLATED_VARIANT: &str = "TemplateExpanded";

const TEMPLATE_FILE: &str = "probe_row.sce-template.xml";

const TEMPLATE_BODY: &str = r#"<sce:template xmlns:sce="http://sce.dev/ext"
              xmlns="http://www.w3.org/2005/07/scxml"
              name="probe_row">
  <sce:param name="k" required="true"/>
  <sce:param name="v" required="true"/>
  <sce:entry key="{$k}" value="{$v}"/>
</sce:template>
"#;

/// A `lookup` whose second row arrives only through template
/// expansion. `sce:default` is present on purpose: it is what turns a
/// dropped row into a plausible answer instead of a visible hole.
const DOC_WITH_USE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="probe">
  <datamodel>
    <data id="k" sce:type="uint8" sce:direction="in"/>
    <data id="probed" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NO">
      <sce:entry key="0" value="NO"/>
      <sce:use template="probe_row.sce-template.xml" k="1" v="TEMPLATE_EXPANDED"/>
    </data>
  </datamodel>
</scxml>
"#;

/// Same document, with the row delivered by `<xi:include>` instead.
/// The two directives share `expand_preprocessors` and share the
/// tag-name filter that drops them, so they are asserted together
/// rather than one standing in for the other.
const DOC_WITH_XINCLUDE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       xmlns:xi="http://www.w3.org/2001/XInclude"
       sce:kind="lookup" name="probe">
  <datamodel>
    <data id="k" sce:type="uint8" sce:direction="in"/>
    <data id="probed" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NO">
      <sce:entry key="0" value="NO"/>
      <xi:include href="probe_row.fragment.xml"/>
    </data>
  </datamodel>
</scxml>
"#;

const FRAGMENT_BODY: &str = r#"<sce:entry xmlns:sce="http://sce.dev/ext" key="1" value="TEMPLATE_EXPANDED"/>
"#;

/// Write the fixture tree and return (dir, main document path).
fn fixture(doc: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join(TEMPLATE_FILE), TEMPLATE_BODY).expect("write template");
    std::fs::write(dir.path().join("probe_row.fragment.xml"), FRAGMENT_BODY).expect("write frag");
    let main = dir.path().join("probe.scxml");
    std::fs::write(&main, doc).expect("write doc");
    (dir, main)
}

fn expand(path: &Path) -> String {
    let content = std::fs::read_to_string(path).expect("read doc");
    let (expanded, _map, _deps) = sce_build::parser::expand_preprocessors(
        &content,
        path.to_str().expect("utf8 path"),
        path.parent(),
        &[],
    )
    .expect("expansion must succeed");
    expanded
}

fn generated_source(output: &sce_build::generator::GeneratedOutput) -> String {
    output
        .files
        .iter()
        .map(|(_name, body)| body.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Positive control: the feature works when the expander runs first.
///
/// Without this, a guard that rejected every `<sce:use>` unconditionally
/// would look correct — the failing assertion below would pass for the
/// wrong reason.
#[test]
fn expanded_forge_lookup_carries_the_templated_row() {
    let (dir, main) = fixture(DOC_WITH_USE);
    let expanded = expand(&main);

    let output = sce_build::compile_forge_with_imports(
        &expanded,
        DocumentLabel::symmetric("probe"),
        Language::Rust,
        dir.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("expanded document must compile");

    let src = generated_source(&output);
    assert!(
        src.contains(TEMPLATED_VARIANT),
        "expanded lookup must carry the templated row; generated source:\n{src}"
    );
}

/// The defect, stated as the assertion it fails.
///
/// Before the fix this compiled cleanly and emitted a one-row table —
/// exit 0, no diagnostic, the templated row simply absent.
#[test]
fn unexpanded_sce_use_is_rejected_rather_than_dropped() {
    let (dir, _main) = fixture(DOC_WITH_USE);

    let result = sce_build::compile_forge_with_imports(
        DOC_WITH_USE,
        DocumentLabel::symmetric("probe"),
        Language::Rust,
        dir.path(),
        &ForgeCompileOptions::default(),
    );

    let err = match result {
        Ok(output) => {
            let src = generated_source(&output);
            panic!(
                "an unexpanded <sce:use> compiled instead of being rejected. \
                 Row present in output: {}. Generated source:\n{src}",
                src.contains(TEMPLATED_VARIANT)
            );
        }
        Err(e) => e,
    };

    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "xml/preprocessor-not-run",
        "rejection must name the missing expansion pass, not a downstream symptom; got: {err}"
    );
}

/// Same guard, `<xi:include>` flavour.
#[test]
fn unexpanded_xinclude_is_rejected_rather_than_dropped() {
    let (dir, _main) = fixture(DOC_WITH_XINCLUDE);

    let result = sce_build::compile_forge_with_imports(
        DOC_WITH_XINCLUDE,
        DocumentLabel::symmetric("probe"),
        Language::Rust,
        dir.path(),
        &ForgeCompileOptions::default(),
    );

    let err = match result {
        Ok(output) => panic!(
            "an unexpanded <xi:include> compiled instead of being rejected. \
             Generated source:\n{}",
            generated_source(&output)
        ),
        Err(e) => e,
    };

    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "xml/preprocessor-not-run",
        "rejection must name the missing expansion pass; got: {err}"
    );
}

/// The file facade does the whole sequence, and reports what it read.
///
/// This is the entry a consumer that just wants code out of a file
/// should reach for. Without a counterpart to `compile_scxml` on the
/// forge half, every caller assembles read-expand-parse-generate for
/// itself, and getting that order wrong is exactly the mistake the
/// guard above exists to catch.
///
/// `deps` is asserted alongside the row because a missing dependency is
/// the same defect wearing different clothes: the artefact goes stale
/// on a template edit, and the build reports success.
#[test]
fn compile_forge_file_expands_and_reports_its_inputs() {
    let (dir, main) = fixture(DOC_WITH_USE);

    let output =
        sce_build::compile_forge_file(&main, Language::Rust, &[], &ForgeCompileOptions::default())
            .expect("file facade must compile a template-bearing document");

    let src = generated_source(&output);
    assert!(
        src.contains(TEMPLATED_VARIANT),
        "file facade dropped the templated row; generated source:\n{src}"
    );

    let template_path = dir.path().join(TEMPLATE_FILE);
    assert!(
        output.deps.iter().any(|d| d.ends_with(TEMPLATE_FILE)),
        "the template is an input to this compile and must appear in deps \
         so a build script can rerun on it. Wanted {}, got {:?}",
        template_path.display(),
        output.deps,
    );
}

/// `orchestrate` resolves fragments through its own search path.
///
/// The multi-doc route resolves documents from paths, so it owns the
/// read step its callers cannot reach — and a search path it does not
/// accept is one an operator cannot supply. With the template held in a
/// sibling directory, document-relative resolution alone cannot find
/// it, so this fails unless `--include-dir` reaches the expander.
#[test]
fn orchestrate_resolves_templates_through_include_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let docs = dir.path().join("docs");
    let shared = dir.path().join("shared");
    let out_dir = dir.path().join("out");
    for d in [&docs, &shared, &out_dir] {
        std::fs::create_dir_all(d).expect("mkdir");
    }
    // Only reachable via the search path, never document-relative.
    std::fs::write(shared.join(TEMPLATE_FILE), TEMPLATE_BODY).expect("write template");
    let main = docs.join("probe.scxml");
    std::fs::write(&main, DOC_WITH_USE).expect("write doc");

    let output = Command::new(codegen_bin())
        .arg("orchestrate")
        .arg("--forge")
        .arg(&main)
        .arg("--include-dir")
        .arg(&shared)
        .arg("-l")
        .arg("rust")
        .arg("-o")
        .arg(&out_dir)
        .output()
        .expect("run sce-codegen orchestrate");

    assert!(
        output.status.success(),
        "orchestrate must resolve the template through --include-dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let mut emitted = String::new();
    for entry in std::fs::read_dir(&out_dir).expect("read out dir") {
        let path = entry.expect("dir entry").path();
        if path.is_file() {
            emitted.push_str(&std::fs::read_to_string(&path).expect("read emitted"));
        }
    }
    assert!(
        emitted.contains(TEMPLATED_VARIANT),
        "orchestrate dropped the templated row. Emitted:\n{emitted}"
    );
}

/// The statechart route carries the same precondition.
///
/// `SCXMLParser::parse_file` expands before parsing, so the file-based
/// entry cannot reach this state — but `parse_string` hands content
/// straight to the parser, and the statechart parser selects children by
/// tag name exactly as the forge kind parsers do. A surviving
/// `<xi:include>` is skipped, and the states it was carrying are absent
/// from a model that reports no error.
#[test]
fn statechart_parse_string_rejects_unexpanded_directives() {
    const STATECHART_WITH_XINCLUDE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:xi="http://www.w3.org/2001/XInclude"
       version="1.0" name="probe" initial="a">
  <state id="a"/>
  <xi:include href="extra_states.xml"/>
</scxml>
"#;

    let err = sce_build::parser::SCXMLParser::new()
        .parse_string(STATECHART_WITH_XINCLUDE, "probe")
        .expect_err("an unexpanded <xi:include> must not parse into a model");

    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "xml/preprocessor-not-run",
        "rejection must name the missing expansion pass; got: {err}"
    );
}

/// End-to-end through the documented entry point.
///
/// `sce-codegen generate` is itself a file facade, and its forge arm
/// took the shortcut this whole guard is about: read the file, hand the
/// bytes to the forge parser, never expand. Its statechart arm goes
/// through `Parser::parse_file`, which expands — so the two arms of one
/// command answered differently about whether templates exist,
/// depending only on the document's kind.
#[test]
fn cli_generate_expands_templates_on_the_forge_route() {
    let (dir, main) = fixture(DOC_WITH_USE);
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("mkdir out");
    let depfile = dir.path().join("probe.d");

    let output = Command::new(codegen_bin())
        .arg("generate")
        .arg(&main)
        .arg("-o")
        .arg(&out_dir)
        .arg("-l")
        .arg("rust")
        .arg("--write-deps")
        .arg(&depfile)
        .output()
        .expect("run sce-codegen");

    assert!(
        output.status.success(),
        "generate must succeed on a template-bearing forge document.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let mut emitted = String::new();
    for entry in std::fs::read_dir(&out_dir).expect("read out dir") {
        let path = entry.expect("dir entry").path();
        if path.is_file() {
            emitted.push_str(&std::fs::read_to_string(&path).expect("read emitted"));
            emitted.push('\n');
        }
    }

    // The depfile field is called `preprocessor_deps`, but while this
    // route skipped expansion it could only ever be handed the import
    // closure. A template edit then left the output stale with the build
    // reporting success — the same silent staleness in a second place.
    let deps = std::fs::read_to_string(&depfile).expect("depfile must be written");
    assert!(
        deps.contains(TEMPLATE_FILE),
        "depfile must name the template as an input, else editing it \
         triggers no rebuild. Depfile:\n{deps}"
    );

    assert!(
        emitted.contains(TEMPLATED_VARIANT),
        "`sce-codegen generate` dropped the templated row. Emitted:\n{emitted}"
    );
}
