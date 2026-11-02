// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the exit status of `sce-relay` says, in the numbers of `sysexits.h`, so that a service
//! manager can tell a relay that will not start whatever it does (do not start it again) from one
//! that could not take its port this time (start it again), and a monitor can tell from a probe's
//! status which part of a relay did not answer (`sce-relay probe`, `crate::probe`).

/// The relay stopped as it was told, or the command did what it was asked: a probe's exchange was
/// answered as the protocol says.
pub const OK: u8 = 0;
/// The command line was not one the program takes (clap's own status).
pub const USAGE: u8 = 2;
/// The store holds what does not verify, or a file that is not a store's: it is the store that
/// must be repaired or restored, and starting again changes nothing.
pub const DATA: u8 = 65;
/// A listening address could not be taken (in use, or not this machine's): it may be free later.
/// For a probe: the relay was not reached (its name did not resolve, or nothing took the
/// connection at its address in time).
pub const UNAVAILABLE: u8 = 69;
/// The program could not run its own machinery (its runtime).
pub const SOFTWARE: u8 = 70;
/// A file could not be made or written (a key that is there already, a directory that is not).
pub const CANNOT_CREATE: u8 = 73;
/// What the command was asked for could not be written out (its standard output was closed). For a
/// probe: the relay answered `unavailable`, because its store did not take a record, and it takes
/// nothing until its operator has looked.
pub const IO_ERROR: u8 = 74;
/// For a probe: the relay took the connection and did not finish its TLS handshake, its upgrade or
/// its answer within the time given, closed the connection before it answered, or answered
/// `rate_limited`. It is there, and may answer later.
pub const TEMPORARY: u8 = 75;
/// For a probe: the relay answered, but not as the protocol says (an upgrade refused for another
/// reason than the token, a text that is not a response, a refusal the probe's requests do not
/// earn, or a read back that is not what was written).
pub const PROTOCOL: u8 = 76;
/// For a probe: the relay refused the token, at the upgrade or as `unauthorized` (it did not issue
/// it, it is for another relay, or it has expired).
pub const NO_PERMISSION: u8 = 77;
/// The configuration, a key, a certificate or the store's directory is wrong or cannot be read:
/// an operator must change something before it starts. For a probe: the authority to believe, the
/// key or the token it was pointed at.
pub const CONFIG: u8 = 78;
/// For a probe: the TLS handshake failed (a certificate under an authority the probe does not
/// believe, for another name, or expired; or no TLS at the address). `sysexits.h` has no number for
/// a server whose identity was not proven, and its last is 78 (`EX_CONFIG`): this is the first past
/// it, so that no reading of `sysexits.h` takes it for another.
pub const TLS_REFUSED: u8 = 79;
