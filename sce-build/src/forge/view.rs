// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A view (docs/adr/0151): a read model, a relational query over a machine's
//! bounded keyed tables, or a document of relations, the rewrites a viewer's
//! relations are derived by. The parser reads a view document into its model
//! ([`crate::forge::parser`]).
//!
//! No backend lowers a view yet: Rust is its lane (D10), once the operators of
//! its forge runtime land, and every other backend and the Interpreter refuse
//! it by name ([`backend_unsupported`]).

use crate::forge::error::GenerateError;
use crate::forge::model::ViewModel;
use crate::generator::Language;

/// The refusal of a view document on `language` (docs/adr/0151, D10):
/// `view/backend-unsupported`, by name, after the document is read.
pub fn backend_unsupported(view: &ViewModel, language: Language) -> GenerateError {
    GenerateError::ViewBackendUnsupported {
        view: view.name.clone(),
        language: crate::forge::codegen_matrix::language_wire_name(language).to_string(),
    }
}
