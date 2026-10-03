// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The field-by-field reading of a document of a non-statechart kind: every
//! value the document states, once, in a table.
//!
//! # Why it is total
//!
//! The reading is not written per kind. It walks the document as the
//! forge AST export writes it ([`super::tree`]), so a field a kind gains
//! appears here on the day it is added to the model, and no kind's reading
//! can quietly stop at the fields someone remembered to list. The one
//! omission is the position a node was written at (`source_location`): a
//! line and a column say where to look in a file, and a printed sheet is
//! read away from the file.
//!
//! # What it makes of a value
//!
//! - A record's scalars are one table of `field | value`; its lists and
//!   records are tables of their own, headed by the path that reaches
//!   them (`document.entries[3].guard`) — the path `--emit-ast` and its
//!   schema use, so a sheet and the export can be read against each other.
//! - A list of records is one table, a row for each, a column for each
//!   scalar field in the order the model declares them. When its columns
//!   cannot be set in the page's width, each row is set as a table of its
//!   own instead ([`super::table::Table::fallback`]) — the same values,
//!   the other way up.
//! - A scalar is written as the AST writes it: text as it is, numbers and
//!   booleans as literals, `null`, and `[]` / `{}` for nothing. Text that
//!   could be mistaken for one of those — an empty string, `true`, a
//!   number, text with spaces at an end — is quoted, so two values that
//!   read alike on the sheet are alike in the document.
//!
//! The sheet's own words (the title, the column heads, "continued") come
//! from [`super::words`], so they are the page's language; field names and
//! values are the document's and are never translated.

use super::boxes::BoxError;
use super::fit::{Page, Refusal};
use super::sheet::Sheet;
use super::table::{self, Cell, Table};
use super::tree::{self, Node};
use super::words::{self, Phrase};
use crate::forge::model::ParsedForge;
use crate::forge::page::Lexicon;

/// The fields no sheet carries: where a node was written in its file.
pub const OMITTED_FIELDS: [&str; 1] = ["source_location"];

/// The head of a column of positions.
const INDEX_HEAD: &str = "#";

/// One table's worth of values, before it is set: what the walk found,
/// with the path that reaches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A record's scalar fields, as `(field, value)`.
    Fields {
        path: String,
        rows: Vec<(String, String)>,
    },
    /// A list's scalar items, as `(position, value)`.
    Column {
        path: String,
        rows: Vec<(usize, String)>,
    },
    /// A list of records: a column for each scalar field any of them has.
    Records {
        path: String,
        columns: Vec<String>,
        rows: Vec<RecordRow>,
    },
}

/// One record of a [`Block::Records`]; a column its record does not have is
/// `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRow {
    pub index: usize,
    pub cells: Vec<Option<String>>,
}

/// `parent` and a field named `key` of it, as a path.
pub fn join_field(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{parent}.{key}")
    }
}

/// `parent` and its item `index`, as a path.
pub fn join_index(parent: &str, index: usize) -> String {
    format!("{parent}[{index}]")
}

/// A scalar, or nothing, as one cell says it.
pub fn inline_text(node: &Node) -> String {
    match node {
        Node::Null => "null".to_string(),
        Node::Bool(b) => b.to_string(),
        Node::Number(n) => n.clone(),
        Node::Text(s) if needs_quotes(s) => {
            serde_json::to_string(s).expect("a string has a JSON spelling")
        }
        Node::Text(s) => s.clone(),
        Node::List(_) => "[]".to_string(),
        Node::Record(_) => "{}".to_string(),
    }
}

/// Whether text would read as something other than the text it is.
fn needs_quotes(s: &str) -> bool {
    s.is_empty()
        || s != s.trim()
        || matches!(s, "null" | "true" | "false" | "[]" | "{}")
        || s.starts_with('"')
        || serde_json::from_str::<serde_json::Number>(s).is_ok()
}

/// The blocks of `root`, in document order. `root` is the document with
/// [`OMITTED_FIELDS`] already dropped ([`Node::without`]).
pub fn blocks(root: &Node) -> Vec<Block> {
    let mut out = Vec::new();
    if root.is_inline() {
        out.push(Block::Fields {
            path: String::new(),
            rows: vec![(String::new(), inline_text(root))],
        });
    } else {
        walk(root, "", &mut out);
    }
    out
}

/// `node`, which has something in it, at `path`.
fn walk(node: &Node, path: &str, out: &mut Vec<Block>) {
    match node {
        Node::Record(fields) => {
            let rows: Vec<(String, String)> = fields
                .iter()
                .filter(|(_, v)| v.is_inline())
                .map(|(k, v)| (k.clone(), inline_text(v)))
                .collect();
            if !rows.is_empty() {
                out.push(Block::Fields {
                    path: path.to_string(),
                    rows,
                });
            }
            for (key, value) in fields.iter().filter(|(_, v)| !v.is_inline()) {
                walk(value, &join_field(path, key), out);
            }
        }
        Node::List(items) => list(items, path, out),
        _ => {}
    }
}

fn list(items: &[Node], path: &str, out: &mut Vec<Block>) {
    let records: Option<Vec<&[(String, Node)]>> = items
        .iter()
        .map(|item| match item {
            Node::Record(fields) if !fields.is_empty() => Some(fields.as_slice()),
            _ => None,
        })
        .collect();
    if let Some(records) = records {
        let mut columns: Vec<String> = Vec::new();
        for fields in &records {
            for (key, value) in *fields {
                if value.is_inline() && !columns.contains(key) {
                    columns.push(key.clone());
                }
            }
        }
        if !columns.is_empty() {
            let rows = records
                .iter()
                .enumerate()
                .map(|(index, fields)| RecordRow {
                    index,
                    cells: columns
                        .iter()
                        .map(|column| {
                            fields
                                .iter()
                                .find(|(key, value)| key == column && value.is_inline())
                                .map(|(_, value)| inline_text(value))
                        })
                        .collect(),
                })
                .collect();
            out.push(Block::Records {
                path: path.to_string(),
                columns,
                rows,
            });
        }
        for (index, fields) in records.iter().enumerate() {
            for (key, value) in fields.iter().filter(|(_, v)| !v.is_inline()) {
                walk(value, &join_field(&join_index(path, index), key), out);
            }
        }
        return;
    }

    // Anything else: a run of scalar items is one column, and an item with
    // something in it is read at its own path.
    let mut run: Vec<(usize, String)> = Vec::new();
    let flush = |run: &mut Vec<(usize, String)>, out: &mut Vec<Block>| {
        if !run.is_empty() {
            out.push(Block::Column {
                path: path.to_string(),
                rows: std::mem::take(run),
            });
        }
    };
    for (index, item) in items.iter().enumerate() {
        if item.is_inline() {
            run.push((index, inline_text(item)));
        } else {
            flush(&mut run, out);
            walk(item, &join_index(path, index), out);
        }
    }
    flush(&mut run, out);
}

/// The words a sheet says that the document does not.
struct Words {
    whole: &'static str,
    field: &'static str,
    value: &'static str,
}

fn heading(path: &str, words: &Words) -> Cell {
    if path.is_empty() {
        Cell::prose(words.whole)
    } else {
        Cell::mono(path)
    }
}

fn fields_table(path: &str, rows: &[(String, String)], words: &Words) -> Table {
    Table {
        heading: heading(path, words),
        head: vec![Cell::prose(words.field), Cell::prose(words.value)],
        rows: rows
            .iter()
            .map(|(k, v)| vec![Cell::mono(k.clone()), Cell::mono(v.clone())])
            .collect(),
        fallback: Vec::new(),
    }
}

fn tables(blocks: &[Block], words: &Words) -> Vec<Table> {
    blocks
        .iter()
        .map(|block| match block {
            Block::Fields { path, rows } => fields_table(path, rows, words),
            Block::Column { path, rows } => Table {
                heading: heading(path, words),
                head: vec![Cell::prose(INDEX_HEAD), Cell::prose(words.value)],
                rows: rows
                    .iter()
                    .map(|(i, v)| vec![Cell::mono(i.to_string()), Cell::mono(v.clone())])
                    .collect(),
                fallback: Vec::new(),
            },
            Block::Records {
                path,
                columns,
                rows,
            } => Table {
                heading: heading(path, words),
                head: std::iter::once(Cell::prose(INDEX_HEAD))
                    .chain(columns.iter().map(|c| Cell::mono(c.clone())))
                    .collect(),
                rows: rows
                    .iter()
                    .map(|row| {
                        std::iter::once(Cell::mono(row.index.to_string()))
                            .chain(
                                row.cells
                                    .iter()
                                    .map(|c| Cell::mono(c.clone().unwrap_or_default())),
                            )
                            .collect()
                    })
                    .collect(),
                // The same values, a record at a time: a table of rows
                // cannot be set when it has more columns than the page is
                // wide.
                fallback: rows
                    .iter()
                    .filter_map(|row| {
                        let present: Vec<(String, String)> = columns
                            .iter()
                            .zip(&row.cells)
                            .filter_map(|(k, v)| v.clone().map(|v| (k.clone(), v)))
                            .collect();
                        (!present.is_empty())
                            .then(|| fields_table(&join_index(path, row.index), &present, words))
                    })
                    .collect(),
            },
        })
        .collect()
}

/// `parsed` read field by field, on as many sheets as it takes at the
/// page's minimum type size.
pub fn pages(parsed: &ParsedForge, lexicon: &Lexicon, page: Page) -> Result<Vec<Sheet>, Refusal> {
    let say = |p: Phrase| {
        words::phrase(lexicon, p).ok_or(Refusal::Box(BoxError::NoPhrases(lexicon.name)))
    };
    let words = Words {
        whole: say(Phrase::WholeDocument)?,
        field: say(Phrase::Field)?,
        value: say(Phrase::Value)?,
    };
    let root = tree::of(parsed)
        .map_err(|e| Refusal::NotATree(e.to_string()))?
        .without(&OMITTED_FIELDS);
    let title = format!(
        "{}: {}",
        say(Phrase::FieldTable)?,
        parsed.document.kind().as_attr()
    );
    table::set(
        &title,
        say(Phrase::Continued)?,
        &tables(&blocks(&root), &words),
        page,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn examples() -> Vec<(String, ParsedForge)> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples");
        let mut found = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("kind-examples") {
            let path = entry.expect("entry").path();
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let text = std::fs::read_to_string(&path).expect("reads");
            let label = DocumentLabel::for_input_path(path.to_str().unwrap());
            if let Some(parsed) =
                parse_forge_with_imports(&text, label).unwrap_or_else(|e| panic!("{name}: {e:?}"))
            {
                found.push((name, parsed));
            }
        }
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found
    }

    /// Every value of `node` with the path that reaches it, written
    /// independently of [`blocks`] from the document's tree itself.
    fn values(node: &Node, path: &str, out: &mut Vec<(String, String)>) {
        match node {
            Node::Record(fields) => {
                let visible: Vec<_> = fields
                    .iter()
                    .filter(|(k, _)| !OMITTED_FIELDS.contains(&k.as_str()))
                    .collect();
                if visible.is_empty() {
                    out.push((path.to_string(), "{}".to_string()));
                }
                for (k, v) in visible {
                    values(v, &join_field(path, k), out);
                }
            }
            Node::List(items) if !items.is_empty() => {
                for (i, item) in items.iter().enumerate() {
                    values(item, &join_index(path, i), out);
                }
            }
            scalar => out.push((path.to_string(), inline_text(scalar))),
        }
    }

    /// The values a list of blocks holds, with the paths they sit at.
    fn held(blocks: &[Block]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for block in blocks {
            match block {
                Block::Fields { path, rows } => {
                    out.extend(rows.iter().map(|(k, v)| (join_field(path, k), v.clone())));
                }
                Block::Column { path, rows } => {
                    out.extend(rows.iter().map(|(i, v)| (join_index(path, *i), v.clone())));
                }
                Block::Records {
                    path,
                    columns,
                    rows,
                } => {
                    for row in rows {
                        for (c, cell) in columns.iter().zip(&row.cells) {
                            if let Some(v) = cell {
                                out.push((join_field(&join_index(path, row.index), c), v.clone()));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Every value of every kind's real example is in exactly one table
    /// cell at the path that reaches it, and nothing else is: the reading
    /// is total over the AST export, less the positions.
    #[test]
    fn every_value_of_every_kind_is_in_one_cell_at_its_path() {
        let all = examples();
        assert!(all.len() >= 17, "the forge kinds: {}", all.len());
        for (name, parsed) in &all {
            let tree = tree::of(parsed).expect("reads");
            let mut expected = Vec::new();
            values(&tree, "", &mut expected);
            let mut got = held(&blocks(&tree.clone().without(&OMITTED_FIELDS)));
            expected.sort();
            got.sort();
            assert_eq!(got, expected, "{name}");
            assert!(!expected.is_empty(), "{name} states something");
        }
    }

    /// What the reading leaves out is only ever a position: wherever the
    /// export writes an omitted field, it holds a file, a line and a
    /// column and nothing else.
    #[test]
    fn what_is_omitted_is_only_ever_a_position() {
        fn check(node: &Node, name: &str) {
            match node {
                Node::List(items) => items.iter().for_each(|i| check(i, name)),
                Node::Record(fields) => {
                    for (key, value) in fields {
                        if OMITTED_FIELDS.contains(&key.as_str()) {
                            let Node::Record(parts) = value else {
                                panic!("{name}: {key} is not a record");
                            };
                            for (part, _) in parts {
                                assert!(
                                    ["file", "line", "col"].contains(&part.as_str()),
                                    "{name}: {key} carries {part}"
                                );
                            }
                        } else {
                            check(value, name);
                        }
                    }
                }
                _ => {}
            }
        }
        for (name, parsed) in examples() {
            check(&tree::of(&parsed).expect("reads"), &name);
        }
    }

    #[test]
    fn scalars_read_as_the_ast_writes_them_and_lookalikes_are_quoted() {
        let text = |s: &str| inline_text(&Node::Text(s.to_string()));
        assert_eq!(inline_text(&Node::Null), "null");
        assert_eq!(inline_text(&Node::Bool(true)), "true");
        assert_eq!(inline_text(&Node::Number("1.5".into())), "1.5");
        assert_eq!(inline_text(&Node::List(vec![])), "[]");
        assert_eq!(inline_text(&Node::Record(vec![])), "{}");
        assert_eq!(text("unlocked"), "unlocked");
        assert_eq!(text("a b"), "a b");
        for lookalike in [
            "", "true", "null", "42", "1e3", " pad", "pad ", "[]", "\"q\"",
        ] {
            assert!(text(lookalike).starts_with('"'), "{lookalike:?}");
        }
        assert_eq!(text("true"), "\"true\"");
    }

    /// A list of records is a table; a field some records lack is an empty
    /// cell and not a column of its own; a record's nested fields follow
    /// under their own paths.
    #[test]
    fn a_list_of_records_is_a_table_and_nested_fields_follow() {
        let rec = |fields: Vec<(&str, Node)>| {
            Node::Record(
                fields
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
            )
        };
        let text = |s: &str| Node::Text(s.to_string());
        let root = rec(vec![(
            "entries",
            Node::List(vec![
                rec(vec![("key", text("a")), ("note", text("x"))]),
                rec(vec![
                    ("key", text("b")),
                    ("inner", rec(vec![("deep", text("d"))])),
                ]),
            ]),
        )]);
        let got = blocks(&root);
        assert_eq!(
            got,
            vec![
                Block::Records {
                    path: "entries".into(),
                    columns: vec!["key".into(), "note".into()],
                    rows: vec![
                        RecordRow {
                            index: 0,
                            cells: vec![Some("a".into()), Some("x".into())]
                        },
                        RecordRow {
                            index: 1,
                            cells: vec![Some("b".into()), None]
                        },
                    ],
                },
                Block::Fields {
                    path: "entries[1].inner".into(),
                    rows: vec![("deep".into(), "d".into())],
                },
            ]
        );
    }

    /// Scalars among records, and lists of lists, are read where they are:
    /// a run of scalars is one column, an item with content is read at its
    /// own path.
    #[test]
    fn a_mixed_list_is_read_item_by_item() {
        let text = |s: &str| Node::Text(s.to_string());
        let root = Node::Record(vec![(
            "mixed".into(),
            Node::List(vec![
                text("a"),
                text("b"),
                Node::List(vec![text("c")]),
                text("d"),
            ]),
        )]);
        assert_eq!(
            blocks(&root),
            vec![
                Block::Column {
                    path: "mixed".into(),
                    rows: vec![(0, "a".into()), (1, "b".into())]
                },
                Block::Column {
                    path: "mixed[2]".into(),
                    rows: vec![(0, "c".into())]
                },
                Block::Column {
                    path: "mixed".into(),
                    rows: vec![(3, "d".into())]
                },
            ]
        );
    }

    /// Every kind's example is set on a page in both languages, deterministic
    /// to the byte, and every value's text is on the sheets it was set on.
    #[test]
    fn every_kind_sets_in_both_languages_with_every_value_on_a_sheet() {
        let page = Page::a4_portrait(7.0);
        for (name, parsed) in examples() {
            for lexicon in [&EN, &KO] {
                let sheets =
                    pages(&parsed, lexicon, page).unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(sheets, pages(&parsed, lexicon, page).unwrap(), "{name}");
                let stream: String = sheets
                    .iter()
                    .flat_map(|s| s.words())
                    .flat_map(str::chars)
                    .filter(|c| !c.is_whitespace())
                    .collect();
                let mut expected = Vec::new();
                values(&tree::of(&parsed).unwrap(), "", &mut expected);
                for (path, text) in expected {
                    let wanted: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                    assert!(stream.contains(&wanted), "{name} {path}: {text:?}");
                }
            }
        }
    }
}
