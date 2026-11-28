// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which functions of a generated set a document of the same set calls, which
//! is what decides whether a Rust function is hinted `#[inline]` (SCE_FORGE.md
//! §4.12).
//!
//! A set is generated into one tree, whose modules name one another as
//! `super::{module}`, and is built as one crate. A function another document of
//! the set calls has its caller in that crate, and there the hint buys nothing:
//! measured 2026-11-28 on a release library, the leaves an algorithm of a set
//! called were inlined into it with or without the hint (the same calls left in
//! it, their own copies the same size), and hinting them changed only the
//! caller's register allocation (403 instructions to 435), which grew it by 161
//! bytes. The hint is for a caller in another crate, which sees no body it is
//! not handed: a runtime or a host. So a leaf carries it only where no document
//! of its set calls it, an entry point of the set.
//!
//! Like the reads of a set's bodies ([`crate::forge::body_reads`]), it is a
//! property of a SET: a document does not know which documents import it. It is
//! read once from the walk that finds the set ([`crate::import_closure::walk`],
//! [`SetCalls::of_units`]) and handed to the generation of each of its
//! documents ([`crate::ForgeCompileOptions::set_calls`]). Empty (the default)
//! is a document generated alone, a set of one: nothing of the set calls it, and
//! a leaf keeps the hint, since a caller in another crate is the one caller a
//! document generated alone can assume.
//!
//! A document of the set calls what it imports: the walk reads an
//! `<sce:import>` wherever it is written, a statechart's and an in-line child's
//! among them, and a document's own generation is what drops an import it never
//! names. Read from the import, and not from what each body names, the answer
//! is one property of the closure: a document is generated alike in every set
//! that reaches it through an import, so two generations of one tree that both
//! reach it write it with the same bytes.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::import_closure::{walk, Unit, WalkOptions};

/// The documents of a generated set whose function another document of the
/// set calls, each by its name: the name the set writes its module under.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetCalls {
    called: BTreeSet<String>,
}

impl SetCalls {
    /// Whether a document of the set calls the function the document named
    /// `document` is called by
    /// ([`crate::forge::generator::StatelessFunction`]): a transform's first
    /// output, a condition's, a lookup's or an algorithm's function.
    pub fn calls(&self, document: &str) -> bool {
        self.called.contains(document)
    }

    /// Whether the set calls no function of its own, as a document generated
    /// alone does.
    pub fn is_empty(&self) -> bool {
        self.called.is_empty()
    }

    /// Of the set whose documents are `units`, as [`walk`] found them: each
    /// unit a caller reaches by a free function that an `<sce:import>` of a
    /// document of the set names.
    pub fn of_units(units: &[Unit]) -> Self {
        Self {
            called: units
                .iter()
                .filter(|unit| unit.imported && unit.callable)
                .map(|unit| unit.name.clone())
                .collect(),
        }
    }

    /// Of the set of `documents` and every document they reach, as
    /// `generate --with-imports` walks them ([`walk`]), their fragments
    /// expanded from `include_dirs`. A set the walk refuses calls nothing here:
    /// its generation is what refuses it, and a function the census misses only
    /// keeps the hint it has alone.
    pub fn of_documents(documents: &[String], include_dirs: &[PathBuf]) -> Self {
        walk(
            documents,
            WalkOptions {
                include_dirs,
                input_root: None,
            },
        )
        .map(|units| Self::of_units(&units))
        .unwrap_or_default()
    }
}
