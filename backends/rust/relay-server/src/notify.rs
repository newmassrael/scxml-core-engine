// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The message a service manager reads to know that a service is ready or stopping (`sd_notify`):
//! one datagram of `KEY=value` lines to the socket `NOTIFY_SOCKET` names. A unit with
//! `Type=notify` counts the relay as started only when it says `READY=1`, which it says once it
//! has read and verified its store and taken its ports, so that an order that waits on it waits
//! for a relay that answers. Without `NOTIFY_SOCKET` nothing is sent.

use std::io;

/// The variable that names the socket.
pub const SOCKET_VARIABLE: &str = "NOTIFY_SOCKET";

/// Send `state` (lines such as `READY=1` and `STATUS=...`) to the service manager. `Ok(false)` when
/// no manager asked for it.
pub fn notify(state: &str) -> io::Result<bool> {
    let Some(socket) = std::env::var_os(SOCKET_VARIABLE) else {
        return Ok(false);
    };
    send(&socket, state)?;
    Ok(true)
}

#[cfg(unix)]
fn send(socket: &std::ffi::OsStr, state: &str) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::UnixDatagram;

    let sender = UnixDatagram::unbound()?;
    match socket.as_bytes() {
        [] => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NOTIFY_SOCKET is empty",
        )),
        [b'@', name @ ..] => send_abstract(&sender, name, state),
        _ => sender.send_to(state.as_bytes(), socket).map(|_| ()),
    }
}

/// A socket in the abstract namespace (`@name`), which only Linux has.
#[cfg(target_os = "linux")]
fn send_abstract(
    sender: &std::os::unix::net::UnixDatagram,
    name: &[u8],
    state: &str,
) -> io::Result<()> {
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::SocketAddr;

    let address = SocketAddr::from_abstract_name(name)?;
    sender.send_to_addr(state.as_bytes(), &address).map(|_| ())
}

#[cfg(all(unix, not(target_os = "linux")))]
fn send_abstract(
    _sender: &std::os::unix::net::UnixDatagram,
    _name: &[u8],
    _state: &str,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "an abstract socket is Linux's alone",
    ))
}

#[cfg(not(unix))]
fn send(_socket: &std::ffi::OsStr, _state: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "a service manager's socket is a Unix one",
    ))
}
