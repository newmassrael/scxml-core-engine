# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The fence of a text template's untrusted hole (docs/adr/0087, section 3.1,
item 4): the value is put between two markers, and every occurrence of either
marker is first removed from it, so that the value cannot close the fence and
write outside it.

The removal repeats, taking the leftmost occurrence of either marker (the longer
first where both start at one place), until neither occurs: a value cannot
rebuild a marker out of two halves around one that is removed. It ends because
each pass shortens the value. The markers are printable ASCII, each one code
point as it is one UTF-8 byte, and no other code point is spelled with an ASCII
byte, so removing them from the code points of a `str` removes what the other
runtimes remove from its UTF-8 bytes.

Holds no state and does no I/O, as every forge runtime module.
"""


def remove_markers(value: str, open_marker: str, close_marker: str) -> str:
    """`value` with every occurrence of `open_marker` and `close_marker`
    removed, by the rule of the module. An empty marker is no marker (a
    template's fence holds none; the build refuses it)."""
    longest = max(len(open_marker), len(close_marker))
    text = value
    start = 0
    while True:
        at = -1
        marker = 0
        for i in range(start, len(text)):
            opens = bool(open_marker) and text.startswith(open_marker, i)
            closes = bool(close_marker) and text.startswith(close_marker, i)
            if opens and closes:
                at, marker = i, longest
            elif opens:
                at, marker = i, len(open_marker)
            elif closes:
                at, marker = i, len(close_marker)
            if at >= 0:
                break
        if at < 0:
            return text
        text = text[:at] + text[at + marker :]
        # Before at - (longest - 1) nothing changed and nothing matched, so a
        # marker the removal made can only start at or after it.
        start = max(0, at - (longest - 1))
