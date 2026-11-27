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
    /// Why the codec is written by its printed encode rather than by the
    /// runtime's one writer over the table (section 3); `None` where the
    /// writer writes it.
    pub not_written: Option<NotWrittenByTable>,
    /// Why the codec's view is printed rather than read by the table
    /// ([`NotATable::OwnView`]), where the build generates views and the view
    /// is a type of its own; `None` where the build has no views or the view
    /// is the codec's value itself, which the table reads.
    pub view: Option<NotATable>,
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
/// (docs/adr/0155 section 2): its shape, or the first field no row states;
/// or why its view is printed where its value is read by its table
/// ([`SetTable::view`]).
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
    /// The codec's view is a type of its own, which its read builds once as a
    /// value from the locals its fields are read into, with no default of it
    /// made first (docs/adr/0137); the walker stores into a value it is
    /// handed, which would be that default. The view is printed, and the
    /// codec's value is read by its table.
    OwnView,
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
    /// It is gated by no earlier carrier.
    GatedByNoCarrier,
    /// It is gated by a carrier a gate decides.
    GatedByGatedCarrier,
    /// It is gated by no flag of its carrier.
    GatedByNoFlag,
    /// It is gated by no input of the codec.
    GatedByNoInput,
    /// It is gated by a kind of a chain's entries no bit of the chain's row
    /// states.
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
            Self::OwnView => f.write_str(
                "the codec's view is a type of its own, built once as a value from the locals \
                 its fields are read into, with no default of it made first",
            ),
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
                    UnstatedField::GatedByChain => {
                        f.write_str("is gated by a kind of a chain no bit of its row states")
                    }
                }
            }
        }
    }
}

/// Why a cold codec read by its table is written by its printed encode rather
/// than by the runtime's one writer over the table (docs/adr/0155 section 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotWrittenByTable {
    /// A row is gated by a disjunction (`a || b`), whose flags do not say
    /// alone whether its field is present: a value can set one of them and
    /// hold the field absent, which the printed encode refuses or derives as
    /// its plan says, and which the writer, opening the row by the flags,
    /// would write as a zero.
    Disjunction,
    /// Its encode can refuse a value, and the writer's one error is the
    /// sink's.
    MayRefuse,
    /// A gate or a body reads the kinds of a chain's entries, which the read
    /// of the chain answers and no source of the value says.
    ReadsKinds,
    /// A parent sets its flags, and its write lowers to its parts.
    LowersToParts,
    /// A parent sets the flag `flag` of `field`, which the writer derives or
    /// which is no number the value says.
    ParentFlag {
        /// The carrier.
        field: String,
        /// The flag.
        flag: String,
    },
    /// The value says an input of the codec.
    SaysInput,
    /// It holds a group or a variant.
    GroupOrVariant,
    /// A row of its table is no field of it.
    RowNotField,
    /// Its encode writes a positional layout.
    PositionalLayout,
    /// The number its encode derives at `element` is no derive of a row.
    Derive {
        /// The element derived, as the encode plan names it.
        element: String,
        /// What no row states of it.
        form: UnstatedDerive,
    },
    /// The field `field` is written by nothing the writer hands it.
    Field {
        /// The field's id.
        field: String,
        /// What the writer does not hand it.
        form: UnwrittenField,
    },
}

/// A number an encode derives that no derive of a row states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnstatedDerive {
    /// A flag says the presence of several fields.
    SeveralFields,
    /// A flag is set where its field is absent.
    SetWhereAbsent,
    /// It is derived from or into a field no row reads.
    NoRow,
    /// It is derived into a field no number row reads.
    NoNumberRow,
    /// It is no flag of its carrier.
    NoFlag,
    /// It is derived from several measures.
    SeveralMeasures,
    /// It is derived from a measure no row states.
    UnstatedMeasure,
    /// It is derived into a flag.
    IntoFlag,
    /// It may not hold the count it is derived from.
    Narrowed,
    /// It holds what the value states where its content is absent.
    StatedWhereAbsent,
    /// It counts a field its row is not counted by.
    CountsOtherRow,
    /// It is derived as the number of no row.
    NoRowNumber,
    /// It is derived twice.
    Twice,
}

/// A field the runtime's one writer hands no value of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnwrittenField {
    /// A number the value does not say and the encode does not derive whole.
    NumberNotSaid,
    /// Bytes the value does not say.
    BytesNotSaid,
    /// Held by a type whose bound is not within its row's.
    TypeBound,
    /// Its presence is no accessor's of the value.
    PresenceNotSaid,
    /// Counted by a number the value states.
    CountStated,
    /// Written by no write of its own.
    NoWrite,
    /// Its write reads the field `named`, which the encode derives and its row
    /// does not hand.
    ReadsDerived {
        /// The field read.
        named: String,
    },
    /// Its write reads the input `named`, which its row does not hand.
    ReadsInput {
        /// The input read.
        named: String,
    },
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
    /// the set's order, as each will be generated: [`Self::of_compiled`] of
    /// one value of options for every document.
    pub fn of_documents(documents: &[String], options: &ForgeCompileOptions) -> Self {
        let compiled: Vec<(String, ForgeCompileOptions)> = documents
            .iter()
            .map(|document| (document.clone(), options.clone()))
            .collect();
        Self::of_compiled(&compiled)
    }

    /// Of the set of `documents`, each generated for Rust in the set's order
    /// under the options it is paired with, the options its own generation
    /// compiles it with (deploy capacities and source hash among them), so
    /// the rows read here are the rows each module prints: the tables they
    /// print, read as [`Self::of`] reads them. A document that does not
    /// generate prints no table here, and is refused where it is generated.
    pub fn of_compiled(documents: &[(String, ForgeCompileOptions)]) -> Self {
        let mut tables = Vec::new();
        for (document, options) in documents {
            let mut census = options.clone();
            census.shared_tables = Self::default();
            let path = Path::new(document);
            let Ok(loaded) = crate::load_forge_source(path, &census.include_dirs) else {
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
            not_written: None,
            view: None,
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

    /// Each document of a set is read under the options it is paired with, as
    /// its own generation compiles it: two codecs of the same rows are one
    /// table where both are read by tables, and none where the build of one
    /// prints its cold codecs.
    #[test]
    fn each_document_is_read_under_the_options_it_is_paired_with() {
        use crate::forge::body_reads::{BodyReadSites, ColdCodecs, InlineDecode};
        const GATED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="NAME">
  <datamodel>
    <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="P" bit="0"/>
    </sce:flags>
    <sce:field id="count" sce:type="uint64" sce:byte="1" sce:bit-size="vle"
               sce:present-if="flags.P"/>
    <sce:field id="content" sce:type="bytes" sce:byte="2" sce:bit-size="length-ref"
               sce:length-field="count" sce:max-size="64" sce:present-if="flags.P"/>
  </datamodel>
</scxml>
"#;
        const ROOT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_paired_root">
  <datamodel>
    <sce:field id="value" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
  </datamodel>
</scxml>
"#;
        let dir = std::env::temp_dir().join(format!("sce_paired_tables_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the set");
        let names = ["codec_paired_root", "codec_paired_a", "codec_paired_b"];
        let documents: Vec<String> = names
            .iter()
            .map(|name| {
                let path = dir.join(format!("{name}.scxml"));
                let text = if *name == "codec_paired_root" {
                    ROOT.to_string()
                } else {
                    GATED.replace("NAME", name)
                };
                std::fs::write(&path, text).expect("a document of the set");
                path.display().to_string()
            })
            .collect();
        let sites = BodyReadSites::of_documents(&documents, &[], |_| false);
        let hot = sites
            .hot_codecs(["codec_paired_root"])
            .unwrap_or_else(|e| panic!("codec_paired_root is a codec of the set: {e}"));
        let tables = ForgeCompileOptions {
            inline_decode: InlineDecode::Codecs(hot),
            body_reads: sites,
            ..ForgeCompileOptions::default()
        };
        let printed = ForgeCompileOptions {
            cold_codecs: ColdCodecs::Printed,
            ..tables.clone()
        };
        let (a, b) = (documents[1].clone(), documents[2].clone());
        let same =
            SharedTables::of_compiled(&[(a.clone(), tables.clone()), (b.clone(), tables.clone())]);
        let apart = SharedTables::of_compiled(&[(a, printed), (b, tables)]);
        std::fs::remove_dir_all(&dir).expect("the set's directory is removed");
        assert_eq!(
            same.homes.get("codec_paired_a"),
            Some(&TableHome::Lent),
            "{same:?}"
        );
        assert!(
            matches!(
                same.homes.get("codec_paired_b"),
                Some(TableHome::Borrowed { .. })
            ),
            "{same:?}"
        );
        assert!(
            apart.homes.is_empty(),
            "a printed codec lends a table: {apart:?}"
        );
    }

    /// A codec whose rows are not the rows the set found keeps its own table.
    #[test]
    fn a_codec_of_other_rows_than_the_set_found_keeps_its_own() {
        let shared = SharedTables::of(&[table("a", "rows 1"), table("c", "rows 1")]);
        assert_eq!(shared.home("c", "rows 2"), TableHome::Own);
    }
}
