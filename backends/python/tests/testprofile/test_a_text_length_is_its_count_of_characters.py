# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The TTCN-3 length of a text (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201
873-1 B.1.4.1 counts a universal charstring in characters, so a character of
two, three or four UTF-8 bytes is one, as a character of one byte is.

The first six cases are the twins of the Rust module ``text_length`` in
``backends/rust/runtime/tests/a_text_length_is_its_count_of_characters.rs``,
each the method ``test_<case>`` of ``TestTextLength``, and of
``tests/test_runtime/TextLengthTest.cpp``, and pin ``text_length`` apart from the
size in UTF-8 bytes, which bounds a text in SCE. The last case holds what only a
Python text has to: a byte that is no UTF-8, held by ``surrogateescape`` as a
lone surrogate. Every character outside ASCII is written as its escape.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime.testprofile import text_length  # noqa: E402


def holds_characters(text: str, size: int, characters: int) -> None:
    """``text`` takes ``size`` bytes of UTF-8 and holds ``characters``
    characters."""
    assert len(text.encode("utf-8", "surrogateescape")) == size, text
    assert text_length(text) == characters, text


class TestTextLength:
    def test_an_empty_text_holds_no_character(self) -> None:
        holds_characters("", 0, 0)

    def test_an_ascii_text_holds_one_character_per_byte(self) -> None:
        holds_characters("started", 7, 7)

    def test_a_character_of_two_bytes_is_one(self) -> None:
        # U+00E9, two bytes: C3 A9.
        holds_characters("café", 5, 4)

    def test_a_character_of_three_bytes_is_one(self) -> None:
        # U+20AC, three bytes: E2 82 AC.
        holds_characters("€5", 4, 2)

    def test_a_character_of_four_bytes_is_one(self) -> None:
        # U+1F600, four bytes: F0 9F 98 80.
        holds_characters("a\U0001f600b", 6, 3)

    def test_characters_of_every_width_are_counted_together(self) -> None:
        # One character of each width in a row: ten bytes, four characters.
        holds_characters("aé€\U0001f600", 10, 4)

    def test_a_byte_that_is_no_utf8_is_one_character(self) -> None:
        # A byte that begins no UTF-8 sequence, a continuation byte with no lead
        # byte before it, and a lead byte cut short are each one character: the
        # decoder holds each as one lone surrogate.
        for raw in (b"a\xffb", b"\x80", b"\xe2\x82"):
            text = raw.decode("utf-8", "surrogateescape")
            holds_characters(text, len(raw), len(raw))
