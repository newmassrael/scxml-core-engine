// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! UTF-8 read by RFC 3629's grammar (section 4): the scalar values, each in its shortest form,
//! and nothing else.

/// The code point at the start of `bytes` and the bytes it takes; `None` when `bytes` is empty
/// or does not start with a well-formed sequence (an overlong form, a surrogate, a code point past
/// U+10FFFF, a continuation byte out of place, a sequence cut short).
pub(crate) fn decode(bytes: &[u8]) -> Option<(u32, usize)> {
    let lead = *bytes.first()?;
    // UTF8-2, UTF8-3 and UTF8-4: the bytes taken, the range of the second byte, and the bits of
    // the lead byte.
    let (width, low, high, bits) = match lead {
        0x00..=0x7F => return Some((u32::from(lead), 1)),
        0xC2..=0xDF => (2, 0x80, 0xBF, lead & 0x1F),
        0xE0 => (3, 0xA0, 0xBF, lead & 0x0F),
        0xE1..=0xEC | 0xEE..=0xEF => (3, 0x80, 0xBF, lead & 0x0F),
        0xED => (3, 0x80, 0x9F, lead & 0x0F),
        0xF0 => (4, 0x90, 0xBF, lead & 0x07),
        0xF1..=0xF3 => (4, 0x80, 0xBF, lead & 0x07),
        0xF4 => (4, 0x80, 0x8F, lead & 0x07),
        _ => return None,
    };
    let tail = bytes.get(1..width)?;
    let mut code_point = u32::from(bits);
    for (index, &byte) in tail.iter().enumerate() {
        let (low, high) = if index == 0 {
            (low, high)
        } else {
            (0x80, 0xBF)
        };
        if !(low..=high).contains(&byte) {
            return None;
        }
        code_point = (code_point << 6) | u32::from(byte & 0x3F);
    }
    Some((code_point, width))
}

/// Whether `bytes` is a sequence of well-formed UTF-8 sequences, the empty one among them.
pub(crate) fn well_formed(mut bytes: &[u8]) -> bool {
    while !bytes.is_empty() {
        match decode(bytes) {
            Some((_, width)) => bytes = &bytes[width..],
            None => return false,
        }
    }
    true
}
