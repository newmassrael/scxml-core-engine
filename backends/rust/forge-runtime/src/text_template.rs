// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The fence of a text template's untrusted hole (docs/adr/0087, section 3.1,
//! item 4): the value is put between two markers, and every occurrence of
//! either marker is first removed from it, so that the value cannot close the
//! fence and write outside it.
//!
//! The removal repeats, taking the leftmost occurrence of either marker (the
//! longer first where both start at one byte), until neither occurs: a value
//! cannot rebuild a marker out of two halves around one that is removed. It
//! ends because each pass shortens the value. The markers are printable ASCII,
//! which no byte of a multi-byte UTF-8 sequence is, so removing them from the
//! UTF-8 bytes of a text leaves a UTF-8 text.
//!
//! One function, in place over bytes, so it needs no allocator: a generated
//! unit copies the value, removes the markers from the copy and puts what is
//! left.

/// Removes every occurrence of `open` and `close` from `text`, in place, by
/// the rule of the module, and answers how many bytes are kept: the value is
/// `text[..kept]`. An empty marker is no marker (a template's fence holds none;
/// the build refuses it).
pub fn remove_markers(text: &mut [u8], open: &[u8], close: &[u8]) -> usize {
    let longest = open.len().max(close.len());
    let mut len = text.len();
    let mut from = 0;
    loop {
        let found = (from..len).find_map(|at| {
            let rest = &text[at..len];
            let opens = !open.is_empty() && rest.starts_with(open);
            let closes = !close.is_empty() && rest.starts_with(close);
            match (opens, closes) {
                (true, true) => Some((at, longest)),
                (true, false) => Some((at, open.len())),
                (false, true) => Some((at, close.len())),
                (false, false) => None,
            }
        });
        let Some((at, marker)) = found else {
            return len;
        };
        text.copy_within(at + marker..len, at);
        len -= marker;
        // Before `at - (longest - 1)` nothing changed and nothing matched, so a
        // marker the removal made can only start at or after it.
        from = at.saturating_sub(longest - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPEN: &[u8] = b"<<<TEXT";
    const CLOSE: &[u8] = b"TEXT>>>";

    fn removed(value: &str) -> heapless::Vec<u8, 64> {
        let mut held: heapless::Vec<u8, 64> =
            heapless::Vec::from_slice(value.as_bytes()).expect("a case fits 64 bytes");
        let kept = remove_markers(&mut held, OPEN, CLOSE);
        held.truncate(kept);
        held
    }

    /// The cases every runtime's removal is held to: a value with no marker is
    /// kept whole; each marker is removed wherever it stands; a marker the
    /// removal rebuilds is removed too, the open one from its two halves and the
    /// close one from a trailing run; and multi-byte text beside a marker is
    /// kept as it was.
    #[test]
    fn every_marker_is_removed_until_none_is_left() {
        let cases: [(&str, &str); 7] = [
            ("plain words", "plain words"),
            ("a<<<TEXTb", "ab"),
            ("aTEXT>>>b<<<TEXTc", "abc"),
            ("<<<TE<<<TEXTXT", ""),
            ("TEXT>>TEXT>>>>", ""),
            ("\u{e9}<<<TEXT\u{fc}", "\u{e9}\u{fc}"),
            ("", ""),
        ];
        for (value, kept) in cases {
            assert_eq!(removed(value).as_slice(), kept.as_bytes(), "{value:?}");
        }
    }

    /// Where both markers start at one byte, the longer is removed.
    #[test]
    fn the_longer_marker_is_removed_where_both_start() {
        let mut held = *b"abcd";
        let kept = remove_markers(&mut held, b"ab", b"abc");
        assert_eq!(&held[..kept], b"d");
    }

    /// An empty marker removes nothing, and the other is still removed.
    #[test]
    fn an_empty_marker_is_no_marker() {
        let mut held = *b"x>>y";
        let kept = remove_markers(&mut held, b"", b">>");
        assert_eq!(&held[..kept], b"xy");
        let mut untouched = *b"abc";
        assert_eq!(remove_markers(&mut untouched, b"", b""), 3);
    }
}
