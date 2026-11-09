// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The types module of a suite (ADR 0091 C2; docs/SCE_TEST_PROFILE.md §6.3,
//! §9.2 E1, §9.5), held as one model: the template writes the module from it
//! and, serialized, it is the module's structural manifest, so the two cannot
//! say different things.
//!
//! The module holds the eight integer types of the type grammar, each its
//! range under its own name; one definition for each enum, event schema and
//! union the suite reaches, in the order of their names; and, for each type
//! a port of the suite carries, the external functions that encode and
//! decode it, which the export declares and does not implement.

use std::collections::BTreeSet;

use serde::Serialize;

use super::names::{unit_name, written, Declared, Namespace, Origin};
use super::reach::{Reach, Reached};
use crate::forge::error::{ForgeError, GenerateError, Located, SourceLocation};
use crate::forge::model::{EnumModel, EventSchemaModel, ForgeDocument, SceType, UnionModel};
use crate::forge::value_type::{TypeOf, TypeScope, ValueType};

/// The version of the manifest's shape, its `v`.
pub const MANIFEST_VERSION: u32 = 1;

/// The integer types of the type grammar, in its order: each a definition of
/// the types module under its own name, its range the type's.
const INTEGERS: [SceType; 8] = [
    SceType::Uint8,
    SceType::Uint16,
    SceType::Uint32,
    SceType::Uint64,
    SceType::Int8,
    SceType::Int16,
    SceType::Int32,
    SceType::Int64,
];

/// The names the codec functions' parameters take: the message and its
/// octets.
pub const CODEC_PARAMETERS: [&str; 2] = ["msg", "octets"];

/// The types module of a suite, and its manifest.
#[derive(Debug, Clone, Serialize)]
pub struct TypesModule {
    /// The version of the manifest's shape ([`MANIFEST_VERSION`]).
    pub v: u32,
    /// The suite's name, which names its modules.
    pub suite: String,
    /// The module's name, `<suite>_Types`.
    pub module: String,
    /// Every type definition, in the order the module holds them.
    pub types: Vec<TypeDef>,
    /// The codec functions, one pair per message type, in the order of the
    /// types' names.
    pub codec: Vec<Codec>,
}

/// One type definition of the module.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum TypeDef {
    /// `type integer <name> (<min> .. <max>)`: an integer type of the type
    /// grammar.
    #[serde(rename = "integer")]
    Integer {
        name: String,
        min: String,
        max: String,
    },
    /// `type enumerated <name> { <variant>(<value>), … }`: an enum, its
    /// variants in declared order with their declared numbers.
    #[serde(rename = "enumerated")]
    Enumerated {
        name: String,
        variants: Vec<Variant>,
    },
    /// `type record <name> { … }`: an event schema, its fields in declared
    /// order, and the event its messages travel under.
    #[serde(rename = "record")]
    Record {
        name: String,
        event: String,
        fields: Vec<Field>,
    },
    /// `type union <name> { … }`: a union, one alternative per case in its
    /// tag enum's order, named by its variant.
    #[serde(rename = "union")]
    Union {
        name: String,
        alternatives: Vec<Alternative>,
    },
}

/// A variant of an enumerated type: its name and its number, the number as
/// the text of its digits, as every value of the manifest is.
#[derive(Debug, Clone, Serialize)]
pub struct Variant {
    pub name: String,
    pub value: String,
}

/// A field of a record.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    /// Whether the field is `optional`: an `optional<T>` field of the schema.
    pub optional: bool,
}

/// An alternative of a union.
#[derive(Debug, Clone, Serialize)]
pub struct Alternative {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
}

/// The type a field, an alternative, an element, a template or a formal
/// parameter is of, each kind named as TTCN-3 spells the type.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum TypeExpr {
    /// A definition of the module, by its name.
    #[serde(rename = "ref")]
    Ref { name: String },
    #[serde(rename = "boolean")]
    Boolean,
    /// A `string[N]`: TTCN-3 counts its length in characters, so the bound,
    /// in bytes of UTF-8, is stated beside it and held by the codec.
    #[serde(rename = "universal charstring")]
    Text { max_utf8_bytes: u32 },
    /// A `bytes[N]`, its bound in octets, stated beside it as a text's is.
    #[serde(rename = "octetstring")]
    Octets { max_octets: u32 },
    /// A `list<T>[M..N]`: `record length (M .. N) of T`.
    #[serde(rename = "record of")]
    List {
        min_length: u32,
        max_length: u32,
        element: Box<TypeExpr>,
    },
    /// A record written in place: the empty record of a union's case that
    /// holds no record.
    #[serde(rename = "record")]
    Record { fields: Vec<Field> },
}

/// The codec functions of one message type.
#[derive(Debug, Clone, Serialize)]
pub struct Codec {
    #[serde(rename = "type")]
    pub ty: String,
    /// `external function <encode>(in <type> msg) return octetstring`.
    pub encode: String,
    /// `external function <decode>(inout octetstring octets, out <type> msg)
    /// return integer`.
    pub decode: String,
}

/// Where a type is read, as a refusal of it says: the document, and what in
/// it the type is of.
pub(super) struct Site {
    pub(super) origin: Origin,
    pub(super) subject: String,
}

impl Site {
    /// The refusal of a type the export has no TTCN-3 form for, for `why`
    /// (`generate/unsupported-feature`).
    pub(super) fn refuse(&self, why: impl std::fmt::Display) -> Located<ForgeError> {
        Located::in_file(
            GenerateError::unsupported_at(
                format!("{}: {why}", self.subject),
                Some(SourceLocation {
                    file: self.origin.label.clone(),
                    line: self.origin.line,
                    col: None,
                }),
            )
            .into(),
            self.origin.label.clone(),
        )
    }
}

/// The TTCN-3 type of `ty`, read in `scope`, which is not itself optional.
///
/// Refused by name: a float, which the profile leaves out (§8.1); a list of
/// values that may be absent, since the elements of a `record of` are never
/// omitted; and a value that may be absent anywhere but at the top of a
/// field, which TTCN-3 writes as `optional` there alone.
pub(super) fn type_expr(
    ty: &ValueType,
    scope: &TypeScope,
    site: &Site,
) -> Result<TypeExpr, Located<ForgeError>> {
    let unread = |alias: &str| site.refuse(format!("'{alias}' names no type its document reads"));
    match ty {
        TypeOf::Scalar(SceType::Bool) => Ok(TypeExpr::Boolean),
        TypeOf::Scalar(SceType::Enum(reference)) => scope
            .enums
            .get(&reference.alias)
            .map(|e| TypeExpr::Ref {
                name: unit_name(&e.name),
            })
            .ok_or_else(|| unread(&reference.alias)),
        TypeOf::Scalar(scalar) if scalar.int_bit_width().is_some() => Ok(TypeExpr::Ref {
            name: scalar.as_attr(),
        }),
        TypeOf::Scalar(scalar) => Err(site.refuse(format!(
            "{} has no type in the test profile, which leaves floats out \
             (docs/SCE_TEST_PROFILE.md §6.3, §8.1)",
            scalar.as_attr()
        ))),
        TypeOf::Text { max } => Ok(TypeExpr::Text {
            max_utf8_bytes: max.max,
        }),
        TypeOf::Bytes { max } => Ok(TypeExpr::Octets {
            max_octets: max.max,
        }),
        TypeOf::Record { alias } => scope
            .records
            .get(alias)
            .map(|r| TypeExpr::Ref {
                name: unit_name(&r.name),
            })
            .ok_or_else(|| unread(alias)),
        TypeOf::Union { alias } => scope
            .unions
            .get(alias)
            .map(|u| TypeExpr::Ref {
                name: unit_name(&u.name),
            })
            .ok_or_else(|| unread(alias)),
        TypeOf::List { elem, max } => {
            if matches!(elem.as_ref(), TypeOf::Optional { .. }) {
                return Err(site.refuse(format!(
                    "{ty} is a list of values that may be absent, and the elements of a \
                     TTCN-3 `record of` are never omitted"
                )));
            }
            Ok(TypeExpr::List {
                min_length: max.min,
                max_length: max.max,
                element: Box::new(type_expr(elem, scope, site)?),
            })
        }
        TypeOf::Optional { .. } => Err(site.refuse(format!(
            "{ty} may be absent where TTCN-3 writes no `optional`: only a field of a \
             record may be absent"
        ))),
    }
}

/// The types module of the suite `suite`, every name it writes entered in
/// `names`; `at` is the suite, where the names the export makes are placed.
pub(super) fn types_module(
    suite: &str,
    reach: &Reach,
    names: &mut Namespace,
    at: &Origin,
) -> Result<TypesModule, Located<ForgeError>> {
    let mut types = Vec::new();
    for scalar in &INTEGERS {
        let name = scalar.as_attr();
        names.definition(&name, Declared::made("the integer type", at.clone()))?;
        let (min, max) = scalar
            .int_value_range()
            .expect("an integer type of the grammar has a range");
        types.push(TypeDef::Integer {
            name,
            min: min.to_string(),
            max: max.to_string(),
        });
    }
    for document in reach.sorted() {
        match &document.parsed.document {
            ForgeDocument::Enum(model) => types.push(enumerated(document, model, names)?),
            ForgeDocument::EventSchema(schema) => types.push(record(document, schema, names)?),
            ForgeDocument::Union(model) => types.push(union(document, model, names)?),
            _ => {}
        }
    }
    let codec = codec(reach, names, at)?;
    Ok(TypesModule {
        v: MANIFEST_VERSION,
        suite: suite.to_string(),
        module: format!("{suite}_Types"),
        types,
        codec,
    })
}

/// An enum as an enumerated type. Refused by name when its set is open
/// (`sce:strict-variants="false"`): an `enumerated` holds its declared values
/// alone, and the enum holds every number of its underlying type.
fn enumerated(
    document: &Reached,
    model: &EnumModel,
    names: &mut Namespace,
) -> Result<TypeDef, Located<ForgeError>> {
    let origin = document.origin();
    names.definition(
        &document.name,
        Declared::read("the enumerated type", origin.clone()),
    )?;
    if !model.strict_variants {
        let site = Site {
            origin,
            subject: format!("enum '{}'", model.name),
        };
        return Err(site.refuse(
            "its set of values is open (sce:strict-variants=\"false\"), and a TTCN-3 \
             `enumerated` holds its declared values alone",
        ));
    }
    let mut variants = Vec::new();
    for variant in &model.variants {
        names.value(
            &variant.name,
            Declared::read("the variant", origin.at(variant.source_line)),
        )?;
        variants.push(Variant {
            name: variant.name.clone(),
            value: variant.value.to_string(),
        });
    }
    Ok(TypeDef::Enumerated {
        name: document.name.clone(),
        variants,
    })
}

/// An event schema as a record, its fields in declared order.
fn record(
    document: &Reached,
    schema: &EventSchemaModel,
    names: &mut Namespace,
) -> Result<TypeDef, Located<ForgeError>> {
    let origin = document.origin();
    names.definition(&document.name, Declared::read("the record", origin.clone()))?;
    let mut fields = Vec::new();
    for field in &schema.fields {
        written(&field.id, &Declared::read("the field", origin.clone()))?;
        let site = Site {
            origin: origin.clone(),
            subject: format!("event-schema '{}': the field '{}'", schema.name, field.id),
        };
        let (ty, optional) = match &field.sce_type {
            TypeOf::Optional { elem } => (type_expr(elem, &document.scope, &site)?, true),
            other => (type_expr(other, &document.scope, &site)?, false),
        };
        fields.push(Field {
            name: field.id.clone(),
            ty,
            optional,
        });
    }
    Ok(TypeDef::Record {
        name: document.name.clone(),
        event: schema.event_name.clone(),
        fields,
    })
}

/// A union as a TTCN-3 union: one alternative per case, in its tag enum's
/// order, named by its variant; a case that holds no record is the empty
/// record (docs/SCE_TEST_PROFILE.md §6.3).
fn union(
    document: &Reached,
    model: &UnionModel,
    names: &mut Namespace,
) -> Result<TypeDef, Located<ForgeError>> {
    let origin = document.origin();
    names.definition(&document.name, Declared::read("the union", origin.clone()))?;
    let site = Site {
        origin: origin.clone(),
        subject: format!("union '{}'", model.name),
    };
    let scoped = crate::forge::import_source::union_in_scope(
        document.identity.clone(),
        model,
        document.scope.clone(),
    )
    .ok_or_else(|| site.refuse(format!("its tag's enum '{}' is not read", model.tag_enum)))?;
    let mut alternatives = Vec::new();
    for case in &scoped.cases {
        let line = model
            .case(&case.variant.name)
            .and_then(|case| case.source_line);
        written(
            &case.variant.name,
            &Declared::read("the alternative", origin.at(line)),
        )?;
        let ty = match &case.record {
            Some(alias) => type_expr(
                &TypeOf::Record {
                    alias: alias.clone(),
                },
                &scoped.scope,
                &site,
            )?,
            None => TypeExpr::Record { fields: Vec::new() },
        };
        alternatives.push(Alternative {
            name: case.variant.name.clone(),
            ty,
        });
    }
    Ok(TypeDef::Union {
        name: document.name.clone(),
        alternatives,
    })
}

/// The codec functions of every type a port type of the suite carries, in
/// the order of the types' names, each placed at the type's document.
fn codec(
    reach: &Reach,
    names: &mut Namespace,
    at: &Origin,
) -> Result<Vec<Codec>, Located<ForgeError>> {
    for parameter in CODEC_PARAMETERS {
        names.nested(parameter, Declared::made("the codec parameter", at.clone()))?;
    }
    let mut messages: BTreeSet<String> = BTreeSet::new();
    for document in reach.sorted() {
        let ForgeDocument::TestPortType(port_type) = &document.parsed.document else {
            continue;
        };
        for message in &port_type.messages {
            let site = Site {
                origin: document.origin().at(message.source_line),
                subject: format!(
                    "test-port-type '{}': the message '{}'",
                    port_type.name, message.schema
                ),
            };
            let ty = type_expr(
                &TypeOf::Record {
                    alias: message.schema.clone(),
                },
                &document.scope,
                &site,
            )?;
            if let TypeExpr::Ref { name } = ty {
                messages.insert(name);
            }
        }
    }
    let mut codec = Vec::new();
    for ty in messages {
        let origin = reach
            .sorted()
            .into_iter()
            .find(|document| {
                document.name == ty
                    && matches!(document.parsed.document, ForgeDocument::EventSchema(_))
            })
            .map_or_else(|| at.clone(), Reached::origin);
        let encode = format!("enc_{ty}");
        let decode = format!("dec_{ty}");
        names.definition(
            &encode,
            Declared::made("the encoding function", origin.clone()),
        )?;
        names.definition(&decode, Declared::made("the decoding function", origin))?;
        codec.push(Codec { ty, encode, decode });
    }
    Ok(codec)
}
