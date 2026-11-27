// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The field tables of a set's cold codecs that are one table (docs/adr/0155
//! section 4).
//!
//! Two cold codecs of a set whose tables are the same rows, row for row, and
//! whose writes derive the same numbers, name one `static`: the module of the
//! first of them in the set's order writes it, and each other names it by its
//! path in the set. Their types stay theirs, and each keeps its own stores.
//!
//! Like the reads of a set's bodies ([`crate::forge::body_reads`]), it is a
//! property of a SET: a document does not know which documents are generated
//! with it. The set is generated once to read the table each cold codec prints
//! ([`SharedTables::of_documents`]), and the same answer is handed to the
//! generation of each of its documents
//! ([`crate::ForgeCompileOptions::shared_tables`]). Empty (the default) is a
//! set whose tables are each its codec's own, which is every set that names no
//! hot root: no golden moves.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::generator::Language;
use crate::{DocumentLabel, ForgeCompileOptions};

/// The field table a cold codec's module prints, as a set reads it
/// (`GeneratedOutput::field_tables`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetTable {
    /// The codec.
    pub codec: String,
    /// The module of the set that holds the codec, by which the set's other
    /// modules name what it declares (`super::{module}`).
    pub module: String,
    /// The name of the codec's `static` table.
    pub name: String,
    /// What the table is: its rows and the numbers its write derives, as the
    /// `static` holds them, with no name of the codec's.
    pub rows: String,
}

/// Where the table of one cold codec of a set is written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TableHome {
    /// In the codec's own module, which no other codec names: a private
    /// `static`.
    #[default]
    Own,
    /// In the codec's own module, and named by a later codec of the set: a
    /// `static` the modules of the set see.
    Lent,
    /// In the module of an earlier codec of the set, at `path`, whose rows are
    /// `rows`.
    Borrowed {
        /// The path of the earlier codec's `static` in the set.
        path: String,
        /// Its rows, which are this codec's.
        rows: String,
    },
}

/// Where the table of each cold codec of a set is written: the codecs whose
/// tables are an earlier codec's name that codec's, and only they and the
/// codecs they name are in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SharedTables {
    homes: BTreeMap<String, TableHome>,
}

impl SharedTables {
    /// Of the tables a set prints, in the set's order: a codec whose rows are
    /// an earlier codec's names that codec's table, and the first codec of
    /// those rows lends its own. A codec printed twice (a document of the set
    /// that another imports) is read where it is first.
    pub fn of(tables: &[SetTable]) -> Self {
        let mut owners: BTreeMap<&str, &SetTable> = BTreeMap::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut homes = BTreeMap::new();
        for table in tables {
            if !seen.insert(table.codec.as_str()) {
                continue;
            }
            match owners.get(table.rows.as_str()) {
                Some(owner) => {
                    homes.insert(owner.codec.clone(), TableHome::Lent);
                    homes.insert(
                        table.codec.clone(),
                        TableHome::Borrowed {
                            path: format!("super::{}::{}", owner.module, owner.name),
                            rows: table.rows.clone(),
                        },
                    );
                }
                None => {
                    owners.insert(table.rows.as_str(), table);
                }
            }
        }
        Self { homes }
    }

    /// Of the set of `documents`, each generated for Rust under `options` in
    /// the set's order, as each will be generated: the tables they print, read
    /// as [`Self::of`] reads them. A document that does not generate prints no
    /// table here, and is refused where it is generated.
    pub fn of_documents(documents: &[String], options: &ForgeCompileOptions) -> Self {
        let mut census = options.clone();
        census.shared_tables = Self::default();
        let mut tables = Vec::new();
        for document in documents {
            let path = Path::new(document);
            let Ok(loaded) = crate::load_forge_source(path, &options.include_dirs) else {
                continue;
            };
            let base = path.parent().unwrap_or_else(|| Path::new(""));
            let spelled = path.to_string_lossy();
            let Ok(output) = crate::compile_forge_with_imports(
                loaded.text(),
                DocumentLabel::for_input_path(&spelled),
                Language::Rust,
                base,
                &census,
            ) else {
                continue;
            };
            tables.extend(output.field_tables);
        }
        Self::of(&tables)
    }

    /// Where the table of `codec` is written, its rows being `rows`: an
    /// earlier codec's only where its rows are the rows the set found.
    pub fn home(&self, codec: &str, rows: &str) -> TableHome {
        match self.homes.get(codec) {
            Some(TableHome::Borrowed { rows: found, .. }) if found != rows => TableHome::Own,
            Some(home) => home.clone(),
            None => TableHome::Own,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(codec: &str, rows: &str) -> SetTable {
        SetTable {
            codec: codec.to_string(),
            module: codec.to_string(),
            name: format!("{}_FIELDS", codec.to_uppercase()),
            rows: rows.to_string(),
        }
    }

    /// The first codec of a set with rows another has lends its table, each
    /// later one names it, and a codec of rows no other has keeps its own.
    #[test]
    fn a_later_codec_of_the_same_rows_names_the_first_ones_table() {
        let shared = SharedTables::of(&[
            table("a", "rows 1"),
            table("b", "rows 2"),
            table("c", "rows 1"),
            table("d", "rows 1"),
            table("a", "rows 1"),
        ]);
        assert_eq!(shared.home("a", "rows 1"), TableHome::Lent);
        assert_eq!(shared.home("b", "rows 2"), TableHome::Own);
        for later in ["c", "d"] {
            assert_eq!(
                shared.home(later, "rows 1"),
                TableHome::Borrowed {
                    path: "super::a::A_FIELDS".to_string(),
                    rows: "rows 1".to_string(),
                },
                "{later}"
            );
        }
        assert_eq!(shared.home("e", "rows 1"), TableHome::Own);
    }

    /// A codec whose rows are not the rows the set found keeps its own table.
    #[test]
    fn a_codec_of_other_rows_than_the_set_found_keeps_its_own() {
        let shared = SharedTables::of(&[table("a", "rows 1"), table("c", "rows 1")]);
        assert_eq!(shared.home("c", "rows 2"), TableHome::Own);
    }
}
