// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The values a machine's deployment supplies (docs/adr/0109, D12): a
//! machine's document declares each as a `<data sce:from="deployment">` of a
//! value type, and its entry in deploy.yaml gives them, written under the
//! machine (`config:`) or in a file the host reads when it starts the machine
//! (`config_file:`).
//!
//! A configuration written in the deployment is checked here, when the
//! machine is generated with the deployment: every value the document
//! declares is given, no key names a value it does not declare, and each
//! value is a value of its type by the type's payload schema (docs/adr/0087,
//! section 1: the wire form of the type, every key of a record required and
//! no other, every bound held). What is accepted is written as the JSON the
//! machine's generated reader reads, so a configuration given in the
//! deployment and one given in a file reach the machine the same way: as one
//! JSON object of the values by their ids, read when the machine is built.
//!
//! A file is the operator's and is read by the host at start, through the
//! reader with the lift's refusals (docs/adr/0087, section 2.3); the build
//! checks only that the document declares a value for it to supply.

use std::collections::BTreeMap;

use crate::forge::model::SceType;
use crate::forge::value_type::{TypeEnv, ValueType};
use crate::mesh::deploy::MachineConfig;
use crate::mesh::error::{ConfigSource, DeployError};
use crate::model::SCXMLModel;

use serde_yaml_ng::Value as Yaml;

/// The file the build writes a machine's checked configuration to, beside the
/// machine's generated code: the deploy name of the machine, `.config.json`.
pub fn config_file_name(machine: &str) -> String {
    format!("{machine}.config.json")
}

/// The first reference token of the strict RFC 6901 pointer `pointer`,
/// unescaped (`~1` is `/`, `~0` is `~`, RFC 6901 section 4); `None` for the
/// empty pointer, which names the whole text.
pub fn first_token(pointer: &str) -> Option<String> {
    let rest = pointer.strip_prefix('/')?;
    let token = rest.split('/').next().unwrap_or_default();
    Some(token.replace("~1", "/").replace("~0", "~"))
}

/// Check the configuration of the machine `machine`, whose deploy entry is
/// `entry` and whose document is `model`, and answer the JSON text of what
/// `config:` gives, which the build writes beside the machine
/// ([`config_file_name`]); `None` where the deployment writes none, because
/// the machine declares no value or names a `config_file:` the host reads.
///
/// # Errors
///
/// `mesh/deploy-config-missing` for a value the document declares and the
/// deployment does not give, `mesh/deploy-config-unknown` for a key of
/// `config:` the document declares no value of, and
/// `mesh/deploy-config-invalid` for a value that is no value of its type,
/// for both sources written, and for a `config_file:` of a machine that
/// reads none.
pub fn check_machine_config(
    machine: &str,
    entry: &MachineConfig,
    model: &SCXMLModel,
) -> Result<Option<String>, DeployError> {
    let invalid =
        |place: ConfigSource, pointer: String, reason: String| DeployError::ConfigInvalid {
            machine: machine.to_string(),
            place,
            pointer,
            reason,
        };
    if entry.config.is_some() && entry.config_file.is_some() {
        return Err(invalid(
            ConfigSource::File,
            String::new(),
            two_sources_reason(),
        ));
    }
    let scope = crate::forge::value_path::statechart_scope(model);
    // Each value the document declares, in declaration order, with its type
    // as the build bounded it. The judge refused a type that states no bound
    // before generation; one met here is refused as the configuration of it.
    let mut declared: Vec<(&str, ValueType)> = Vec::new();
    for variable in model.variables.iter().filter(|v| v.from_deployment) {
        let ty = variable
            .value_type
            .clone()
            .and_then(|written| written.bounded().ok())
            .ok_or_else(|| {
                invalid(
                    ConfigSource::Inline,
                    pointer_below("", &variable.id),
                    "the document's type of it states no bound, so no value of it can be \
                     checked"
                        .to_string(),
                )
            })?;
        declared.push((variable.id.as_str(), ty));
    }
    let names: Vec<String> = declared.iter().map(|(id, _)| id.to_string()).collect();
    let missing = |id: &str, ty: &ValueType| DeployError::ConfigMissing {
        machine: machine.to_string(),
        value: id.to_string(),
        written_type: ty.to_string(),
    };
    let config: &BTreeMap<String, Yaml> =
        match (&entry.config, &entry.config_file) {
            (Some(config), _) => config,
            // The file is the operator's, read by the host at start: there is
            // nothing of it to check, unless the machine reads no configuration.
            (None, Some(_)) if declared.is_empty() => return Err(invalid(
                ConfigSource::File,
                String::new(),
                "the machine's document declares no value its deployment supplies, so it reads \
                 no configuration"
                    .to_string(),
            )),
            (None, Some(_)) => return Ok(None),
            (None, None) => {
                return match declared.first() {
                    Some((id, ty)) => Err(missing(id, ty)),
                    None => Ok(None),
                }
            }
        };
    if let Some(key) = config.keys().find(|key| !names.contains(key)) {
        let candidates = names
            .iter()
            .filter(|name| !config.contains_key(name.as_str()))
            .cloned()
            .collect();
        return Err(DeployError::ConfigUnknown {
            machine: machine.to_string(),
            key: key.clone(),
            declared: names,
            candidates,
        });
    }
    if declared.is_empty() {
        return Ok(None);
    }
    let mut members = Vec::with_capacity(declared.len());
    for (id, ty) in &declared {
        let value = config.get(*id).ok_or_else(|| missing(id, ty))?;
        let json = conform(value, ty, &scope, &pointer_below("", id))
            .map_err(|fault| invalid(ConfigSource::Inline, fault.pointer, fault.reason))?;
        members.push(format!("{}:{json}", quoted(id)));
    }
    Ok(Some(format!("{{{}}}", members.join(","))))
}

/// Why a machine entry that writes both `config:` and `config_file:` is
/// refused: one checked where it is read, so the two cannot disagree.
pub(crate) fn two_sources_reason() -> String {
    "both `config:` and `config_file:` are written, and a machine's values come from one of \
     them"
        .to_string()
}

/// A value of the configuration refused: the strict RFC 6901 pointer to the
/// place, from the configuration's root, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Fault {
    pointer: String,
    reason: String,
}

/// `token` below the strict RFC 6901 pointer `prefix`: `/` and the token,
/// `~` written `~0` and `/` written `~1` (RFC 6901, section 3).
fn pointer_below(prefix: &str, token: &str) -> String {
    format!("{prefix}/{}", token.replace('~', "~0").replace('/', "~1"))
}

/// `text` as a JSON string, as every writer of SCE spells one.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("a text is a JSON string")
}

/// What a YAML value is, as a refusal names it.
fn kind(value: &Yaml) -> &'static str {
    match value {
        Yaml::Null => "null",
        Yaml::Bool(_) => "a bool",
        Yaml::Number(_) => "a number",
        Yaml::String(_) => "a text",
        Yaml::Sequence(_) => "a list",
        Yaml::Mapping(_) => "a mapping",
        Yaml::Tagged(_) => "a tagged value",
    }
}

/// The JSON of `value`, a value of `ty` in the scope `env`, at `at`: the wire
/// form the type's payload schema states (docs/adr/0087, section 1.1), each
/// record's keys in the order its document declares them.
fn conform(value: &Yaml, ty: &ValueType, env: &dyn TypeEnv, at: &str) -> Result<String, Fault> {
    let fault = |reason: String| Fault {
        pointer: at.to_string(),
        reason,
    };
    match ty {
        ValueType::Optional { elem } => match value {
            Yaml::Null => Ok("null".to_string()),
            present => conform(present, elem, env, at),
        },
        _ if value.is_null() => Err(fault(format!(
            "is null, and a value of `{ty}` is never absent"
        ))),
        ValueType::Scalar(scalar) => scalar_json(value, scalar, env).map_err(fault),
        ValueType::Text { max } => {
            let Yaml::String(text) = value else {
                return Err(fault(format!("is {}, and `{ty}` is a text", kind(value))));
            };
            if text.contains('\0') {
                return Err(fault("holds U+0000, which no text holds".to_string()));
            }
            if text.len() > max.max as usize {
                return Err(fault(format!(
                    "is {} UTF-8 bytes, past the {} `{ty}` holds",
                    text.len(),
                    max.max
                )));
            }
            Ok(quoted(text))
        }
        ValueType::Bytes { max } => {
            let Yaml::String(text) = value else {
                return Err(fault(format!(
                    "is {}, and `{ty}` is written as the text of its bytes",
                    kind(value)
                )));
            };
            if let Some(c) = text.chars().find(|c| u32::from(*c) > 0xff) {
                return Err(fault(format!(
                    "holds `{c}`, which is no byte: the text of a byte string holds characters \
                     up to U+00FF, one per byte"
                )));
            }
            let held = text.chars().count();
            if held > max.max as usize {
                return Err(fault(format!(
                    "is {held} bytes, past the {} `{ty}` holds",
                    max.max
                )));
            }
            Ok(quoted(text))
        }
        ValueType::List { elem, max } => {
            let Yaml::Sequence(items) = value else {
                return Err(fault(format!("is {}, and `{ty}` is a list", kind(value))));
            };
            if items.len() > max.max as usize {
                return Err(fault(format!(
                    "holds {} elements, past the {} `{ty}` holds",
                    items.len(),
                    max.max
                )));
            }
            if items.len() < max.min as usize {
                return Err(fault(format!(
                    "holds {} elements, short of the {} `{ty}` holds at least",
                    items.len(),
                    max.min
                )));
            }
            let mut written = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                written.push(conform(
                    item,
                    elem,
                    env,
                    &pointer_below(at, &index.to_string()),
                )?);
            }
            Ok(format!("[{}]", written.join(",")))
        }
        ValueType::Record { alias } => {
            let fields = env.record_fields(alias).ok_or_else(|| {
                fault(format!(
                    "`{ty}` names no record the machine's document imports"
                ))
            })?;
            let inner = env.record_scope(alias).unwrap_or(env);
            record_json(value, ty, &fields, inner, at, None)
        }
        ValueType::Union { alias } => {
            let union = env.union_type(alias).ok_or_else(|| {
                fault(format!(
                    "`{ty}` names no union the machine's document imports"
                ))
            })?;
            let Yaml::Mapping(map) = value else {
                return Err(fault(format!("is {}, and `{ty}` is a union", kind(value))));
            };
            let tag_at = pointer_below(at, &union.tag);
            let variant = match map.get(union.tag.as_str()) {
                Some(Yaml::String(variant)) => variant,
                Some(other) => {
                    return Err(Fault {
                        pointer: tag_at,
                        reason: format!(
                            "is {}, and the tag of `{ty}` is the name of a case",
                            kind(other)
                        ),
                    })
                }
                None => {
                    return Err(Fault {
                        pointer: tag_at,
                        reason: format!("is missing, and the tag of `{ty}` names its case"),
                    })
                }
            };
            let Some(case) = union.cases.iter().find(|c| c.variant.name == *variant) else {
                let cases: Vec<&str> = union
                    .cases
                    .iter()
                    .map(|c| c.variant.name.as_str())
                    .collect();
                return Err(Fault {
                    pointer: tag_at,
                    reason: format!(
                        "names no case of `{ty}`, whose cases are {}",
                        cases.join(", ")
                    ),
                });
            };
            let (fields, inner): (Vec<(String, ValueType)>, &dyn TypeEnv) = match &case.record {
                None => (Vec::new(), &union.scope as &dyn TypeEnv),
                Some(record) => (
                    union.scope.record_fields(record).ok_or_else(|| {
                        fault(format!(
                            "the case `{variant}` of `{ty}` holds a record its document does \
                             not import"
                        ))
                    })?,
                    union
                        .scope
                        .record_scope(record)
                        .unwrap_or(&union.scope as &dyn TypeEnv),
                ),
            };
            record_json(
                value,
                ty,
                &fields,
                inner,
                at,
                Some((union.tag.as_str(), variant.as_str())),
            )
        }
        ValueType::Map { .. } | ValueType::Set { .. } => Err(fault(format!(
            "`{ty}` has no wire form yet, so no deployment supplies one (docs/adr/0108, D7)"
        ))),
    }
}

/// The JSON of `value`, a record of `fields` read in `env`, the value of `ty`
/// at `at`: an object of every field, in the order the record declares them,
/// and no other key; a union's case leads with its tag, `lead`.
fn record_json(
    value: &Yaml,
    ty: &ValueType,
    fields: &[(String, ValueType)],
    env: &dyn TypeEnv,
    at: &str,
    lead: Option<(&str, &str)>,
) -> Result<String, Fault> {
    let Yaml::Mapping(map) = value else {
        return Err(Fault {
            pointer: at.to_string(),
            reason: format!("is {}, and `{ty}` is a record", kind(value)),
        });
    };
    for key in map.keys() {
        let Some(name) = key.as_str() else {
            return Err(Fault {
                pointer: at.to_string(),
                reason: format!("has the key {}, and a field is named by a text", kind(key)),
            });
        };
        let known =
            lead.is_some_and(|(tag, _)| tag == name) || fields.iter().any(|(id, _)| id == name);
        if !known {
            let declared: Vec<&str> = fields.iter().map(|(id, _)| id.as_str()).collect();
            return Err(Fault {
                pointer: pointer_below(at, name),
                reason: format!(
                    "names no field of `{ty}`, whose fields are {}",
                    if declared.is_empty() {
                        "none".to_string()
                    } else {
                        declared.join(", ")
                    }
                ),
            });
        }
    }
    let mut members = Vec::with_capacity(fields.len() + 1);
    if let Some((tag, variant)) = lead {
        members.push(format!("{}:{}", quoted(tag), quoted(variant)));
    }
    for (id, field_ty) in fields {
        let place = pointer_below(at, id);
        let Some(field) = map.get(id.as_str()) else {
            let absent = if matches!(field_ty, ValueType::Optional { .. }) {
                ", and an absent value is written `null`"
            } else {
                ""
            };
            return Err(Fault {
                pointer: place,
                reason: format!("is missing, and `{ty}` declares it, of `{field_ty}`{absent}"),
            });
        };
        members.push(format!(
            "{}:{}",
            quoted(id),
            conform(field, field_ty, env, &place)?
        ));
    }
    Ok(format!("{{{}}}", members.join(",")))
}

/// The JSON of `value`, a value of the scalar `scalar` in `env`: a number of at
/// most 32 bits, a real or a bool as itself, a 64-bit integer as the text of
/// its digits, and a value of an enum as the name of its variant.
fn scalar_json(value: &Yaml, scalar: &SceType, env: &dyn TypeEnv) -> Result<String, String> {
    let spelled = scalar.as_attr();
    match scalar {
        SceType::Bool => match value {
            Yaml::Bool(truth) => Ok(truth.to_string()),
            other => Err(format!("is {}, and `bool` is true or false", kind(other))),
        },
        SceType::Uint64 | SceType::Int64 => integer(value, scalar).map(|n| quoted(&n.to_string())),
        SceType::Uint8
        | SceType::Uint16
        | SceType::Uint32
        | SceType::Int8
        | SceType::Int16
        | SceType::Int32 => integer(value, scalar).map(|n| n.to_string()),
        SceType::Float32 | SceType::Float64 => {
            let Yaml::Number(number) = value else {
                return Err(format!("is {}, and `{spelled}` is a number", kind(value)));
            };
            let real = number
                .as_f64()
                .ok_or_else(|| format!("is {number}, which no `{spelled}` holds"))?;
            if !real.is_finite() {
                return Err(format!(
                    "is {number}, and a `{spelled}` of the wire is a finite number"
                ));
            }
            if matches!(scalar, SceType::Float32) && real.abs() > f64::from(f32::MAX) {
                return Err(format!("is {number}, past the largest finite `float32`"));
            }
            serde_json::Number::from_f64(real)
                .map(|n| n.to_string())
                .ok_or_else(|| format!("is {number}, which JSON writes no number for"))
        }
        SceType::Enum(reference) => {
            let declared = env
                .enum_type(&reference.alias)
                .ok_or_else(|| format!("`{spelled}` names no enum the document imports"))?;
            let Yaml::String(name) = value else {
                return Err(format!(
                    "is {}, and a value of `{spelled}` is the name of a variant",
                    kind(value)
                ));
            };
            if declared.variants.iter().any(|v| v.name == *name) {
                Ok(quoted(name))
            } else {
                let names: Vec<&str> = declared.variants.iter().map(|v| v.name.as_str()).collect();
                Err(format!(
                    "names no variant of `{spelled}`, whose variants are {}",
                    names.join(", ")
                ))
            }
        }
        SceType::String | SceType::Bytes => Err(format!(
            "`{spelled}` states no bound, so no value of it can be checked"
        )),
    }
}

/// `value` as a whole number of the integer type `scalar`, in its range: a
/// YAML integer, or for a 64-bit type the text of its digits too, the form it
/// crosses the wire in (docs/adr/0086, section 3.1).
fn integer(value: &Yaml, scalar: &SceType) -> Result<i128, String> {
    let spelled = scalar.as_attr();
    let (least, most) = scalar
        .int_value_range()
        .ok_or_else(|| format!("`{spelled}` is no integer type"))?;
    let n = match value {
        Yaml::Number(number) => match (number.as_u64(), number.as_i64()) {
            (Some(unsigned), _) => i128::from(unsigned),
            (None, Some(signed)) => i128::from(signed),
            (None, None) => return Err(format!("is {number}, and `{spelled}` is a whole number")),
        },
        Yaml::String(text) if matches!(scalar, SceType::Uint64 | SceType::Int64) => {
            let digits = text.strip_prefix('-').unwrap_or(text);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!(
                    "is the text '{text}', which is not the digits of a whole number"
                ));
            }
            text.parse::<i128>()
                .map_err(|_| format!("is the text '{text}', past every `{spelled}`"))?
        }
        other => {
            return Err(format!(
                "is {}, and `{spelled}` is a whole number",
                kind(other)
            ))
        }
    };
    if n < least {
        Err(format!("is {n}, below the least {spelled} ({least})"))
    } else if n > most {
        Err(format!("is {n}, past the largest {spelled} ({most})"))
    } else {
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::{
        ScopedEnum, ScopedRecord, ScopedVariant, TypeScope, WrittenType,
    };

    fn ty(text: &str) -> ValueType {
        WrittenType::read(text)
            .expect("a type")
            .bounded()
            .expect("a bounded type")
    }

    /// A scope of one record, `Limits { max_rows: uint32, label: string[16],
    /// note: optional<string[8]> }`, and one enum, `Level { low, high }`.
    fn scope() -> TypeScope {
        let mut scope = TypeScope::default();
        scope.records.insert(
            "Limits".into(),
            ScopedRecord {
                identity: "/here/limits.scxml".into(),
                name: "limits".into(),
                fields: vec![
                    ("max_rows".into(), ty("uint32")),
                    ("label".into(), ty("string[16]")),
                    ("note".into(), ty("optional<string[8]>")),
                ],
                scope: TypeScope::default(),
                added: Default::default(),
            },
        );
        scope.enums.insert(
            "Level".into(),
            ScopedEnum {
                name: "level".into(),
                variants: vec![
                    ScopedVariant {
                        name: "low".into(),
                        value: 0,
                    },
                    ScopedVariant {
                        name: "high".into(),
                        value: 1,
                    },
                ],
                underlying: SceType::Uint8,
                strict: true,
            },
        );
        scope
    }

    fn yaml(text: &str) -> Yaml {
        serde_yaml_ng::from_str(text).expect("YAML")
    }

    fn json(text: &str, written: &str) -> Result<String, Fault> {
        conform(&yaml(text), &ty(written), &scope(), "/v")
    }

    /// A value of each kind is written as the wire form its payload schema
    /// states: a 64-bit integer as its digits, a record's keys in their
    /// declared order whatever order the YAML wrote them in, an absent value as
    /// `null`.
    #[test]
    fn a_value_of_its_type_is_written_as_its_wire_form() {
        assert_eq!(json("200", "uint32"), Ok("200".to_string()));
        assert_eq!(json("-3", "int8"), Ok("-3".to_string()));
        assert_eq!(
            json("18446744073709551615", "uint64"),
            Ok("\"18446744073709551615\"".to_string())
        );
        assert_eq!(json("'42'", "int64"), Ok("\"42\"".to_string()));
        assert_eq!(json("true", "bool"), Ok("true".to_string()));
        assert_eq!(json("1.5", "float64"), Ok("1.5".to_string()));
        assert_eq!(json("high", "enum:Level"), Ok("\"high\"".to_string()));
        assert_eq!(json("main", "string[16]"), Ok("\"main\"".to_string()));
        assert_eq!(json("~", "optional<uint8>"), Ok("null".to_string()));
        assert_eq!(
            json("[a.example, b.example]", "list<string[16]>[4]"),
            Ok("[\"a.example\",\"b.example\"]".to_string())
        );
        assert_eq!(
            json("{ note: ~, label: main, max_rows: 200 }", "record:Limits"),
            Ok("{\"max_rows\":200,\"label\":\"main\",\"note\":null}".to_string())
        );
    }

    /// Each refusal names the place, by its strict RFC 6901 pointer, and why.
    #[test]
    fn a_value_that_is_no_value_of_its_type_is_refused_at_its_place() {
        let refused = |text: &str, written: &str| json(text, written).expect_err("refused");
        let fault = refused("256", "uint8");
        assert_eq!(fault.pointer, "/v");
        assert!(
            fault.reason.contains("past the largest uint8 (255)"),
            "{}",
            fault.reason
        );
        assert!(refused("-1", "uint32")
            .reason
            .contains("below the least uint32"));
        assert!(refused("1.5", "uint32").reason.contains("whole number"));
        assert!(refused("x", "bool").reason.contains("true or false"));
        assert!(refused("medium", "enum:Level").reason.contains("low, high"));
        assert!(refused("seventeen bytes!!", "string[16]")
            .reason
            .contains("past the 16"));
        assert!(refused("~", "uint32").reason.contains("never absent"));
        assert!(refused("[1, 2, 3]", "list<uint8>[2]")
            .reason
            .contains("past the 2"));
        assert!(refused("[1]", "list<uint8>[2..4]")
            .reason
            .contains("short of the 2"));
        let fault = refused("[1, 300]", "list<uint8>[4]");
        assert_eq!(fault.pointer, "/v/1");
        let fault = refused("{ max_rows: 1, label: x }", "record:Limits");
        assert_eq!(fault.pointer, "/v/note");
        assert!(fault.reason.contains("written `null`"), "{}", fault.reason);
        let fault = refused(
            "{ max_rows: 1, label: x, note: ~, extra: 1 }",
            "record:Limits",
        );
        assert_eq!(fault.pointer, "/v/extra");
        assert!(
            fault.reason.contains("max_rows, label, note"),
            "{}",
            fault.reason
        );
        let fault = refused("{ max_rows: -1, label: x, note: ~ }", "record:Limits");
        assert_eq!(fault.pointer, "/v/max_rows");
        assert!(refused("[]", "map<uint8, bool>[2]")
            .reason
            .contains("no wire form"));
    }

    /// The statechart `calendar` under `sce-static`, whose `<datamodel>` rows
    /// are `variables`.
    fn model(variables: &str) -> SCXMLModel {
        let xml = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                version="1.0" datamodel="sce-static" initial="a" name="calendar">
              <datamodel>{variables}</datamodel>
              <state id="a"/>
            </scxml>"#
        );
        let mut model = crate::parser::SCXMLParser::new()
            .parse_string(&xml, "calendar")
            .expect("the statechart parses");
        crate::analyzer::analyze(&mut model, "calendar.scxml");
        model
    }

    /// The deploy entry of `calendar`, with `lines` added to it.
    fn entry(lines: &str) -> MachineConfig {
        let yaml = format!(
            "version: \"1.0\"\ntopology:\n  cloud:\n    machines:\n      calendar:\n        \
             source: calendar.scxml\n{lines}"
        );
        let cfg = crate::mesh::deploy::parse_deploy_str(&yaml).expect("the deployment parses");
        cfg.topology["cloud"].machines["calendar"].clone()
    }

    /// Two values the deployment supplies and a variable of the machine's own.
    const DECLARED: &str = r#"<data id="limit" sce:type="uint32" sce:from="deployment"/>
      <data id="hosts" sce:type="list&lt;string[16]&gt;[2]" sce:from="deployment"/>
      <data id="count" sce:type="uint32" expr="0"/>"#;

    /// A configuration of every value the document declares is written as the
    /// JSON object of them, in declaration order whatever order the YAML wrote
    /// them in; a file is the host's to read, and a machine that declares none
    /// and is given none has nothing written.
    #[test]
    fn a_configuration_of_every_declared_value_is_written_as_its_json() {
        let declared = model(DECLARED);
        let json = check_machine_config(
            "calendar",
            &entry("        config:\n          hosts: [a.example]\n          limit: 3"),
            &declared,
        )
        .expect("a configuration of every value");
        assert_eq!(
            json.as_deref(),
            Some("{\"limit\":3,\"hosts\":[\"a.example\"]}")
        );
        assert!(matches!(
            check_machine_config("calendar", &entry("        config_file: c.json"), &declared),
            Ok(None)
        ));
        let own = model(r#"<data id="count" sce:type="uint32" expr="0"/>"#);
        assert!(matches!(
            check_machine_config("calendar", &entry(""), &own),
            Ok(None)
        ));
        assert_eq!(config_file_name("calendar"), "calendar.config.json");
    }

    /// Each refusal of a configuration, on its own code.
    #[test]
    fn each_refusal_of_a_configuration_is_its_own() {
        let declared = model(DECLARED);
        let check = |lines: &str| check_machine_config("calendar", &entry(lines), &declared);
        // No configuration at all: the first value declared is the one missing.
        match check("") {
            Err(DeployError::ConfigMissing {
                value,
                written_type,
                ..
            }) => {
                assert_eq!(value, "limit");
                assert_eq!(written_type, "uint32");
            }
            other => panic!("expected ConfigMissing, got {other:?}"),
        }
        match check("        config:\n          limit: 3") {
            Err(DeployError::ConfigMissing { value, .. }) => assert_eq!(value, "hosts"),
            other => panic!("expected ConfigMissing, got {other:?}"),
        }
        match check("        config:\n          limit: 3\n          hosts: []\n          limits: 4")
        {
            Err(DeployError::ConfigUnknown {
                key,
                declared,
                candidates,
                ..
            }) => {
                assert_eq!(key, "limits");
                assert_eq!(declared, ["limit", "hosts"]);
                // Both are given: no name replaces the key without giving one
                // twice.
                assert!(candidates.is_empty(), "{candidates:?}");
            }
            other => panic!("expected ConfigUnknown, got {other:?}"),
        }
        // A key beside a value left out: the one left out is the name it can
        // become, and only that one.
        match check("        config:\n          limit: 3\n          host: [a]") {
            Err(DeployError::ConfigUnknown {
                key, candidates, ..
            }) => {
                assert_eq!(key, "host");
                assert_eq!(candidates, ["hosts"]);
            }
            other => panic!("expected ConfigUnknown, got {other:?}"),
        }
        match check("        config:\n          limit: -1\n          hosts: []") {
            Err(DeployError::ConfigInvalid { place, pointer, .. }) => {
                assert_eq!(place, ConfigSource::Inline);
                assert_eq!(pointer, "/limit");
            }
            other => panic!("expected ConfigInvalid, got {other:?}"),
        }
        match check("        config:\n          limit: 1\n          hosts: [a, b, c]") {
            Err(DeployError::ConfigInvalid {
                pointer, reason, ..
            }) => {
                assert_eq!(pointer, "/hosts");
                assert!(reason.contains("past the 2"), "{reason}");
            }
            other => panic!("expected ConfigInvalid, got {other:?}"),
        }
        // A machine that declares none: a key of `config:` is unknown, and a
        // file is one it never reads.
        let own = model(r#"<data id="count" sce:type="uint32" expr="0"/>"#);
        match check_machine_config(
            "calendar",
            &entry("        config:\n          limit: 1"),
            &own,
        ) {
            Err(DeployError::ConfigUnknown { key, declared, .. }) => {
                assert_eq!(key, "limit");
                assert!(declared.is_empty());
            }
            other => panic!("expected ConfigUnknown, got {other:?}"),
        }
        match check_machine_config("calendar", &entry("        config_file: c.json"), &own) {
            Err(DeployError::ConfigInvalid { place, .. }) => {
                assert_eq!(place, ConfigSource::File);
            }
            other => panic!("expected ConfigInvalid, got {other:?}"),
        }
    }

    /// The first token of a pointer, unescaped, is what a refusal is placed by.
    #[test]
    fn the_first_token_of_a_pointer_names_the_value() {
        assert_eq!(first_token("/limits/max_rows"), Some("limits".to_string()));
        assert_eq!(first_token("/a~1b~0c"), Some("a/b~c".to_string()));
        assert_eq!(first_token(""), None);
    }
}
