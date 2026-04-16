// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SOME/IP numeric ID rendering. The C++ vsomeip API accepts integer
// literals in any base, but the project convention (both inline deploy.yaml
// values and resolved bindings emitted by external.rs) is `0xNNNN` — a
// zero-padded 4-digit uppercase-hex u16. This module is the single source
// of truth so a widening or format drift is a one-edit change.

/// Render a u16 SOME/IP ID as the canonical `0xNNNN` hex literal used by
/// deploy.yaml and the generated C++ template. Callers MUST go through
/// this helper rather than open-coding `format!("0x{:04X}", …)`.
pub fn hex_id(value: u16) -> String {
    format!("0x{value:04X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_pads_to_four_digits() {
        assert_eq!(hex_id(0), "0x0000");
        assert_eq!(hex_id(0x1), "0x0001");
        assert_eq!(hex_id(0x42), "0x0042");
    }

    #[test]
    fn uppercases_hex_digits() {
        assert_eq!(hex_id(0xabcd), "0xABCD");
    }

    #[test]
    fn full_u16_range() {
        assert_eq!(hex_id(u16::MAX), "0xFFFF");
    }
}
