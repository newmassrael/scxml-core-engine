// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The diagnostic family of the formal judges (docs/adr/0146, D1), beside
//! `ForgeError`, `MeshError` and `CliError`: its variants map one to one to
//! the `formal/*` codes, under the stage `formal`.
//!
//! A judge prints one line per run and per problem of the declaration as it
//! goes, so a reader sees which ones; the diagnostic it leaves through is the
//! verdict of the whole manifest. The manifest's path rides `actual` and not
//! the record's key, so the same verdict on another checkout is the same
//! diagnostic.

use crate::forge::diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticPayload, SingleDiagnostic, Stage, ToDiagnostics,
};

/// Why a formal judge did not answer that its manifest holds.
#[derive(Debug, thiserror::Error)]
pub enum FormalError {
    /// The manifest is not as the tree is (`formal/declaration`): something it
    /// does not declare, something it declares that is not there, a part no run
    /// takes out, a property no run shows can fail, a key it does not know, or
    /// a text that is no manifest. Every problem is listed, in the order found.
    #[error("{manifest}: the declaration is not as the tree is: {}", problems.join("; "))]
    Declaration {
        manifest: String,
        problems: Vec<String>,
    },

    /// A run, a proof or a mutant answered otherwise than the manifest
    /// declares (`formal/verdict`). `failures` names each, with what it
    /// answered.
    #[error("{manifest}: {} of {runs} run(s) are not as declared: {}", failures.len(), failures.join("; "))]
    Verdict {
        manifest: String,
        runs: usize,
        failures: Vec<String>,
    },

    /// The judge could not run (`formal/tool-unavailable`): a pinned tool or
    /// its runtime is missing, or is not the one pinned. No verdict about the
    /// manifest.
    #[error("{tool} is not available: {detail}")]
    ToolUnavailable { tool: String, detail: String },

    /// A document carries constructs with no translation into its model over
    /// Appendix D (`formal/no-translation`; docs/adr/0115, D7), so
    /// `sce-codegen model` writes no model of it, never a model of part of it.
    /// `refusals` names each construct and where it is written, in the order
    /// found.
    #[error(
        "{document}: no model is written, since {} construct(s) have no translation into \
         Appendix D: {}",
        refusals.len(),
        refusals.join("; ")
    )]
    NoTranslation {
        document: String,
        refusals: Vec<String>,
    },
}

impl FormalError {
    /// The inputs were read and the answer is no, the class `SCE_ERROR_CONTRACT.md`
    /// §6 gives `check-trace`.
    pub const EXIT_ANSWER_NO: i32 = 1;

    /// The judge could not run (`formal/tool-*`, `SCE_ERROR_CONTRACT.md` §6).
    pub const EXIT_TOOL_UNAVAILABLE: i32 = 15;

    /// A document has no model: the class `SCE_ERROR_CONTRACT.md` §6 gives a
    /// generator's refusal of a construct it does not lower (`generate/*`).
    pub const EXIT_NO_TRANSLATION: i32 = 7;
}

impl ToDiagnostics for FormalError {
    fn exit_code(&self) -> i32 {
        match self {
            FormalError::Declaration { .. } | FormalError::Verdict { .. } => Self::EXIT_ANSWER_NO,
            FormalError::ToolUnavailable { .. } => Self::EXIT_TOOL_UNAVAILABLE,
            FormalError::NoTranslation { .. } => Self::EXIT_NO_TRANSLATION,
        }
    }

    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        vec![self.to_single_diagnostic()]
    }
}

impl SingleDiagnostic for FormalError {
    fn diagnostic_payload(&self) -> DiagnosticPayload {
        let (code, key_fragments, actual) = match self {
            FormalError::Declaration { manifest, problems } => (
                DiagnosticCode::FormalDeclaration,
                problems.clone(),
                Some(manifest.clone()),
            ),
            FormalError::Verdict {
                manifest, failures, ..
            } => (
                DiagnosticCode::FormalVerdict,
                failures.clone(),
                Some(manifest.clone()),
            ),
            FormalError::ToolUnavailable { tool, detail } => (
                DiagnosticCode::FormalToolUnavailable,
                vec![tool.clone()],
                Some(detail.clone()),
            ),
            FormalError::NoTranslation { document, refusals } => (
                DiagnosticCode::FormalNoTranslation,
                refusals.clone(),
                Some(document.clone()),
            ),
        };
        DiagnosticPayload {
            code,
            stage: Stage::Formal,
            expected: None,
            actual,
            fix: None,
            key_fragments,
        }
    }
}
