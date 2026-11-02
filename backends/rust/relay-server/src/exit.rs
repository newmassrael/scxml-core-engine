// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the exit status of `sce-relay` says, in the numbers of `sysexits.h`, so that a service
//! manager can tell a relay that will not start whatever it does (do not start it again) from one
//! that could not take its port this time (start it again).

/// The relay stopped as it was told, or the command did what it was asked.
pub const OK: u8 = 0;
/// The command line was not one the program takes (clap's own status).
pub const USAGE: u8 = 2;
/// The store holds what does not verify, or a file that is not a store's: it is the store that
/// must be repaired or restored, and starting again changes nothing.
pub const DATA: u8 = 65;
/// A listening address could not be taken (in use, or not this machine's): it may be free later.
pub const UNAVAILABLE: u8 = 69;
/// The program could not run its own machinery (its runtime).
pub const SOFTWARE: u8 = 70;
/// A file could not be made or written (a key that is there already, a directory that is not).
pub const CANNOT_CREATE: u8 = 73;
/// What the command was asked for could not be written out (its standard output was closed).
pub const IO_ERROR: u8 = 74;
/// The configuration, a key, a certificate or the store's directory is wrong or cannot be read:
/// an operator must change something before it starts.
pub const CONFIG: u8 = 78;
