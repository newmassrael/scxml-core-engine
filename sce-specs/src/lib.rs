// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The specs folder: where a specification and what was made of it are kept.
//!
//! A person writes a specification in prose, an AI client turns it into a model through the
//! authoring MCP, and an application (the workbench) shows both side by side. For them to work on
//! the same thing, the thing has to live somewhere none of them owns: a folder of files with one
//! definition of how it is written. This crate is that definition. The MCP reaches it through
//! `sce-spec`; an application links it.
//!
//! - [`store`] is the specs folder: specs, immutable revisions, saves that refuse to overwrite
//!   what the caller has not seen.
//! - [`revision`] is what a revision is: the digest of its bytes.
//! - [`commands`] is the JSON command layer every entrance goes through.
//! - [`figures`] and [`review`] are what SCE says of a model, asked of the product's own
//!   generator: the figures it draws, its check and its pseudocode page.
//! - [`requests`] and [`bundle`] are the asking for a model and what comes of it: a request an
//!   executor holds for a lease, the candidate it writes and the bundle that publishes it.
//! - [`requirements`], [`acceptance`] and [`acceptance_run`] are the requirement list a text is
//!   read into and what the owner accepted of a design against it: the product measures,
//!   records and re-checks, and the specs keep the files.
//!
//! Which AI clients there are, how one is reached and with whose credentials, is not here: that
//! is an application's, and a request made for one carries what the application wrote for it
//! unread ([`requests::Pin`]). Nothing in this crate knows what a specification is about either:
//! it stores text and the history of that text.

pub mod acceptance;
pub mod acceptance_run;
pub mod answers;
pub mod bundle;
pub mod clock;
pub mod commands;
pub mod document_files;
pub mod error;
pub mod figures;
pub mod journal;
pub mod lock;
pub mod model_set;
mod request_life;
pub mod requests;
pub mod requirements;
pub mod review;
pub mod revision;
pub mod revision_report;
pub mod store;
#[cfg(feature = "testing")]
pub mod testing;

pub use acceptance::{Acceptance, Basis, Snapshot};
pub use acceptance_run::{Acceptor, CheckOutcome, RequirementOutcome, RequirementsReport, Taken};
pub use answers::{Answers, AnswersError};
pub use clock::{Clock, FixedClock, ManualClock, SystemClock};
pub use commands::{call, call_with, CommandError, NoPins, Pins, COMMANDS, COMMAND_SET_VERSION};
pub use error::StoreError;
pub use figures::{
    default_renderer, renderer_with_bundle, FigureRenderer, FigureRequest, FigureSet, NoRenderer,
    RenderError, SceCodegen, Sheet,
};
pub use model_set::{Document, ModelError, ModelFiles};
pub use requests::{ConnectionId, ConnectionRef, Pin};
pub use requirements::{Requirements, RequirementsError};
pub use review::{
    Check, ModelReviewer, PageRefusal, Product, Record, Review, ReviewRequest, Unresolved, Verdict,
};
pub use revision::Revision;
pub use store::{
    default_root, AcceptanceText, Adapter, AdapterListing, AdapterReport, AdapterStatus,
    AnswersText, BundleRead, CandidateTexts, CandidateWrite, ClaimedHead, HistoryEntry, Host,
    HostListing, HostReport, HostStatus, HostWaiting, LaneWait, Listing, ModelText, Published,
    Registered, Registration, RequestHead, RequestView, RequirementsText, Saved, SourceText, Spec,
    SpecHeads, SpecId, SpecSnapshot, SpecStore, Transition, Unreadable, ADAPTER_LIVE_SECONDS,
    MAX_ACCEPTANCE_BYTES, MAX_ANSWERS_BYTES, MAX_BUNDLE_BYTES, MAX_MODEL_BYTES,
    MAX_REQUIREMENTS_BYTES, MAX_SOURCE_BYTES, WAITING_MAX,
};
