# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The key-value store (docs/SCE_HOST_FEATURES.md, section 3), in memory."""

from __future__ import annotations

from typing import TYPE_CHECKING, Dict, List, NamedTuple, Optional, Tuple

from .records import Outcome, failure, record

if TYPE_CHECKING:
    from .host import ReferenceHost


class Entry(NamedTuple):
    """A stored value and the version its write gave the key."""

    value: str
    version: int


class Store:
    """Keys and values, with the quota the embedding set. A `str` compares by code
    point, which for text is the order of its UTF-8 bytes, the order the
    specification gives."""

    def __init__(self) -> None:
        self.keys: Dict[str, Entry] = {}
        self.quota: Optional[int] = None
        # The last number a write or a removal took: versions are numbered from
        # this one counter of the store (docs/SCE_HOST_FEATURES.md, section 8), so
        # a key is never given a version it had, even after it was removed.
        self._last_version = 0

    def entries(self) -> List[Tuple[str, str]]:
        return sorted((key, entry.value) for key, entry in self.keys.items())

    def next_version(self) -> int:
        """The counter's next number, taken."""
        self._last_version += 1
        return self._last_version

    def version_of(self, key: str) -> int:
        """The version `key` is at: 0 for a key the store does not hold."""
        entry = self.keys.get(key)
        return entry.version if entry is not None else 0

    def over_quota(self, key: str) -> bool:
        """Whether writing `key` would hold one key more than the quota allows."""
        return key not in self.keys and self.quota is not None and len(self.keys) >= self.quota

    def write(self, key: str, value: str) -> int:
        """Keep `value` under `key` at the counter's next number; that number."""
        version = self.next_version()
        self.keys[key] = Entry(value, version)
        return version


def valid_key(key: str) -> bool:
    """A key is not empty and holds no U+0000 (its 256-byte bound is the record's
    own)."""
    return key != "" and "\x00" not in key


def valid_version(text: str) -> bool:
    """An expected version is one or more of the digits 0 to 9, and nothing else:
    not `str.isdigit`, which takes the digits of other scripts too."""
    return text != "" and all("0" <= c <= "9" for c in text)


def get(host: "ReferenceHost", key: str) -> Outcome:
    entry = host.store.keys.get(key)
    if entry is not None:
        return record({"found": True, "value": entry.value, "version": str(entry.version)})
    return record({"found": False, "value": "", "version": "0"})


def put(host: "ReferenceHost", key: str, value: str) -> Outcome:
    store = host.store
    if store.over_quota(key):
        return failure("sce-host-kv-put", "quota_exceeded")
    replaced = key in store.keys
    store.write(key, value)
    return record({"replaced": replaced})


def put_if(host: "ReferenceHost", key: str, value: str, expected: str) -> Outcome:
    """Write `value` under `key` only when the key is at `expected`, a text of
    digits the refusal already judged. It is compared as a number, so leading
    zeros do not count, and one past the largest 64-bit unsigned integer names no
    version: every version the store gives is below it."""
    store = host.store
    current = store.version_of(key)
    if int(expected) != current:
        return record({"written": False, "version": str(current)})
    if store.over_quota(key):
        return failure("sce-host-kv-put-if", "quota_exceeded")
    return record({"written": True, "version": str(store.write(key, value))})


def delete(host: "ReferenceHost", key: str) -> Outcome:
    store = host.store
    existed = store.keys.pop(key, None) is not None
    if existed:
        store.next_version()
    return record({"existed": existed})


def next_key(host: "ReferenceHost", after: str, prefix: str) -> Outcome:
    """The first key, in ascending order of UTF-8 bytes, that is greater than
    `after` and begins with `prefix`; `after` is excluded, and an empty `after`
    starts at the least key."""
    for key, _ in host.store.entries():
        if (after == "" or key > after) and key.startswith(prefix):
            return record({"found": True, "key": key})
    return record({"found": False, "key": ""})
