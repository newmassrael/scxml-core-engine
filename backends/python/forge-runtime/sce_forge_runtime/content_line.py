# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The content lines (RFC 5545 §3.1) a ``sce:encoding="content-line"`` codec
reads and writes (SCE_FORGE.md §4.6.4, docs/adr/0010): ``BEGIN:<component>``,
properties — a name, ``;``-separated parameters, ``:`` and a value — and
``END:<component>``, folded at 75 octets.

Mirrors ``backends/rust/forge-runtime/src/content_line.rs``, rule for rule. A
generated codec calls these; it spells no line grammar of its own. The rules
are written once in SCE_FORGE.md §4.6.4 and implemented once per backend
runtime. Every refusal is a typed :class:`~sce_forge_runtime.codec.CodecError`
below, which a generated ``decode`` turns into ``None``, the Python decode
convention, and a generated ``encode`` lets through.
"""

from __future__ import annotations

from typing import Callable, List, Optional, Tuple

from .codec import CodecError, NeedMoreBytes, SceSink


class LineMalformed(CodecError):
    """A line, a parameter or an ``END:`` the grammar does not admit."""


class LineRequiredMissing(CodecError):
    """A property or parameter declared ``sce:required="true"`` is absent; on
    encode, a parameter was given without the property it belongs to."""


class LineTooMany(CodecError):
    """A property that holds one value occurred twice, a parameter was given
    twice in one line, or a list passed its ``sce:max-count``."""


class LineTooLong(CodecError):
    """A value passed its ``sce:max-size``."""


class LineBadEscape(CodecError):
    """A TEXT carried an escape other than ``\\\\``, ``\\;``, ``\\,``, ``\\n``
    and ``\\N``."""


class LineBadValue(CodecError):
    """A value its entry cannot hold: a control character, invalid UTF-8, an
    integer or ``bool`` out of its type, a parameter of more values than one,
    a ``"`` in a parameter value."""


#: The most octets of one physical line (SCE_FORGE.md §4.6.4, *Folding*).
FOLD_WIDTH = 75

_CR = 13
_LF = 10
_SPACE = 32
_TAB = 9
_QUOTE = 34
_BACKSLASH = 92


def _is_control(b: int) -> bool:
    """A control character a value never holds: below U+0020 but the tab, and
    U+007F."""
    return (b < 0x20 and b != _TAB) or b == 0x7F


def _is_name_byte(b: int) -> bool:
    """A byte of a property or parameter name: letters, digits and hyphens."""
    return 0x30 <= b <= 0x39 or 0x41 <= b <= 0x5A or 0x61 <= b <= 0x7A or b == 0x2D


def _lower(b: int) -> int:
    return b + 0x20 if 0x41 <= b <= 0x5A else b


#: The variants of an enum an entry is read and written by, in declaration
#: order: each variant's text and the carrier value it stands for
#: (docs/adr/0015). The generated codec holds it as data.
EnumTexts = Tuple[Tuple[str, int], ...]


def _longest_text(table: EnumTexts) -> int:
    return max((len(text) for text, _ in table), default=0)


def _carrier_of(value: bytes, table: EnumTexts) -> int:
    """The carrier of the first variant whose text is ``value``, compared
    ASCII case-insensitively: ``bytes.lower`` folds the 26 letters and no other
    byte, so a character Unicode folds to one of them (U+017F, U+0131) is not
    that letter."""
    folded = value.lower()
    for text, carrier in table:
        if text.encode("ascii").lower() == folded:
            return carrier
    raise LineBadValue()


def declared(value):
    """``value``, the enum member a carrier was taken to, or
    :class:`LineBadValue` where the enum is closed and the carrier is none of its
    variants. The table of a codec holds only declared carriers, so this does not
    refuse one a generated codec reads; it is the type's own answer, asked."""
    if value is None:
        raise LineBadValue()
    return value


class _Scan:
    """A walk over the bytes of one logical line, ``raw[pos:limit]``, with its
    folds removed as it goes: a line break and the one space or tab after it
    are not part of the text, wherever the sender cut."""

    def __init__(self, raw: bytes, limit: int, pos: int) -> None:
        self.raw = raw
        self.limit = limit
        self.pos = pos

    def _blank(self, i: int) -> bool:
        return i < self.limit and self.raw[i] in (_SPACE, _TAB)

    def skip_folds(self) -> None:
        """Step over every fold at the current position."""
        while True:
            raw, pos = self.raw, self.pos
            if pos + 1 < self.limit and raw[pos] == _CR and raw[pos + 1] == _LF and self._blank(pos + 2):
                self.pos += 3
            elif pos < self.limit and raw[pos] == _LF and self._blank(pos + 1):
                self.pos += 2
            else:
                return

    def peek(self) -> int:
        """The next byte without taking it, or -1 at the end of the line."""
        self.skip_folds()
        return self.raw[self.pos] if self.pos < self.limit else -1

    def bump(self) -> int:
        b = self.peek()
        if b >= 0:
            self.pos += 1
        return b


def _unfolded_eq(raw: bytes, start: int, end: int, expected: str) -> bool:
    """Whether the unfolded text of ``raw[start:end]`` is ``expected``,
    without regard to case."""
    scan = _Scan(raw, end, start)
    for ch in expected.encode("ascii"):
        b = scan.bump()
        if b < 0 or _lower(b) != _lower(ch):
            return False
    return scan.peek() < 0


class _Head:
    """The name of a line and what follows it, as indexes into the input."""

    def __init__(self, name_end: int, separator: int, rest_start: int) -> None:
        self.name_end = name_end
        self.separator = separator
        self.rest_start = rest_start


def _head_of(raw: bytes, start: int, end: int) -> Optional[_Head]:
    scan = _Scan(raw, end, start)
    named = False
    while True:
        b = scan.peek()
        if b >= 0 and _is_name_byte(b):
            scan.bump()
            named = True
        else:
            break
    if not named:
        return None
    name_end = scan.pos
    separator = scan.peek()
    if separator not in (ord(";"), ord(":")):
        return None
    scan.bump()
    return _Head(name_end, separator, scan.pos)


def _utf8(buf: bytes) -> str:
    try:
        return buf.decode("utf-8")
    except UnicodeDecodeError:
        raise LineBadValue() from None


# ── Reading ─────────────────────────────────────────────────────────────


class ContentLineReader:
    """A reader over the properties of one component."""

    def __init__(self, data: bytes, component: str) -> None:
        self._input = bytes(data)
        self._component = component
        self._pos = 0
        #: How many nested components the walk is inside (``VALARM`` in
        #: ``VEVENT``).
        self._depth = 0

    @classmethod
    def begin(cls, data: bytes, component: str) -> "ContentLineReader":
        """Skip lines up to the first ``BEGIN:<component>`` and stand after
        it. An input that ends before it raises :class:`NeedMoreBytes`."""
        reader = cls(data, component)
        while True:
            line = reader._next_raw_line()
            if line is None or not line[2]:
                raise NeedMoreBytes()
            start, end, _ = line
            head = _head_of(reader._input, start, end)
            if (
                head is not None
                and head.separator == ord(":")
                and _unfolded_eq(reader._input, start, head.name_end, "BEGIN")
                and _unfolded_eq(reader._input, head.rest_start, end, component)
            ):
                return reader

    def consumed(self) -> int:
        """How many bytes of the input the walk has passed — after
        ``END:<component>`` once :meth:`next_property` has answered
        ``None``."""
        return self._pos

    def _next_raw_line(self) -> Optional[Tuple[int, int, bool]]:
        """The next logical line from the walk's position as (start, end,
        terminated), or ``None`` at the end of the input. A line ends at a line
        break (CRLF or LF) that no space or tab follows."""
        data = self._input
        if self._pos >= len(data):
            return None
        start = self._pos
        i = start
        while True:
            i = data.find(b"\n", i)
            if i < 0:
                self._pos = len(data)
                return start, len(data), False
            folded = i + 1 < len(data) and data[i + 1] in (_SPACE, _TAB)
            if not folded:
                end = i - 1 if i > start and data[i - 1] == _CR else i
                self._pos = i + 1
                return start, end, True
            i += 1

    def next_property(self) -> Optional["ContentLineProperty"]:
        """The next property line of the component, or ``None`` after its
        ``END:<component>``.

        A nested component is skipped through its ``END:`` without reading its
        lines. An input that ends before ``END:<component>`` raises
        :class:`NeedMoreBytes`."""
        data = self._input
        while True:
            line = self._next_raw_line()
            if line is None:
                raise NeedMoreBytes()
            start, end, terminated = line
            head = _head_of(data, start, end)
            if self._depth > 0:
                if head is not None and head.separator == ord(":"):
                    if _unfolded_eq(data, start, head.name_end, "BEGIN"):
                        self._depth += 1
                    elif _unfolded_eq(data, start, head.name_end, "END"):
                        self._depth -= 1
                continue
            # A line with no name is cut short at the end of the input and
            # malformed anywhere else.
            cut: CodecError = LineMalformed() if terminated else NeedMoreBytes()
            if head is None:
                raise cut
            if head.separator == ord(":"):
                if _unfolded_eq(data, start, head.name_end, "END"):
                    if _unfolded_eq(data, head.rest_start, end, self._component):
                        return None
                    raise cut
                if terminated and _unfolded_eq(data, start, head.name_end, "BEGIN"):
                    self._depth = 1
                    continue
            if not terminated:
                raise NeedMoreBytes()
            return ContentLineProperty(data, start, end, head.name_end)


_AT_SEPARATOR = 0
_PARAM_VALUE = 1
_DONE = 2


class ContentLineProperty:
    """One property line the codec has been handed.

    Ask :meth:`is_named` whether it is one the codec reads. Then, for a
    property that declares parameters, loop on :meth:`next_param` and read the
    ones the codec declares; then read the value. A parameter no one reads is
    skipped by the next call, and a property read by value alone skips them
    all."""

    def __init__(self, raw: bytes, start: int, end: int, name_end: int) -> None:
        self._raw = raw
        self._start = start
        self._name_end = name_end
        self._scan = _Scan(raw, end, name_end)
        self._param_start = -1
        self._param_end = 0
        self._phase = _AT_SEPARATOR

    def is_named(self, name: str) -> bool:
        """Whether this property is ``name``, without regard to case."""
        return _unfolded_eq(self._raw, self._start, self._name_end, name)

    def param_is(self, name: str) -> bool:
        """Whether the parameter :meth:`next_param` stands on is ``name``,
        without regard to case."""
        return self._param_start >= 0 and _unfolded_eq(self._raw, self._param_start, self._param_end, name)

    def next_param(self) -> bool:
        """Stand on the next parameter, skipping the value of one not read:
        ``True`` when there is one, ``False`` when the value is next."""
        if self._phase == _PARAM_VALUE:
            self._skip_param_value()
        elif self._phase == _DONE:
            raise LineMalformed()
        scan = self._scan
        nxt = scan.peek()
        if nxt == ord(":"):
            return False
        if nxt != ord(";"):
            raise LineMalformed()
        scan.bump()
        scan.skip_folds()
        start = scan.pos
        while True:
            b = scan.peek()
            if b >= 0 and _is_name_byte(b):
                scan.bump()
            else:
                break
        end = scan.pos
        if end == start or scan.bump() != ord("="):
            raise LineMalformed()
        self._param_start = start
        self._param_end = end
        self._phase = _PARAM_VALUE
        return True

    def _scan_param_value(self, emit: Optional[Callable[[int], None]]) -> bool:
        """Scan one parameter value — a quoted string, or text up to ``;``,
        ``:``, ``,`` or ``"`` — handing each byte of it to ``emit``. Answers
        whether another value follows a ``,``."""
        scan = self._scan
        if scan.peek() == _QUOTE:
            scan.bump()
            while True:
                b = scan.bump()
                if b < 0:
                    raise LineMalformed()
                if b == _QUOTE:
                    break
                if emit is not None:
                    emit(b)
        else:
            while True:
                b = scan.peek()
                if b < 0 or b in (ord(";"), ord(":"), ord(",")):
                    break
                if b == _QUOTE:
                    raise LineMalformed()
                scan.bump()
                if emit is not None:
                    emit(b)
        follow = scan.peek()
        if follow == ord(","):
            scan.bump()
            return True
        if follow in (ord(";"), ord(":")):
            return False
        raise LineMalformed()

    def _skip_param_value(self) -> None:
        while self._scan_param_value(None):
            pass
        self._param_start = -1
        self._phase = _AT_SEPARATOR

    def read_param_string(self, max_size: int) -> str:
        """Read the value of the parameter :meth:`next_param` stands on, into
        at most ``max_size`` bytes. A second value is :class:`LineBadValue`."""
        if self._phase != _PARAM_VALUE:
            raise LineMalformed()
        buf = bytearray()

        def emit(b: int) -> None:
            if _is_control(b):
                raise LineBadValue()
            if len(buf) == max_size:
                raise LineTooLong()
            buf.append(b)

        if self._scan_param_value(emit):
            raise LineBadValue()
        self._param_start = -1
        self._phase = _AT_SEPARATOR
        return _utf8(bytes(buf))

    def _begin_value(self) -> None:
        """Stand at the value: skip the parameters still unread and step over
        ``:``."""
        while self.next_param():
            pass
        self._scan.bump()
        self._phase = _DONE

    def read_string(self, max_size: int, text: bool) -> str:
        """Read the value as a ``string`` of at most ``max_size`` bytes. With
        ``text``, ``\\\\``, ``\\;``, ``\\,``, ``\\n`` and ``\\N`` are escapes;
        an unescaped ``;`` or ``,`` is itself."""
        self._begin_value()
        scan = self._scan
        buf = bytearray()
        while True:
            b = scan.bump()
            if b < 0:
                break
            value = b
            if text and b == _BACKSLASH:
                e = scan.bump()
                if e in (_BACKSLASH, ord(";"), ord(",")):
                    value = e
                elif e in (ord("n"), ord("N")):
                    value = _LF
                else:
                    raise LineBadEscape()
            elif _is_control(b):
                raise LineBadValue()
            if len(buf) == max_size:
                raise LineTooLong()
            buf.append(value)
        return _utf8(bytes(buf))

    def read_strings(self, separator: str, max_values: int, max_size: int, text: bool) -> List[str]:
        """Read the value as a list of ``string`` cut at ``separator``, at most
        ``max_values`` of them and each at most ``max_size`` bytes
        (docs/adr/0014). The value is cut before it is unescaped: with ``text``
        a separator that a backslash precedes is part of the value, and with
        anything else every separator cuts. The parts are judged left to right
        and the first failure is the line's: a part past ``max_values`` is
        :class:`LineTooMany`, even one that is empty or too long; an empty part
        is :class:`LineBadValue`."""
        self._begin_value()
        scan = self._scan
        cut = ord(separator)
        values: List[str] = []
        buf = bytearray()
        while True:
            b = scan.bump()
            if b < 0 or b == cut:
                if not buf:
                    raise LineBadValue()
                values.append(_utf8(bytes(buf)))
                if b < 0:
                    return values
                buf = bytearray()
                # A separator opens another part, which may not pass the bound.
                if len(values) >= max_values:
                    raise LineTooMany()
                continue
            value = b
            if text and b == _BACKSLASH:
                e = scan.bump()
                if e in (_BACKSLASH, ord(";"), ord(",")):
                    value = e
                elif e in (ord("n"), ord("N")):
                    value = _LF
                else:
                    raise LineBadEscape()
            elif _is_control(b):
                raise LineBadValue()
            if len(buf) == max_size:
                raise LineTooLong()
            buf.append(value)

    def _read_decimal(self, allow_minus: bool) -> int:
        """The decimal the rest of the value is: an optional sign, digits. A
        ``-`` is read only when ``allow_minus``, so an unsigned type refuses
        ``-0`` as well."""
        self._begin_value()
        scan = self._scan
        nxt = scan.bump()
        negative = nxt == ord("-")
        if negative and not allow_minus:
            raise LineBadValue()
        if negative or nxt == ord("+"):
            nxt = scan.bump()
        magnitude = 0
        digits = 0
        while nxt >= 0:
            if not ord("0") <= nxt <= ord("9"):
                raise LineBadValue()
            magnitude = magnitude * 10 + (nxt - ord("0"))
            digits += 1
            nxt = scan.bump()
        if digits == 0:
            raise LineBadValue()
        return -magnitude if negative else magnitude

    def read_uint(self, maximum: int) -> int:
        """Read the value as an unsigned integer of at most ``maximum``."""
        value = self._read_decimal(False)
        if value > maximum:
            raise LineBadValue()
        return value

    def read_int(self, minimum: int, maximum: int) -> int:
        """Read the value as a signed integer within ``minimum..maximum``."""
        value = self._read_decimal(True)
        if value < minimum or value > maximum:
            raise LineBadValue()
        return value

    def read_bool(self) -> bool:
        """Read the value as ``TRUE`` or ``FALSE``, in either case."""
        self._begin_value()
        scan = self._scan
        word = bytearray()
        while True:
            b = scan.bump()
            if b < 0:
                break
            if len(word) == 5:
                raise LineBadValue()
            word.append(_lower(b))
        if bytes(word) == b"true":
            return True
        if bytes(word) == b"false":
            return False
        raise LineBadValue()

    def read_enum(self, table: EnumTexts) -> int:
        """Read the value as the text of a variant of an enum (docs/adr/0015):
        the carrier of the first row of ``table`` whose text it is, ASCII
        case-insensitively. A value no row names is :class:`LineBadValue`,
        whatever its bytes are: it is no TEXT, so a backslash in it is not an
        escape, and nothing of it is kept past the longest text."""
        self._begin_value()
        scan = self._scan
        longest = _longest_text(table)
        word = bytearray()
        while True:
            b = scan.bump()
            if b < 0:
                break
            if len(word) <= longest:
                word.append(b)
        return _carrier_of(bytes(word), table)

    def read_param_enum(self, table: EnumTexts) -> int:
        """Read the value of the parameter :meth:`next_param` stands on as the
        text of a variant of an enum (docs/adr/0015). The whole value is scanned
        before it is judged, so a line the grammar refuses is
        :class:`LineMalformed` before it is :class:`LineBadValue`; a second value
        is :class:`LineBadValue`."""
        if self._phase != _PARAM_VALUE:
            raise LineMalformed()
        longest = _longest_text(table)
        word = bytearray()

        def emit(b: int) -> None:
            if len(word) <= longest:
                word.append(b)

        more = self._scan_param_value(emit)
        self._param_start = -1
        self._phase = _AT_SEPARATOR
        if more:
            raise LineBadValue()
        return _carrier_of(bytes(word), table)


# ── Writing ─────────────────────────────────────────────────────────────


class ContentLineWriter:
    """A writer of the lines of one component into a sink.

    Write :meth:`begin`; then a property by :meth:`property`, each present
    parameter by :meth:`param`, and the value by one of the value methods,
    which ends the line; then :meth:`finish`. Each raises the typed error that
    refused it."""

    def __init__(self, sink: SceSink, component: str) -> None:
        self._sink = sink
        self._component = component
        #: Octets already on the current physical line.
        self._column = 0

    def _raw(self, text: str) -> None:
        self._sink.write_bytes(text.encode("ascii"))

    def begin(self) -> None:
        """Write ``BEGIN:<component>``."""
        self._raw("BEGIN:" + self._component + "\r\n")

    def finish(self) -> None:
        """Write ``END:<component>``."""
        self._raw("END:" + self._component + "\r\n")

    def _unit(self, data: bytes) -> None:
        """Write one unit — a character, or an escape — on the current line,
        after cutting the line if it would pass :data:`FOLD_WIDTH` octets."""
        if self._column + len(data) > FOLD_WIDTH:
            self._sink.write_bytes(b"\r\n ")
            self._column = 1
        self._sink.write_bytes(data)
        self._column += len(data)

    def _ascii_units(self, text: str) -> None:
        for b in text.encode("ascii"):
            self._unit(bytes([b]))

    def _value_units(self, value: str, text: bool) -> None:
        """The units of the value: one character each, escaped when ``text``."""
        for ch in value:
            if text and ch in "\\;,\n":
                self._unit(b"\\" + {"\\": b"\\", ";": b";", ",": b",", "\n": b"n"}[ch])
            else:
                self._unit(ch.encode("utf-8"))

    @staticmethod
    def _bytes_of(value: str) -> bytes:
        try:
            return value.encode("utf-8")
        except UnicodeEncodeError:
            raise LineBadValue() from None

    def property(self, name: str) -> None:
        """Start a property's line with its name."""
        self._column = 0
        self._ascii_units(name)

    def param(self, name: str, value: str, max_size: int) -> None:
        """Write ``;<name>=<value>``, quoting the value when it holds ``:``,
        ``;`` or ``,``. A value past ``max_size`` is :class:`LineTooLong`; one
        with a control character or a ``"`` is :class:`LineBadValue`."""
        data = self._bytes_of(value)
        if len(data) > max_size:
            raise LineTooLong()
        if any(b == _QUOTE or _is_control(b) for b in data):
            raise LineBadValue()
        quoted = any(b in (ord(":"), ord(";"), ord(",")) for b in data)
        self._unit(b";")
        self._ascii_units(name)
        self._unit(b"=")
        if quoted:
            self._unit(b'"')
        self._value_units(value, False)
        if quoted:
            self._unit(b'"')

    def _end_line(self) -> None:
        self._column = 0
        self._raw("\r\n")

    def string(self, value: str, text: bool, max_size: int) -> None:
        """Write ``:<value>`` and end the line. With ``text``, ``\\``, ``;``,
        ``,`` and a line feed are written as escapes. A value past
        ``max_size`` is :class:`LineTooLong`; one with a control character
        (but a TEXT's line feed) is :class:`LineBadValue`."""
        data = self._bytes_of(value)
        if len(data) > max_size:
            raise LineTooLong()
        if any(_is_control(b) and not (text and b == _LF) for b in data):
            raise LineBadValue()
        self._unit(b":")
        self._value_units(value, text)
        self._end_line()

    def strings(self, values: List[str], separator: str, text: bool, max_size: int, max_values: int) -> None:
        """Write ``:<value>{separator}<value>…`` and end the line
        (docs/adr/0014). No values is :class:`LineRequiredMissing` and more than
        ``max_values`` is :class:`LineTooMany`. A value past ``max_size`` is
        :class:`LineTooLong`; one with a control character (but a TEXT's line
        feed), an empty one, and one that is not a TEXT and holds the separator
        are :class:`LineBadValue`, because a reader would cut or refuse them.
        Every value is held before any of the line is written."""
        if not values:
            raise LineRequiredMissing()
        if len(values) > max_values:
            raise LineTooMany()
        cut = separator.encode("ascii")
        for value in values:
            data = self._bytes_of(value)
            if len(data) > max_size:
                raise LineTooLong()
            if any(_is_control(b) and not (text and b == _LF) for b in data):
                raise LineBadValue()
            if not data or (not text and cut in data):
                raise LineBadValue()
        self._unit(b":")
        for index, value in enumerate(values):
            if index:
                self._unit(cut)
            self._value_units(value, text)
        self._end_line()

    def _digits(self, text: str) -> None:
        self._unit(b":")
        self._ascii_units(text)
        self._end_line()

    def uint(self, value: int) -> None:
        """Write ``:<value>`` as decimal digits and end the line."""
        self._digits(str(value))

    def integer(self, value: int) -> None:
        """Write ``:<value>`` as decimal digits, ``-`` first when negative, and
        end the line."""
        self._digits(str(value))

    def boolean(self, value: bool) -> None:
        """Write ``:TRUE`` or ``:FALSE`` and end the line."""
        self._digits("TRUE" if value else "FALSE")

    @staticmethod
    def _text_of(table: EnumTexts, carrier: int) -> str:
        """The text of the first variant with ``carrier``, as the enum declares
        it; a carrier of an open enum that no variant declares has none, and is
        :class:`LineBadValue` (docs/adr/0015)."""
        for text, held in table:
            if held == carrier:
                return text
        raise LineBadValue()

    def enum_param(self, name: str, table: EnumTexts, carrier: int) -> None:
        """Write ``;<name>=<text>`` for the variant with ``carrier``. A text is
        letters, digits and hyphens, so it is never quoted."""
        text = self._text_of(table, carrier)
        self._unit(b";")
        self._ascii_units(name)
        self._unit(b"=")
        self._ascii_units(text)

    def enum_value(self, table: EnumTexts, carrier: int) -> None:
        """Write ``:<text>`` for the variant with ``carrier`` and end the
        line."""
        self._digits(self._text_of(table, carrier))
