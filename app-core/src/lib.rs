// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The part of the specification workbench that has no screen.
//!
//! A person writes a specification in prose, an AI client turns it into a model
//! through the authoring MCP, and the workbench shows both side by side. For the
//! three of them (the app, the MCP, the browser shell used during development) to
//! work on the same thing, the thing has to live somewhere none of them owns: a
//! folder of files with one definition of how it is written. This crate is that
//! definition. (The MCP reaches it through `sce-work`.)
//!
//! - [`store`] is the works folder: works, immutable revisions, saves that
//!   refuse to overwrite what the caller has not seen.
//! - [`revision`] is what a revision is: the digest of its bytes.
//! - [`commands`] is the JSON command layer every entrance goes through.
//! - [`figures`] and [`review`] are what SCE says of a model, asked of the
//!   product's own generator: the figures it draws, its check and its pseudocode
//!   page. The workbench works none of it out.
//!
//! Nothing in this crate knows what a specification is about. It stores text and
//! the history of that text; what the text means is the authoring tools' business.

pub mod clock;
pub mod commands;
pub mod error;
pub mod figures;
mod lock;
pub mod review;
pub mod revision;
pub mod store;

pub use clock::{Clock, FixedClock, SystemClock};
pub use commands::{call, CommandError, COMMANDS, COMMAND_SET_VERSION};
pub use error::StoreError;
pub use figures::{
    default_renderer, FigureRenderer, FigureRequest, FigureSet, NoRenderer, RenderError,
    SceCodegen, Sheet,
};
pub use review::{
    Check, ModelReviewer, PageRefusal, Product, Record, Review, ReviewRequest, Unresolved, Verdict,
};
pub use revision::Revision;
pub use store::{
    default_root, HistoryEntry, Listing, ModelText, Saved, SourceText, Unreadable, Work, WorkId,
    WorkStore, MAX_MODEL_BYTES, MAX_SOURCE_BYTES,
};
