// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The relay of a shared list as a server process (`sce-relay`, `docs/SCE_RELAY_DEPLOY.md`).
//!
//! What the program is made of, each part a module its tests reach:
//!
//! * [`config`] - the configuration: one TOML file, the command line over it, and every value
//!   checked before anything is opened.
//! * [`secret`] - the files only the relay may read (the authority's key, the TLS private key),
//!   refused when anyone else may.
//! * [`slots`] - the store directory: a file store for each slot of the relay, and the check that
//!   each list's file is in the slot its name belongs to.
//! * [`notify`] - the service manager's readiness message (`sd_notify`), with no library.
//! * [`logline`] - one line on stderr for each thing that happened: `key=value` words, never a
//!   secret and never what a list holds.
//! * [`exit`] - what the process's exit status says.

#![forbid(unsafe_code)]

pub mod config;
pub mod exit;
pub mod logline;
pub mod notify;
pub mod secret;
pub mod slots;
