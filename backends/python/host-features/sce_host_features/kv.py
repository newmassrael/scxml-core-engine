# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The key-value store (docs/SCE_HOST_FEATURES.md, section 3), in memory."""

from __future__ import annotations

from typing import TYPE_CHECKING, Dict, List, Optional, Tuple

from .records import Outcome, failure, record

if TYPE_CHECKING:
    from .host import ReferenceHost


class Store:
    """Keys and values, with the quota the embedding set. A `str` compares by code
    point, which for text is the order of its UTF-8 bytes, the order the
    specification gives."""

    def __init__(self) -> None:
        self.keys: Dict[str, str] = {}
        self.quota: Optional[int] = None

    def entries(self) -> List[Tuple[str, str]]:
        return sorted(self.keys.items())


def valid_key(key: str) -> bool:
    """A key is not empty and holds no U+0000 (its 256-byte bound is the record's
    own)."""
    return key != "" and "\x00" not in key


def get(host: "ReferenceHost", key: str) -> Outcome:
    if key in host.store.keys:
        return record({"found": True, "value": host.store.keys[key]})
    return record({"found": False, "value": ""})


def put(host: "ReferenceHost", key: str, value: str) -> Outcome:
    store = host.store
    replaced = key in store.keys
    if not replaced and store.quota is not None and len(store.keys) >= store.quota:
        return failure("sce-host-kv-put", "quota_exceeded")
    store.keys[key] = value
    return record({"replaced": replaced})


def delete(host: "ReferenceHost", key: str) -> Outcome:
    return record({"existed": host.store.keys.pop(key, None) is not None})


def next_key(host: "ReferenceHost", after: str, prefix: str) -> Outcome:
    """The first key, in ascending order of UTF-8 bytes, that is greater than
    `after` and begins with `prefix`; `after` is excluded, and an empty `after`
    starts at the least key."""
    for key, _ in host.store.entries():
        if (after == "" or key > after) and key.startswith(prefix):
            return record({"found": True, "key": key})
    return record({"found": False, "key": ""})
