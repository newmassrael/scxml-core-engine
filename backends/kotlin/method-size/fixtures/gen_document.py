#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Write the statechart whose transition content passes one JVM method.

The code of one JVM method is at most 65,535 bytes (JVM Specification,
section 4.7.3), and kotlinc refuses a longer one with
`MethodTooLargeException`. Measured 2026-11-05: a large `sce-static`
statechart's Kotlin machine did not compile, because one generated method,
`executeTransitionContent`, held the content of every transition. The Kotlin
generator now writes each element's code as a method of its own and splits
every dispatch into parts of at most 256 arms
(`tools/codegen/templates/kotlin/dispatch.kt.jinja2`).

This document is the evidence the Kotlin lane compiles and runs: STATES states
in one compound state with a deep history, each with an `<onentry>` and an
`<onexit>` and a `next` transition whose content is ADDENDS + 1 checked
additions. Written as one method, the content alone is STATES * (ADDENDS + 1)
checked additions of more than 22 bytes each, past the limit; and STATES is
past 256, so every dispatch over the states is split. `JvmMethodSizeTest`
drives the machine and reads each sum back against this document.

The document is written by this program rather than by hand because its size
is the point.

Usage, from the repository root:

    python3 backends/kotlin/method-size/fixtures/gen_document.py
    python3 backends/kotlin/method-size/fixtures/gen_document.py --check
    python3 backends/kotlin/method-size/fixtures/gen_document.py --out DIR

`--check` exits 1 when the committed document differs from what this writes;
`--out` writes the document into DIR instead of beside this file.
"""

import argparse
import sys
from pathlib import Path

STATES = 300
"""States inside `all`: past 256, the most arms one dispatch method holds."""

ADDENDS = 11
"""Variables `a0` .. `a10` each transition adds to, besides `steps`."""

NAME = "jvm_method_size_static.scxml"


def step(state: int, addend: int) -> int:
    """What the transition of state `state` adds to `a<addend>`: 1 to 7."""
    return (state + addend) % 7 + 1


def document() -> str:
    """The statechart, as the file holds it."""
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        "<!--",
        "  Written by gen_document.py beside this file; edit that, not this.",
        "",
        f"  datamodel=\"sce-static\": {STATES} states in `all`, each with an",
        f"  <onentry>, an <onexit> and a `next` transition whose content is {ADDENDS + 1}",
        "  checked additions. Under the Kotlin machine's old shape, the content of",
        "  every transition was one method past the 65,535 bytes of code one JVM",
        "  method holds; the machine now compiles. JvmMethodSizeTest drives it.",
        "",
        "  Kotlin compile+run gate: scripts/regen_jvm_method_size_kotlin.sh commits",
        "  the generated machine, and JvmMethodSizeTest.kt drives it.",
        "-->",
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"',
        '       version="1.0" initial="all" datamodel="sce-static" name="jvm_method_size_static">',
        "  <datamodel>",
        '    <data id="steps" sce:type="uint32" expr="0" sce:direction="out"/>',
        '    <data id="entered" sce:type="uint32" expr="0" sce:direction="out"/>',
        '    <data id="left" sce:type="uint32" expr="0" sce:direction="out"/>',
    ]
    for addend in range(ADDENDS):
        lines.append(
            f'    <data id="a{addend}" sce:type="uint32" expr="0" sce:direction="out"/>'
        )
    lines += [
        "  </datamodel>",
        '  <state id="all" initial="s0">',
        '    <history id="back" type="deep">',
        '      <transition target="s0"/>',
        "    </history>",
    ]
    for state in range(STATES):
        target = (state + 1) % STATES
        lines += [
            f'    <state id="s{state}">',
            '      <onentry><assign location="entered" expr="entered + 1"/></onentry>',
            '      <onexit><assign location="left" expr="left + 1"/></onexit>',
            f'      <transition event="next" target="s{target}">',
            '        <assign location="steps" expr="steps + 1"/>',
        ]
        for addend in range(ADDENDS):
            lines.append(
                f'        <assign location="a{addend}" expr="a{addend} + {step(state, addend)}"/>'
            )
        lines += [
            "      </transition>",
            "    </state>",
        ]
    lines += [
        '    <transition event="pause" target="paused"/>',
        "  </state>",
        '  <state id="paused">',
        '    <transition event="resume" target="back"/>',
        "  </state>",
        "</scxml>",
    ]
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--out", type=Path, default=Path(__file__).resolve().parent)
    args = parser.parse_args()
    path = args.out / NAME
    content = document()
    if args.check:
        if not path.is_file() or path.read_text(encoding="utf-8") != content:
            print(f"not what gen_document.py writes: {path}", file=sys.stderr)
            return 1
        return 0
    path.write_text(content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
