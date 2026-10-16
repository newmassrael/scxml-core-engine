// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import (
	"sort"
	"strings"
)

// store is the key-value store, in memory, with the quota the embedding set. Go
// compares strings by their bytes, which for UTF-8 is the order the specification
// gives.
type store struct {
	keys     map[string]string
	quota    int
	hasQuota bool
}

func (s *store) entries() [][2]string {
	names := make([]string, 0, len(s.keys))
	for key := range s.keys {
		names = append(names, key)
	}
	sort.Strings(names)
	out := make([][2]string, 0, len(names))
	for _, key := range names {
		out = append(out, [2]string{key, s.keys[key]})
	}
	return out
}

// validKey is a key: it holds no U+0000, and is not empty (its 256-byte bound is
// the record's own).
func validKey(key string) bool {
	return key != "" && !strings.ContainsRune(key, 0)
}

func (h *ReferenceHost) kvGet(key string) Outcome {
	if value, ok := h.store.keys[key]; ok {
		return record(map[string]any{"found": true, "value": value})
	}
	return record(map[string]any{"found": false, "value": ""})
}

func (h *ReferenceHost) kvPut(key, value string) Outcome {
	_, replaced := h.store.keys[key]
	if !replaced && h.store.hasQuota && len(h.store.keys) >= h.store.quota {
		return failure("sce-host-kv-put", "quota_exceeded")
	}
	if h.store.keys == nil {
		h.store.keys = map[string]string{}
	}
	h.store.keys[key] = value
	return record(map[string]any{"replaced": replaced})
}

func (h *ReferenceHost) kvDelete(key string) Outcome {
	_, existed := h.store.keys[key]
	delete(h.store.keys, key)
	return record(map[string]any{"existed": existed})
}

// kvNextKey is the first key, in ascending order of UTF-8 bytes, that is greater
// than after and begins with prefix; after is excluded, and an empty after starts
// at the least key.
func (h *ReferenceHost) kvNextKey(after, prefix string) Outcome {
	for _, entry := range h.store.entries() {
		key := entry[0]
		if (after == "" || key > after) && strings.HasPrefix(key, prefix) {
			return record(map[string]any{"found": true, "key": key})
		}
	}
	return record(map[string]any{"found": false, "key": ""})
}
