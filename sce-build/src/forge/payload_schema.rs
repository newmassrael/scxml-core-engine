// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The payload schema of an event schema (docs/adr/0087, section 1): the wire
//! form of a value of the payload, stated twice, as a JSON Schema (2020-12) and
//! as a GBNF grammar, folded from the one value type by the recursion that
//! sizes it ([`crate::forge::value_type::json_schema_of`],
//! [`crate::forge::value_type::gbnf_of`]).
//!
//! The JSON Schema is the canonical form, compact, its size that of the type
//! and not of its bounds: every schema unit declares it, `JSON_SCHEMA`, beside
//! `EVENT_NAME`, `REVISION`, `DATA_MAX_LEN` and `SCHEMA_ID`, each bound as the
//! build resolved it. The GBNF grammar counts every text's bytes exactly, so
//! its size is linear in the largest text bound; it is a compilation target
//! written only where a grammar is used: `sce-codegen payload-schema --format
//! gbnf`, and the table of the answer schemas a machine names
//! (docs/adr/0087, "Owner's answers", item 4). `sce-codegen payload-schema`
//! prints the unit's JSON Schema byte for byte. The JSON Schema is a wire
//! surface (SCE_WIRE_CONTRACTS.md, the "Payload schema" row), whose subset of
//! 2020-12 is `schemas/sce-payload-schema.v1.schema.json`; the GBNF grammar is
//! its second spelling, in a format SCE does not own, held by the goldens of
//! [`crate::forge::value_type`] and by the recognizer of this module's tests.
//! Both texts are ASCII: a character past it is escaped.
//!
//! Both describe the answer to write; the reader of a payload decides what is
//! accepted, and accepts every text of their languages and more. Both state a
//! record's keys in the order its document declares them, since a decoder
//! held to either writes them in the order it states
//! ([`crate::forge::value_type::SchemaObject`]); the reader takes them in any
//! order.
//!
//! A union document has a payload schema of its own (docs/adr/0092, section
//! 3.1, item 4), so a union may be named as an answer by itself: the same two
//! texts, whose root is the `oneOf` of its cases and the alternation of them,
//! not a record, named by its tag enum's revision (section 8.1).

use std::path::Path;

use crate::forge::model::{EventSchemaModel, ForgeImport, UnionModel};
use crate::forge::value_type::{
    gbnf_object, gbnf_union, json_schema_object, json_schema_union, GbnfRules, GrammarError,
    Keyword, SchemaDefs, SchemaNode, SchemaObject, ScopedUnion, TypeEnv, ValueType,
};

/// The stability status of `schemas/sce-payload-schema.v1.schema.json`, held to
/// the schema file's own header by `schema_file_declares_status`.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
pub const PAYLOAD_SCHEMA_STATUS: &str = "pre-release";

/// The dialect every payload schema is written in.
pub const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// What a payload schema's `description` says of its texts' bounds.
pub const BOUND_NOTE: &str =
    "x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters";

/// The id of the payload schema of the schema `name` at `revision`: its
/// `$id`, and the `SCHEMA_ID` of its unit.
pub fn schema_id(name: &str, revision: u32) -> String {
    format!("urn:sce:schema:{name}:{revision}")
}

/// Which of the two spellings of a payload schema is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadFormat {
    /// The JSON Schema (2020-12).
    JsonSchema,
    /// The GBNF grammar.
    Gbnf,
}

/// The payload schema of an event schema, or of a union document: its id and
/// its two texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadSchema {
    /// `urn:sce:schema:<document name>:<revision>` ([`schema_id`]).
    pub id: String,
    /// The JSON Schema, compact: each object's keywords in the order of
    /// [`Keyword`], a record's keys in the order its document declares them.
    pub json_schema: String,
    /// The GBNF grammar, one rule a line, `root` first.
    pub gbnf: String,
}

/// The JSON Schema of the payload of the schema `name` at `revision`, a record
/// of `fields` read in `env`: what every schema unit declares as
/// `JSON_SCHEMA`. `source_hash` is the `source-hash` of the files the schema
/// is written into (`forge::drift`), carried as `x-sce-source-hash`; a run
/// that writes no headed file has none, and the key is left out. The text is
/// ASCII, every other character escaped.
pub fn json_schema(
    name: &str,
    revision: u32,
    fields: &[(String, ValueType)],
    env: &dyn TypeEnv,
    source_hash: Option<&str>,
) -> Result<String, GrammarError> {
    let mut defs = SchemaDefs::default();
    let root = json_schema_object(fields, env, &mut defs)?;
    Ok(headed(root, name, revision, defs, source_hash))
}

/// The JSON Schema of a value of `union`, the union document of that name at
/// `revision`, its tag enum's (docs/adr/0092, sections 3.1 and 8.1): the
/// `oneOf` of its cases at the root, under the header [`json_schema`] writes.
pub fn union_json_schema(
    revision: u32,
    union: &ScopedUnion,
    source_hash: Option<&str>,
) -> Result<String, GrammarError> {
    let mut defs = SchemaDefs::default();
    let root = json_schema_union(union, &mut defs)?;
    Ok(headed(root, &union.name, revision, defs, source_hash))
}

/// `root`, the schema of the value, with what names the schema and what it
/// says of itself, and the definitions its fold gathered: the text of the
/// payload schema of `name` at `revision`.
fn headed(
    mut root: SchemaObject,
    name: &str,
    revision: u32,
    defs: SchemaDefs,
    source_hash: Option<&str>,
) -> String {
    root.insert(Keyword::Schema, SchemaNode::value(DIALECT));
    root.insert(Keyword::Id, SchemaNode::value(schema_id(name, revision)));
    if let Some(hash) = source_hash {
        root.insert(Keyword::SourceHash, SchemaNode::value(hash));
    }
    root.insert(Keyword::Description, SchemaNode::value(BOUND_NOTE));
    if let Some(defs) = defs.into_node() {
        root.insert(Keyword::Defs, defs);
    }
    root.text()
}

/// The GBNF grammar of the payload, a record of `fields` read in `env`: a
/// compilation target of the type, written where a grammar is used
/// (`sce-codegen payload-schema --format gbnf`, an answer schema's table), not
/// by every unit, since its size is linear in its largest text bound. The text
/// is ASCII, every other character escaped.
pub fn gbnf(fields: &[(String, ValueType)], env: &dyn TypeEnv) -> Result<String, GrammarError> {
    let mut rules = GbnfRules::default();
    let body = gbnf_object(fields, env, &mut rules)?;
    Ok(rules.grammar(&body))
}

/// The GBNF grammar of a value of `union`: its `root` one alternative per
/// case, each beginning with its tag (docs/adr/0092, section 3.1).
pub fn union_gbnf(union: &ScopedUnion) -> Result<String, GrammarError> {
    let mut rules = GbnfRules::default();
    let body = gbnf_union(union, &mut rules)?;
    Ok(rules.grammar(&body))
}

/// The union the union document `model`, read from `path`, is, with the
/// imports `imports` written relative to its directory, and its revision,
/// which is its tag enum's (docs/adr/0092, section 8.1). A tag enum that
/// cannot be read leaves the union no grammar, refused as an alias that names
/// nothing; the import pass reports why in its own words.
fn read_union(
    model: &UnionModel,
    imports: &[ForgeImport],
    path: &Path,
) -> Result<(u32, ScopedUnion), GrammarError> {
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let unresolved = || GrammarError::Unresolved {
        alias: model.tag_enum.clone(),
    };
    let revision = crate::forge::import_source::enum_model_in(imports, base_dir, &model.tag_enum)
        .ok_or_else(unresolved)?
        .revision();
    let union = crate::forge::import_source::scoped_union(path, model, imports, base_dir)
        .ok_or_else(unresolved)?;
    Ok((revision, union))
}

/// The text of `format` of a value of the union document `model`, read from
/// `path` with its imports `imports`: what `sce-codegen payload-schema` prints
/// for a union document (docs/adr/0092, section 3.1, item 4).
pub fn text_of_union(
    model: &UnionModel,
    imports: &[ForgeImport],
    path: &Path,
    format: PayloadFormat,
    source_hash: Option<&str>,
) -> Result<String, GrammarError> {
    let (revision, union) = read_union(model, imports, path)?;
    match format {
        PayloadFormat::JsonSchema => union_json_schema(revision, &union, source_hash),
        PayloadFormat::Gbnf => union_gbnf(&union),
    }
}

/// The text of `format` of the payload of `schema`, whose imports `imports`
/// are written relative to `base_dir`, the schema's directory: what
/// `sce-codegen payload-schema` prints, read in the scope a unit's sizes are
/// read in ([`crate::forge::import_source::type_scope`]).
pub fn text_of_event_schema(
    schema: &EventSchemaModel,
    imports: &[ForgeImport],
    base_dir: &Path,
    format: PayloadFormat,
    source_hash: Option<&str>,
) -> Result<String, GrammarError> {
    let scope = crate::forge::import_source::type_scope(imports, base_dir);
    let fields = schema.typed_fields();
    match format {
        PayloadFormat::JsonSchema => json_schema(
            &schema.name,
            schema.revision(),
            &fields,
            &scope,
            source_hash,
        ),
        PayloadFormat::Gbnf => gbnf(&fields, &scope),
    }
}

impl PayloadSchema {
    /// Both texts of the payload schema of the schema `name` at `revision`
    /// ([`json_schema`], [`gbnf`]): what a table of the answer schemas a
    /// machine names holds for each.
    pub fn of(
        name: &str,
        revision: u32,
        fields: &[(String, ValueType)],
        env: &dyn TypeEnv,
        source_hash: Option<&str>,
    ) -> Result<Self, GrammarError> {
        Ok(Self {
            id: schema_id(name, revision),
            json_schema: json_schema(name, revision, fields, env, source_hash)?,
            gbnf: gbnf(fields, env)?,
        })
    }

    /// Both texts of the payload schema of `schema`, read as
    /// [`text_of_event_schema`] reads it.
    pub fn of_event_schema(
        schema: &EventSchemaModel,
        imports: &[ForgeImport],
        base_dir: &Path,
        source_hash: Option<&str>,
    ) -> Result<Self, GrammarError> {
        let scope = crate::forge::import_source::type_scope(imports, base_dir);
        Self::of(
            &schema.name,
            schema.revision(),
            &schema.typed_fields(),
            &scope,
            source_hash,
        )
    }

    /// Both texts of the payload schema of a value of `union`, the union
    /// document of that name at `revision` ([`union_json_schema`],
    /// [`union_gbnf`]).
    pub fn of_union(
        revision: u32,
        union: &ScopedUnion,
        source_hash: Option<&str>,
    ) -> Result<Self, GrammarError> {
        Ok(Self {
            id: schema_id(&union.name, revision),
            json_schema: union_json_schema(revision, union, source_hash)?,
            gbnf: union_gbnf(union)?,
        })
    }

    /// Both texts of the payload schema of the union document `model`, read
    /// as [`text_of_union`] reads it.
    pub fn of_union_document(
        model: &UnionModel,
        imports: &[ForgeImport],
        path: &Path,
        source_hash: Option<&str>,
    ) -> Result<Self, GrammarError> {
        let (revision, union) = read_union(model, imports, path)?;
        Self::of_union(revision, &union, source_hash)
    }

    /// The text of one spelling.
    pub fn text(&self, format: PayloadFormat) -> &str {
        match format {
            PayloadFormat::JsonSchema => &self.json_schema,
            PayloadFormat::Gbnf => &self.gbnf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{ForgeDocument, SceType};
    use crate::forge::value_type::{
        ScopedCase, ScopedEnum, ScopedRecord, ScopedVariant, TypeScope, WrittenType,
    };
    use sce_rust_runtime::json::{number_text, parse, write, Value};
    use std::path::PathBuf;

    const SCHEMA: &str = include_str!("../../../schemas/sce-payload-schema.v1.schema.json");

    /// A source hash, as a run's header states one.
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn wire_schema() -> serde_json::Value {
        serde_json::from_str(SCHEMA).expect("the payload schema's schema is JSON")
    }

    /// Why `instance` is not a payload schema SCE writes, one sentence a
    /// violation; empty when it is one.
    fn wire_violations(instance: &serde_json::Value) -> Vec<String> {
        let schema = wire_schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("the payload schema's schema compiles");
        let outcome = validator.validate(instance);
        match outcome {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        }
    }

    /// `text`, a payload schema, compiled by an implementation of JSON Schema
    /// 2020-12 that is not the generator: compiling it validates it against
    /// the official 2020-12 meta-schema first.
    fn compiled(text: &str) -> jsonschema::JSONSchema {
        let schema: serde_json::Value =
            serde_json::from_str(text).expect("a payload schema is JSON");
        jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .compile(&schema)
            .unwrap_or_else(|e| panic!("not a 2020-12 schema ({e}): {text}"))
    }

    fn json_admits(schema: &jsonschema::JSONSchema, text: &str) -> bool {
        schema.is_valid(&serde_json::from_str(text).expect("a value is JSON"))
    }

    fn ty(text: &str) -> ValueType {
        WrittenType::read(text)
            .expect("a type")
            .bounded()
            .expect("every bound stated")
    }

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The payload schema of the event-schema document at `path`, as
    /// `sce-codegen payload-schema` reads it, or `None` for a document of
    /// another kind or one that does not parse.
    fn of_document(path: &Path) -> Option<Result<PayloadSchema, GrammarError>> {
        let content = std::fs::read_to_string(path).ok()?;
        let label = path.to_string_lossy().into_owned();
        let parsed = crate::forge::parser::parse_forge_with_imports(
            &content,
            crate::DocumentLabel::for_input_path(&label),
        )
        .ok()??;
        let ForgeDocument::EventSchema(schema) = &parsed.document else {
            return None;
        };
        let base = path.parent().expect("a fixture has a directory");
        Some(PayloadSchema::of_event_schema(
            schema,
            &parsed.imports,
            base,
            Some(HASH),
        ))
    }

    /// The schema docs/adr/0087 section 1.1 writes out.
    fn answer_slot() -> PayloadSchema {
        of_document(&manifest_dir().join("tests/fixtures/payload_schema/answer_slot.scxml"))
            .expect("an event schema")
            .expect("a grammar")
    }

    #[test]
    fn schema_file_declares_status() {
        assert_eq!(
            wire_schema()["x-sce-schema-status"].as_str(),
            Some(PAYLOAD_SCHEMA_STATUS),
            "schemas/sce-payload-schema.v1.schema.json x-sce-schema-status disagrees with \
             PAYLOAD_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    #[test]
    fn the_example_of_the_decision_is_written_as_it_reads() {
        let schema = answer_slot();
        assert_eq!(schema.id, "urn:sce:schema:answer_slot:1");
        assert_eq!(
            schema.json_schema,
            format!(
                "{{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\
                 \"$id\":\"urn:sce:schema:answer_slot:1\",\
                 \"x-sce-source-hash\":\"{HASH}\",\
                 \"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts \
                 characters\",\
                 \"type\":\"object\",\
                 \"properties\":{{\"title\":{{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16}},\
                 \"slot\":{{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\
                 \"kind\":{{\"enum\":[\"meeting\",\"task\"]}},\
                 \"note\":{{\"anyOf\":[{{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8}},\
                 {{\"type\":\"null\"}}]}}}},\
                 \"required\":[\"title\",\"slot\",\"kind\",\"note\"],\
                 \"additionalProperties\":false}}"
            )
        );
        let lines: Vec<&str> = schema.gbnf.lines().collect();
        assert_eq!(
            lines[..2],
            [
                "root ::= \"{\\\"title\\\":\" \"\\\"\" t16 \"\\\"\" \",\\\"slot\\\":\" uint16 \
                 \",\\\"kind\\\":\" ( \"\\\"meeting\\\"\" | \"\\\"task\\\"\" ) \
                 \",\\\"note\\\":\" ( \"null\" | \"\\\"\" t8 \"\\\"\" ) \"}\"",
                "uint16 ::= \"0\" | [1-9] [0-9]{0,3} | [1-5] [0-9]{4} | \"6\" [0-4] [0-9]{3} | \
                 \"65\" [0-4] [0-9]{2} | \"655\" [0-2] [0-9] | \"6553\" [0-5]",
            ]
        );
        // The characters, then one chain up to the larger text bound.
        assert_eq!(lines.len(), 2 + 4 + 17, "{}", schema.gbnf);
        assert_eq!(
            lines.last(),
            Some(&"t16 ::= \"\" | c1 t15 | c2 t14 | c3 t13 | c4 t12")
        );
        assert_eq!(schema.text(PayloadFormat::Gbnf), schema.gbnf);
        assert_eq!(schema.text(PayloadFormat::JsonSchema), schema.json_schema);
        // The one text each spelling asked for is the text of both.
        let path = manifest_dir().join("tests/fixtures/payload_schema/answer_slot.scxml");
        let content = std::fs::read_to_string(&path).unwrap();
        let label = path.to_string_lossy().into_owned();
        let parsed = crate::forge::parser::parse_forge_with_imports(
            &content,
            crate::DocumentLabel::for_input_path(&label),
        )
        .unwrap()
        .unwrap();
        let ForgeDocument::EventSchema(model) = &parsed.document else {
            unreachable!("an event schema");
        };
        for (format, text) in [
            (PayloadFormat::JsonSchema, &schema.json_schema),
            (PayloadFormat::Gbnf, &schema.gbnf),
        ] {
            assert_eq!(
                text_of_event_schema(
                    model,
                    &parsed.imports,
                    path.parent().unwrap(),
                    format,
                    Some(HASH)
                )
                .as_ref(),
                Ok(text),
                "{format:?}"
            );
        }
    }

    #[test]
    fn a_run_that_writes_no_headed_file_states_no_source_hash() {
        let fields = [("on".to_string(), ty("bool"))];
        let without =
            PayloadSchema::of("probe", 3, &fields, &TypeScope::default(), None).expect("a grammar");
        let parsed: serde_json::Value = serde_json::from_str(&without.json_schema).unwrap();
        assert_eq!(parsed.get("x-sce-source-hash"), None, "{parsed}");
        assert_eq!(parsed["$id"], "urn:sce:schema:probe:3");
        assert_eq!(wire_violations(&parsed), Vec::<String>::new());
        let with = PayloadSchema::of("probe", 3, &fields, &TypeScope::default(), Some(HASH))
            .expect("a grammar");
        assert_eq!(with.gbnf, without.gbnf, "the grammar carries no hash");
    }

    /// A record's keys are stated in the order its document declares them, in
    /// both spellings, at the root and in a record of `$defs`: a decoder held
    /// to a schema writes an object's keys in the order the schema states
    /// them, and the declared order is the author's. Neither is ascending.
    #[test]
    fn a_record_states_its_keys_in_the_order_its_document_declares_them() {
        let mut scope = TypeScope::default();
        scope.records.insert(
            "When".into(),
            ScopedRecord {
                identity: "when.scxml".into(),
                name: "when".into(),
                fields: vec![
                    ("year".into(), ty("uint16")),
                    ("month".into(), ty("uint8")),
                    ("day".into(), ty("uint8")),
                ],
                scope: TypeScope::default(),
            },
        );
        let fields = [
            ("title".to_string(), ty("string[4]")),
            ("when".to_string(), ty("record:When")),
            ("milestone".to_string(), ty("bool")),
        ];
        let schema = PayloadSchema::of("event", 1, &fields, &scope, None).expect("a grammar");
        assert_eq!(
            schema.json_schema,
            "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\
             \"$id\":\"urn:sce:schema:event:1\",\
             \"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts \
             characters\",\
             \"type\":\"object\",\
             \"properties\":{\"title\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\
             \"when\":{\"$ref\":\"#/$defs/when\"},\"milestone\":{\"type\":\"boolean\"}},\
             \"required\":[\"title\",\"when\",\"milestone\"],\
             \"additionalProperties\":false,\
             \"$defs\":{\"when\":{\"type\":\"object\",\
             \"properties\":{\"year\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\
             \"month\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\
             \"day\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\
             \"required\":[\"year\",\"month\",\"day\"],\"additionalProperties\":false}}}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&schema.json_schema).unwrap();
        assert_eq!(wire_violations(&parsed), Vec::<String>::new());
        let lines: Vec<&str> = schema.gbnf.lines().collect();
        assert_eq!(
            lines[..2],
            [
                "root ::= \"{\\\"title\\\":\" \"\\\"\" t4 \"\\\"\" \",\\\"when\\\":\" record-1 \
                 \",\\\"milestone\\\":\" ( \"true\" | \"false\" ) \"}\"",
                "record-1 ::= \"{\\\"year\\\":\" uint16 \",\\\"month\\\":\" uint8 \
                 \",\\\"day\\\":\" uint8 \"}\"",
            ]
        );
        // The grammar admits the declared order alone; the JSON Schema, for
        // which an object's order is insignificant, admits both.
        let declared =
            "{\"title\":\"a\",\"when\":{\"year\":2026,\"month\":10,\"day\":21},\"milestone\":false}";
        let ascending =
            "{\"milestone\":false,\"title\":\"a\",\"when\":{\"day\":21,\"month\":10,\"year\":2026}}";
        let grammar = gbnf::Grammar::read(&schema.gbnf);
        assert!(grammar.admits(declared), "{declared}");
        assert!(!grammar.admits(ascending), "{ascending}");
        let json = compiled(&schema.json_schema);
        assert!(json_admits(&json, declared) && json_admits(&json, ascending));
    }

    #[test]
    fn a_schema_whose_type_has_no_grammar_has_no_payload_schema() {
        let mut scope = TypeScope::default();
        scope.enums.insert(
            "Code".into(),
            ScopedEnum {
                name: "code".into(),
                variants: vec![ScopedVariant {
                    name: "ok".into(),
                    value: 0,
                }],
                underlying: SceType::Uint8,
                strict: false,
            },
        );
        assert_eq!(
            PayloadSchema::of("open", 1, &[("code".into(), ty("enum:Code"))], &scope, None),
            Err(GrammarError::OpenEnum {
                alias: "Code".into()
            })
        );
    }

    // ── The wire surface ────────────────────────────────────────────────

    /// The directories of the fixtures whose event schemas the sweeps below
    /// read: the union fixtures hold a union in a field, in a list and in an
    /// optional (docs/adr/0092, section 1.1, item 3).
    const FIXTURE_DIRS: [&str; 3] = [
        "tests/fixtures/static_datamodel",
        "tests/fixtures/payload_schema",
        "tests/fixtures/union",
    ];

    /// Every event schema of the conformance fixtures that has a grammar,
    /// with the payload schema the build writes for it, and the names of
    /// those that have none, each refused for an open enum. A fixture is
    /// named by its path from the crate's directory.
    fn fixture_schemas() -> (Vec<(String, PayloadSchema)>, Vec<String>) {
        let mut written = Vec::new();
        let mut refused = Vec::new();
        for dir in FIXTURE_DIRS {
            let mut paths: Vec<PathBuf> = std::fs::read_dir(manifest_dir().join(dir))
                .expect("a fixture directory")
                .map(|entry| entry.expect("an entry").path())
                .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
                .collect();
            paths.sort();
            for path in paths {
                let file = path.file_name().unwrap().to_string_lossy().into_owned();
                let name = format!("{dir}/{file}");
                match of_document(&path) {
                    None => {}
                    Some(Ok(schema)) => written.push((name, schema)),
                    Some(Err(GrammarError::OpenEnum { .. })) => refused.push(name),
                    Some(Err(other)) => panic!("{name} has no payload schema: {other}"),
                }
            }
        }
        assert!(
            written.len() >= 40,
            "only {} fixture schemas were read; a sweep that reads nothing checks nothing",
            written.len()
        );
        (written, refused)
    }

    #[test]
    fn every_schema_the_product_writes_validates_against_the_wire_schema() {
        let (written, _) = fixture_schemas();
        for (name, schema) in &written {
            let parsed: serde_json::Value =
                serde_json::from_str(&schema.json_schema).expect("a payload schema is JSON");
            assert_eq!(wire_violations(&parsed), Vec::<String>::new(), "{name}");
            // And against the official 2020-12 meta-schema, by compiling it.
            compiled(&schema.json_schema);
            // Compact and ASCII: the text a writer that keeps each object's
            // members in order writes back, every character past ASCII escaped.
            let read = parse(&schema.json_schema).expect("a payload schema is JSON");
            assert_eq!(schema.json_schema, ascii(&write(&read)), "{name}");
            assert!(
                schema.json_schema.is_ascii() && schema.gbnf.is_ascii(),
                "{name}"
            );
            // Its keywords in the one order at every depth, each record's
            // `properties` in the order of its `required`.
            keywords_in_order(&read, name);
        }
    }

    /// The keywords a payload schema is written in, in the one order every
    /// schema object writes them in (docs/adr/0087, section 1.1, item 4;
    /// `const` and `oneOf`, docs/adr/0092, section 3.1, item 2).
    const KEYWORD_ORDER: [&str; 22] = [
        "$schema",
        "$id",
        "x-sce-source-hash",
        "description",
        "$ref",
        "type",
        "const",
        "enum",
        "anyOf",
        "oneOf",
        "minimum",
        "maximum",
        "maxLength",
        "x-sce-max-bytes",
        "pattern",
        "items",
        "minItems",
        "maxItems",
        "properties",
        "required",
        "additionalProperties",
        "$defs",
    ];

    /// `text` with every character past ASCII written as JSON's `\u` escape,
    /// a UTF-16 pair past U+FFFF, its hex in lowercase.
    fn ascii(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            if c.is_ascii() {
                out.push(c);
            } else {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units).iter() {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
        out
    }

    /// Asserts that `schema`, a schema object read with its members in the
    /// order they are written, writes its keywords in [`KEYWORD_ORDER`] at
    /// every depth, and the members of each `properties` in the order of the
    /// `required` beside it. `at` names the place, for the message.
    fn keywords_in_order(schema: &Value, at: &str) {
        let Value::Object(members) = schema else {
            panic!("{at} is a schema object: {schema:?}");
        };
        let ranks: Vec<usize> = members
            .iter()
            .map(|(keyword, _)| {
                KEYWORD_ORDER
                    .iter()
                    .position(|k| k == keyword)
                    .unwrap_or_else(|| panic!("{at}: `{keyword}` is a keyword of the subset"))
            })
            .collect();
        assert!(
            ranks.windows(2).all(|pair| pair[0] < pair[1]),
            "{at}: the keywords stand in the one order: {members:?}"
        );
        for (keyword, value) in members {
            let inner = format!("{at}/{keyword}");
            match (keyword.as_str(), value) {
                ("properties" | "$defs", Value::Object(named)) => {
                    for (name, schema) in named {
                        keywords_in_order(schema, &format!("{inner}/{name}"));
                    }
                }
                ("items", schema) => keywords_in_order(schema, &inner),
                ("anyOf" | "oneOf", Value::Array(schemas)) => {
                    for schema in schemas {
                        keywords_in_order(schema, &inner);
                    }
                }
                _ => {}
            }
        }
        if let (Some(Value::Object(properties)), Some(Value::Array(required))) =
            (schema.member("properties"), schema.member("required"))
        {
            let keys: Vec<&str> = properties.iter().map(|(key, _)| key.as_str()).collect();
            let named: Vec<&str> = required
                .iter()
                .map(|key| match key {
                    Value::Text(key) => key.as_str(),
                    other => panic!("{at}: a required key is a text, not {other:?}"),
                })
                .collect();
            assert_eq!(keys, named, "{at}: properties in the order of required");
        }
    }

    #[test]
    fn the_wire_schema_rejects_what_the_producer_does_not_write() {
        let valid: serde_json::Value =
            serde_json::from_str(&answer_slot().json_schema).expect("JSON");
        assert_eq!(wire_violations(&valid), Vec::<String>::new());
        let changed = |change: &dyn Fn(&mut serde_json::Value)| {
            let mut instance = valid.clone();
            change(&mut instance);
            instance
        };
        let changes: [(&str, serde_json::Value); 11] = [
            (
                "a tag outside a union's case",
                changed(&|s| s["properties"]["kind"] = serde_json::json!({"const": "meeting"})),
            ),
            (
                "a record's root that is also a union's",
                changed(&|s| {
                    let case = serde_json::json!({
                        "type": "object",
                        "properties": {"kind": {"const": "meeting"}},
                        "required": ["kind"],
                        "additionalProperties": false
                    });
                    s["oneOf"] = serde_json::json!([case]);
                }),
            ),
            (
                "a schema without its id",
                changed(&|s| {
                    s.as_object_mut().unwrap().remove("$id");
                }),
            ),
            (
                "an id that is not a schema's",
                changed(&|s| s["$id"] = "https://example.test/answer_slot".into()),
            ),
            (
                "another dialect",
                changed(&|s| s["$schema"] = "http://json-schema.org/draft-07/schema#".into()),
            ),
            (
                "a record that admits a key it does not declare",
                changed(&|s| s["additionalProperties"] = true.into()),
            ),
            (
                "a text that does not state its bound in bytes",
                changed(&|s| {
                    s["properties"]["title"]
                        .as_object_mut()
                        .unwrap()
                        .remove("x-sce-max-bytes");
                }),
            ),
            (
                "a keyword outside the subset",
                changed(&|s| s["properties"]["title"]["minLength"] = 1.into()),
            ),
            (
                "an integer with no ends",
                changed(&|s| s["properties"]["slot"] = serde_json::json!({"type": "integer"})),
            ),
            (
                "a source hash that is no sha256",
                changed(&|s| s["x-sce-source-hash"] = "abc".into()),
            ),
            (
                "a reference outside the definitions",
                changed(&|s| {
                    s["properties"]["kind"] = serde_json::json!({"$ref": "other.json#/kind"})
                }),
            ),
        ];
        for (what, instance) in changes {
            assert!(
                !wire_violations(&instance).is_empty(),
                "{what} is refused: {instance}"
            );
        }
        // A union document's schema, valid as written, and each change of one
        // thing of it that the producer never writes.
        let valid: serde_json::Value =
            serde_json::from_str(&union_shape().json_schema).expect("JSON");
        assert_eq!(wire_violations(&valid), Vec::<String>::new());
        let changed = |change: &dyn Fn(&mut serde_json::Value)| {
            let mut instance = valid.clone();
            change(&mut instance);
            instance
        };
        let changes: [(&str, serde_json::Value); 6] = [
            (
                "a tag that is no text",
                changed(&|s| s["oneOf"][1]["properties"]["kind"] = serde_json::json!({"const": 1})),
            ),
            (
                "a union's root beside a record's keyword",
                changed(&|s| s["type"] = "object".into()),
            ),
            (
                "a root that is neither a record nor a union",
                changed(&|s| {
                    s.as_object_mut().unwrap().remove("oneOf");
                }),
            ),
            (
                "a union of no case",
                changed(&|s| s["oneOf"] = serde_json::json!([])),
            ),
            (
                "a case that admits a key it does not declare",
                changed(&|s| s["oneOf"][0]["additionalProperties"] = true.into()),
            ),
            (
                "a case of no key",
                changed(&|s| {
                    s["oneOf"][0]["properties"] = serde_json::json!({});
                    s["oneOf"][0]["required"] = serde_json::json!([]);
                }),
            ),
        ];
        for (what, instance) in changes {
            assert!(
                !wire_violations(&instance).is_empty(),
                "{what} is refused: {instance}"
            );
        }
    }

    // ── A union document (docs/adr/0092, section 3.1) ───────────────────

    fn union_fixture(file: &str) -> PathBuf {
        manifest_dir().join("tests/fixtures/union").join(file)
    }

    /// The payload schema of the union document at `path`, as `sce-codegen
    /// payload-schema` reads it.
    fn of_union_document(path: &Path) -> Result<PayloadSchema, GrammarError> {
        let content = std::fs::read_to_string(path).expect("the fixture reads");
        let label = path.to_string_lossy().into_owned();
        let parsed = crate::forge::parser::parse_forge_with_imports(
            &content,
            crate::DocumentLabel::for_input_path(&label),
        )
        .expect("the union parses")
        .expect("a forge document");
        let ForgeDocument::Union(model) = &parsed.document else {
            panic!("a union document: {}", path.display());
        };
        PayloadSchema::of_union_document(model, &parsed.imports, path, Some(HASH))
    }

    /// The shape union of the fixtures: a dot of no field, a circle of a
    /// radius and a rect of a width and a height, its tag under `kind`.
    fn union_shape() -> PayloadSchema {
        of_union_document(&union_fixture("union_shape.scxml")).expect("a grammar")
    }

    /// Every `oneOf` of `schema`, at any depth.
    fn every_one_of(schema: &serde_json::Value) -> Vec<&Vec<serde_json::Value>> {
        let mut found = Vec::new();
        match schema {
            serde_json::Value::Object(members) => {
                for (keyword, value) in members {
                    if let ("oneOf", serde_json::Value::Array(cases)) = (keyword.as_str(), value) {
                        found.push(cases);
                    }
                    found.extend(every_one_of(value));
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    found.extend(every_one_of(item));
                }
            }
            _ => {}
        }
        found
    }

    #[test]
    fn a_union_document_states_one_object_per_case_its_tag_first() {
        let schema = union_shape();
        assert_eq!(schema.id, "urn:sce:schema:union_shape:1");
        // The cases in the tag enum's order (dot, circle, rect), not the
        // document's (rect, dot, circle); the root is the `oneOf`, no record.
        assert_eq!(
            schema.json_schema,
            format!(
                concat!(
                    r#"{{"$schema":"https://json-schema.org/draft/2020-12/schema","#,
                    r#""$id":"urn:sce:schema:union_shape:1","x-sce-source-hash":"{}","#,
                    r#""description":"x-sce-max-bytes bounds a text in UTF-8 bytes; "#,
                    r#"maxLength counts characters","oneOf":["#,
                    r#"{{"type":"object","properties":{{"kind":{{"const":"dot"}}}},"#,
                    r#""required":["kind"],"additionalProperties":false}},"#,
                    r#"{{"type":"object","properties":{{"kind":{{"const":"circle"}},"#,
                    r#""radius":{{"type":"integer","minimum":0,"maximum":65535}}}},"#,
                    r#""required":["kind","radius"],"additionalProperties":false}},"#,
                    r#"{{"type":"object","properties":{{"kind":{{"const":"rect"}},"#,
                    r#""width":{{"type":"integer","minimum":0,"maximum":65535}},"#,
                    r#""height":{{"type":"integer","minimum":0,"maximum":65535}}}},"#,
                    r#""required":["kind","width","height"],"additionalProperties":false}}"#,
                    r#"]}}"#,
                ),
                HASH
            )
        );
        let lines: Vec<&str> = schema.gbnf.lines().collect();
        assert_eq!(
            lines,
            [
                concat!(
                    r#"root ::= "{\"kind\":\"dot\"}""#,
                    r#" | "{\"kind\":\"circle\",\"radius\":" uint16 "}""#,
                    r#" | "{\"kind\":\"rect\",\"width\":" uint16 ",\"height\":" uint16 "}""#,
                ),
                concat!(
                    r#"uint16 ::= "0" | [1-9] [0-9]{0,3} | [1-5] [0-9]{4} | "6" [0-4] [0-9]{3} | "#,
                    r#""65" [0-4] [0-9]{2} | "655" [0-2] [0-9] | "6553" [0-5]"#,
                ),
            ]
        );
        // A schema of the wire surface, of 2020-12, compact and ASCII, its
        // keywords in the one order.
        let parsed: serde_json::Value = serde_json::from_str(&schema.json_schema).unwrap();
        assert_eq!(wire_violations(&parsed), Vec::<String>::new());
        compiled(&schema.json_schema);
        let read = parse(&schema.json_schema).expect("a payload schema is JSON");
        assert_eq!(schema.json_schema, ascii(&write(&read)));
        assert!(schema.json_schema.is_ascii() && schema.gbnf.is_ascii());
        keywords_in_order(&read, "union_shape");
        // A run that writes no headed file states no source hash, and the
        // grammar carries none.
        let path = union_fixture("union_shape.scxml");
        let content = std::fs::read_to_string(&path).unwrap();
        let label = path.to_string_lossy().into_owned();
        let parsed = crate::forge::parser::parse_forge_with_imports(
            &content,
            crate::DocumentLabel::for_input_path(&label),
        )
        .unwrap()
        .unwrap();
        let ForgeDocument::Union(model) = &parsed.document else {
            unreachable!("a union document");
        };
        let unhashed = text_of_union(
            model,
            &parsed.imports,
            &path,
            PayloadFormat::JsonSchema,
            None,
        )
        .unwrap();
        assert!(!unhashed.contains("x-sce-source-hash"), "{unhashed}");
        assert_eq!(
            text_of_union(model, &parsed.imports, &path, PayloadFormat::Gbnf, None).as_ref(),
            Ok(&schema.gbnf)
        );
        assert_eq!(
            text_of_union(
                model,
                &parsed.imports,
                &path,
                PayloadFormat::JsonSchema,
                Some(HASH)
            )
            .as_ref(),
            Ok(&schema.json_schema)
        );
    }

    /// docs/adr/0092, section 3.1, item 2: llguidance accepts a `oneOf` only
    /// where it proves every two cases disjoint, which two objects are when a
    /// key both require holds different string `const` literals, and a `$ref`
    /// is never proven so. Every `oneOf` the build writes, at the root of a
    /// union document and in the `$defs` of a schema that holds a union,
    /// requires its tag in each case, holds it as a string `const` first, a
    /// literal of its own, and writes the case's fields inline.
    #[test]
    fn every_case_requires_its_tag_and_holds_it_as_a_string_const() {
        let event = of_document(&union_fixture("event_shapes.scxml"))
            .expect("an event schema")
            .expect("a grammar");
        let mut seen = 0usize;
        for text in [&union_shape().json_schema, &event.json_schema] {
            // The tag first in `required`, and `properties` in the order of
            // `required` (read by a reader that keeps an object's order).
            keywords_in_order(&parse(text).expect("JSON"), "the schema");
            let schema: serde_json::Value = serde_json::from_str(text).unwrap();
            let unions = every_one_of(&schema);
            assert_eq!(unions.len(), 1, "one union, defined once: {text}");
            for cases in unions {
                let mut literals = std::collections::BTreeSet::new();
                for case in cases {
                    let required = case["required"].as_array().expect("required keys");
                    assert_eq!(required.first(), Some(&serde_json::json!("kind")), "{case}");
                    let properties = case["properties"].as_object().expect("properties");
                    let tag = &properties["kind"];
                    assert_eq!(tag.as_object().map(|t| t.len()), Some(1), "{case}");
                    let literal = tag["const"].as_str().expect("a string const");
                    assert!(literals.insert(literal.to_string()), "{literal} twice");
                    assert_eq!(case.get("$ref"), None, "{case}: written inline");
                    assert_eq!(case["additionalProperties"], false, "{case}");
                    seen += 1;
                }
            }
        }
        assert_eq!(seen, 6, "three cases in each of the two schemas");
    }

    /// Both forms hold a value of a union to exactly one case, chosen by its
    /// tag: each case's smallest and largest value is admitted, a field of
    /// another case, a field of its case missing and a tag no variant names
    /// are refused. The grammar alone refuses the tag after a field and the
    /// keys out of their declared order, which the JSON Schema, for which an
    /// object's order is insignificant, admits, as the reader will.
    #[test]
    fn the_recognizer_holds_a_union_to_one_case_by_its_tag() {
        let schema = union_shape();
        let grammar = gbnf::Grammar::read(&schema.gbnf);
        let json = compiled(&schema.json_schema);
        for text in [
            r#"{"kind":"dot"}"#,
            r#"{"kind":"circle","radius":0}"#,
            r#"{"kind":"circle","radius":65535}"#,
            r#"{"kind":"rect","width":0,"height":0}"#,
            r#"{"kind":"rect","width":65535,"height":65535}"#,
        ] {
            assert!(grammar.admits(text), "the grammar admits {text}");
            assert!(json_admits(&json, text), "the schema admits {text}");
        }
        for text in [
            r#"{"kind":"dot","radius":1}"#,
            r#"{"kind":"circle","width":1,"height":2}"#,
            r#"{"kind":"rect","width":1}"#,
            r#"{"kind":"circle","radius":65536}"#,
            r#"{"kind":"square"}"#,
            r#"{"kind":1}"#,
            r#"{"kind":null}"#,
            r#"{"radius":1}"#,
            r#"{}"#,
        ] {
            assert!(!grammar.admits(text), "the grammar refuses {text}");
            assert!(!json_admits(&json, text), "the schema refuses {text}");
        }
        for text in [
            r#"{"radius":1,"kind":"circle"}"#,
            r#"{"kind":"rect","height":2,"width":1}"#,
        ] {
            assert!(!grammar.admits(text), "the grammar refuses {text}");
            assert!(json_admits(&json, text), "the schema admits {text}");
        }
        // Held in a field, a list of at least one and an optional, by a
        // definition the three share.
        let event = of_document(&union_fixture("event_shapes.scxml"))
            .expect("an event schema")
            .expect("a grammar");
        let grammar = gbnf::Grammar::read(&event.gbnf);
        let json = compiled(&event.json_schema);
        for (text, admitted) in [
            (
                r#"{"one":{"kind":"dot"},"many":[{"kind":"circle","radius":3}],"maybe":null}"#,
                true,
            ),
            (
                r#"{"one":{"kind":"rect","width":1,"height":2},"many":[{"kind":"dot"},{"kind":"dot"},{"kind":"dot"}],"maybe":{"kind":"circle","radius":9}}"#,
                true,
            ),
            (r#"{"one":{"kind":"dot"},"many":[],"maybe":null}"#, false),
            (
                r#"{"one":{"kind":"dot"},"many":[{"kind":"dot"},{"kind":"dot"},{"kind":"dot"},{"kind":"dot"}],"maybe":null}"#,
                false,
            ),
            (
                r#"{"one":{"kind":"dot"},"many":[{"kind":"dot","radius":3}],"maybe":null}"#,
                false,
            ),
            (
                r#"{"one":{"kind":"dot"},"many":[{"kind":"dot"}],"maybe":{"kind":"rect"}}"#,
                false,
            ),
        ] {
            assert_eq!(grammar.admits(text), admitted, "the grammar: {text}");
            assert_eq!(json_admits(&json, text), admitted, "the schema: {text}");
        }
    }

    /// A union document whose tag is a value of an open enum has no grammar,
    /// refused as an enum field's is; the import refuses it first, in its own
    /// words (`validation/attribute-rule-violated`).
    #[test]
    fn a_union_of_an_open_tag_has_no_payload_schema() {
        assert_eq!(
            of_union_document(&union_fixture("union_shape_open_tag.scxml")),
            Err(GrammarError::OpenEnum {
                alias: "ShapeKind".into()
            })
        );
    }

    // ── Values written by the runtime's writer ──────────────────────────

    /// How the largest text of a bound is made: of control characters at
    /// their longest escape, or of characters of every width, widest first.
    #[derive(Clone, Copy)]
    enum Fill {
        Controls,
        Wide,
    }

    /// A text of exactly `bytes` UTF-8 bytes.
    fn full_text(bytes: u32, fill: Fill) -> String {
        match fill {
            Fill::Controls => "\u{1}".repeat(bytes as usize),
            Fill::Wide => {
                let mut text = "\u{1F600}".repeat((bytes / 4) as usize);
                text.push_str(match bytes % 4 {
                    0 => "",
                    1 => "a",
                    2 => "\u{e9}",
                    _ => "\u{20ac}",
                });
                text
            }
        }
    }

    /// The fields of the record `alias` names, in the order its document
    /// declares them, the order both spellings state its keys in, and the
    /// scope they are read in.
    fn record_of<'e>(
        alias: &str,
        env: &'e dyn TypeEnv,
    ) -> (Vec<(String, ValueType)>, &'e dyn TypeEnv) {
        let fields = env.record_fields(alias).expect("a record of the fixture");
        (fields, env.record_scope(alias).unwrap_or(env))
    }

    fn union_of<'e>(alias: &str, env: &'e dyn TypeEnv) -> &'e ScopedUnion {
        env.union_type(alias).expect("a union of the fixture")
    }

    /// The fields of the record `case` of `union` holds, none for an empty
    /// case, and the scope they are read in.
    fn case_of<'e>(
        union: &'e ScopedUnion,
        case: &ScopedCase,
    ) -> (Vec<(String, ValueType)>, &'e dyn TypeEnv) {
        let scope: &'e dyn TypeEnv = &union.scope;
        match &case.record {
            Some(alias) => record_of(alias, scope),
            None => (Vec::new(), scope),
        }
    }

    /// The value of `case` of `union` whose fields are `members`: the tag
    /// first, then the fields in the order the case's record declares them,
    /// as both forms state them (docs/adr/0092, section 3.1, item 1).
    fn case_value(union: &ScopedUnion, case: &ScopedCase, members: Members) -> Value {
        let mut object = vec![(union.tag.clone(), Value::Text(case.variant.name.clone()))];
        object.extend(members);
        Value::Object(object)
    }

    /// The case of `union` of the most fields, the first of them in its tag
    /// enum's order: the case a largest value of the union is in.
    fn widest_case(union: &ScopedUnion) -> &ScopedCase {
        let mut widest = union.cases.first().expect("a union has a case");
        for case in &union.cases {
            if case_of(union, case).0.len() > case_of(union, widest).0.len() {
                widest = case;
            }
        }
        widest
    }

    fn names_of<'e>(t: &SceType, env: &'e dyn TypeEnv) -> &'e ScopedEnum {
        let SceType::Enum(reference) = t else {
            unreachable!("an enum")
        };
        env.enum_type(&reference.alias)
            .expect("an enum of the fixture")
    }

    /// The value of `ty` whose every number is at its longest spelling, every
    /// text and byte string at its bound, every list full.
    fn largest(ty: &ValueType, env: &dyn TypeEnv, fill: Fill) -> Value {
        match ty {
            ValueType::Scalar(t) => match t {
                SceType::Uint64 => Value::Text(u64::MAX.to_string()),
                SceType::Int64 => Value::Text(i64::MIN.to_string()),
                SceType::Float32 => Value::Number(number_text(-f64::from(f32::MAX))),
                SceType::Float64 => Value::Number(number_text(f64::MIN)),
                SceType::Bool => Value::Bool(false),
                SceType::Enum(_) => {
                    let names = &names_of(t, env).variants;
                    let longest = names.iter().map(|v| &v.name).max_by_key(|n| n.len());
                    Value::Text(longest.expect("a variant").clone())
                }
                _ => {
                    let (lo, hi) = t.int_value_range().expect("an integer");
                    let longest = if lo < 0 { lo } else { hi };
                    Value::Number(longest.to_string())
                }
            },
            ValueType::Text { max } => Value::Text(full_text(max.max, fill)),
            // Byte zero, then the widest character of a byte string.
            ValueType::Bytes { max } => Value::Text(
                std::iter::once('\0')
                    .chain(std::iter::repeat('\u{ff}'))
                    .take(max.max as usize)
                    .collect(),
            ),
            ValueType::Optional { elem } => largest(elem, env, fill),
            ValueType::Record { alias } => {
                let (fields, scope) = record_of(alias, env);
                Value::Object(
                    fields
                        .iter()
                        .map(|(id, field)| (id.clone(), largest(field, scope, fill)))
                        .collect(),
                )
            }
            ValueType::List { elem, max } => {
                Value::Array(vec![largest(elem, env, fill); max.max as usize])
            }
            // Its case of the most fields, each at its largest.
            ValueType::Union { alias } => {
                let union = union_of(alias, env);
                let case = widest_case(union);
                let (fields, scope) = case_of(union, case);
                let members = fields
                    .iter()
                    .map(|(id, field)| (id.clone(), largest(field, scope, fill)))
                    .collect();
                case_value(union, case, members)
            }
        }
    }

    /// The value of `ty` that holds least: a list as many of its element's
    /// smallest as its lower count needs (docs/adr/0086, section 15).
    fn smallest(ty: &ValueType, env: &dyn TypeEnv) -> Value {
        match ty {
            ValueType::Scalar(t) => match t {
                SceType::Uint64 | SceType::Int64 => Value::Text("0".into()),
                SceType::Bool => Value::Bool(true),
                SceType::Enum(_) => Value::Text(names_of(t, env).variants[0].name.clone()),
                _ => Value::Number("0".into()),
            },
            ValueType::Text { .. } | ValueType::Bytes { .. } => Value::Text(String::new()),
            ValueType::Optional { .. } => Value::Null,
            ValueType::Record { alias } => {
                let (fields, scope) = record_of(alias, env);
                Value::Object(
                    fields
                        .iter()
                        .map(|(id, field)| (id.clone(), smallest(field, scope)))
                        .collect(),
                )
            }
            ValueType::List { elem, max } => {
                Value::Array(vec![smallest(elem, env); max.min as usize])
            }
            // Its first case in its tag enum's order, each field at its
            // smallest.
            ValueType::Union { alias } => {
                let union = union_of(alias, env);
                let case = union.cases.first().expect("a union has a case");
                let (fields, scope) = case_of(union, case);
                let members = fields
                    .iter()
                    .map(|(id, field)| (id.clone(), smallest(field, scope)))
                    .collect();
                case_value(union, case, members)
            }
        }
    }

    /// Whether a value one past a bound is refused by the JSON Schema too, or
    /// admitted by it: a text past its bound in bytes within its bound in
    /// characters, which `maxLength` cannot see.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum JsonSchemaSees {
        Refused,
        Admitted,
    }

    /// The largest value of `ty` with one place past its bound, for each place
    /// a bound is written, and whether the JSON Schema refuses it; and, for a
    /// list that states a lower count, the list one short of it, which both
    /// forms refuse (docs/adr/0086, section 15).
    fn past_each_bound(ty: &ValueType, env: &dyn TypeEnv) -> Vec<(Value, JsonSchemaSees)> {
        match ty {
            ValueType::Scalar(_) => Vec::new(),
            ValueType::Text { max } => {
                let ascii = "a".repeat(max.max as usize - 1);
                vec![
                    (Value::Text(format!("{ascii}aa")), JsonSchemaSees::Refused),
                    (
                        Value::Text(format!("{ascii}\u{e9}")),
                        JsonSchemaSees::Admitted,
                    ),
                ]
            }
            ValueType::Bytes { max } => vec![
                (
                    Value::Text("\u{ff}".repeat(max.max as usize + 1)),
                    JsonSchemaSees::Refused,
                ),
                (Value::Text("\u{100}".into()), JsonSchemaSees::Refused),
            ],
            ValueType::Optional { elem } => past_each_bound(elem, env),
            ValueType::Record { alias } => {
                let (fields, scope) = record_of(alias, env);
                past_in_fields(&fields, scope)
                    .into_iter()
                    .map(|(members, sees)| (Value::Object(members), sees))
                    .collect()
            }
            // Each case's, its tag first.
            ValueType::Union { alias } => {
                let union = union_of(alias, env);
                let mut out = Vec::new();
                for case in &union.cases {
                    let (fields, scope) = case_of(union, case);
                    for (members, sees) in past_in_fields(&fields, scope) {
                        out.push((case_value(union, case, members), sees));
                    }
                }
                out
            }
            ValueType::List { elem, max } => {
                let full = max.max as usize;
                let mut out = vec![(
                    Value::Array(vec![largest(elem, env, Fill::Wide); full + 1]),
                    JsonSchemaSees::Refused,
                )];
                if max.min > 0 {
                    out.push((
                        Value::Array(vec![largest(elem, env, Fill::Wide); max.min as usize - 1]),
                        JsonSchemaSees::Refused,
                    ));
                }
                for (past, sees) in past_each_bound(elem, env) {
                    let mut items = vec![largest(elem, env, Fill::Wide); full];
                    items[0] = past;
                    out.push((Value::Array(items), sees));
                }
                out
            }
        }
    }

    /// The members of an object, each key with its value, in order.
    type Members = Vec<(String, Value)>;

    /// The members of an object of `fields`, read in `scope`, with one field
    /// past a bound and every other at its largest, for each place a bound is
    /// written in each field, and whether the JSON Schema refuses it.
    fn past_in_fields(
        fields: &[(String, ValueType)],
        scope: &dyn TypeEnv,
    ) -> Vec<(Members, JsonSchemaSees)> {
        let mut out = Vec::new();
        for (at, (_, field)) in fields.iter().enumerate() {
            for (past, sees) in past_each_bound(field, scope) {
                let members = fields
                    .iter()
                    .enumerate()
                    .map(|(i, (id, other))| {
                        let value = if i == at {
                            past.clone()
                        } else {
                            largest(other, scope, Fill::Wide)
                        };
                        (id.clone(), value)
                    })
                    .collect();
                out.push((members, sees));
            }
        }
        out
    }

    /// The payload of `schema` as the record of its own fields, in the scope
    /// its document's imports name.
    struct Payload<'a> {
        fields: Vec<(String, ValueType)>,
        scope: &'a TypeScope,
    }

    impl TypeEnv for Payload<'_> {
        fn record_fields(&self, alias: &str) -> Option<Vec<(String, ValueType)>> {
            if alias.is_empty() {
                Some(self.fields.clone())
            } else {
                self.scope.record_fields(alias)
            }
        }
        fn enum_type(&self, alias: &str) -> Option<&ScopedEnum> {
            self.scope.enum_type(alias)
        }
        fn record_scope(&self, alias: &str) -> Option<&dyn TypeEnv> {
            if alias.is_empty() {
                Some(self.scope)
            } else {
                self.scope.record_scope(alias)
            }
        }
        fn union_type(&self, alias: &str) -> Option<&ScopedUnion> {
            self.scope.union_type(alias)
        }
    }

    /// Every fixture schema admits, in both forms, its largest values and its
    /// smallest, as the runtime's writer spells them, each record's keys in
    /// the order its document declares them; its GBNF grammar refuses each
    /// value one past a bound, and its JSON Schema each but a text past its
    /// bound in bytes within its bound in characters. Its JSON Schema requires
    /// the payload's keys in the order the document declares them.
    #[test]
    fn a_fixture_schema_holds_the_values_its_type_holds_and_no_other() {
        let (written, _) = fixture_schemas();
        let mut past_cases = 0usize;
        let mut looser = 0usize;
        for (name, schema) in &written {
            let path = manifest_dir().join(name);
            let content = std::fs::read_to_string(&path).unwrap();
            let label = path.to_string_lossy().into_owned();
            let parsed = crate::forge::parser::parse_forge_with_imports(
                &content,
                crate::DocumentLabel::for_input_path(&label),
            )
            .unwrap()
            .unwrap();
            let ForgeDocument::EventSchema(model) = &parsed.document else {
                unreachable!("an event schema");
            };
            let scope =
                crate::forge::import_source::type_scope(&parsed.imports, path.parent().unwrap());
            let payload = Payload {
                fields: model.typed_fields(),
                scope: &scope,
            };
            let declared: Vec<Value> = payload
                .fields
                .iter()
                .map(|(id, _)| Value::Text(id.clone()))
                .collect();
            let read = parse(&schema.json_schema).expect("a payload schema is JSON");
            assert_eq!(
                read.member("required"),
                Some(&Value::Array(declared)),
                "{name}: the keys in the order the document declares them"
            );
            let root = ValueType::Record {
                alias: String::new(),
            };
            let grammar = gbnf::Grammar::read(&schema.gbnf);
            let json = compiled(&schema.json_schema);
            for value in [
                largest(&root, &payload, Fill::Controls),
                largest(&root, &payload, Fill::Wide),
                smallest(&root, &payload),
            ] {
                let text = write(&value);
                assert!(grammar.admits(&text), "{name}: the grammar admits {text}");
                assert!(
                    json_admits(&json, &text),
                    "{name}: the schema admits {text}"
                );
            }
            for (value, sees) in past_each_bound(&root, &payload) {
                let text = write(&value);
                assert!(!grammar.admits(&text), "{name}: the grammar refuses {text}");
                assert_eq!(
                    json_admits(&json, &text),
                    sees == JsonSchemaSees::Admitted,
                    "{name}: the schema sees {text} as {sees:?}"
                );
                past_cases += 1;
                looser += usize::from(sees == JsonSchemaSees::Admitted);
            }
        }
        assert!(
            past_cases > 0 && looser > 0,
            "{past_cases} past, {looser} looser"
        );
    }

    /// The payload schema of one field `x` of `t`, its grammar read by the
    /// test, and its JSON Schema compiled.
    fn one_field(t: &str) -> (gbnf::Grammar, jsonschema::JSONSchema) {
        let schema = PayloadSchema::of(
            "probe",
            1,
            &[("x".into(), ty(t))],
            &TypeScope::default(),
            None,
        )
        .expect("a grammar");
        (
            gbnf::Grammar::read(&schema.gbnf),
            compiled(&schema.json_schema),
        )
    }

    /// A list that states a lower count is held to it by both forms, as to its
    /// bound: the JSON Schema by `minItems`, the grammar by the chain of the
    /// elements it must hold (docs/adr/0086, sections 15.9 and 15.10), so the
    /// empty list is no answer either admits.
    #[test]
    fn a_list_is_held_to_its_lower_count_by_both_forms() {
        let schema = PayloadSchema::of(
            "probe",
            1,
            &[("x".into(), ty("list<uint8>[2..3]"))],
            &TypeScope::default(),
            None,
        )
        .expect("a grammar");
        assert!(
            schema.json_schema.contains("\"minItems\":2,\"maxItems\":3"),
            "{}",
            schema.json_schema
        );
        let (grammar, json) = one_field("list<uint8>[2..3]");
        for (text, admitted) in [
            ("{\"x\":[]}", false),
            ("{\"x\":[1]}", false),
            ("{\"x\":[1,2]}", true),
            ("{\"x\":[1,2,3]}", true),
            ("{\"x\":[1,2,3,4]}", false),
        ] {
            assert_eq!(grammar.admits(text), admitted, "the grammar: {text}");
            assert_eq!(json_admits(&json, text), admitted, "the schema: {text}");
        }
        // A list that states none admits the empty list in both, as before.
        let (grammar, json) = one_field("list<uint8>[3]");
        assert!(grammar.admits("{\"x\":[]}") && json_admits(&json, "{\"x\":[]}"));
    }

    /// Each range rule admits its ends and refuses one past each. A spelling
    /// that is no JSON (`01`), or one JSON Schema reads as the same number (`-0`
    /// and `1.0` are the integers 0 and 1 there), is the grammar's alone to
    /// refuse.
    #[test]
    fn an_integer_rule_holds_its_ends_and_refuses_one_past() {
        for (t, admitted, refused, refused_by_the_grammar) in [
            (
                "uint16",
                &["0", "9", "10", "65529", "65535"][..],
                &["65536", "-1", "\"1\""][..],
                &["01", "-0", "1.0"][..],
            ),
            (
                "uint64",
                &["\"0\"", "\"18446744073709551615\""][..],
                &["\"18446744073709551616\"", "\"-1\"", "\"01\"", "0"][..],
                &[][..],
            ),
            (
                "int64",
                &[
                    "\"-9223372036854775808\"",
                    "\"9223372036854775807\"",
                    "\"0\"",
                ][..],
                &[
                    "\"-9223372036854775809\"",
                    "\"9223372036854775808\"",
                    "\"-0\"",
                    "-1",
                ][..],
                &[][..],
            ),
        ] {
            let (grammar, json) = one_field(t);
            let text = |value: &str| format!("{{\"x\":{value}}}");
            for &value in admitted {
                assert!(
                    grammar.admits(&text(value)),
                    "{t}: the grammar admits {value}"
                );
                assert!(
                    json_admits(&json, &text(value)),
                    "{t}: the schema admits {value}"
                );
            }
            for &value in refused {
                assert!(
                    !grammar.admits(&text(value)),
                    "{t}: the grammar refuses {value}"
                );
                assert!(
                    !json_admits(&json, &text(value)),
                    "{t}: the schema refuses {value}"
                );
            }
            for &value in refused_by_the_grammar {
                assert!(
                    !grammar.admits(&text(value)),
                    "{t}: the grammar refuses {value}"
                );
            }
        }
    }

    #[test]
    fn the_grammar_states_one_spelling_of_each_value() {
        let (grammar, json) = one_field("optional<string[4]>");
        // Whitespace, a key the schema does not declare, and an escape the
        // writer does not write are the reader's to accept, not the grammar's.
        for text in [
            "{\"x\": null}",
            " {\"x\":null}",
            "{\"x\":\"\\/\"}",
            "{\"x\":\"\\u00e9\"}",
            "{\"x\":\"\\u0000\"}",
            "{\"x\":\"\\u000A\"}",
        ] {
            assert!(!grammar.admits(text), "the grammar refuses {text}");
        }
        for text in [
            "{\"x\":null}",
            "{\"x\":\"\\n\\u001f\"}",
            "{\"x\":\"\u{1F600}\"}",
        ] {
            assert!(grammar.admits(text), "the grammar admits {text}");
            assert!(json_admits(&json, text), "the schema admits {text}");
        }
        let (grammar, json) = one_field("float32");
        // A single's magnitude is the one bound the grammar does not hold.
        let past = format!("{{\"x\":{}}}", number_text(3.5e38));
        assert!(grammar.admits(&past), "{past}");
        assert!(!json_admits(&json, &past), "{past}");
        for text in ["{\"x\":0}", "{\"x\":-1.5e-7}", "{\"x\":12.25}"] {
            assert!(grammar.admits(text) && json_admits(&json, text), "{text}");
        }
        for text in ["{\"x\":01}", "{\"x\":.5}", "{\"x\":1.}", "{\"x\":+1}"] {
            assert!(!grammar.admits(text), "{text}");
        }
    }

    #[test]
    fn the_recognizer_reads_the_grammar_format() {
        let grammar = gbnf::Grammar::read(
            "root ::= ( \"a\" | [b-c] )+ \"-\"{2} x? y* [^z]{1,2}\n\
             # a comment\n\
             x ::= \"\\x41\\u00e9\\U0001F600\"\n\
             y ::= \"\" | [\\-\\]]\n",
        );
        for text in ["a--q", "abc--A\u{e9}\u{1F600}-]qq"] {
            assert!(grammar.admits(text), "{text}");
        }
        for text in [
            "",
            "--q",
            "a-q",
            "a--",
            "a--qqq",
            "a--z",
            "a--A\u{e9}\u{1F600}z",
        ] {
            assert!(!grammar.admits(text), "{text}");
        }
    }

    /// A GBNF grammar read by the test and recognized by an Earley recognizer
    /// the test holds, not the generator: the rules of llama.cpp's
    /// `grammars/README.md` (literals with their escapes, classes and their
    /// ranges, groups, alternatives, `?`, `*`, `+` and `{m,n}`), each
    /// repetition rewritten into rules as that reader rewrites it.
    mod gbnf {
        use std::collections::{HashMap, HashSet};

        #[derive(Clone, Debug, PartialEq, Eq)]
        enum Symbol {
            Rule(usize),
            Chars {
                ranges: Vec<(u32, u32)>,
                negated: bool,
            },
        }

        pub struct Grammar {
            rules: Vec<(usize, Vec<Symbol>)>,
            by_lhs: Vec<Vec<usize>>,
            nullable: Vec<bool>,
            root: usize,
        }

        struct Reader {
            text: Vec<char>,
            at: usize,
            names: HashMap<String, usize>,
            defined: HashSet<usize>,
            symbols: usize,
            rules: Vec<(usize, Vec<Symbol>)>,
        }

        fn is_word(c: char) -> bool {
            c.is_ascii_alphanumeric() || c == '-'
        }

        impl Reader {
            fn peek(&self) -> Option<char> {
                self.text.get(self.at).copied()
            }

            fn expect(&mut self, c: char) {
                assert_eq!(self.peek(), Some(c), "at {}", self.at);
                self.at += 1;
            }

            fn space(&mut self, newlines: bool) {
                while let Some(c) = self.peek() {
                    match c {
                        ' ' | '\t' => self.at += 1,
                        '\r' | '\n' if newlines => self.at += 1,
                        '#' => {
                            while self.peek().is_some_and(|c| c != '\n' && c != '\r') {
                                self.at += 1;
                            }
                        }
                        _ => return,
                    }
                }
            }

            fn fresh(&mut self) -> usize {
                self.symbols += 1;
                self.symbols - 1
            }

            fn named(&mut self, name: &str) -> usize {
                if let Some(&id) = self.names.get(name) {
                    return id;
                }
                let id = self.fresh();
                self.names.insert(name.to_string(), id);
                id
            }

            fn name(&mut self) -> String {
                let start = self.at;
                while self.peek().is_some_and(is_word) {
                    self.at += 1;
                }
                assert!(self.at > start, "a name at {start}");
                self.text[start..self.at].iter().collect()
            }

            fn integer(&mut self) -> u32 {
                let start = self.at;
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.at += 1;
                }
                let digits: String = self.text[start..self.at].iter().collect();
                digits.parse().expect("an integer")
            }

            fn hex(&mut self, digits: usize) -> u32 {
                let text: String = self.text[self.at..self.at + digits].iter().collect();
                self.at += digits;
                u32::from_str_radix(&text, 16).expect("hex digits")
            }

            /// One character of a literal or a class, its escape read.
            fn character(&mut self) -> u32 {
                let c = self.peek().expect("a character");
                self.at += 1;
                if c != '\\' {
                    return u32::from(c);
                }
                let escaped = self.peek().expect("an escape");
                self.at += 1;
                match escaped {
                    'x' => self.hex(2),
                    'u' => self.hex(4),
                    'U' => self.hex(8),
                    't' => u32::from('\t'),
                    'r' => u32::from('\r'),
                    'n' => u32::from('\n'),
                    '\\' | '"' | '[' | ']' | '-' => u32::from(escaped),
                    other => panic!("unknown escape \\{other}"),
                }
            }

            fn rule(&mut self) {
                let name = self.name();
                let lhs = self.named(&name);
                self.space(false);
                for c in "::=".chars() {
                    self.expect(c);
                }
                self.space(true);
                for alternative in self.alternatives(false) {
                    self.rules.push((lhs, alternative));
                }
                self.defined.insert(lhs);
                match self.peek() {
                    None => {}
                    Some('\r' | '\n') => self.at += 1,
                    Some(other) => panic!("a rule ends at a line's end, not {other:?}"),
                }
            }

            fn alternatives(&mut self, nested: bool) -> Vec<Vec<Symbol>> {
                let mut out = vec![self.sequence(nested)];
                while self.peek() == Some('|') {
                    self.at += 1;
                    self.space(true);
                    out.push(self.sequence(nested));
                }
                out
            }

            fn sequence(&mut self, nested: bool) -> Vec<Symbol> {
                let mut sequence = Vec::new();
                let mut last = 0;
                loop {
                    match self.peek() {
                        Some('"') => {
                            self.at += 1;
                            last = sequence.len();
                            while self.peek() != Some('"') {
                                let c = self.character();
                                sequence.push(Symbol::Chars {
                                    ranges: vec![(c, c)],
                                    negated: false,
                                });
                            }
                            self.at += 1;
                        }
                        Some('[') => {
                            self.at += 1;
                            let negated = self.peek() == Some('^');
                            if negated {
                                self.at += 1;
                            }
                            let mut ranges = Vec::new();
                            while self.peek() != Some(']') {
                                let lo = self.character();
                                let range = self.peek() == Some('-')
                                    && self.text.get(self.at + 1) != Some(&']');
                                let hi = if range {
                                    self.at += 1;
                                    self.character()
                                } else {
                                    lo
                                };
                                ranges.push((lo, hi));
                            }
                            self.at += 1;
                            last = sequence.len();
                            sequence.push(Symbol::Chars { ranges, negated });
                        }
                        Some('(') => {
                            self.at += 1;
                            self.space(true);
                            let alternatives = self.alternatives(true);
                            self.expect(')');
                            let group = self.fresh();
                            for alternative in alternatives {
                                self.rules.push((group, alternative));
                            }
                            last = sequence.len();
                            sequence.push(Symbol::Rule(group));
                        }
                        Some(c) if is_word(c) => {
                            let name = self.name();
                            let id = self.named(&name);
                            last = sequence.len();
                            sequence.push(Symbol::Rule(id));
                        }
                        Some('*') => {
                            self.at += 1;
                            self.repeat(&mut sequence, last, 0, None);
                        }
                        Some('+') => {
                            self.at += 1;
                            self.repeat(&mut sequence, last, 1, None);
                        }
                        Some('?') => {
                            self.at += 1;
                            self.repeat(&mut sequence, last, 0, Some(1));
                        }
                        Some('{') => {
                            self.at += 1;
                            self.space(nested);
                            let min = self.integer();
                            self.space(nested);
                            let max = if self.peek() == Some('}') {
                                Some(min)
                            } else {
                                self.expect(',');
                                self.space(nested);
                                if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                                    Some(self.integer())
                                } else {
                                    None
                                }
                            };
                            self.space(nested);
                            self.expect('}');
                            self.repeat(&mut sequence, last, min, max);
                        }
                        _ => break,
                    }
                    self.space(nested);
                }
                sequence
            }

            /// The symbols of `sequence` from `last` on, repeated from `min`
            /// to `max` times (without end for `None`).
            fn repeat(
                &mut self,
                sequence: &mut Vec<Symbol>,
                last: usize,
                min: u32,
                max: Option<u32>,
            ) {
                let body = sequence.split_off(last);
                let item = self.fresh();
                self.rules.push((item, body));
                for _ in 0..min {
                    sequence.push(Symbol::Rule(item));
                }
                match max {
                    None => {
                        let more = self.fresh();
                        self.rules
                            .push((more, vec![Symbol::Rule(item), Symbol::Rule(more)]));
                        self.rules.push((more, Vec::new()));
                        sequence.push(Symbol::Rule(more));
                    }
                    Some(max) => {
                        let mut tail: Option<usize> = None;
                        for _ in min..max {
                            let optional = self.fresh();
                            let mut alternative = vec![Symbol::Rule(item)];
                            alternative.extend(tail.map(Symbol::Rule));
                            self.rules.push((optional, alternative));
                            self.rules.push((optional, Vec::new()));
                            tail = Some(optional);
                        }
                        sequence.extend(tail.map(Symbol::Rule));
                    }
                }
            }
        }

        impl Grammar {
            /// The grammar `text` writes. Panics where it is not GBNF, or
            /// names a rule it does not define.
            pub fn read(text: &str) -> Self {
                let mut reader = Reader {
                    text: text.chars().collect(),
                    at: 0,
                    names: HashMap::new(),
                    defined: HashSet::new(),
                    symbols: 0,
                    rules: Vec::new(),
                };
                let root = reader.named("root");
                loop {
                    reader.space(true);
                    if reader.peek().is_none() {
                        break;
                    }
                    reader.rule();
                }
                for (name, id) in &reader.names {
                    assert!(reader.defined.contains(id), "the rule {name} is defined");
                }
                let mut by_lhs = vec![Vec::new(); reader.symbols];
                for (at, (lhs, _)) in reader.rules.iter().enumerate() {
                    by_lhs[*lhs].push(at);
                }
                let mut nullable = vec![false; reader.symbols];
                loop {
                    let mut changed = false;
                    for (lhs, rhs) in &reader.rules {
                        if !nullable[*lhs]
                            && rhs
                                .iter()
                                .all(|s| matches!(s, Symbol::Rule(r) if nullable[*r]))
                        {
                            nullable[*lhs] = true;
                            changed = true;
                        }
                    }
                    if !changed {
                        break;
                    }
                }
                Self {
                    rules: reader.rules,
                    by_lhs,
                    nullable,
                    root,
                }
            }

            /// Whether `text` is in the grammar's language: Earley's
            /// recognizer, with a nullable rule completed where it is
            /// predicted (Aycock and Horspool, "Practical Earley Parsing",
            /// 2002).
            pub fn admits(&self, text: &str) -> bool {
                type Item = (usize, usize, usize);
                fn add(sets: &mut [Vec<Item>], seen: &mut [HashSet<Item>], at: usize, item: Item) {
                    if seen[at].insert(item) {
                        sets[at].push(item);
                    }
                }
                let input: Vec<u32> = text.chars().map(u32::from).collect();
                let n = input.len();
                let mut sets: Vec<Vec<Item>> = vec![Vec::new(); n + 1];
                let mut seen: Vec<HashSet<Item>> = vec![HashSet::new(); n + 1];
                for &rule in &self.by_lhs[self.root] {
                    add(&mut sets, &mut seen, 0, (rule, 0, 0));
                }
                for i in 0..=n {
                    let mut j = 0;
                    while j < sets[i].len() {
                        let (rule, dot, origin) = sets[i][j];
                        match self.rules[rule].1.get(dot) {
                            None => {
                                let lhs = self.rules[rule].0;
                                let mut k = 0;
                                while k < sets[origin].len() {
                                    let (waiting, at, from) = sets[origin][k];
                                    if self.rules[waiting].1.get(at) == Some(&Symbol::Rule(lhs)) {
                                        add(&mut sets, &mut seen, i, (waiting, at + 1, from));
                                    }
                                    k += 1;
                                }
                            }
                            Some(Symbol::Rule(next)) => {
                                for &predicted in &self.by_lhs[*next] {
                                    add(&mut sets, &mut seen, i, (predicted, 0, i));
                                }
                                if self.nullable[*next] {
                                    add(&mut sets, &mut seen, i, (rule, dot + 1, origin));
                                }
                            }
                            Some(Symbol::Chars { ranges, negated }) => {
                                if let Some(&c) = input.get(i) {
                                    let inside =
                                        ranges.iter().any(|(lo, hi)| (*lo..=*hi).contains(&c));
                                    if inside != *negated {
                                        add(&mut sets, &mut seen, i + 1, (rule, dot + 1, origin));
                                    }
                                }
                            }
                        }
                        j += 1;
                    }
                }
                sets[n].iter().any(|&(rule, dot, origin)| {
                    origin == 0
                        && self.rules[rule].0 == self.root
                        && dot == self.rules[rule].1.len()
                })
            }
        }
    }
}
