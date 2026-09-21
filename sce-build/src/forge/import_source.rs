// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The document an `<sce:import>` names, read the one way every reader
//! reads it.
//!
//! ⚠ WHY THIS IS ONE PLACE. Six readers resolved, read, labelled and
//! parsed an imported document each on its own: the import enrichment
//! that reports what is wrong with an import, the member-surface table of
//! [`crate::forge::cross_kind_check`], the statechart parser's sibling
//! schemas and enums, and one `variants_of` apiece in
//! [`crate::forge::coverage`], [`crate::forge::retention`] and
//! [`crate::forge::cycle_check`] — three bodies identical but for their
//! comments. Two of the six labelled the document by its path and four by
//! its basename, so what an imported document was called depended on who
//! asked (measured 2026-09-21).
//!
//! The label is the path, resolved against the importing document's
//! directory, because the one reader that reports an imported document's
//! failures — the enrichment — always used it: an imported document is
//! refused under that name, and every other reader stays silent.

use std::path::{Path, PathBuf};

use crate::forge::error::{ForgeError, ImportError, Located};
use crate::forge::model::{ForgeDocument, ForgeImport, ParsedForge};
use crate::DocumentLabel;

/// An imported document's text, and the path it was read from.
pub struct ImportSource {
    pub path: PathBuf,
    pub content: String,
}

impl ImportSource {
    /// Read the document `import` names, resolved against `base_dir` —
    /// the importing document's directory.
    pub fn read(base_dir: &Path, import: &ForgeImport) -> Result<Self, ImportError> {
        let path = base_dir.join(&import.src);
        if !path.exists() {
            return Err(ImportError::FileNotFound {
                src: import.src.clone(),
                searched: path.display().to_string(),
            });
        }
        let content = std::fs::read_to_string(&path).map_err(|source| ImportError::ReadError {
            src: import.src.clone(),
            source,
        })?;
        Ok(Self { path, content })
    }

    /// The label the document parses under: its file stem identifies it,
    /// and its path labels its diagnostics.
    pub fn label(&self) -> DocumentLabel<'_> {
        let identifier = self
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        DocumentLabel {
            identifier,
            diagnostic_label: self.path.to_str().unwrap_or(identifier),
        }
    }

    /// The document parsed, or `None` when it is a plain statechart —
    /// one this pipeline does not own.
    pub fn parse(&self) -> Result<Option<ParsedForge>, Located<ForgeError>> {
        crate::forge::parser::parse_forge_with_imports(&self.content, self.label())
    }
}

/// The document `import` names, parsed — or `None` when it cannot be
/// read, is refused, or is not a forge document.
///
/// ⚠ SILENT BY DESIGN, and only for readers that run where the import
/// enrichment has already had its say: that pass reports a missing,
/// unreadable or refused import, and a second reader saying it again would
/// double-emit. A reader that is the ONLY voice for what it resolves — a
/// cycle's `of=` is one, since the enrichment never looks at it — must
/// turn `None` into its own refusal rather than skip.
pub fn parse_quietly(base_dir: &Path, import: &ForgeImport) -> Option<ParsedForge> {
    ImportSource::read(base_dir, import).ok()?.parse().ok()?
}

/// The variant names the enum imported as `alias` declares, in document
/// order — or `None` when `alias` names no readable enum. Silent as
/// [`parse_quietly`] is, for the same reason.
pub fn enum_variants(parsed: &ParsedForge, base_dir: &Path, alias: &str) -> Option<Vec<String>> {
    let import = parsed.imports.iter().find(|i| i.alias == alias)?;
    match parse_quietly(base_dir, import)?.document {
        ForgeDocument::Enum(e) => Some(e.variants.into_iter().map(|v| v.name).collect()),
        _ => None,
    }
}
