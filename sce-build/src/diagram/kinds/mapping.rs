// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Lookups, enums and event schemas as the one table each of them is.
//!
//! These three kinds are a list of pairs — an input and the value it
//! selects, a variant and its number, a field and its type — so the picture
//! is that list with nothing else on the page. The field table says the
//! same things, but spread across a table for each record and carrying the
//! requirement ids and the housekeeping of the model; this is the page a
//! reviewer reads to see what the document maps to what.
//!
//! A lookup's table ends with the row for everything it does not list,
//! because what the document says about a miss — a default value, or that it
//! is an error — is the part a specification has to decide, and a row that is
//! always last is a row that cannot be missed.

use super::{numbered, say, Picture};
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::table::{self, Cell, Table};
use crate::diagram::words::Phrase;
use crate::forge::model::{EnumModel, EventSchemaModel, LookupModel, MissPolicy};
use crate::forge::page::Lexicon;

fn single(
    title: String,
    continued: &str,
    table: Table,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    Ok(numbered(
        "mapping",
        table::set(&title, continued, &[table], page)?,
    ))
}

/// A lookup: each key, the value it selects, and what a miss gives.
pub fn lookup(m: &LookupModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Picture>, Refusal> {
    let mut rows: Vec<Vec<Cell>> = m
        .entries
        .iter()
        .map(|e| vec![Cell::mono(e.key.clone()), Cell::mono(e.value.clone())])
        .collect();
    let (key, value) = match &m.miss_policy {
        MissPolicy::Default(v) => (say(lexicon, Phrase::Otherwise)?, v.clone()),
        MissPolicy::Error => (say(lexicon, Phrase::Otherwise)?, "error".to_string()),
    };
    rows.push(vec![Cell::prose(key), Cell::mono(value)]);
    let table = Table {
        heading: Cell::mono(format!("{} -> {}", m.input.id, m.output.id)),
        head: vec![
            Cell::mono(m.input.id.clone()),
            Cell::mono(m.output.id.clone()),
        ],
        rows,
        fallback: Vec::new(),
    };
    single(
        format!("{}: {}", m.name, say(lexicon, Phrase::Mapping)?),
        say(lexicon, Phrase::Continued)?,
        table,
        page,
    )
}

/// An enum: each variant and its number.
pub fn enumeration(m: &EnumModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Picture>, Refusal> {
    if m.variants.is_empty() {
        return Ok(Vec::new());
    }
    let rows = m
        .variants
        .iter()
        .map(|v| {
            // The number as the document spelled it where it kept that
            // (`0x10`), the value otherwise.
            let value = if v.value_text.is_empty() {
                v.value.to_string()
            } else {
                v.value_text.clone()
            };
            vec![Cell::mono(v.name.clone()), Cell::mono(value)]
        })
        .collect();
    let table = Table {
        heading: Cell::mono(format!("{} ({})", m.name, m.underlying_type.as_attr())),
        head: vec![
            Cell::prose(say(lexicon, Phrase::Name)?),
            Cell::prose(say(lexicon, Phrase::Value)?),
        ],
        rows,
        fallback: Vec::new(),
    };
    single(
        format!("{}: {}", m.name, say(lexicon, Phrase::Variants)?),
        say(lexicon, Phrase::Continued)?,
        table,
        page,
    )
}

/// An event schema: the fields the event carries and their types, and the
/// bound of each that has one.
pub fn event_schema(
    m: &EventSchemaModel,
    lexicon: &Lexicon,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    if m.fields.is_empty() {
        return Ok(Vec::new());
    }
    let bounded = m.fields.iter().any(|f| f.max_size.is_some());
    let mut head = vec![
        Cell::prose(say(lexicon, Phrase::Field)?),
        Cell::prose(say(lexicon, Phrase::Type)?),
    ];
    if bounded {
        head.push(Cell::mono("max_size"));
    }
    let rows = m
        .fields
        .iter()
        .map(|f| {
            let mut row = vec![Cell::mono(f.id.clone()), Cell::mono(f.sce_type.as_attr())];
            if bounded {
                row.push(Cell::mono(
                    f.max_size.map(|n| n.to_string()).unwrap_or_default(),
                ));
            }
            row
        })
        .collect();
    let table = Table {
        heading: Cell::mono(m.event_name.clone()),
        head,
        rows,
        fallback: Vec::new(),
    };
    single(
        format!("{}: {}", m.name, say(lexicon, Phrase::Payload)?),
        say(lexicon, Phrase::Continued)?,
        table,
        page,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::ForgeDocument;
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn kind(name: &str) -> ForgeDocument {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join(format!("{name}.scxml"));
        let text = std::fs::read_to_string(&path).expect("reads");
        parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
            .expect("parses")
            .expect("not a statechart")
            .document
    }

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    /// The signal-quality lookup: four keys and what they select, then the
    /// row for every other level, last.
    #[test]
    fn a_lookup_ends_with_the_row_for_a_miss() {
        let ForgeDocument::Lookup(m) = kind("lookup") else {
            panic!("a lookup")
        };
        let pictures = lookup(&m, &EN, page()).expect("draws");
        assert_eq!(pictures.len(), 1);
        let words = pictures[0].sheet.words();
        let at = |w: &str| {
            words
                .iter()
                .position(|x| *x == w)
                .unwrap_or_else(|| panic!("{w}: {words:?}"))
        };
        for expected in [
            "lookup: mapping",
            "level -> quality",
            "level",
            "quality",
            "0",
            "NONE",
            "3",
            "HIGH",
            "otherwise",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert!(at("HIGH") < at("otherwise"), "the miss row is last");
        assert_eq!(words.last(), Some(&"NONE"), "and holds the default");
        let korean = lookup(&m, &KO, page()).expect("draws");
        assert_ne!(korean[0].sheet.words(), words, "in the page's language");
    }

    /// A lookup that treats a miss as an error says so on its last row.
    #[test]
    fn a_miss_that_is_an_error_says_so() {
        let ForgeDocument::Lookup(mut m) = kind("lookup") else {
            panic!("a lookup")
        };
        m.miss_policy = MissPolicy::Error;
        let words = lookup(&m, &EN, page()).expect("draws").remove(0).sheet;
        assert_eq!(words.words().last(), Some(&"error"));
    }

    /// An enum is its variants and their numbers; an empty one has no
    /// picture.
    #[test]
    fn an_enum_is_its_variants_and_their_numbers() {
        let ForgeDocument::Enum(mut m) = kind("enum") else {
            panic!("an enum")
        };
        let words = enumeration(&m, &EN, page()).expect("draws").remove(0).sheet;
        let words = words.words();
        for expected in [
            "enum: variants",
            "enum (uint8)",
            "name",
            "value",
            "ok",
            "error",
            "timeout",
            "0",
            "1",
            "2",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        m.variants.clear();
        assert!(enumeration(&m, &EN, page()).expect("draws").is_empty());
    }

    /// An event schema is its fields and their types.
    #[test]
    fn an_event_schema_is_its_fields_and_types() {
        let ForgeDocument::EventSchema(m) = kind("event-schema") else {
            panic!("an event schema")
        };
        let sheet = event_schema(&m, &EN, page())
            .expect("draws")
            .remove(0)
            .sheet;
        let words = sheet.words();
        for expected in [
            "event-schema: payload",
            "job.completed",
            "field",
            "type",
            "elapsed_ms",
            "uint32",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert!(
            !words.contains(&"max_size"),
            "no bound column when none is bounded"
        );
    }

    /// A long lookup continues across pages, each file numbered, every
    /// entry once.
    #[test]
    fn a_long_lookup_is_numbered_across_files_with_every_entry_once() {
        let ForgeDocument::Lookup(mut m) = kind("lookup") else {
            panic!("a lookup")
        };
        let template = m.entries[0].clone();
        m.entries = (0..400)
            .map(|i| {
                let mut e = template.clone();
                e.key = format!("k{i}");
                e.value = format!("v{i}");
                e
            })
            .collect();
        let pictures = lookup(&m, &EN, page()).expect("draws");
        assert!(pictures.len() > 1, "400 rows need more than one file");
        let stems: Vec<&str> = pictures.iter().map(|p| p.stem.as_str()).collect();
        assert_eq!(stems[0], "mapping");
        assert_eq!(stems[1], "mapping-2");
        let keys: usize = pictures
            .iter()
            .flat_map(|p| p.sheet.words())
            .filter(|w| w.starts_with('k') && w[1..].parse::<u32>().is_ok())
            .count();
        assert_eq!(keys, 400);
    }
}
