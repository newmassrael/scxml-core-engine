// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which addresses this program will listen on.
//!
//! The commands read and write a person's files, so the server is not offered to
//! a network this program cannot tell is private. Loopback is always fine, and a
//! Tailscale address is too (the tailnet is the one network a developer reaching
//! a remote machine from a phone is already on); anything else, `0.0.0.0` above
//! all, is refused with the way to do what the person meant.

use std::net::IpAddr;

/// Tailscale's IPv4 range: 100.64.0.0/10, the shared address space for carrier-grade NAT.
fn in_tailnet_v4(octets: [u8; 4]) -> bool {
    octets[0] == 100 && (octets[1] & 0xC0) == 0x40
}

/// Tailscale's IPv6 range: fd7a:115c:a1e0::/48.
fn in_tailnet_v6(segments: [u16; 8]) -> bool {
    segments[0] == 0xfd7a && segments[1] == 0x115c && segments[2] == 0xa1e0
}

/// Whether the server may listen on `ip`, and if not, what to do instead.
pub fn check_bind(ip: IpAddr) -> Result<(), String> {
    let allowed = match ip {
        IpAddr::V4(v4) => v4.is_loopback() || in_tailnet_v4(v4.octets()),
        IpAddr::V6(v6) => v6.is_loopback() || in_tailnet_v6(v6.segments()),
    };
    if allowed {
        return Ok(());
    }
    Err(format!(
        "{ip} would offer the works folder to a network this program cannot tell is private. \
         Listen on 127.0.0.1 and forward the port (ssh -L), or listen on this machine's \
         Tailscale address"
    ))
}
