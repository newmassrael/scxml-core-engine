# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The canonical form of a message's value in the test log
(docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3 and answer 8): one JSON value
per value of the profile's types, built from texts, truth values, lists and dicts
alone, so that the JSON Canonicalization Scheme
(``sce_runtime.saved_json.write_jcs``) writes two equal values as the same bytes
on every runtime.

- an integer of any width is the text of its decimal digits, since an ``int64``
  or a ``uint64`` is past what a JSON reader that holds numbers as doubles reads
  exactly;
- a ``bool`` is a truth value;
- a value of an enum is the name its document declares for its variant;
- a text is a text;
- a byte string is the text of its octets in upper-case hex, two digits each;
- a record is a dict of its fields by their declared ids, a field that is absent
  omitted, as TTCN-3's ``omit`` leaves it out;
- a list is a list of its elements in order;
- a value of a union is a dict of one member, named by its case's variant, whose
  value is the case's record, or ``{}`` for a case that holds none.

The generated unit of a test template over a record spells its ``canonical``
with these, by the types its document declares
(``forge/python/test_template.py.jinja2``): the form is the profile type's, not
the language's, so it is written from the document's types on every backend
alike, where a byte string and a list of ``uint8`` are one type in some
languages and two in others.

The writers of a byte string and of a list are ``byte_string`` and ``list_of``,
where the other runtimes name them ``bytes`` and ``list``, so that neither hides
the built-in of the same name.

Cross-language siblings: ``sce_rust_runtime::test::canonical`` (Rust),
``com.sce.runtime.test.Canonical`` (Kotlin).
"""

from __future__ import annotations

from typing import Any, Dict, Iterable, List, Optional, Tuple, Union

__all__ = [
    "CanonicalValue",
    "boolean",
    "byte_string",
    "enumerated",
    "integer",
    "list_of",
    "open_enumerated",
    "record",
    "text",
    "union_case",
]

#: A value in the canonical form: a text, a truth value, a list of values, or a
#: dict of values by their names. Never a number and never ``None``.
CanonicalValue = Union[str, bool, List[Any], Dict[str, Any]]


def integer(value: int) -> str:
    """An integer of any width the profile has: the text of its decimal digits.

    Raises ``TypeError`` for a ``bool``, which Python holds as an ``int`` but
    which is no integer of the profile, and for any value that is not an
    ``int``."""
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{type(value).__name__} is no integer of the test profile")
    return str(int(value))


def boolean(value: bool) -> bool:
    """A ``bool``."""
    return value


def text(value: str) -> str:
    """A text."""
    return value


def byte_string(value: bytes) -> str:
    """A byte string: its octets in upper-case hex, two digits each, as TTCN-3
    writes an ``octetstring`` (``'00FF'O``) without its quotes and its ``O``."""
    return value.hex().upper()


def enumerated(declared: str) -> str:
    """A value of a closed enum: the name its document declares for the
    variant."""
    return declared


def open_enumerated(declared: Optional[str], number: int) -> str:
    """A value of an enum of an open set (``sce:strict-variants="false"``): the
    name its document declares for it, or, for a number no variant declares, the
    text of that number's decimal digits (:func:`integer`). No variant's name is
    such a text, since a name is an identifier. The TTCN-3 export refuses an
    enum of an open set (docs/SCE_TEST_PROFILE.md §9.5), so only a native log
    holds this form."""
    if declared is not None:
        return declared
    return integer(number)


def record(*fields: Tuple[str, Optional[CanonicalValue]]) -> Dict[str, CanonicalValue]:
    """A record: each field by its declared id, in any order, since the scheme
    sorts a dict's members; a field given as ``None`` is absent and omitted."""
    return {field: value for field, value in fields if value is not None}


def list_of(elements: Iterable[CanonicalValue]) -> List[CanonicalValue]:
    """A list: its elements in order."""
    return list(elements)


def union_case(variant: str, case_record: Optional[CanonicalValue]) -> Dict[str, CanonicalValue]:
    """A value of a union: one member, named by the variant of its case, holding
    the case's record, or the empty dict for a case that holds none."""
    return {variant: case_record if case_record is not None else {}}
