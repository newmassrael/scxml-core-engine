# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The reader and writer a saved state's JSON text goes through.

A saved state is one JSON document, the same on every backend, and what one
backend wrote another reads (docs/adr/0045). That asks two things of the reader
that the ``json`` module does not give: a number keeps the spelling it was
written in, so a 64-bit value never passes through a ``float``, and an object
that names a member twice is refused, since a reader that takes the first and one
that takes the last would read two values from one text. A value is a ``dict``
(an object, in the order its members were written), a ``list``, a ``str``, a
``bool``, a :class:`JsonNumber` or ``None``.

Cross-language siblings: ``backends/go/runtime/saved_json.go``,
``com.sce.runtime.Json`` (Kotlin) and ``sce_rust_runtime::json`` (Rust).
"""

from __future__ import annotations

from typing import Any, List

__all__ = ["JsonError", "JsonNumber", "parse_json", "write_json"]


class JsonNumber:
    """A number kept in the spelling it was written in, so no width is lost."""

    __slots__ = ("text",)

    def __init__(self, text: str) -> None:
        self.text = text

    def __eq__(self, other: object) -> bool:
        return isinstance(other, JsonNumber) and other.text == self.text

    def __hash__(self) -> int:
        return hash(("JsonNumber", self.text))

    def __repr__(self) -> str:
        return f"JsonNumber({self.text!r})"


class JsonError(Exception):
    """Why a text is not one JSON value."""


def _malformed(what: str) -> JsonError:
    return JsonError(f"not JSON ({what})")


def parse_json(text: str) -> Any:
    """Read ``text`` as exactly one JSON value, surrounding whitespace aside."""
    reader = _Reader(text)
    value = reader.read_value()
    reader.skip_whitespace()
    if not reader.at_end():
        raise JsonError("more than one JSON value")
    return value


class _Reader:
    def __init__(self, text: str) -> None:
        self._text = text
        self._at = 0

    def at_end(self) -> bool:
        return self._at >= len(self._text)

    def skip_whitespace(self) -> None:
        while self._at < len(self._text) and self._text[self._at] in " \t\n\r":
            self._at += 1

    def read_value(self) -> Any:
        self.skip_whitespace()
        if self.at_end():
            raise _malformed("it ends where a value was expected")
        c = self._text[self._at]
        if c == "{":
            return self._read_object()
        if c == "[":
            return self._read_array()
        if c == '"':
            return self._read_string()
        if c == "t":
            return self._read_keyword("true", True)
        if c == "f":
            return self._read_keyword("false", False)
        if c == "n":
            return self._read_keyword("null", None)
        if c == "-" or "0" <= c <= "9":
            return self._read_number()
        raise _malformed(f"unexpected '{c}'")

    def _read_object(self) -> dict:
        self._at += 1  # '{'
        out: dict = {}
        self.skip_whitespace()
        if not self.at_end() and self._text[self._at] == "}":
            self._at += 1
            return out
        while True:
            self.skip_whitespace()
            if self.at_end() or self._text[self._at] != '"':
                raise _malformed("a field name was expected")
            key = self._read_string()
            # An object whose names repeat has no single meaning: a reader that
            # takes the first and one that takes the last would read two values
            # from one text, so it is refused as the other backends refuse it.
            if key in out:
                raise _malformed(f'the field "{key}" appears twice')
            self.skip_whitespace()
            if self.at_end() or self._text[self._at] != ":":
                raise _malformed("a ':' was expected")
            self._at += 1
            out[key] = self.read_value()
            self.skip_whitespace()
            if self.at_end():
                raise _malformed("it ends unclosed")
            c = self._text[self._at]
            if c == ",":
                self._at += 1
            elif c == "}":
                self._at += 1
                return out
            else:
                raise _malformed("a ',' or '}' was expected")

    def _read_array(self) -> List[Any]:
        self._at += 1  # '['
        out: List[Any] = []
        self.skip_whitespace()
        if not self.at_end() and self._text[self._at] == "]":
            self._at += 1
            return out
        while True:
            out.append(self.read_value())
            self.skip_whitespace()
            if self.at_end():
                raise _malformed("it ends unclosed")
            c = self._text[self._at]
            if c == ",":
                self._at += 1
            elif c == "]":
                self._at += 1
                return out
            else:
                raise _malformed("a ',' or ']' was expected")

    def _read_string(self) -> str:
        self._at += 1  # '"'
        out: List[str] = []
        while True:
            if self.at_end():
                raise _malformed("a text ends unclosed")
            c = self._text[self._at]
            self._at += 1
            if c == '"':
                return "".join(out)
            if c != "\\":
                out.append(c)
                continue
            if self.at_end():
                raise _malformed("an escape ends the text")
            escape = self._text[self._at]
            self._at += 1
            simple = {
                '"': '"',
                "\\": "\\",
                "/": "/",
                "b": "\b",
                "f": "\f",
                "n": "\n",
                "r": "\r",
                "t": "\t",
            }
            if escape in simple:
                out.append(simple[escape])
            elif escape == "u":
                unit = self._read_unit()
                # A high surrogate followed by a `\u` low one is one character; one
                # on its own is no character, and reads as U+FFFD as the other
                # backends read it.
                if 0xD800 <= unit <= 0xDFFF:
                    low = self._read_low_surrogate(unit)
                    if low is None:
                        out.append("�")
                    else:
                        out.append(chr(0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)))
                else:
                    out.append(chr(unit))
            else:
                raise _malformed(f"unknown escape '\\{escape}'")

    def _read_unit(self) -> int:
        digits = self._text[self._at : self._at + 4]
        if len(digits) < 4:
            raise _malformed("a \\u escape is short")
        if any(d not in "0123456789abcdefABCDEF" for d in digits):
            raise _malformed("a \\u escape is not hex")
        self._at += 4
        return int(digits, 16)

    def _read_low_surrogate(self, high: int):
        """The ``\\u`` low surrogate that completes ``high``, consumed only when
        it does."""
        if high >= 0xDC00 or not self._text.startswith("\\u", self._at):
            return None
        saved = self._at
        self._at += 2
        try:
            low = self._read_unit()
        except JsonError:
            self._at = saved
            return None
        if not 0xDC00 <= low <= 0xDFFF:
            self._at = saved
            return None
        return low

    def _read_number(self) -> JsonNumber:
        start = self._at

        def digits() -> None:
            while not self.at_end() and "0" <= self._text[self._at] <= "9":
                self._at += 1

        if self._text[self._at] == "-":
            self._at += 1
        digits()
        if not self.at_end() and self._text[self._at] == ".":
            self._at += 1
            digits()
        if not self.at_end() and self._text[self._at] in "eE":
            self._at += 1
            if not self.at_end() and self._text[self._at] in "+-":
                self._at += 1
            digits()
        text = self._text[start : self._at]
        if text in ("", "-"):
            raise _malformed("a number has no digits")
        return JsonNumber(text)

    def _read_keyword(self, word: str, value: Any) -> Any:
        if not self._text.startswith(word, self._at):
            raise _malformed(f"expected '{word}'")
        self._at += len(word)
        return value


def _escape(text: str) -> str:
    """The one form every engine writes a text in (ARCHITECTURE.md, "JSON Text
    (Single Source of Truth)"): ``"`` and ``\\`` escaped, the five short forms,
    every other U+0000-U+001F as ``\\u00xx`` in lowercase hex, everything else as
    it is."""
    out: List[str] = []
    for c in text:
        if c == '"':
            out.append('\\"')
        elif c == "\\":
            out.append("\\\\")
        elif c == "\b":
            out.append("\\b")
        elif c == "\f":
            out.append("\\f")
        elif c == "\n":
            out.append("\\n")
        elif c == "\r":
            out.append("\\r")
        elif c == "\t":
            out.append("\\t")
        elif ord(c) < 0x20:
            out.append(f"\\u{ord(c):04x}")
        else:
            out.append(c)
    return "".join(out)


def write_json(value: Any) -> str:
    """``value`` as compact JSON text. A number is written in the spelling it
    carries, so a value :func:`parse_json` read is written back unchanged. A value
    of any type but the six a JSON value is held as is a programming error."""
    out: List[str] = []
    _write_into(value, out)
    return "".join(out)


def _write_into(value: Any, out: List[str]) -> None:
    if value is None:
        out.append("null")
    elif value is True:
        out.append("true")
    elif value is False:
        out.append("false")
    elif isinstance(value, JsonNumber):
        out.append(value.text)
    elif isinstance(value, str):
        out.append('"' + _escape(value) + '"')
    elif isinstance(value, list):
        out.append("[")
        for at, item in enumerate(value):
            if at:
                out.append(",")
            _write_into(item, out)
        out.append("]")
    elif isinstance(value, dict):
        out.append("{")
        for at, (key, item) in enumerate(value.items()):
            if at:
                out.append(",")
            out.append('"' + _escape(key) + '":')
            _write_into(item, out)
        out.append("}")
    else:
        raise TypeError(
            f"{type(value).__name__} is not a JSON value; wrap a number in JsonNumber"
        )
