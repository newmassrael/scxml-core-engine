// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Python `sce-static` machine holds a union (docs/adr/0092, sections 4, 6.1
//! and 7, the Python row), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The union is a class of the machine and one frozen dataclass per case
//!   derived from it, holding the case's record as `value` and its tag as a
//!   `kind` its class holds, built whole by the class of its case.
//! * The lift reads each union's tag first (`UnionOf`), then the record of the
//!   case the tag names; a `<sce:lift>` reads a text into a union the same way
//!   (`lift_value`).
//! * Each read is spelled by the cases the tests leave: the tag by `kind`, a
//!   field of the one case left through that case's proven read, and a field
//!   two cases share through its accessor.
//! * A field of a case is written by assigning the variable the next value
//!   whole, the case's record with the field replaced.
//! * The machine's saved form writes a union as its tag beside its case and
//!   reads it back by its tag first; a case record that holds a list writes
//!   and reads it by its type, held to its counts.
//! * The payload class holds a list of unions to the lower count its type
//!   states where a host makes it.
//!
//! What the generated machine does at run time is held by the scenario
//! `static_union.json`, replayed over the machine
//! `scripts/regen_static_datamodel_python.sh` generates by
//! `backends/python/tests/integration/static_datamodel/test_static_scenarios.py`
//! (`test_a_union_payload_is_read_by_its_tag_and_every_union_restores`).

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The Python machine of the fixture: every file it is generated as, one after
/// the other.
fn machine() -> String {
    let language = Language::Python;
    let path = fixtures().join("static_union.scxml");
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Python holds a union: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    assert!(
        names.iter().any(|name| name.ends_with("_sm.py")),
        "the machine's module is generated among {names:?}"
    );
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

const UNION: &str = "StaticUnionUnionShapeUnion";

/// Pinned by `PythonComposed::union_type` and `PythonComposed::union_def`
/// (`sce-build/src/forge/composed_lowering/python.rs`).
#[test]
fn the_payload_holds_the_union_in_the_machines_own_class() {
    let code = machine();
    holds(
        &code,
        &[
            &format!(
                "class {UNION}:\n    \"\"\"SCE Accepted Subset §2.15: a `union:union_shape` \
                 value"
            ),
            "    def proven_rect(self) -> StaticUnionRectRecord:\n",
            "    def proven_field_radius(self) -> int:\n",
            &format!(
                "@sce_dataclasses.dataclass(frozen=True)\nclass {UNION}Dot({UNION}):\n    \"\"\"The \
                 case `dot`, its tag alone.\"\"\"\n\n    kind = StaticUnionKindEnum.DOT\n"
            ),
            &format!(
                "class {UNION}Path({UNION}):\n    \"\"\"The case `path`, holding its record.\"\"\"\n\n    \
                 value: StaticUnionPathRecord\n    kind = StaticUnionKindEnum.PATH\n"
            ),
            // The lift reads the tag first, then the case it names.
            &format!(
                "sce_event_payload.UnionOf(\"kind\", StaticUnionKindEnum, \
                 [sce_event_payload.CaseOf(StaticUnionKindEnum.DOT, None, {UNION}Dot), "
            ),
        ],
    );
}

/// Pinned by `python_union_member` and the read the expression lowering spells
/// by the cases the facts leave (`lower_union_reads`, `sce-build/src/forge/expr.rs`),
/// and by `PythonComposed::union_member_write` and the `<assign>` that assigns
/// the holder the next value (`union_field_place`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn each_read_and_write_is_spelled_by_the_cases_the_tests_leave() {
    let code = machine();
    holds(
        &code,
        &[
            // The tag by its attribute.
            ".one.kind == StaticUnionKindEnum.DOT",
            // A field of the one case left, through that case's proven read.
            ".one.proven_rect().width",
            // A field two cases share, through its accessor.
            ".one.proven_field_radius()",
            // A field of the built variable's proven case, written by assigning
            // the variable the next value whole.
            &format!(
                "self.v_built = {UNION}Rect(sce_dataclasses.replace(self.v_built.proven_rect(), \
                 width=("
            ),
            "self.v_built.proven_rect().width",
        ],
    );
}

/// Pinned by `union_variable_field` (`sce-build/src/forge/static_lowering.rs`)
/// and `PythonComposed::union_value`: a variable of the union starts at the
/// case its `<sce:set>`s build, and one that may be absent starts absent.
#[test]
fn a_union_variable_starts_built_or_absent() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("v_built: {UNION} = {UNION}Rect(StaticUnionRectRecord(width="),
            &format!("v_kept: Optional[{UNION}] = None"),
            &format!("v_lifted: Optional[{UNION}] = None"),
        ],
    );
}

/// Pinned by `PythonComposed::saved_union_ext`, `PythonComposed::saved_record_ext`
/// and the saved form the Python template writes a variable by
/// (`tools/codegen/templates/_macros/python_saved_value.jinja2`).
#[test]
fn a_saved_union_is_its_tag_beside_its_case_and_is_read_back_by_its_tag() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("def _{UNION}_to_saved(self):\n"),
            "        return sce_saved.saved_write_union(\"kind\", \"circle\", \
             self.value.to_saved())\n",
            "        return sce_saved.saved_write_union(\"kind\", \"dot\", None)\n",
            "    return sce_saved.saved_read_union(value, what, \"kind\", \"union_shape\", {\n",
            &format!(
                "        \"circle\": lambda: {UNION}Circle(StaticUnionRadiusRecord.from_saved(value, \
                 what)),\n"
            ),
            &format!("{UNION}.from_saved = classmethod(_{UNION}_from_saved)"),
            // A case record that holds a list writes and reads it by its type,
            // held to its counts.
            "def _StaticUnionPathRecord_to_saved(self):\n",
            "        \"points\": (lambda sce_saved0: sce_saved.saved_write_list(sce_saved0, \
             sce_saved.saved_write_uint8))(self.points),\n",
            "sce_saved.saved_read_list_within(sce_saved0, sce_what0, 0, 3, \
             sce_saved.saved_read_uint8)",
            // Each variable writes and reads its union by the union's own form.
            "policy.v_built.to_saved()",
            "sce_saved.saved_write_optional(policy.v_kept, lambda v: v.to_saved())",
            &format!("{UNION}.from_saved(grown.variable(\"built\"), \"built\")"),
            &format!(
                "sce_saved.saved_read_optional(grown.variable(\"kept\"), \"kept\", \
                 {UNION}.from_saved)"
            ),
        ],
    );
}

/// Pinned by the Python lift (`PythonTarget::lift`,
/// `sce-build/src/forge/static_lowering.rs`): a text is read into the union
/// whole by the reader a delivery's field of it is read by.
#[test]
fn a_text_is_lifted_into_the_union_by_its_reader() {
    let code = machine();
    holds(
        &code,
        &[
            "sce_lifted, sce_refusal = sce_event_payload.lift_value(",
            ", 2, 256, sce_event_payload.UnionOf(\"kind\", StaticUnionKindEnum, [",
            "self.v_lifted = sce_lifted",
        ],
    );
}

/// Pinned by `python_check_type` (`sce-build/src/forge/composed_lowering/python.rs`):
/// the payload class of `shapes.sent` holds `many` to the lower count its type
/// states where a host makes it, each union of it a value its case's class
/// built.
#[test]
fn the_payload_class_holds_a_list_of_unions_to_its_lower_count() {
    let code = machine();
    holds(
        &code,
        &[
            "sce_event_payload.check_value(\"many\", sce_event_payload.ListOf(None, 3, least=1), \
           self.many)",
        ],
    );
}
