// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A backend writes a kind in part before it writes it in whole, and what it does not write it must
// refuse by name, at every door a document comes in by. Swift was the first backend in
// `Language::ALL` before it had an emitter, and it is decided at every site that decides per
// backend: it writes a statechart, an algorithm and an event schema in part.
//
// The alternative is worse than a refusal. A backend with no arm in a dispatch either panics in a
// helper the document reached by chance, or falls into another backend's arm and emits that
// language's source under this one's file name. Both are silent until a build fails somewhere far
// from the cause.
//
// `codegen_matrix::template_ships` is the one registry that says which backend emits which kind,
// and `codegen_matrix::emitting_in_part` which writes one in part. This test derives the backends
// that write a kind in part from the registry, so a backend that moves to `template_ships` leaves
// the set by itself, and one that gains a partial emitter joins it.
//
// What this file used to hold as well was the refusal of a backend with no emitter at all, which
// it derived the same way (a backend no kind is emitted for) and held to the matrix's refusal,
// the file entry and the string entry. Swift wrote its last kind, the queue, and that set became
// empty: those tests held nothing, and a test that cannot fail is the failure it was written
// against. A backend added to `Language::ALL` without an emitter is stopped earlier than a test:
// `kind_class`, `template_ships` and every per-backend dispatch are exhaustive over `Language`,
// so the build refuses it until each site has decided.

use sce_build::forge::codegen_matrix::{emitting, emitting_in_part};
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::{compile_forge_from_string, compile_scxml_lang_typed, find_template_dir_for};
use sce_build::{forge::codegen_matrix::language_wire_name, DocumentLabel};

#[test]
fn a_statechart_a_backend_writes_in_part_is_generated_and_the_rest_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("machine.scxml");
    let in_part: Vec<Language> = emitting_in_part(ForgeKind::Statechart)
        .filter(|language| emitting(ForgeKind::Statechart).all(|emits| emits != *language))
        .collect();
    assert!(
        !in_part.is_empty(),
        "no backend writes a statechart in part, so this measures nothing; delete it \
         when the last one moves to `template_ships`"
    );
    for language in in_part {
        // What it lowers is generated.
        std::fs::write(
            &path,
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a" name="machine" datamodel="null">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>"#,
        )
        .unwrap();
        let output = compile_scxml_lang_typed(
            path.to_str().unwrap(),
            &find_template_dir_for(language),
            language,
        )
        .unwrap_or_else(|e| panic!("{language:?} refused a machine it lowers: {e:?}"));
        assert!(!output.files.is_empty(), "{language:?} wrote nothing");

        // What it does not is refused by the construct, naming the backend. The construct is one
        // no lowering will ever write, so the probe does not outlive the backend's growth: a
        // `<donedata>` of a `null` document needs a script engine to read its value, and a
        // `null` document has none.
        std::fs::write(
            &path,
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a" name="machine" datamodel="null">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"><donedata><content>done</content></donedata></final>
</scxml>"#,
        )
        .unwrap();
        let error = compile_scxml_lang_typed(
            path.to_str().unwrap(),
            &find_template_dir_for(language),
            language,
        )
        .err()
        .unwrap_or_else(|| panic!("{language:?} generated a machine it does not lower"));
        let text = format!("{error:?}");
        assert!(
            text.contains(&format!("{language:?}")) && text.contains("<donedata>"),
            "{language:?}: the refusal names neither the backend nor the construct: {text}"
        );
    }
}

#[test]
fn an_algorithm_a_backend_writes_in_part_is_generated_and_the_rest_is_refused_by_name() {
    let in_part: Vec<Language> = emitting_in_part(ForgeKind::Algorithm)
        .filter(|language| emitting(ForgeKind::Algorithm).all(|emits| emits != *language))
        .collect();
    assert!(
        !in_part.is_empty(),
        "no backend writes an algorithm in part, so this measures nothing; delete it \
         when the last one moves to `template_ships`"
    );
    let label = DocumentLabel {
        identifier: "clamp",
        diagnostic_label: "clamp.scxml",
    };
    for language in in_part {
        // A body of scalars is generated.
        let lowered = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
          version="1.0" sce:kind="algorithm" name="clamp">
  <sce:signature>
    <sce:param name="value" type="uint8"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="uint8" init="value"/>
    <sce:if cond="out > 9"><sce:assign target="out" expr="9"/></sce:if>
    <sce:return expr="out"/>
  </sce:body>
</scxml>"#;
        let output = compile_forge_from_string(lowered, label, language)
            .unwrap_or_else(|e| panic!("{language:?} refused an algorithm it lowers: {e:?}"));
        assert!(!output.files.is_empty(), "{language:?} wrote nothing");

        // A construct it does not lower is refused by the construct, naming the backend. A local
        // named for a word the backend reserves is one no lowering will ever write, so the probe
        // does not outlive the backend's growth the way a missing construct does.
        let unlowered = lowered.replace("out", "protocol");
        assert_ne!(unlowered, lowered, "the replacement did not apply");
        let error = compile_forge_from_string(&unlowered, label, language)
            .err()
            .unwrap_or_else(|| panic!("{language:?} generated an algorithm it does not lower"));
        let text = format!("{error:?}");
        // The validation that refuses a reserved word names the backend by its wire name.
        assert!(
            text.contains(language_wire_name(language)) && text.contains("sce:var"),
            "{language:?}: the refusal names neither the backend nor the construct: {text}"
        );
    }
}

#[test]
fn an_event_schema_a_backend_writes_in_part_is_generated_and_the_rest_is_refused_by_name() {
    let in_part: Vec<Language> = emitting_in_part(ForgeKind::EventSchema)
        .filter(|language| emitting(ForgeKind::EventSchema).all(|emits| emits != *language))
        .collect();
    assert!(
        !in_part.is_empty(),
        "no backend writes an event schema in part, so this measures nothing; delete it \
         when the last one moves to `template_ships`"
    );
    let label = DocumentLabel {
        identifier: "schema_line",
        diagnostic_label: "schema_line.scxml",
    };
    for language in in_part {
        // A schema of scalar fields is generated.
        let lowered = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
          version="1.0" sce:kind="event-schema" name="schema_line" sce:event-name="line.add">
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="note" sce:type="string" sce:max-size="16" sce:direction="in"/>
  </datamodel>
</scxml>"#;
        let output = compile_forge_from_string(lowered, label, language)
            .unwrap_or_else(|e| panic!("{language:?} refused a schema it lowers: {e:?}"));
        assert!(!output.files.is_empty(), "{language:?} wrote nothing");

        // A field it does not lower is refused by the field, naming the backend: a
        // value that may be a byte string, which no backend of this list holds in a
        // slot (it is bounded, as a byte string of a schema is).
        let unlowered = lowered.replace(
            r#"<data id="id" sce:type="uint32" sce:direction="in"/>"#,
            r#"<data id="id" sce:type="optional&lt;bytes&gt;" sce:max-size="8" sce:direction="in"/>"#,
        );
        assert_ne!(unlowered, lowered, "the replacement did not apply");
        let error = compile_forge_from_string(&unlowered, label, language)
            .err()
            .unwrap_or_else(|| panic!("{language:?} generated a schema it does not lower"));
        let text = format!("{error:?}");
        assert!(
            text.contains(&format!("{language:?}")) && text.contains("optional field `id`"),
            "{language:?}: the refusal names neither the backend nor the field: {text}"
        );
    }
}
