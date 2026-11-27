// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The formal judges of ADR 0115, as one command of the code generator
//! (docs/adr/0146): `sce-codegen formal models`, `proofs`, `traces`, `toolchain`
//! and `install`, run by any repository on a manifest it is given, this one
//! included.
//!
//! - [`toolchain`]: the tools the judges run, pinned by the judge and compiled
//!   into it (ADR 0146, D3), and how each is fetched and admitted by its digest.
//! - [`tlc`]: the one runner of TLC every level uses (ADR 0146, D4).
//! - [`apalache`]: the one runner of Apalache (ADR 0115, D3; ADR 0146, D4).
//! - [`models`]: the judge of a models manifest (ADR 0115, D5 and D9).
//! - [`inductive`]: the inductive invariants a models manifest declares, and
//!   their judgement for every length of execution (ADR 0115, D3).
//! - [`proofs`]: the judge of a proofs manifest, by Verus (ADR 0115, D10 and D13).
//! - [`isolation`]: the judge of what changes a check, commit by commit over
//!   the ranges it is given (ADR 0115, D11).
//! - [`document_model`]: the model of one document over Appendix D, which
//!   `sce-codegen model` writes (ADR 0115, D7).
//! - [`properties`]: the properties of one machine and their observers over
//!   that model, with the runs that judge them (ADR 0115, D15).
//! - [`traces`]: the judge of a traces manifest, every recorded run of the code
//!   a behaviour of its model (ADR 0115, D4 and D8).
//! - [`error`]: the `formal/*` diagnostics a judge leaves through
//!   (`SCE_ERROR_CONTRACT.md` §6).

pub mod apalache;
pub mod document_model;
pub mod error;
pub mod inductive;
pub mod isolation;
pub mod models;
pub mod proofs;
pub mod properties;
pub mod tlc;
pub mod toolchain;
pub mod traces;

pub use error::FormalError;

/// A fresh directory under the system's temporary directory, removed when the
/// value is dropped: TLC writes its state files beside the module it reads, so
/// each run is given a directory of its own.
pub(crate) struct WorkDir {
    path: std::path::PathBuf,
}

impl WorkDir {
    pub(crate) fn new(purpose: &str) -> Result<Self, FormalError> {
        Self::new_in(&std::env::temp_dir(), purpose)
    }

    /// A fresh directory under `parent`: a tool is unpacked beside the place it
    /// is installed in, so that moving it there is a rename on one file system
    /// and not a copy across two, which a rename cannot do.
    pub(crate) fn new_in(parent: &std::path::Path, purpose: &str) -> Result<Self, FormalError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let path = parent.join(format!(
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

/// Copy every module of `sources` (each `.tla` there) into `into`, where a
/// tool reads them, and write `extra` (name, text) beside them. Both runners
/// stage a run so: each reads the modules of one directory and the files the
/// judge writes for that run.
pub(crate) fn stage_modules(
    sources: &std::path::Path,
    into: &std::path::Path,
    extra: &[(String, String)],
) -> Result<(), FormalError> {
    let copy_failed = |e: std::io::Error| FormalError::ToolUnavailable {
        tool: "a working directory".to_string(),
        detail: format!(
            "cannot copy the modules of {} into {}: {e}",
            sources.display(),
            into.display()
        ),
    };
    for entry in std::fs::read_dir(sources).map_err(copy_failed)? {
        let path = entry.map_err(copy_failed)?.path();
        if path.extension().is_some_and(|ext| ext == "tla") {
            if let Some(name) = path.file_name() {
                std::fs::copy(&path, into.join(name)).map_err(copy_failed)?;
            }
        }
    }
    for (name, text) in extra {
        std::fs::write(into.join(name), text).map_err(copy_failed)?;
    }
    Ok(())
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
