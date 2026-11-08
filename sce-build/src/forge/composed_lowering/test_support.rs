// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The schemas, unions and enums the tests of the walks read, shared by the
//! tests of the walks themselves ([`super`]) and by those of each backend's
//! spelling of them, so every spelling is held to the same values.

use super::*;

/// An event schema read from `text` as the parser reads it.
fn schema(text: &str) -> EventSchemaModel {
    match crate::forge::parser::parse_forge_with_imports(
        text,
        crate::DocumentLabel::symmetric("probe"),
    )
    .expect("the schema parses")
    .expect("a forge document")
    .document
    {
        crate::forge::model::ForgeDocument::EventSchema(schema) => schema,
        _ => panic!("an event schema"),
    }
}

const ITEM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="item" sce:event-name="item.sent">
  <sce:import kind="enum" src="level.scxml" as="Level"/>
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="level" sce:type="enum:Level" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
    <data id="blob" sce:type="bytes[4]" sce:direction="in"/>
    <data id="tags" sce:type="list&lt;string[8]&gt;[4]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

const BATCH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="batch" sce:event-name="batch.sent" sce:revision="2">
  <sce:import kind="event-schema" src="item.scxml" as="Item"/>
  <datamodel>
    <data id="items" sce:type="list&lt;record:Item&gt;[3]" sce:direction="in"/>
    <data id="grid" sce:type="list&lt;list&lt;uint8&gt;[2]&gt;[2]" sce:direction="in"/>
    <data id="late" sce:type="list&lt;uint8&gt;[2]" sce:direction="in" sce:since="2"/>
  </datamodel>
</scxml>
"#;

/// A statechart that imports `batch` as `Batch` and `item` as `Item`, and
/// whose `batch` names `item` as `Item` too.
pub(super) struct TwoRecords {
    batch: EventSchemaModel,
    item: EventSchemaModel,
}

impl RecordEnv for TwoRecords {
    fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String> {
        match (within, alias) {
            (None | Some("Batch"), "Item") => Ok(HeldRecord {
                alias: "Item",
                schema: &self.item,
            }),
            (None, "Batch") => Ok(HeldRecord {
                alias: "Batch",
                schema: &self.batch,
            }),
            _ => Err(format!("no record `{alias}`")),
        }
    }
    fn union(&self, _within: Option<&str>, alias: &str) -> Result<HeldUnion<'_>, String> {
        Err(format!("no union `{alias}`"))
    }
    fn union_of_key(&self, key: &str) -> Result<HeldUnion<'_>, String> {
        Err(format!("no union `{key}`"))
    }
}

const RADIUS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="shape_radius" sce:event-name="shape.radius">
  <datamodel>
    <data id="radius" sce:type="uint16" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A statechart that imports `shape_radius` as `Radius` and the tag's enum
/// as `Kind`, and reads a `shape`: a dot of nothing, and a circle and a
/// disc that both hold `Radius`.
pub(super) struct Shapes {
    radius: EventSchemaModel,
    pub(super) union: ScopedUnion,
}

pub(super) const SHAPE_KEY: &str = "union:shape";

impl RecordEnv for Shapes {
    fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String> {
        match (within, alias) {
            (None | Some(SHAPE_KEY), "Radius") => Ok(HeldRecord {
                alias: "Radius",
                schema: &self.radius,
            }),
            _ => Err(format!("no record `{alias}`")),
        }
    }
    fn union(&self, within: Option<&str>, alias: &str) -> Result<HeldUnion<'_>, String> {
        match (within, alias) {
            (None, "Shape") => self.union_of_key(SHAPE_KEY),
            _ => Err(format!("no union `{alias}`")),
        }
    }
    fn union_of_key(&self, key: &str) -> Result<HeldUnion<'_>, String> {
        (key == SHAPE_KEY)
            .then(|| HeldUnion {
                key: key.to_string(),
                union: &self.union,
                tag_enum: Some("Kind".to_string()),
            })
            .ok_or_else(|| format!("no union `{key}`"))
    }
}

pub(super) fn shapes() -> Shapes {
    use crate::forge::value_type::{ScopedEnum, ScopedRecord, ScopedVariant};
    let variants: Vec<ScopedVariant> = ["dot", "circle", "disc"]
        .iter()
        .zip(0..)
        .map(|(name, value)| ScopedVariant {
            name: name.to_string(),
            value,
        })
        .collect();
    let mut scope = TypeScope::default();
    scope.enums.insert(
        "ShapeKind".to_string(),
        ScopedEnum {
            name: "shape_kind".to_string(),
            variants: variants.clone(),
            underlying: SceType::Uint8,
            strict: true,
        },
    );
    scope.records.insert(
        "Radius".to_string(),
        ScopedRecord {
            identity: "shape_radius".to_string(),
            name: "shape_radius".to_string(),
            fields: vec![("radius".to_string(), ty("uint16"))],
            scope: TypeScope::default(),
        },
    );
    let cases = variants
        .into_iter()
        .map(|variant| ScopedCase {
            record: (variant.name != "dot").then(|| "Radius".to_string()),
            variant,
        })
        .collect();
    Shapes {
        radius: schema(RADIUS),
        union: ScopedUnion {
            identity: "shape".to_string(),
            name: "shape".to_string(),
            tag: "kind".to_string(),
            tag_enum: "ShapeKind".to_string(),
            cases,
            scope,
        },
    }
}

pub(super) fn kinds() -> BTreeMap<String, MachineEnum> {
    [(
        "Kind".to_string(),
        MachineEnum {
            alias: "Kind".to_string(),
            ty: "MKindEnum".to_string(),
            variants: vec![
                ("dot".to_string(), "Dot".to_string()),
                ("circle".to_string(), "Circle".to_string()),
                ("disc".to_string(), "Disc".to_string()),
            ],
        },
    )]
    .into()
}

pub(super) fn held() -> TwoRecords {
    TwoRecords {
        batch: schema(BATCH),
        item: schema(ITEM),
    }
}

pub(super) fn enums() -> BTreeMap<String, MachineEnum> {
    [(
        "Level".to_string(),
        MachineEnum {
            alias: "Level".to_string(),
            ty: "MLevelEnum".to_string(),
            variants: vec![
                ("low".to_string(), "Low".to_string()),
                ("high".to_string(), "High".to_string()),
            ],
        },
    )]
    .into()
}

pub(super) fn ty(text: &str) -> ValueType {
    crate::forge::value_type::WrittenType::read(text)
        .expect("a type")
        .bounded()
        .expect("every bound stated")
}
