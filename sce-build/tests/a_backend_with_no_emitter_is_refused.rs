// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A backend that is in `Language::ALL` before it has an emitter must say so,
// by name, at every door a document comes in by. Swift was the first: it is
// decided at every site that decides per backend, and renders every kind but a
// statechart, an algorithm and an event schema not at all, which it renders in
// part.
//
// The alternative is worse than a refusal. A backend with no arm in a
// dispatch either panics in a helper the document reached by chance, or falls
// into another backend's arm and emits that language's source under this one's
// file name. Both are silent until a build fails somewhere far from the cause.
//
// `codegen_matrix::template_ships` is the one registry that says which
// backend emits which kind, and `codegen_matrix::check` is the one door that
// turns its answer into the diagnostic. This test holds each entry point to
// that door rather than to a list of its own: it derives the backends with no
// emitter from the registry, so a backend that gains one leaves the set by
// itself, and a backend that is added without one joins it.

use sce_build::forge::codegen_matrix::{
    check, emitting, emitting_in_part, lookup, ships_in_part, EmitOutcome,
};
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::{compile_forge_from_string, compile_scxml_lang_typed, find_template_dir_for};
use sce_build::{forge::codegen_matrix::language_wire_name, DocumentLabel};

fn every_kind() -> Vec<ForgeKind> {
    ForgeKind::ALL_ATTR_NAMES
        .iter()
        .map(|name| {
            ForgeKind::from_attr(name)
                .unwrap_or_else(|| panic!("`{name}` is listed as a kind but does not parse"))
        })
        .collect()
}

/// The backends no kind is emitted for: derived from the registry, not named.
fn backends_with_no_emitter() -> Vec<Language> {
    Language::ALL
        .iter()
        .copied()
        .filter(|language| {
            every_kind()
                .into_iter()
                .all(|kind| lookup(kind, *language) != EmitOutcome::Emit)
        })
        .collect()
}

#[test]
fn a_backend_with_no_emitter_exists_so_that_this_measures_something() {
    assert!(
        !backends_with_no_emitter().is_empty(),
        "every backend emits something, so the refusals below are held to nothing; \
         delete this file only when no backend can be added without an emitter"
    );
}

#[test]
fn every_kind_is_refused_for_it_by_the_matrix_and_named() {
    for language in backends_with_no_emitter() {
        // A declaration writes no file on any backend, so a backend with no
        // emitter serves it as every other backend does. A kind that awaits
        // its runtime (ADR 0062) passes the matrix too, since the refusal is
        // the dispatch's, which holds the document and names it
        // (`broadcast-log/runtime-missing`); the test below holds that. A kind
        // the backend writes in part passes as well: its renderer refuses by the
        // construct, which the tests below hold.
        let (declarations, renders): (Vec<ForgeKind>, Vec<ForgeKind>) =
            every_kind().into_iter().partition(|kind| {
                ships_in_part(*kind, language)
                    || matches!(
                        lookup(*kind, language),
                        EmitOutcome::Declaration | EmitOutcome::RuntimeMissing
                    )
            });
        for kind in declarations {
            assert!(check(kind, language).is_ok(), "{kind} on {language:?}");
        }
        for kind in renders {
            let refusal = check(kind, language)
                .expect_err(&format!("{kind} was admitted for {language:?}"))
                .to_string();
            assert!(
                refusal.contains(&format!("'{}'", language_wire_name(language))),
                "{kind}/{language:?}: the refusal does not name the backend: {refusal}"
            );
        }
        assert!(
            emitting(ForgeKind::Statechart).all(|emits| emits != language),
            "{language:?} has no emitter and is listed as emitting a statechart"
        );
    }
}

#[test]
fn a_statechart_is_refused_through_the_file_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("machine.scxml");
    std::fs::write(
        &path,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a" name="machine">
  <state id="a"/>
</scxml>"#,
    )
    .unwrap();
    // A backend that writes a statechart in part is held by the test below: it
    // generates what it lowers and refuses the rest by the construct.
    for language in backends_with_no_emitter()
        .into_iter()
        .filter(|language| !ships_in_part(ForgeKind::Statechart, *language))
    {
        let error = compile_scxml_lang_typed(
            path.to_str().unwrap(),
            &find_template_dir_for(language),
            language,
        )
        .err()
        .unwrap_or_else(|| panic!("{language:?} generated a machine it has no emitter for"));
        let text = format!("{error:?}");
        assert!(
            text.contains(language_wire_name(language)) && text.contains("statechart"),
            "{language:?}: the refusal names neither the backend nor the kind: {text}"
        );
    }
}

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

        // What it does not is refused by the construct, naming the backend.
        std::fs::write(
            &path,
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a" name="machine">
  <state id="a"/>
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
            text.contains(&format!("{language:?}")) && text.contains("datamodel"),
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

        // A construct it does not lower is refused by the construct, naming the backend.
        let unlowered = lowered.replace(
            "<sce:signature>",
            "<sce:const name=\"LIMIT\" type=\"uint8\" init=\"9\"/>\n  <sce:signature>",
        );
        assert_ne!(unlowered, lowered, "the replacement did not apply");
        let error = compile_forge_from_string(&unlowered, label, language)
            .err()
            .unwrap_or_else(|| panic!("{language:?} generated an algorithm it does not lower"));
        let text = format!("{error:?}");
        assert!(
            text.contains(&format!("{language:?}")) && text.contains("<sce:const"),
            "{language:?}: the refusal names neither the backend nor the construct: {text}"
        );
    }
}

#[test]
fn a_forge_document_is_refused_through_the_string_entry() {
    // A kind no backend of this list writes, in whole or in part: a timer.
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="timer" name="diag_scheduler">
  <sce:period>2s</sce:period>
  <sce:reset-on event="diag.heartbeat"/>
  <sce:cancel-on state-exit="diag.idle"/>
  <sce:fire-event>diag.tick</sce:fire-event>
</scxml>"#;
    for language in backends_with_no_emitter() {
        let error = compile_forge_from_string(
            document,
            DocumentLabel {
                identifier: "diag_scheduler",
                diagnostic_label: "diag_scheduler.scxml",
            },
            language,
        )
        .err()
        .unwrap_or_else(|| panic!("{language:?} generated a timer it has no emitter for"));
        let text = format!("{error:?}");
        assert!(
            text.contains(language_wire_name(language)),
            "{language:?}: the refusal does not name the backend: {text}"
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
