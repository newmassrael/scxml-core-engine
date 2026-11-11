// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A union crosses a statechart's call of an algorithm (docs/adr/0092, section
//! 6.1 item 7, and "Decided while landing step 5"), held to
//! `sce-build/tests/fixtures/union/statechart_shape_crossing.scxml`, which hands
//! its `union:Shape` variable to `algorithm_shape_grown.scxml` and
//! `algorithm_shape_extent.scxml` and takes the grown shape back:
//!
//! * Kotlin, Rust and Go, the lanes that lower an algorithm over a union
//!   (`ALGORITHM_UNION_BACKENDS`), map the machine's own sum into the type the
//!   union's unit declares in the call's adapter, case by case, each case's
//!   record copied as a record argument is, and map what the algorithm returns
//!   back the same way; the machine imports the union's unit and each case's
//!   record unit (Go only where the adapter names them: for what it hands
//!   over, since what it takes back is read through its methods).
//! * A union is handed over by the name of a variable of the same union; one
//!   that may be absent is refused by name.
//! * Every other lane refuses the statechart.
//!
//! What the generated machine does at run time is not run here: no committed
//! tree holds this statechart.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// The files `language` generates for the statechart at `path`, or the
/// refusal, as its debug text.
fn generated(path: &Path, language: Language) -> Result<Vec<(String, String)>, String> {
    compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .map(|output| output.files)
    .map_err(|e| format!("{e:?}"))
}

/// The machine `language` generates for the fixture, the one file whose name
/// `machine` matches.
fn machine(language: Language, machine: &dyn Fn(&str) -> bool) -> String {
    let files = generated(
        &fixtures().join("statechart_shape_crossing.scxml"),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} hands a union to an algorithm: {e}"));
    let names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    files
        .into_iter()
        .find(|(name, _)| machine(name))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("no machine among {names:?}"))
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

const MACHINE_UNION: &str = "StatechartShapeCrossingUnionShapeMinimalUnion";

/// Pinned by `KotlinTarget::state_adapter`, `AdapterSpec::union_crossing` and
/// `kotlin_union_crossing` (`sce-build/src/forge/static_lowering.rs`): a `when`
/// over the machine's sealed interface builds the unit's case of each, the
/// record from the case's `value`, and a `when` over the unit's builds the
/// machine's back.
#[test]
fn kotlin_maps_the_machines_union_into_the_units_and_back() {
    let code = machine(Language::Kotlin, &|name| name.ends_with("Sm.kt"));
    let unit = "UnionShapeMinimal";
    holds(
        &code,
        &[
            "import com.sce.generated.union_shape_minimal.*",
            "import com.sce.generated.shape_radius_minimal.*",
            "import com.sce.generated.shape_rect_minimal.*",
            // The union handed over.
            &format!("private fun call_Extent__v(p0: {MACHINE_UNION}): UShort {{"),
            "when (val sceCase = p0) { ",
            &format!("{MACHINE_UNION}.DOT -> {unit}.Dot"),
            &format!(
                "is {MACHINE_UNION}.CIRCLE -> {unit}.Circle(ShapeRadiusMinimalPayload(radius = \
                 sceCase.value.radius))"
            ),
            &format!(
                "is {MACHINE_UNION}.RECT -> {unit}.Rect(ShapeRectMinimalPayload(width = \
                 sceCase.value.width, height = sceCase.value.height))"
            ),
            // The union taken back.
            &format!("): {MACHINE_UNION} {{"),
            "when (val sceCase = r) { ",
            &format!("{unit}.Dot -> {MACHINE_UNION}.DOT"),
            &format!(
                "is {unit}.Circle -> {MACHINE_UNION}.CIRCLE(StatechartShapeCrossingRadiusRecord(\
                 radius = sceCase.value.radius))"
            ),
            // Each call made through its adapter, by the variable's name.
            "vShape = call_Grown__v(vShape, ",
            "vExtent = call_Extent__v(vShape)",
        ],
    );
}

/// Pinned by `RustTarget::state_adapter`, `AdapterSpec::union_crossing` and
/// `rust_union_crossing` (`sce-build/src/forge/static_lowering.rs`): the
/// machine's enum is lent and matched case by case into the unit's, and what the
/// algorithm returns matched by reference back into the machine's.
#[test]
fn rust_maps_the_machines_union_into_the_units_and_back() {
    let code = machine(Language::Rust, &|name| {
        name == "statechart_shape_crossing_sm.rs"
    });
    let unit = "union_shape_minimal::UnionShapeMinimal";
    holds(
        &code,
        &[
            "use super::union_shape_minimal;",
            "use super::shape_radius_minimal;",
            "use super::shape_rect_minimal;",
            // The union handed over.
            &format!("fn call_Extent__v(p0: &{MACHINE_UNION}) -> u16 {{"),
            "match p0 { ",
            &format!("{MACHINE_UNION}::Dot => {unit}::Dot"),
            &format!(
                "{MACHINE_UNION}::Circle(sce_case) => {unit}::Circle(\
                 shape_radius_minimal::ShapeRadiusMinimalPayload {{ radius: sce_case.radius }})"
            ),
            &format!(
                "{MACHINE_UNION}::Rect(sce_case) => {unit}::Rect(\
                 shape_rect_minimal::ShapeRectMinimalPayload {{ width: sce_case.width, height: \
                 sce_case.height }})"
            ),
            // The union taken back.
            &format!(") -> {MACHINE_UNION} {{"),
            "match &r { ",
            &format!("{unit}::Dot => {MACHINE_UNION}::Dot"),
            &format!(
                "{unit}::Circle(sce_case) => {MACHINE_UNION}::Circle(\
                 StatechartShapeCrossingRadiusRecord {{ radius: sce_case.radius }})"
            ),
            // Each call made through its adapter, the variable lent.
            "call_Grown__v(&self.shape, ",
            "call_Extent__v(&self.shape)",
        ],
    );
}

/// The module path a Go machine imports the generated packages under.
const GO_PREFIX: &str = "example.com/generated";

/// The Go machine of the fixture, generated through `sce-codegen generate`,
/// the entry that takes the module path a Go statechart that calls an
/// algorithm needs (`--go-module-prefix`).
fn go_machine() -> String {
    let out = tempfile::tempdir().expect("a directory");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(&repo_root)
        .args(["generate", "-l", "go", "--go-module-prefix", GO_PREFIX])
        .arg(fixtures().join("statechart_shape_crossing.scxml"))
        .arg("-o")
        .arg(out.path())
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "Go hands a union to an algorithm:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(out.path().join("statechart_shape_crossing_sm.go"))
        .expect("the Go machine")
}

/// Pinned by `GoTarget::state_adapter`, `AdapterSpec::union_crossing` and
/// `go_union_crossing` (`sce-build/src/forge/static_lowering.rs`): a `switch`
/// over the declared name of the machine's tag builds the unit's case of each
/// by its constructor, the record from the machine's proven read of the case,
/// and a `switch` over the declared name of the unit's tag builds the
/// machine's back; a value in no case panics.
#[test]
fn go_maps_the_machines_union_into_the_units_and_back() {
    let code = go_machine();
    let unit = "sce_union_shape_minimal.UnionShapeMinimal";
    holds(
        &code,
        &[
            &format!("\tsce_union_shape_minimal \"{GO_PREFIX}/union_shape_minimal\"\n"),
            &format!("\tsce_shape_radius_minimal \"{GO_PREFIX}/shape_radius_minimal\"\n"),
            &format!("\tsce_shape_rect_minimal \"{GO_PREFIX}/shape_rect_minimal\"\n"),
            // The union handed over.
            &format!(
                "func sceAdapt_StatechartShapeCrossing_Extent__v(p0 {MACHINE_UNION}) uint16 {{"
            ),
            &format!("\tvar a0 {unit}\n\tswitch p0.Kind().String() {{\n"),
            &format!("\tcase \"dot\":\n\t\ta0 = {unit}Dot()\n"),
            &format!(
                "\tcase \"circle\":\n\t\tsceCase := p0.ProvenCircle()\n\t\ta0 = {unit}Circle(\
                 sce_shape_radius_minimal.ShapeRadiusMinimalPayload{{Radius: sceCase.radius}})\n"
            ),
            &format!(
                "\tcase \"rect\":\n\t\tsceCase := p0.ProvenRect()\n\t\ta0 = {unit}Rect(\
                 sce_shape_rect_minimal.ShapeRectMinimalPayload{{Width: sceCase.width, Height: \
                 sceCase.height}})\n"
            ),
            "\tdefault:\n\t\tpanic(\"a value of union_shape_minimal is in no case of its tag\")\n",
            "\treturn sce_algorithm_shape_extent.AlgorithmShapeExtent(a0)\n",
            // The union taken back.
            &format!(") {MACHINE_UNION} {{"),
            &format!("\tvar out {MACHINE_UNION}\n\tswitch r.Kind().DeclaredName().Proven() {{\n"),
            &format!("\tcase \"dot\":\n\t\tout = {MACHINE_UNION}Dot()\n"),
            &format!(
                "\tcase \"circle\":\n\t\tsceCase := r.ProvenCircle()\n\t\tout = \
                 {MACHINE_UNION}Circle(StatechartShapeCrossingRadiusRecord{{radius: \
                 sceCase.Radius}})\n"
            ),
            // Each call made through its adapter, by the variable's name.
            "sceAdapt_StatechartShapeCrossing_Grown__v(p.vShape, ",
            "sceAdapt_StatechartShapeCrossing_Extent__v(p.vShape)",
        ],
    );
}

/// The fixture with `edit` applied to its text, written beside the documents it
/// imports; what `language` makes of it.
fn edited(
    edit: &dyn Fn(&str) -> String,
    language: Language,
) -> Result<Vec<(String, String)>, String> {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for fixture in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
        "algorithm_shape_extent.scxml",
        "algorithm_shape_grown.scxml",
    ] {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("copy a fixture");
    }
    let text = std::fs::read_to_string(fixtures().join("statechart_shape_crossing.scxml"))
        .expect("the fixture");
    let path = dir.path().join("statechart_shape_crossing.scxml");
    std::fs::write(&path, edit(&text)).expect("write the statechart");
    generated(&path, language)
}

/// A union that may be absent is no union the algorithm takes: its present
/// value is put into a union variable first, which the refusal says.
#[test]
fn a_union_that_may_be_absent_is_refused_as_an_argument_by_name() {
    let refusal = edited(
        &|text| {
            text.replace(
                "    <data id=\"extent\"",
                "    <data id=\"kept\" sce:type=\"optional&lt;union:Shape&gt;\" expr=\"null\"/>\n    \
                 <data id=\"extent\"",
            )
            .replace("expr=\"Extent(shape)\"", "expr=\"Extent(kept)\"")
        },
        Language::Kotlin,
    )
    .expect_err("an optional union is no argument of a union slot");
    assert!(
        refusal.contains("`kept` may be absent, and the algorithm takes a union of `Shape`"),
        "{refusal}"
    );
}

/// A lane that does not lower an algorithm over a union refuses a statechart
/// that hands one a union, rather than write a call no unit answers.
#[test]
fn a_lane_that_lowers_no_union_crossing_refuses_the_statechart() {
    for language in [Language::Swift, Language::Python] {
        assert!(
            generated(
                &fixtures().join("statechart_shape_crossing.scxml"),
                language
            )
            .is_err(),
            "{language:?} generated a statechart that hands an algorithm a union"
        );
    }
}
