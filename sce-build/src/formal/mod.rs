// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The formal judges of ADR 0115, as one command of the code generator
//! (docs/adr/0146): `sce-codegen formal models`, `proofs`, `toolchain` and
//! `install`, run by any repository on a manifest it is given, this one included.
//!
//! - [`toolchain`]: the tools the judges run, pinned by the judge and compiled
//!   into it (ADR 0146, D3), and how each is fetched and admitted by its digest.
//! - [`tlc`]: the one runner of TLC every level uses (ADR 0146, D4).
//! - [`models`]: the judge of a models manifest (ADR 0115, D5 and D9).
//! - [`proofs`]: the judge of a proofs manifest, by Verus (ADR 0115, D10 and D13).
//! - [`document_model`]: the model of one document over Appendix D, which
//!   `sce-codegen model` writes (ADR 0115, D7).
//! - [`error`]: the `formal/*` diagnostics a judge leaves through
//!   (`SCE_ERROR_CONTRACT.md` §6).

pub mod document_model;
pub mod error;
pub mod models;
pub mod proofs;
pub mod tlc;
pub mod toolchain;

pub use error::FormalError;

/// A fresh directory under the system's temporary directory, removed when the
/// value is dropped: TLC writes its state files beside the module it reads, so
/// each run is given a directory of its own.
pub(crate) struct WorkDir {
    path: std::path::PathBuf,
}

impl WorkDir {
    pub(crate) fn new(purpose: &str) -> Result<Self, FormalError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let path = std::env::temp_dir().join(format!(
            "sce-formal-{purpose}-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).map_err(|e| FormalError::ToolUnavailable {
            tool: "a working directory".to_string(),
            detail: format!("cannot create {}: {e}", path.display()),
        })?;
        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
