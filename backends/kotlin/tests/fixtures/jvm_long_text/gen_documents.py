#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Write the two documents whose texts are past one JVM class-file constant.

A string constant of a class file holds at most 65,535 bytes of modified
UTF-8 (JVM Specification, section 4.4.7): U+0001 to U+007F take one byte,
U+0000 and up to U+07FF two, up to U+FFFF three, and a character past U+FFFF
six, the two halves of its UTF-16 pair three each. kotlinc cuts a longer
string written as an expression into pieces that each fit
(`sce-build/src/filters.rs`, at `kotlin_string_val`, states where in the
compiler), and these documents put such a text at every place the Kotlin
backend writes document text as an expression, so the Kotlin lane compiles
the machines and `JvmLongTextTest` runs them and reads each text back.

The texts are written by this program rather than by hand because their
lengths are the point: each is exactly as long as the table below says, in
modified UTF-8, with multi-byte characters around every multiple of 65,535
bytes, where a cut falls. U+0000 is not among them: XML 1.0 admits no U+0000
in a document, not even as a character reference (section 2.2, `Char`), so
no document can carry one; `modified_utf8_len`'s own tests hold its width.

Usage, from the repository root:

    python3 backends/kotlin/tests/fixtures/jvm_long_text/gen_documents.py
    python3 backends/kotlin/tests/fixtures/jvm_long_text/gen_documents.py --check
    python3 backends/kotlin/tests/fixtures/jvm_long_text/gen_documents.py --out DIR

`--check` exits 1 when a committed document differs from what this writes;
`--out` writes the documents into DIR instead of beside this file.
"""

import argparse
import sys
from pathlib import Path

LIMIT = 65_535
"""The bytes of modified UTF-8 one class-file constant holds."""

BAND = 64
"""How far on each side of a multiple of LIMIT the multi-byte band reaches.

A site may write a few characters of its own around the text in the same
constant (quotes, a JSON string's delimiters), which moves where kotlinc
cuts; the band is wide enough that the cut falls inside it at every site.
"""

PAST_ONE = LIMIT + 65
"""A text one band past one constant."""

PAST_TWO = 2 * LIMIT + 130
"""A text one band past two constants: three pieces."""

BMP = ["é", "€"]
"""Two bytes and three bytes: characters every engine binding carries.

The ECMAScript document's texts cross into a script engine, and the JNI
bindings (Lua, QuickJS) hand a string over in modified UTF-8, which spells a
character past U+FFFF as two halves; that crossing is not what these
documents measure, so its texts stay in the Basic Multilingual Plane.
"""

WIDE = BMP + ["\U00010348"]
"""And a character past U+FFFF: six bytes, a UTF-16 pair kotlinc may part."""

DOLLARS = "$tail${tail}"
"""Kotlin's template openers: unescaped, the literal would read `tail`."""

BACKSLASH = "\\"
"""Kotlin's escape opener, for the texts no ECMAScript parser reads."""


def width(c: str) -> int:
    """The bytes `c` takes in modified UTF-8."""
    cp = ord(c)
    if cp == 0:
        return 2
    if cp <= 0x7F:
        return 1
    if cp <= 0x7FF:
        return 2
    if cp <= 0xFFFF:
        return 3
    return 6


def modified_len(text: str) -> int:
    """The bytes `text` takes in modified UTF-8."""
    return sum(width(c) for c in text)


def text(tag: str, total: int, band: list, specials: str) -> str:
    """A text of exactly `total` bytes of modified UTF-8.

    It opens with `tag` and `specials` and closes with `tag`, so a text that
    arrived at another site's place is told apart; between them it is `a`,
    except within BAND bytes of every multiple of LIMIT, where it cycles the
    characters of `band`.
    """
    head = f"{tag}[{specials}]"
    tail = f"[{tag}]"
    body_end = total - modified_len(tail)
    centres = [k * LIMIT for k in range(1, total // LIMIT + 1)]
    out = [head]
    at = modified_len(head)
    turn = 0
    while at < body_end:
        ch = "a"
        if any(c - BAND <= at < c + BAND for c in centres):
            ch = band[turn % len(band)]
            turn += 1
        if at + width(ch) > body_end:
            ch = "a"
        out.append(ch)
        at += width(ch)
    out.append(tail)
    result = "".join(out)
    assert modified_len(result) == total, (tag, modified_len(result), total)
    return result


def static_document() -> str:
    """The `sce-static` machine: texts lowered into Kotlin with no engine."""
    data = text("data", PAST_TWO, WIDE, "")
    assigned = text("assign", PAST_ONE, WIDE, "")
    joined = text("join", PAST_ONE, WIDE, "")
    done = text("done", PAST_ONE, WIDE, DOLLARS + BACKSLASH)
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<!--
  Written by gen_documents.py beside this file; edit that, not this.

  datamodel="sce-static": each text below is past the 65,535 bytes of modified
  UTF-8 one JVM class-file constant holds, at a place the Kotlin backend
  writes document text as an expression, which kotlinc cuts into pieces that
  each fit. `fromData` is a variable's initial value (a property initializer,
  three pieces), `fromAssign` a literal an <assign> lowers, `fromJoin` a
  literal joined to a value at run time (a concatenation whose constant part
  kotlinc moves out of its recipe), and the final's <donedata> inline content
  the done event's data. JvmLongTextTest reads each back.

  Kotlin compile+run gate: tools/codegen/trees/kotlin-integration.toml declares
  the generated machine, and JvmLongTextTest.kt drives it.
-->
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="run" datamodel="sce-static" name="jvm_long_text_static">
  <datamodel>
    <data id="fromData" sce:type="string[200000]" expr="'{data}'" sce:direction="out"/>
    <data id="fromAssign" sce:type="string[200000]" expr="''" sce:direction="out"/>
    <data id="fromJoin" sce:type="string[200000]" expr="''" sce:direction="out"/>
    <data id="tail" sce:type="string[8]" expr="'!'"/>
  </datamodel>
  <state id="run">
    <onentry>
      <assign location="fromAssign" expr="'{assigned}'"/>
      <assign location="fromJoin" expr="'{joined}' + tail"/>
    </onentry>
    <transition event="finish" target="done"/>
  </state>
  <final id="done">
    <donedata>
      <content>{done}</content>
    </donedata>
  </final>
</scxml>
"""


def ecmascript_document() -> str:
    """The ECMAScript machine: texts written into Kotlin for its engine."""
    inline = text("inline", PAST_ONE, BMP, DOLLARS + BACKSLASH)
    data = text("data", PAST_ONE, BMP, DOLLARS)
    joined = text("join", PAST_ONE, BMP, DOLLARS)
    sent = text("send", PAST_ONE, BMP, DOLLARS + BACKSLASH)
    done = text("done", PAST_ONE, BMP, DOLLARS + BACKSLASH)
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<!--
  Written by gen_documents.py beside this file; edit that, not this.

  datamodel="ecmascript": each text below is past the 65,535 bytes of modified
  UTF-8 one JVM class-file constant holds, at a place the Kotlin backend
  writes document text into a literal its script engine is handed, which
  kotlinc cuts into pieces that each fit. `inlineText` is a variable's inline
  content, `fromData` a variable's `expr`, `fromJoin` an <assign> joining a
  literal to a value, `fromSend` a <send>'s inline <content> read back as the
  event's data, and `fromDone` a final's <donedata> inline content read back
  from the done event. JvmLongTextTest reads each back.

  Kotlin compile+run gate: tools/codegen/trees/kotlin-integration.toml declares
  the generated machine, and JvmLongTextTest.kt drives it.
-->
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" initial="run" datamodel="ecmascript" name="jvm_long_text_ecmascript">
  <datamodel>
    <data id="inlineText">{inline}</data>
    <data id="fromData" expr="'{data}'"/>
    <data id="tail" expr="'!'"/>
    <data id="fromInline" expr="''"/>
    <data id="fromJoin" expr="''"/>
    <data id="fromSend" expr="''"/>
    <data id="fromDone" expr="''"/>
  </datamodel>
  <state id="run" initial="waiting">
    <onentry>
      <assign location="fromInline" expr="inlineText"/>
      <assign location="fromJoin" expr="'{joined}' + tail"/>
      <send event="carry">
        <content>{sent}</content>
      </send>
    </onentry>
    <state id="waiting">
      <transition event="carry" target="finished">
        <assign location="fromSend" expr="_event.data"/>
      </transition>
    </state>
    <final id="finished">
      <donedata>
        <content>{done}</content>
      </donedata>
    </final>
    <transition event="done.state.run" target="end">
      <assign location="fromDone" expr="_event.data"/>
    </transition>
  </state>
  <final id="end"/>
</scxml>
"""


DOCUMENTS = {
    "jvm_long_text_static.scxml": static_document,
    "jvm_long_text_ecmascript.scxml": ecmascript_document,
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--out", type=Path, default=Path(__file__).resolve().parent)
    args = parser.parse_args()
    stale = []
    for name, build in DOCUMENTS.items():
        path = args.out / name
        content = build()
        if args.check:
            if not path.is_file() or path.read_text(encoding="utf-8") != content:
                stale.append(str(path))
            continue
        path.write_text(content, encoding="utf-8")
    if stale:
        print("not what gen_documents.py writes: " + ", ".join(stale), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
