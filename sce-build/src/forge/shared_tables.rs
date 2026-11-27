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
/// (`GeneratedOutput::cold_codecs`).
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

/// What the generation of one cold codec of a set says of its table
/// (docs/adr/0155 section 2): the table its module prints, or why it is
/// printed, as the printers of docs/adr/0081 answer the forms they lower.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColdCodec {
    /// The codec is read by the runtime's one walker over this table.
    Table(SetTable),
    /// The codec is printed, for the reason `why`.
    Printed {
        /// The codec.
        codec: String,
        /// Why no table states it.
        why: NotATable,
    },
}

/// Why a cold codec is printed rather than read by a table of its fields
/// (docs/adr/0155 section 2): its shape, or the first field no row states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotATable {
    /// The codec's shape is no table's.
    Codec(CodecShape),
    /// The field `field`, the first in wire order no row states, is of the
    /// form `form`.
    Field {
        /// The field's id.
        field: String,
        /// What no row states of it.
        form: UnstatedField,
    },
}

/// A shape of a codec no table states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecShape {
    /// Its fields are at fixed offsets, read in place and by no stream.
    FixedLayout,
    /// It is not positional.
    NotPositional,
    /// It dispatches a variant.
    Variant,
    /// It has no field.
    NoField,
    /// It has more fields than a table has rows.
    TooManyFields,
    /// A gate group holds fields of it as one.
    GateGroup,
    /// A field has no context the generation states.
    NoContext,
}

/// The form of a field no row of a table states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnstatedField {
    /// A gate group holds it.
    HeldByGateGroup,
    /// It is placed or measured by a form of its own: a DMA alignment, a
    /// length's arithmetic, an embedded body's length, a quantity.
    OwnForm,
    /// It is no unsigned integer.
    NotUnsigned,
    /// The generation prints no read of it.
    NoPrintedRead,
    /// Its read refuses a value: a range, a conversion.
    RefusesValue,
    /// It is in the native byte order.
    NativeOrder,
    /// It is a number of a width no row reads.
    NumberWidth {
        /// Its width, as the document spells it.
        bits: String,
    },
    /// It is counted, and is no byte string or text.
    NotBytesOrText,
    /// It is counted by no field.
    CountedByNoField,
    /// It is counted by no earlier number.
    CountedByNoEarlierNumber,
    /// It is gated otherwise than the number that counts it.
    GatedOtherwiseThanCount,
    /// Its length has no bound.
    NoLengthBound,
    /// It is held to a least length.
    LeastLength,
    /// It is a body whose read names the field `named`, which no number of a
    /// row a body is handed states.
    NamesUnstatedNumber {
        /// The field the read names.
        named: String,
    },
    /// It is a body whose read names more numbers than a row hands.
    NamesTooManyNumbers,
    /// It is the bytes left in the frame.
    Tail,
    /// It is a byte string or a text of a value no storage profile holds.
    NoStorageProfile,
    /// It is gated by several clauses.
    SeveralClauses,
    /// It is gated by no earlier carrier.
    GatedByNoCarrier,
    /// It is gated by a carrier a gate decides.
    GatedByGatedCarrier,
    /// It is gated by no flag of its carrier.
    GatedByNoFlag,
    /// It is gated by no input of the codec.
    GatedByNoInput,
    /// It is gated by the entries of a chain.
    GatedByChain,
}

impl std::fmt::Display for NotATable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(shape) => f.write_str(match shape {
                CodecShape::FixedLayout => "the codec is read in place at fixed offsets",
                CodecShape::NotPositional => "the codec is not positional",
                CodecShape::Variant => "the codec dispatches a variant",
                CodecShape::NoField => "the codec has no field",
                CodecShape::TooManyFields => "the codec has more fields than a table has rows",
                CodecShape::GateGroup => "a gate group holds fields of the codec as one",
                CodecShape::NoContext => "a field has no context",
            }),
            Self::Field { field, form } => {
                write!(f, "field '{field}' ")?;
                match form {
                    UnstatedField::HeldByGateGroup => f.write_str("is held by a gate group"),
                    UnstatedField::OwnForm => f.write_str("has a form no row states"),
                    UnstatedField::NotUnsigned => f.write_str("is no unsigned integer"),
                    UnstatedField::NoPrintedRead => f.write_str("has no printed read"),
                    UnstatedField::RefusesValue => {
                        f.write_str("is read by a read that refuses a value")
                    }
                    UnstatedField::NativeOrder => f.write_str("is in the native byte order"),
                    UnstatedField::NumberWidth { bits } => {
                        write!(f, "is a number of {bits} bits no row reads")
                    }
                    UnstatedField::NotBytesOrText => f.write_str("is a length of no bytes or text"),
                    UnstatedField::CountedByNoField => f.write_str("is counted by no field"),
                    UnstatedField::CountedByNoEarlierNumber => {
                        f.write_str("is counted by no earlier number")
                    }
                    UnstatedField::GatedOtherwiseThanCount => {
                        f.write_str("is gated otherwise than its count")
                    }
                    UnstatedField::NoLengthBound => f.write_str("has no length bound"),
                    UnstatedField::LeastLength => f.write_str("is held to a least length"),
                    UnstatedField::NamesUnstatedNumber { named } => write!(
                        f,
                        "is read by a statement that names '{named}', which no number states"
                    ),
                    UnstatedField::NamesTooManyNumbers => f.write_str(
                        "is read by a statement that names more numbers than a row hands",
                    ),
                    UnstatedField::Tail => f.write_str("is the bytes left in the frame"),
                    UnstatedField::NoStorageProfile => f.write_str("is held by no storage profile"),
                    UnstatedField::SeveralClauses => f.write_str("is gated by several clauses"),
                    UnstatedField::GatedByNoCarrier => {
                        f.write_str("is gated by no earlier carrier")
                    }
                    UnstatedField::GatedByGatedCarrier => {
                        f.write_str("is gated by a carrier a gate decides")
                    }
                    UnstatedField::GatedByNoFlag => {
                        f.write_str("is gated by no flag of its carrier")
                    }
                    UnstatedField::GatedByNoInput => {
                        f.write_str("is gated by no input of the codec")
                    }
                    UnstatedField::GatedByChain => f.write_str("is gated by a chain's entries"),
                }
            }
        }
    }
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
            tables.extend(
                output
                    .cold_codecs
                    .into_iter()
                    .filter_map(|cold| match cold {
                        ColdCodec::Table(table) => Some(table),
                        ColdCodec::Printed { .. } => None,
                    }),
            );
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
