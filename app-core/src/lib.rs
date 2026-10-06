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
//! - [`requests`], [`bundle`] and [`runner`] are the asking for a model and what comes of
//!   it: a request an executor holds for a lease, the candidate it writes and the bundle
//!   that publishes it, and the host that takes requests for a generator.
//! - [`connection`] and its store are the ways a person reaches a model (which client, which
//!   model, where its credentials come from), kept in their own settings folder and not
//!   with the works, which are shared and moved.
//! - [`auth_policy`] is which ways of signing in the workbench uses: a table of routes, each
//!   allowed, conditional, forbidden or unconfirmed, as a constant of the build.
//! - [`directory`] finds the generator for the connection a request was made for, and
//!   [`claude_status`] is what a screen says of the Claude Code a person has.
//! - [`requirements`], [`acceptance`] and [`acceptance_run`] are the requirement list
//!   a text is read into and what the owner accepted of a design against it: the
//!   product measures, records and re-checks, and the workbench keeps the files and
//!   asks.
//!
//! Nothing in this crate knows what a specification is about. It stores text and
//! the history of that text; what the text means is the authoring tools' business.

pub mod acceptance;
pub mod acceptance_run;
pub mod answers;
pub mod auth_policy;
pub mod bundle;
pub mod claude_code;
pub mod claude_status;
mod client_find;
mod client_run;
pub mod clock;
pub mod codex;
pub mod codex_environment;
pub mod codex_status;
pub mod codex_support;
pub mod commands;
pub mod connection;
pub mod directory;
pub mod error;
pub mod figures;
pub mod host;
pub mod http_client;
pub mod installed;
mod lock;
pub mod mcp_client;
pub mod model_set;
pub mod requests;
pub mod requirements;
pub mod review;
pub mod revision;
pub mod runner;
pub mod store;

pub use acceptance::{Acceptance, Basis, Snapshot};
pub use acceptance_run::{Acceptor, CheckOutcome, RequirementOutcome, RequirementsReport, Taken};
pub use answers::{Answers, AnswersError};
pub use auth_policy::{Decision, Observed, Policy, Reason, Route, Status};
pub use clock::{Clock, FixedClock, ManualClock, SystemClock};
pub use commands::{call, call_in, CommandError, Context, Entrance, COMMANDS, COMMAND_SET_VERSION};
pub use connection::{AdapterKind, AuthSource, Connection, ConnectionId, Limits};
pub use error::StoreError;
pub use figures::{
    default_renderer, renderer_with_bundle, FigureRenderer, FigureRequest, FigureSet, NoRenderer,
    RenderError, SceCodegen, Sheet,
};
pub use model_set::{Document, ModelError, ModelFiles};
pub use requirements::{Requirements, RequirementsError};
pub use review::{
    Check, ModelReviewer, PageRefusal, Product, Record, Review, ReviewRequest, Unresolved, Verdict,
};
pub use revision::Revision;
pub use store::{
    default_root, default_settings_root, AcceptanceText, Adapter, AdapterListing, AdapterReport,
    AdapterStatus, AnswersText, BundleRead, CandidateTexts, CandidateWrite, ClaimedHead,
    ConnectionListing, ConnectionStore, HistoryEntry, Host, HostListing, HostReport, HostStatus,
    HostWaiting, Listing, ModelText, Published, Registered, Registration, RequestHead, RequestView,
    RequirementsText, Saved, SourceText, StoredConnection, Transition, Unreadable, Work, WorkHeads,
    WorkId, WorkSnapshot, WorkStore, ADAPTER_LIVE_SECONDS, MAX_ACCEPTANCE_BYTES, MAX_ANSWERS_BYTES,
    MAX_BUNDLE_BYTES, MAX_MODEL_BYTES, MAX_REQUIREMENTS_BYTES, MAX_SOURCE_BYTES, WAITING_MAX,
};
