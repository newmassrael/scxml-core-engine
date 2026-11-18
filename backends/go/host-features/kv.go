// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import (
	"sort"
	"strconv"
	"strings"
)

// entry is a stored value and the version its write gave the key.
type entry struct {
	value   string
	version uint64
}

// store is the key-value store, in memory, with the quota the embedding set. Go
// compares strings by their bytes, which for UTF-8 is the order the specification
// gives.
type store struct {
	keys     map[string]entry
	quota    int
	hasQuota bool
	// lastVersion is the last number a write or a removal took: versions are
	// numbered from this one counter of the store (docs/SCE_HOST_FEATURES.md,
	// section 8), so a key is never given a version it had, even after it was
	// removed.
	lastVersion uint64
}

func (s *store) entries() [][2]string {
	names := make([]string, 0, len(s.keys))
	for key := range s.keys {
		names = append(names, key)
	}
	sort.Strings(names)
	out := make([][2]string, 0, len(names))
	for _, key := range names {
		out = append(out, [2]string{key, s.keys[key].value})
	}
	return out
}

// nextVersion takes the counter's next number.
func (s *store) nextVersion() uint64 {
	s.lastVersion++
	return s.lastVersion
}

// versionOf is the version key is at: 0 for a key the store does not hold.
func (s *store) versionOf(key string) uint64 {
	return s.keys[key].version
}

// overQuota is whether writing key would hold one key more than the quota allows.
func (s *store) overQuota(key string) bool {
	_, held := s.keys[key]
	return !held && s.hasQuota && len(s.keys) >= s.quota
}

// write keeps value under key at the counter's next number, and answers that
// number.
func (s *store) write(key, value string) uint64 {
	if s.keys == nil {
		s.keys = map[string]entry{}
	}
	version := s.nextVersion()
	s.keys[key] = entry{value: value, version: version}
	return version
}

// validKey is a key: it holds no U+0000, and is not empty (its 256-byte bound is
// the record's own).
func validKey(key string) bool {
	return key != "" && !strings.ContainsRune(key, 0)
}

// validVersion is an expected version: one or more of the digits 0 to 9, and
// nothing else.
func validVersion(text string) bool {
	if text == "" {
		return false
	}
	for i := 0; i < len(text); i++ {
		if text[i] < '0' || text[i] > '9' {
			return false
		}
	}
	return true
}

func (h *ReferenceHost) kvGet(key string) Outcome {
	if stored, ok := h.store.keys[key]; ok {
		return record(map[string]any{
			"found":   true,
			"value":   stored.value,
			"version": strconv.FormatUint(stored.version, 10),
		})
	}
	return record(map[string]any{"found": false, "value": "", "version": "0"})
}

func (h *ReferenceHost) kvPut(key, value string) Outcome {
	if h.store.overQuota(key) {
		return failure("sce-host-kv-put", "quota_exceeded")
	}
	_, replaced := h.store.keys[key]
	h.store.write(key, value)
	return record(map[string]any{"replaced": replaced})
}

// kvPutIf writes value under key only when the key is at expected, a text of
// digits the refusal already judged. It is compared as a number, so leading
// zeros do not count, and one past the largest uint64 names no version.
func (h *ReferenceHost) kvPutIf(key, value, expected string) Outcome {
	current := h.store.versionOf(key)
	if wanted, err := strconv.ParseUint(expected, 10, 64); err != nil || wanted != current {
		return record(map[string]any{"written": false, "version": strconv.FormatUint(current, 10)})
	}
	if h.store.overQuota(key) {
		return failure("sce-host-kv-put-if", "quota_exceeded")
	}
	version := h.store.write(key, value)
	return record(map[string]any{"written": true, "version": strconv.FormatUint(version, 10)})
}

func (h *ReferenceHost) kvDelete(key string) Outcome {
	_, existed := h.store.keys[key]
	if existed {
		delete(h.store.keys, key)
		h.store.nextVersion()
	}
	return record(map[string]any{"existed": existed})
}

// kvNextKey is the first key, in ascending order of UTF-8 bytes, that is greater
// than after and begins with prefix; after is excluded, and an empty after starts
// at the least key.
func (h *ReferenceHost) kvNextKey(after, prefix string) Outcome {
	for _, pair := range h.store.entries() {
		key := pair[0]
		if (after == "" || key > after) && strings.HasPrefix(key, prefix) {
			return record(map[string]any{"found": true, "key": key})
		}
	}
	return record(map[string]any{"found": false, "key": ""})
}
