# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The length a template's length restriction holds a text to
(docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201 873-1 B.1.4.1 restricts the
length of a ``universal charstring`` in characters, each one Unicode code point.

Cross-language siblings: ``SCE::Test::textLength`` (C++, over UTF-8 bytes),
``com.sce.runtime.test.textLength`` (Kotlin, over UTF-16 units),
``testprofile.TextLength`` (Go, over UTF-8 bytes); Rust needs no function, its
count being ``str::chars().count()``.
"""

from __future__ import annotations


def text_length(text: str) -> int:
    """The TTCN-3 length of ``text``: how many characters it holds.

    A Python ``str`` is a sequence of code points, so the count is its ``len``,
    where the other runtimes each have to undo an encoding. It is a function
    all the same, the one a generated template's ``length-min`` and
    ``length-max`` on a ``string[N]`` position call, so that the count those
    compare is named as on every backend, and not the text's size in UTF-8
    bytes, which is what bounds it in SCE (docs/SCE_TEST_PROFILE.md §6.3).

    A text decoded with ``surrogateescape`` holds a lone surrogate for each
    byte that is no UTF-8; each is one code point, and so one character, as a
    decoder that read the byte as U+FFFD counts it. A text of a generated
    record is valid UTF-8, and the two counts agree on it.
    """
    return len(text)
