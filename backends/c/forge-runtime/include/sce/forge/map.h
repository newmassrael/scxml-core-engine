/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The value of a `map<K, V>[B]` and of a `set<K>[B]` (docs/adr/0108, D1 to D8) as a
 * C11 machine holds it: one algorithm for both, the set being the map whose entries
 * carry no value.
 *
 * A map holds at most `B` entries in the order their keys were first put (D2). Its
 * storage is the value itself, reserved whole where the machine holds it (D4,
 * "Storage"), and it is (D8):
 *
 *   - an array of `B` entry slots with a free list of the unused ones, so a put takes
 *     a free slot and a remove gives it back, and no operation moves an entry;
 *   - a doubly linked order through the slots, which is the order a walk, the saved
 *     form and a host's view follow: a put links its slot last, a replace leaves the
 *     links alone, and a remove unlinks its slot;
 *   - an open-addressing index of exactly twice the bound's cells, from the hash of a
 *     key to its slot, its first cell chosen by multiplying the hash into that range,
 *     probed linearly, with tombstones a put reuses and a rebuild clears once they
 *     outnumber the live entries.
 *
 * A slot is named by its position, one past its place in the array, so that 0 is no
 * slot and storage of zero bytes is a map with no entry.
 *
 * The hash is SipHash-1-3 under the 128-bit seed each map is started with
 * (`sce_map_seed_t`), over the canonical bytes of the key: two keys are one key
 * exactly when they write the same bytes. A fixed-width integer writes its value as
 * the 16 little-endian bytes of the 128-bit integer it is, whatever its own width, so
 * a narrower or a wider value finds the key of the same number; a `bool` one byte; a
 * value of an enum the number its constant carries, as an unsigned integer; a text
 * and a byte string their length as eight little-endian bytes and then their bytes;
 * a record its fields in the order its schema declares them. A key is looked up by a
 * pointer to a value of any type and the function that writes that value's canonical
 * bytes (`sce_map_feed_fn`), so a probe of another width, a text pointer and a
 * payload taken whole each find the key whose bytes they write. Nothing a document
 * or a host sees depends on the hash: the order is the links', and a map restored
 * under another seed is the same map.
 *
 * C has no generic type, so `SCE_MAP_DECLARE` and `SCE_SET_DECLARE` declare the type
 * of one key, value and bound, with the few functions that take its key and value by
 * type; every other operation is one function of this header over the map's address,
 * which the type describes to it through the layout it is started with
 * (`sce_map_layout_t`).
 *
 * A map may hold a map or a set as its value (docs/adr/0108, D12), declared with
 * `SCE_MAP_NESTED_DECLARE`: each such value is a started map, written where it stands
 * (`sce_map_value_mut`, `sce_map_put_entry`) and never copied through a frame, and the
 * value a read at an absent key gives is a started empty map too, so a read through it
 * finds nothing and never meets storage no map was started in. Each inner map hashes
 * under the seed drawn from the seed of the map that holds it (`sce_map_seed_of_inner`).
 *
 * A machine without an entropy source takes the seed from its host. A host with no
 * entropy hands a fixed seed, and a map of such a machine can then be flooded by a
 * peer that knows the seed: a deployment that faces peers it does not trust hands a
 * seed drawn from entropy.
 *
 * Header-only, with no allocator: freestanding `<stdbool.h>`, `<stddef.h>`,
 * `<stdint.h>` and `<string.h>` only, and the runtime's byte view and failure record.
 */
#ifndef SCE_FORGE_MAP_H
#define SCE_FORGE_MAP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce/forge/algorithm.h"
#include "sce/forge/bytes.h"
#include "sce/portability.h"
#include "sce/text.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ── the seed ──────────────────────────────────────────────────────────────────── */

/* The 128-bit key a map hashes its keys under (docs/adr/0108, D8): `k0` its low half and
 * `k1` its high half, as SipHash reads a key. */
typedef struct {
    uint64_t k0;
    uint64_t k1;
} sce_map_seed_t;

/* The seed of the two halves of a 128-bit key, low first. */
static inline sce_map_seed_t sce_map_seed_from_halves(uint64_t low, uint64_t high) {
    sce_map_seed_t seed;
    seed.k0 = low;
    seed.k1 = high;
    return seed;
}

/* The seed of the 16 bytes `bytes`: the first eight its low half and the last eight its
 * high half, each little-endian, as SipHash reads a key. */
static inline sce_map_seed_t sce_map_seed_of_bytes(const uint8_t bytes[16]) {
    uint64_t low = 0u;
    uint64_t high = 0u;
    for (unsigned i = 0u; i < 8u; ++i) {
        low |= (uint64_t)bytes[i] << (8u * i);
        high |= (uint64_t)bytes[i + 8u] << (8u * i);
    }
    return sce_map_seed_from_halves(low, high);
}

/* ── SipHash ───────────────────────────────────────────────────────────────────── */

/* SipHash with `compress_rounds` compression rounds and `final_rounds` finalization
 * rounds over a 128-bit key (Aumasson and Bernstein, "SipHash: a fast short-input PRF",
 * 2012). A map hashes with SipHash-1-3; the rounds are arguments so the one
 * implementation is held to the paper's SipHash-2-4 vectors. */
typedef struct {
    uint64_t v0;
    uint64_t v1;
    uint64_t v2;
    uint64_t v3;
    /* The bytes of the message not yet compressed, little-endian, at most 7. */
    uint64_t tail;
    /* How many bytes `tail` holds. */
    unsigned held;
    /* How many bytes were written in all; its low byte ends the message. */
    uint64_t length;
    unsigned compress_rounds;
    unsigned final_rounds;
} sce_siphash_t;

static inline uint64_t sce_siphash_rotate_(uint64_t value, unsigned by) {
    return (value << by) | (value >> (64u - by));
}

static inline void sce_siphash_round_(sce_siphash_t *s) {
    s->v0 += s->v1;
    s->v1 = sce_siphash_rotate_(s->v1, 13u);
    s->v1 ^= s->v0;
    s->v0 = sce_siphash_rotate_(s->v0, 32u);
    s->v2 += s->v3;
    s->v3 = sce_siphash_rotate_(s->v3, 16u);
    s->v3 ^= s->v2;
    s->v0 += s->v3;
    s->v3 = sce_siphash_rotate_(s->v3, 21u);
    s->v3 ^= s->v0;
    s->v2 += s->v1;
    s->v1 = sce_siphash_rotate_(s->v1, 17u);
    s->v1 ^= s->v2;
    s->v2 = sce_siphash_rotate_(s->v2, 32u);
}

static inline void sce_siphash_compress_(sce_siphash_t *s, uint64_t word) {
    s->v3 ^= word;
    for (unsigned i = 0u; i < s->compress_rounds; ++i) {
        sce_siphash_round_(s);
    }
    s->v0 ^= word;
}

/* A SipHash of the given rounds under `key`, with nothing written yet. */
static inline void sce_siphash_begin(sce_siphash_t *s, sce_map_seed_t key, unsigned compress_rounds,
                                     unsigned final_rounds) {
    s->v0 = key.k0 ^ UINT64_C(0x736f6d6570736575);
    s->v1 = key.k1 ^ UINT64_C(0x646f72616e646f6d);
    s->v2 = key.k0 ^ UINT64_C(0x6c7967656e657261);
    s->v3 = key.k1 ^ UINT64_C(0x7465646279746573);
    s->tail = 0u;
    s->held = 0u;
    s->length = 0u;
    s->compress_rounds = compress_rounds;
    s->final_rounds = final_rounds;
}

/* The next `len` bytes of the message: a message written in pieces hashes as the same
 * message written whole. */
static inline void sce_siphash_write(sce_siphash_t *s, const uint8_t *bytes, size_t len) {
    s->length += (uint64_t)len;
    for (size_t i = 0u; i < len; ++i) {
        s->tail |= (uint64_t)bytes[i] << (8u * s->held);
        s->held += 1u;
        if (s->held == 8u) {
            sce_siphash_compress_(s, s->tail);
            s->tail = 0u;
            s->held = 0u;
        }
    }
}

/* The hash of the message written, which ends it. */
static inline uint64_t sce_siphash_finish(sce_siphash_t *s) {
    sce_siphash_compress_(s, ((s->length & 0xffu) << 56) | s->tail);
    s->v2 ^= 0xffu;
    for (unsigned i = 0u; i < s->final_rounds; ++i) {
        sce_siphash_round_(s);
    }
    return s->v0 ^ s->v1 ^ s->v2 ^ s->v3;
}

/* The seed of the map numbered `ordinal` among those one machine holds: two SipHash-1-3
 * values of the number under `seed`, so each map of a machine is keyed apart from the
 * others and none of them by the seed itself. */
static inline sce_map_seed_t sce_map_seed_derive(sce_map_seed_t seed, uint32_t ordinal) {
    uint64_t halves[2];
    for (unsigned side = 0u; side < 2u; ++side) {
        sce_siphash_t sip;
        uint8_t message[5];
        message[0] = (uint8_t)side;
        for (unsigned i = 0u; i < 4u; ++i) {
            message[1u + i] = (uint8_t)(ordinal >> (8u * i));
        }
        sce_siphash_begin(&sip, seed, 1u, 3u);
        sce_siphash_write(&sip, message, sizeof(message));
        halves[side] = sce_siphash_finish(&sip);
    }
    return sce_map_seed_from_halves(halves[0], halves[1]);
}

/* The ordinal the seed of a map held as a value is drawn under, from the seed of the map
 * that holds it: past every ordinal a machine numbers its own maps with, so no inner map is
 * keyed as one of the machine's maps is. */
#define SCE_MAP_SEED_INNER_ORDINAL UINT32_MAX

/* The seed of each map the map seeded `outer` holds as a value (docs/adr/0108, D12 item 7):
 * one ordinal at every depth, each depth's seed drawn from the one above it, so the maps of
 * one depth are keyed apart from those of another and a restore draws them again from the
 * outer seed alone. */
static inline sce_map_seed_t sce_map_seed_of_inner(sce_map_seed_t outer) {
    return sce_map_seed_derive(outer, SCE_MAP_SEED_INNER_ORDINAL);
}

/* The seed a machine hands the child it starts under the invoke id `invoke_id`, from its own
 * `seed`: two SipHash-1-3 values of the id under `seed`, each message opened by a tag
 * (2 and 3) no seed of the machine's own maps opens its message with
 * (`sce_map_seed_derive`, 0 and 1), so the child's maps are keyed apart from the parent's,
 * and each child of one parent apart from the others. */
static inline sce_map_seed_t sce_map_seed_of_child(sce_map_seed_t seed, const char *invoke_id) {
    uint64_t halves[2];
    for (unsigned side = 0u; side < 2u; ++side) {
        sce_siphash_t sip;
        const uint8_t tag = (uint8_t)(2u + side);
        sce_siphash_begin(&sip, seed, 1u, 3u);
        sce_siphash_write(&sip, &tag, 1u);
        sce_siphash_write(&sip, (const uint8_t *)invoke_id, sce_text_length(invoke_id));
        halves[side] = sce_siphash_finish(&sip);
    }
    return sce_map_seed_from_halves(halves[0], halves[1]);
}

/* ── keys ──────────────────────────────────────────────────────────────────────── */

/* Where a key writes its canonical bytes: the hash, or a comparison with another key.
 * Each sink below begins with one of these, which its `write` is handed. */
typedef struct sce_map_sink sce_map_sink_t;

struct sce_map_sink {
    void (*write)(sce_map_sink_t *sink, const uint8_t *bytes, size_t len);
};

/* What writes the canonical bytes of the key at `key` into `sink`. */
typedef void (*sce_map_feed_fn)(const void *key, sce_map_sink_t *sink);

/* An unsigned integer of any width, as the 128-bit integer it is. */
static inline void sce_map_feed_uint(sce_map_sink_t *sink, uint64_t value) {
    uint8_t bytes[16];
    for (unsigned i = 0u; i < 8u; ++i) {
        bytes[i] = (uint8_t)(value >> (8u * i));
        bytes[8u + i] = 0u;
    }
    sink->write(sink, bytes, sizeof(bytes));
}

/* A signed integer of any width, as the 128-bit integer it is: its high half all ones
 * below zero. */
static inline void sce_map_feed_int(sce_map_sink_t *sink, int64_t value) {
    const uint64_t bits = (uint64_t)value;
    const uint8_t fill = (uint8_t)(value < 0 ? 0xffu : 0u);
    uint8_t bytes[16];
    for (unsigned i = 0u; i < 8u; ++i) {
        bytes[i] = (uint8_t)(bits >> (8u * i));
        bytes[8u + i] = fill;
    }
    sink->write(sink, bytes, sizeof(bytes));
}

static inline void sce_map_feed_bool(sce_map_sink_t *sink, bool value) {
    const uint8_t byte = (uint8_t)(value ? 1u : 0u);
    sink->write(sink, &byte, 1u);
}

/* A byte string: its length as eight little-endian bytes, then its bytes. */
static inline void sce_map_feed_bytes(sce_map_sink_t *sink, const uint8_t *data, size_t len) {
    uint8_t length[8];
    for (unsigned i = 0u; i < 8u; ++i) {
        length[i] = (uint8_t)((uint64_t)len >> (8u * i));
    }
    sink->write(sink, length, sizeof(length));
    if (len != 0u) {
        sink->write(sink, data, len);
    }
}

/* A text: the byte string of its UTF-8 bytes. */
static inline void sce_map_feed_text(sce_map_sink_t *sink, const char *text) {
    sce_map_feed_bytes(sink, (const uint8_t *)text, sce_text_length(text));
}

/* The feeds of the keys whose type this header knows: each integer type, `bool`, a
 * text held in a buffer whose text begins it (`sce_static_string_<N>_t`), and the
 * probes a read looks a key up by (`SCE_MAP_KEY_*`): a 64-bit integer of either sign,
 * a pointer to a text and a byte view. */
#define SCE_MAP_INTEGER_KEY_(name, held, feed, wide)                                                                   \
    static inline void sce_map_key_##name(const void *key, sce_map_sink_t *sink) {                                     \
        feed(sink, (wide)(*(const held *)key));                                                                        \
    }
SCE_MAP_INTEGER_KEY_(uint8, uint8_t, sce_map_feed_uint, uint64_t)
SCE_MAP_INTEGER_KEY_(uint16, uint16_t, sce_map_feed_uint, uint64_t)
SCE_MAP_INTEGER_KEY_(uint32, uint32_t, sce_map_feed_uint, uint64_t)
SCE_MAP_INTEGER_KEY_(uint64, uint64_t, sce_map_feed_uint, uint64_t)
SCE_MAP_INTEGER_KEY_(int8, int8_t, sce_map_feed_int, int64_t)
SCE_MAP_INTEGER_KEY_(int16, int16_t, sce_map_feed_int, int64_t)
SCE_MAP_INTEGER_KEY_(int32, int32_t, sce_map_feed_int, int64_t)
SCE_MAP_INTEGER_KEY_(int64, int64_t, sce_map_feed_int, int64_t)
#undef SCE_MAP_INTEGER_KEY_

static inline void sce_map_key_bool(const void *key, sce_map_sink_t *sink) {
    sce_map_feed_bool(sink, *(const bool *)key);
}

/* A text buffer, whose first member is the text it holds. */
static inline void sce_map_key_text(const void *key, sce_map_sink_t *sink) {
    sce_map_feed_text(sink, (const char *)key);
}

/* A pointer to a text. */
static inline void sce_map_key_text_at(const void *key, sce_map_sink_t *sink) {
    sce_map_feed_text(sink, *(const char *const *)key);
}

/* A byte view. */
static inline void sce_map_key_bytes_view(const void *key, sce_map_sink_t *sink) {
    const sce_forge_bytes_view_t *view = (const sce_forge_bytes_view_t *)key;
    sce_map_feed_bytes(sink, view->data, view->len);
}

/* The probe of a key, a value read where the probe is made and held for the expression
 * it stands in, which the feed of the same name reads: an integer as a 64-bit integer
 * of its sign (`sce_map_key_int64`, `sce_map_key_uint64`), a value of an enum as the
 * unsigned number its constant carries (`sce_map_key_uint64`), a `bool`
 * (`sce_map_key_bool`), a text (`sce_map_key_text_at`) and a byte string, a buffer or
 * a view (`sce_map_key_bytes_view`). C code only: each is a compound literal. */
#define SCE_MAP_KEY_INT(value) (&(const int64_t){(int64_t)(value)})
#define SCE_MAP_KEY_UINT(value) (&(const uint64_t){(uint64_t)(value)})
#define SCE_MAP_KEY_BOOL(value) (&(const bool){(value)})
#define SCE_MAP_KEY_TEXT(value) (&(const char *const){(value)})
#define SCE_MAP_KEY_BYTES(value) (&(const sce_forge_bytes_view_t){(value).data, (value).len})

/* How many bytes a key writes. */
typedef struct {
    sce_map_sink_t sink;
    size_t length;
} sce_map_count_sink_t;

static inline void sce_map_count_write_(sce_map_sink_t *sink, const uint8_t *bytes, size_t len) {
    (void)bytes;
    ((sce_map_count_sink_t *)sink)->length += len;
}

/* The bytes `expected` held against the bytes a key writes from position `start` on:
 * whether every one of them is equal, and how many were met. */
typedef struct {
    sce_map_sink_t sink;
    size_t start;
    const uint8_t *expected;
    size_t expected_len;
    size_t at;
    size_t met;
    bool equal;
} sce_map_window_t;

static inline void sce_map_window_write_(sce_map_sink_t *sink, const uint8_t *bytes, size_t len) {
    sce_map_window_t *window = (sce_map_window_t *)sink;
    const size_t begin = window->at;
    const size_t end = begin + len;
    const size_t low = begin > window->start ? begin : window->start;
    const size_t stop = window->start + window->expected_len;
    const size_t high = end < stop ? end : stop;
    window->at = end;
    if (low < high) {
        if (memcmp(bytes + (low - begin), window->expected + (low - window->start), high - low) != 0) {
            window->equal = false;
        }
        window->met += high - low;
    }
}

/* The bytes one key writes, each run held against the same run of the bytes `other`
 * writes, which is written again for each run: no key's bytes are held anywhere, so two
 * keys of any bound compare with nothing reserved for them. */
typedef struct {
    sce_map_sink_t sink;
    const void *other;
    sce_map_feed_fn other_feed;
    size_t at;
    bool equal;
} sce_map_matcher_t;

static inline void sce_map_matcher_write_(sce_map_sink_t *sink, const uint8_t *bytes, size_t len) {
    sce_map_matcher_t *matcher = (sce_map_matcher_t *)sink;
    if (matcher->equal && len != 0u) {
        sce_map_window_t window;
        window.sink.write = sce_map_window_write_;
        window.start = matcher->at;
        window.expected = bytes;
        window.expected_len = len;
        window.at = 0u;
        window.met = 0u;
        window.equal = true;
        matcher->other_feed(matcher->other, &window.sink);
        matcher->equal = window.equal && window.met == len;
    }
    matcher->at += len;
}

/* Whether the key `a`, written by `feed_a`, and the key `b`, written by `feed_b`, are
 * one key: they write the same bytes. */
static inline bool sce_map_same_key(const void *a, sce_map_feed_fn feed_a, const void *b, sce_map_feed_fn feed_b) {
    sce_map_count_sink_t length;
    sce_map_matcher_t matcher;
    length.sink.write = sce_map_count_write_;
    length.length = 0u;
    feed_b(b, &length.sink);
    matcher.sink.write = sce_map_matcher_write_;
    matcher.other = b;
    matcher.other_feed = feed_b;
    matcher.at = 0u;
    matcher.equal = true;
    feed_a(a, &matcher.sink);
    return matcher.equal && matcher.at == length.length;
}

typedef struct {
    sce_map_sink_t sink;
    sce_siphash_t sip;
} sce_map_hash_sink_t;

static inline void sce_map_hash_write_(sce_map_sink_t *sink, const uint8_t *bytes, size_t len) {
    sce_siphash_write(&((sce_map_hash_sink_t *)sink)->sip, bytes, len);
}

/* The SipHash-1-3 of the key `key`, written by `feed`, under `seed`. */
static inline uint64_t sce_map_hash(sce_map_seed_t seed, const void *key, sce_map_feed_fn feed) {
    sce_map_hash_sink_t hash;
    hash.sink.write = sce_map_hash_write_;
    sce_siphash_begin(&hash.sip, seed, 1u, 3u);
    feed(key, &hash.sink);
    return sce_siphash_finish(&hash.sip);
}

/* The cell of an index of `cells` cells a hash starts its probe at: the high 64 bits of
 * the product of the two, which spreads a hash over any number of cells. */
static inline size_t sce_map_home(uint64_t hash, uint32_t cells) {
    const uint64_t low = (hash & UINT64_C(0xffffffff)) * (uint64_t)cells;
    const uint64_t high = (hash >> 32) * (uint64_t)cells;
    return (size_t)((high + (low >> 32)) >> 32);
}

/* ── the map ───────────────────────────────────────────────────────────────────── */

/* No slot: the end of a list of slots, and the position before the first. */
#define SCE_MAP_NO_ENTRY 0u
/* An index cell no key ever took. */
#define SCE_MAP_CELL_EMPTY 0u
/* An index cell whose key was removed: a probe goes on past it, and a put may take it. */
#define SCE_MAP_CELL_TOMBSTONE UINT32_MAX

/* What a put did (docs/adr/0108, D4), which a statechart's `<sce:put outcome>` writes as
 * the variant of the same name of the standard `put_outcome` enum. */
typedef enum {
    /* The key was absent, and its entry is now the last in the map's order. */
    SCE_MAP_PUT_INSERTED = 0,
    /* The key was present, and its value is replaced where it stands. */
    SCE_MAP_PUT_REPLACED = 1,
    /* The key was absent and the map holds its bound: the map is as it was. */
    SCE_MAP_PUT_FULL = 2
} sce_map_put_outcome_t;

/* What a remove did, which `<sce:remove outcome>` writes as the variant of the same name
 * of the standard `remove_outcome` enum. */
typedef enum {
    /* The key was present, and its entry is gone; the others keep their order. */
    SCE_MAP_REMOVE_REMOVED = 0,
    /* The key was absent: the map is as it was. */
    SCE_MAP_REMOVE_ABSENT = 1
} sce_map_remove_outcome_t;

/* The first member of each slot: the hash of its key, which a rebuild of the index reads
 * instead of hashing again, and its neighbours, in the map's order for a slot in use and
 * on the free list (`next` alone) for one that is not. */
typedef struct {
    uint64_t hash;
    uint32_t prev;
    uint32_t next;
    bool used;
} sce_map_links_t;

/* What starts the zeroed storage at `value` as the empty map or set of one type, its keys
 * hashed under `seed`: the start of a map's value that is itself a map. */
typedef void (*sce_map_start_fn)(void *value, sce_map_seed_t seed);

/* What makes the started map or set at `to` take the entries of the one at `from`
 * (`sce_map_assign`): the put of a map's value that is itself a map. */
typedef void (*sce_map_assign_fn)(void *to, const void *from);

/* What one map type is, as `SCE_MAP_DECLARE` and `SCE_SET_DECLARE` describe it: its
 * bound, where a slot holds its key and its value and how large each is, where the map
 * holds its index, its slots and the value a read at an absent key gives, the feed of its
 * key, and what starts a value that is a map or a set and what puts one into another (both
 * NULL for a value of any other type, whose empty value is all zero and which a put copies
 * byte for byte). A value is reached as a map through these alone, so no code a map of
 * other values compiles reads its value as a map. A set's value is of size 0. */
typedef struct {
    uint32_t capacity;
    size_t slot_size;
    size_t key_offset;
    size_t key_size;
    size_t value_offset;
    size_t value_size;
    size_t index_offset;
    size_t slots_offset;
    size_t absent_offset;
    sce_map_feed_fn feed;
    sce_map_start_fn start_value;
    sce_map_assign_fn assign_value;
} sce_map_layout_t;

/* The first member of each map: what it is, the seed it hashes its keys under, the first
 * and the last entry of its order, the first free slot, how many slots it has taken
 * since it was emptied, how many entries it holds, and how many of its index's cells are
 * tombstones. */
typedef struct {
    const sce_map_layout_t *layout;
    sce_map_seed_t seed;
    uint32_t head;
    uint32_t tail;
    uint32_t free;
    uint32_t taken;
    uint32_t live;
    uint32_t tombstones;
} sce_map_core_t;

static inline const sce_map_core_t *sce_map_core_(const void *map) {
    return (const sce_map_core_t *)map;
}

static inline uint32_t sce_map_cells_(const void *map) {
    return 2u * sce_map_core_(map)->layout->capacity;
}

static inline uint32_t *sce_map_index_(void *map) {
    return (uint32_t *)((unsigned char *)map + sce_map_core_(map)->layout->index_offset);
}

static inline const uint32_t *sce_map_index_of_(const void *map) {
    return (const uint32_t *)((const unsigned char *)map + sce_map_core_(map)->layout->index_offset);
}

static inline unsigned char *sce_map_slot_(void *map, uint32_t at) {
    const sce_map_layout_t *layout = sce_map_core_(map)->layout;
    return (unsigned char *)map + layout->slots_offset + (size_t)(at - 1u) * layout->slot_size;
}

static inline const unsigned char *sce_map_slot_of_(const void *map, uint32_t at) {
    const sce_map_layout_t *layout = sce_map_core_(map)->layout;
    return (const unsigned char *)map + layout->slots_offset + (size_t)(at - 1u) * layout->slot_size;
}

static inline sce_map_links_t *sce_map_links_(void *map, uint32_t at) {
    return (sce_map_links_t *)sce_map_slot_(map, at);
}

static inline const sce_map_links_t *sce_map_links_of_(const void *map, uint32_t at) {
    return (const sce_map_links_t *)sce_map_slot_of_(map, at);
}

/* Empty the map, which then takes up to its bound again; its seed and its type stay. */
static inline void sce_map_clear(void *map) {
    sce_map_core_t *core = (sce_map_core_t *)map;
    memset(sce_map_index_(map), 0, (size_t)sce_map_cells_(map) * sizeof(uint32_t));
    core->head = SCE_MAP_NO_ENTRY;
    core->tail = SCE_MAP_NO_ENTRY;
    core->free = SCE_MAP_NO_ENTRY;
    core->taken = 0u;
    core->live = 0u;
    core->tombstones = 0u;
}

/* The value at `value` of a map of the type `layout` describes, its keys hashed under
 * `outer`, made the empty value of its type: all zero, and a map or a set started under
 * the seed drawn for the maps it holds (docs/adr/0108, D12 items 4 and 7). */
static inline void sce_map_value_empty_(const sce_map_layout_t *layout, void *value, sce_map_seed_t outer) {
    memset(value, 0, layout->value_size);
    if (layout->start_value != NULL) {
        layout->start_value(value, sce_map_seed_of_inner(outer));
    }
}

/* Start the storage at `map` as the empty map of the type `layout` describes, its keys
 * hashed under `seed`. The value a read at an absent key gives is the empty value: a
 * value that is a map is a started empty map, which a read through it finds nothing in. */
static inline void sce_map_start(void *map, const sce_map_layout_t *layout, sce_map_seed_t seed) {
    sce_map_core_t *core = (sce_map_core_t *)map;
    core->layout = layout;
    core->seed = seed;
    sce_map_clear(map);
    if (layout->value_size != 0u) {
        sce_map_value_empty_(layout, (unsigned char *)map + layout->absent_offset, seed);
    }
}

/* How many entries the map holds, a `uint32` as a list's length is (`len(m)`). */
static inline uint32_t sce_map_count(const void *map) {
    return sce_map_core_(map)->live;
}

/* The position of the first entry in the map's order, or `SCE_MAP_NO_ENTRY`. */
static inline uint32_t sce_map_first(const void *map) {
    return sce_map_core_(map)->head;
}

/* The position of the entry after the one at `at`, or `SCE_MAP_NO_ENTRY` after the last. */
static inline uint32_t sce_map_next(const void *map, uint32_t at) {
    return sce_map_links_of_(map, at)->next;
}

/* The key of the entry at `at`. */
static inline const void *sce_map_key_at(const void *map, uint32_t at) {
    return sce_map_slot_of_(map, at) + sce_map_core_(map)->layout->key_offset;
}

/* The value of the entry at `at`. */
static inline const void *sce_map_value_at(const void *map, uint32_t at) {
    return sce_map_slot_of_(map, at) + sce_map_core_(map)->layout->value_offset;
}

/* What a probe of the index found for a key: the key, at a cell and a position; or not
 * the key and the cell a put takes for it; or not the key and no cell for it. */
typedef struct {
    size_t cell;
    uint32_t at;
    bool found;
    bool room;
} sce_map_probe_t;

/* Where `key`, written by `feed`, of hash `hash`, is in the index of `map`, or the cell a
 * put of it takes: the first tombstone the probe passed, or the empty cell it ended at. */
static inline sce_map_probe_t sce_map_probe_(const void *map, const void *key, sce_map_feed_fn feed, uint64_t hash) {
    const sce_map_layout_t *layout = sce_map_core_(map)->layout;
    const uint32_t cells = sce_map_cells_(map);
    const uint32_t *held_at = sce_map_index_of_(map);
    sce_map_probe_t probe;
    size_t cell = sce_map_home(hash, cells);
    bool vacant = false;
    size_t vacant_cell = 0u;
    probe.cell = 0u;
    probe.at = SCE_MAP_NO_ENTRY;
    probe.found = false;
    probe.room = false;
    for (uint32_t step = 0u; step < cells; ++step) {
        const uint32_t held = held_at[cell];
        if (held == SCE_MAP_CELL_EMPTY) {
            probe.cell = vacant ? vacant_cell : cell;
            probe.room = true;
            return probe;
        }
        if (held == SCE_MAP_CELL_TOMBSTONE) {
            if (!vacant) {
                vacant = true;
                vacant_cell = cell;
            }
        } else if (sce_map_links_of_(map, held)->hash == hash &&
                   sce_map_same_key(sce_map_key_at(map, held), layout->feed, key, feed)) {
            probe.cell = cell;
            probe.at = held;
            probe.found = true;
            return probe;
        }
        cell = cell + 1u == cells ? 0u : cell + 1u;
    }
    probe.cell = vacant_cell;
    probe.room = vacant;
    return probe;
}

/* The index again, every entry at the first cell its stored hash probes free, and no
 * tombstone left. */
static inline void sce_map_rebuild_(void *map) {
    sce_map_core_t *core = (sce_map_core_t *)map;
    const uint32_t cells = sce_map_cells_(map);
    uint32_t *held_at = sce_map_index_(map);
    memset(held_at, 0, (size_t)cells * sizeof(uint32_t));
    core->tombstones = 0u;
    for (uint32_t at = core->head; at != SCE_MAP_NO_ENTRY; at = sce_map_links_(map, at)->next) {
        size_t cell = sce_map_home(sce_map_links_(map, at)->hash, cells);
        while (held_at[cell] != SCE_MAP_CELL_EMPTY) {
            cell = cell + 1u == cells ? 0u : cell + 1u;
        }
        held_at[cell] = at;
    }
}

/* Where `key`, written by `feed` (NULL for the map's own), is in the index of `map`. */
static inline sce_map_probe_t sce_map_find_(const void *map, const void *key, sce_map_feed_fn feed) {
    const sce_map_core_t *core = sce_map_core_(map);
    const sce_map_feed_fn by = feed != NULL ? feed : core->layout->feed;
    return sce_map_probe_(map, key, by, sce_map_hash(core->seed, key, by));
}

/* Whether `key`, written by `feed`, is a key of `map` (`has(m, k)`). A `feed` of NULL is
 * the map's own: `key` is a value of its key's type. */
static inline bool sce_map_holds(const void *map, const void *key, sce_map_feed_fn feed) {
    return sce_map_find_(map, key, feed).found;
}

/* The value at a key, or the zero value the map holds for a key it does not: whether the
 * key was found, which `sce_map_lookup_take` reads. */
typedef struct {
    const void *value;
    bool found;
} sce_map_found_t;

/* The value of `map` at `key`, written by `feed` (NULL for the map's own) (`m[k]`). */
static inline sce_map_found_t sce_map_lookup(const void *map, const void *key, sce_map_feed_fn feed) {
    const sce_map_core_t *core = sce_map_core_(map);
    const sce_map_probe_t probe = sce_map_find_(map, key, feed);
    sce_map_found_t found;
    found.found = probe.found;
    found.value = probe.found ? sce_map_value_at(map, probe.at)
                              : (const void *)((const unsigned char *)map + core->layout->absent_offset);
    return found;
}

/* The value of `map` at `key`, written by `feed` (NULL for the map's own), to be written
 * where the map holds it, or NULL for a key the map does not hold: what a write into a value
 * the map holds, a map, a list or a record's field, is made through (docs/adr/0108, D12
 * item 2). The index, the order and the free list are not touched, so the entry keeps its
 * slot and its place, and no value passes through the caller's frame. */
static inline void *sce_map_value_mut(void *map, const void *key, sce_map_feed_fn feed) {
    const sce_map_probe_t probe = sce_map_find_(map, key, feed);
    if (!probe.found) {
        return NULL;
    }
    return sce_map_slot_(map, probe.at) + sce_map_core_(map)->layout->value_offset;
}

/* The value `found` holds: a key that was not found fails the read `out-of-range`, as an
 * index outside a list does, and gives the map's zero value beside the failure. */
static inline const void *sce_map_lookup_take(sce_forge_algorithm_failure_t *failure, sce_map_found_t found) {
    if (!found.found) {
        sce_forge_algorithm_fail(failure, SCE_FORGE_ALGORITHM_OUT_OF_RANGE);
    }
    return found.value;
}

/* The value `found` holds, for a read whose key the build proved present (an algorithm that
 * does not declare `may-fail` reads a map at a key only where `has` holds it, docs/adr/0108,
 * D6 item 2): no failure is recorded, since none can happen. */
static inline const void *sce_map_lookup_found(sce_map_found_t found) {
    return found.value;
}

/* The value a read at a key gives, typed as the map holds its values: `found`, the
 * pointer `sce_map_lookup_take` answered, read as a value of the map `map` names. C code
 * only, and `map` is read for its type alone. */
#define SCE_MAP_VALUE_OF(map, found) (*(const __typeof__((map).slots[0].value) *)(found))

/* The value a read at a key gives, read as a value of the type `V`: what a map is read as
 * where it is reached through the address of its storage, which names no type of its bound
 * (an algorithm's map parameter, which views the map of any bound its caller lends, its values
 * held in the type the algorithm names, docs/adr/0108, D6 items 1 and 4). C code only. */
#define SCE_MAP_VALUE_AS(V, found) (*(const V *)(found))

/* The slot of the key at `key`, of `key_size` bytes, and in `outcome` what a put of it
 * does: a key that is present keeps its slot and its place, and one that is absent takes a
 * slot linked last, its key written and its value as the slot held it, while the map holds
 * fewer than its bound. Into a full map an absent key is not put, the map is as it was, and
 * the slot is NULL. A key smaller than the map's own is a text buffer of a smaller bound,
 * which the map's buffer holds with its rest zero. */
static inline unsigned char *sce_map_entry_(void *map, const void *key, size_t key_size,
                                            sce_map_put_outcome_t *outcome) {
    sce_map_core_t *core = (sce_map_core_t *)map;
    const sce_map_layout_t *layout = core->layout;
    const uint64_t hash = sce_map_hash(core->seed, key, layout->feed);
    const sce_map_probe_t probe = sce_map_probe_(map, key, layout->feed, hash);
    uint32_t *held_at = sce_map_index_(map);
    unsigned char *slot;
    sce_map_links_t *links;
    uint32_t at;
    if (probe.found) {
        *outcome = SCE_MAP_PUT_REPLACED;
        return sce_map_slot_(map, probe.at);
    }
    /* The index is twice the bound and its tombstones never outnumber the live entries, so
     * a map below its bound always has a cell for one more. */
    if (core->live >= layout->capacity || !probe.room) {
        *outcome = SCE_MAP_PUT_FULL;
        return NULL;
    }
    if (core->free != SCE_MAP_NO_ENTRY) {
        at = core->free;
        core->free = sce_map_links_(map, at)->next;
    } else {
        core->taken += 1u;
        at = core->taken;
    }
    slot = sce_map_slot_(map, at);
    links = (sce_map_links_t *)slot;
    links->hash = hash;
    links->prev = core->tail;
    links->next = SCE_MAP_NO_ENTRY;
    links->used = true;
    memset(slot + layout->key_offset, 0, layout->key_size);
    memcpy(slot + layout->key_offset, key, key_size);
    if (core->tail == SCE_MAP_NO_ENTRY) {
        core->head = at;
    } else {
        sce_map_links_(map, core->tail)->next = at;
    }
    core->tail = at;
    if (held_at[probe.cell] == SCE_MAP_CELL_TOMBSTONE) {
        core->tombstones -= 1u;
    }
    held_at[probe.cell] = at;
    core->live += 1u;
    *outcome = SCE_MAP_PUT_INSERTED;
    return slot;
}

/* Put the key at `key`, of `key_size` bytes, with the empty value of the map's value type,
 * and answer where that value is held, for the caller to write it there (docs/adr/0108, D12
 * items 2 and 4): a key that is present keeps its place and its value is made empty, and one
 * that is absent goes last while the map holds fewer than its bound. The empty value is all
 * zero, and a value that is a map or a set is started empty under the seed drawn from this
 * map's (`sce_map_seed_of_inner`). Into a full map an absent key is not put, the map is as it
 * was, and the answer is NULL. `outcome`, which is not NULL, takes what the put did, as
 * `sce_map_put` answers it. No value passes through the caller's frame, so a value of any
 * bound is put where the map holds it. A set's entry carries no value: the address answered
 * holds no byte. */
static inline void *sce_map_put_entry(void *map, const void *key, size_t key_size, sce_map_put_outcome_t *outcome) {
    const sce_map_core_t *core = sce_map_core_(map);
    unsigned char *slot = sce_map_entry_(map, key, key_size, outcome);
    void *value;
    if (slot == NULL) {
        return NULL;
    }
    value = slot + core->layout->value_offset;
    if (core->layout->value_size != 0u) {
        sce_map_value_empty_(core->layout, value, core->seed);
    }
    return value;
}

/* Put the key at `key`, of `key_size` bytes, with the value at `value`, of `value_size`:
 * a key that is present keeps its place and takes the value, and one that is absent goes
 * last while the map holds fewer than its bound. Into a full map an absent key is not
 * put, and the map is as it was. A key or a value smaller than the map's own is a text
 * buffer of a smaller bound, which the map's buffer holds with its rest zero. A value that
 * is a map or a set is a started one, whose entries the empty map the entry holds takes
 * under its own seed (the layout's `assign_value`), and a `value` of NULL puts the empty
 * value. */
static inline sce_map_put_outcome_t sce_map_put(void *map, const void *key, size_t key_size, const void *value,
                                                size_t value_size) {
    const sce_map_layout_t *layout = sce_map_core_(map)->layout;
    sce_map_put_outcome_t outcome;
    void *entry = sce_map_put_entry(map, key, key_size, &outcome);
    if (entry != NULL && value != NULL && layout->value_size != 0u) {
        if (layout->assign_value != NULL) {
            layout->assign_value(entry, value);
        } else {
            memcpy(entry, value, value_size);
        }
    }
    return outcome;
}

/* Take the entry at `key`, written by `feed` (NULL for the map's own), out of the map, the
 * others keeping their order and their slots; a key that is absent leaves the map as it
 * was. */
static inline sce_map_remove_outcome_t sce_map_remove(void *map, const void *key, sce_map_feed_fn feed) {
    sce_map_core_t *core = (sce_map_core_t *)map;
    const sce_map_probe_t probe = sce_map_find_(map, key, feed);
    sce_map_links_t *links;
    if (!probe.found) {
        return SCE_MAP_REMOVE_ABSENT;
    }
    sce_map_index_(map)[probe.cell] = SCE_MAP_CELL_TOMBSTONE;
    core->tombstones += 1u;
    links = sce_map_links_(map, probe.at);
    if (links->prev == SCE_MAP_NO_ENTRY) {
        core->head = links->next;
    } else {
        sce_map_links_(map, links->prev)->next = links->next;
    }
    if (links->next == SCE_MAP_NO_ENTRY) {
        core->tail = links->prev;
    } else {
        sce_map_links_(map, links->next)->prev = links->prev;
    }
    links->used = false;
    links->prev = SCE_MAP_NO_ENTRY;
    links->next = core->free;
    core->free = probe.at;
    core->live -= 1u;
    if (core->tombstones > core->live) {
        sce_map_rebuild_(map);
    }
    return SCE_MAP_REMOVE_REMOVED;
}

/* `to` takes the entries of `from` whole, in its order, each key and value as `from` holds
 * it, under the seed `to` was started with (docs/adr/0108, D2): what an `<assign>` of one
 * map variable to another writes. A value that is a map is taken the same way, entry by
 * entry, into the empty map `to` holds for it under the seed drawn from `to`'s. The build
 * held `from` to a type `to`'s fits (a bound no larger, each text of a bound no larger), so
 * no entry is refused. */
static inline void sce_map_assign(void *to, const void *from) {
    const sce_map_layout_t *held = sce_map_core_(from)->layout;
    if (to == from) {
        return;
    }
    sce_map_clear(to);
    for (uint32_t at = sce_map_first(from); at != SCE_MAP_NO_ENTRY; at = sce_map_next(from, at)) {
        (void)sce_map_put(to, sce_map_key_at(from, at), held->key_size, sce_map_value_at(from, at), held->value_size);
    }
}

/* `to`, storage of the type of `from`, takes the entries of `from` in its order, for a walk
 * and nothing else: the walk a `<foreach>` makes over the map as it was when it began, whose
 * body writes the map (docs/adr/0108, D6 item 3; §scxml-4.6). Its index is not copied, so
 * `to` is read through its order alone. */
static inline void sce_map_copy_for_walk(void *to, const void *from) {
    const sce_map_core_t *core = sce_map_core_(from);
    memcpy(to, from, sizeof(sce_map_core_t));
    if (core->taken != 0u) {
        memcpy((unsigned char *)to + core->layout->slots_offset,
               (const unsigned char *)from + core->layout->slots_offset, (size_t)core->taken * core->layout->slot_size);
    }
}

/* The type `prefix##_t` of a map of at most `B` entries, each a key of `K` and a value of `V`,
 * the key's canonical bytes written by `FEED` (a `sce_map_feed_fn`): its slot
 * `prefix##_slot_t`, the view a host reads it through `prefix##_view_t`, and the functions
 * that take its key and value by type. `B` is at least 1, and twice it an array length of
 * 32-bit cells. The value a read at an absent key gives is the zero value the map holds
 * beside its slots. A map whose value is a map or a set is declared with
 * `SCE_MAP_NESTED_DECLARE`, whose empty value is started rather than zero. */
#define SCE_MAP_DECLARE(prefix, K, V, B, FEED) SCE_MAP_DECLARE_STARTED_(prefix, K, V, B, FEED, NULL, NULL)

/* The type `prefix##_t` of a map of at most `B` entries, each a key of `K` and a value of
 * the map or set type `INNER##_t` that `SCE_MAP_DECLARE`, `SCE_MAP_NESTED_DECLARE` or
 * `SCE_SET_DECLARE` declared under the prefix `INNER` (docs/adr/0108, D12), declared as
 * `SCE_MAP_DECLARE` declares a map. Each value it holds is a started map: the value of a
 * key put by `prefix##_put_entry` is started empty under the seed drawn from the outer
 * map's (`sce_map_seed_of_inner`) and written where it stands through
 * `prefix##_value_mut`, and the value a read at an absent key gives is a started empty map,
 * itself holding a started empty value at every depth below it. */
#define SCE_MAP_NESTED_DECLARE(prefix, K, INNER, B, FEED)                                                              \
    SCE_MAP_DECLARE_STARTED_(prefix, K, INNER##_t, B, FEED, INNER##_start_value, sce_map_assign)

/* The map type of `SCE_MAP_DECLARE`, its value started by `START` (a `sce_map_start_fn`) and
 * put by `ASSIGN` (a `sce_map_assign_fn`), both NULL for a value whose empty value is all
 * zero and which a put copies. */
#define SCE_MAP_DECLARE_STARTED_(prefix, K, V, B, FEED, START, ASSIGN)                                                 \
    SCE_STATIC_ASSERT((B) >= 1u && (B) <= 0x7fffffffu, "a map's bound is at least 1 and twice it 32 bits");            \
    typedef struct {                                                                                                   \
        sce_map_links_t links;                                                                                         \
        K key;                                                                                                         \
        V value;                                                                                                       \
    } prefix##_slot_t;                                                                                                 \
    typedef struct {                                                                                                   \
        sce_map_core_t core;                                                                                           \
        uint32_t index[2u * (B)];                                                                                      \
        prefix##_slot_t slots[B];                                                                                      \
        V absent;                                                                                                      \
    } prefix##_t;                                                                                                      \
    /* What a host reads: the slots, the position of the first entry (0 for none), and how                             \
     * many entries there are. The entry at position `p` is `entries[p - 1]`, and its                                  \
     * `links.next` is the position of the entry after it in the map's order, 0 after the last. */                     \
    typedef struct {                                                                                                   \
        const prefix##_slot_t *entries;                                                                                \
        uint32_t first;                                                                                                \
        size_t len;                                                                                                    \
    } prefix##_view_t;                                                                                                 \
    static inline const sce_map_layout_t *prefix##_layout(void) {                                                      \
        static const sce_map_layout_t layout = {(uint32_t)(B),                                                         \
                                                sizeof(prefix##_slot_t),                                               \
                                                offsetof(prefix##_slot_t, key),                                        \
                                                sizeof(K),                                                             \
                                                offsetof(prefix##_slot_t, value),                                      \
                                                sizeof(V),                                                             \
                                                offsetof(prefix##_t, index),                                           \
                                                offsetof(prefix##_t, slots),                                           \
                                                offsetof(prefix##_t, absent),                                          \
                                                FEED,                                                                  \
                                                START,                                                                 \
                                                ASSIGN};                                                               \
        return &layout;                                                                                                \
    }                                                                                                                  \
    static inline void prefix##_start(prefix##_t *map, sce_map_seed_t seed) {                                          \
        sce_map_start(map, prefix##_layout(), seed);                                                                   \
    }                                                                                                                  \
    /* The start of the zeroed storage at `value` as an empty map of this type: what starts a                          \
     * map's value of this type in place. */                                                                           \
    static inline void prefix##_start_value(void *value, sce_map_seed_t seed) {                                        \
        sce_map_start(value, prefix##_layout(), seed);                                                                 \
    }                                                                                                                  \
    static inline sce_map_put_outcome_t prefix##_put(prefix##_t *map, K key, V value) {                                \
        return sce_map_put(map, &key, sizeof(K), &value, sizeof(V));                                                   \
    }                                                                                                                  \
    /* `sce_map_put_entry` of a key of `K`: the empty value of the key, to be written in place,                        \
     * or NULL when the map is full. */                                                                                \
    static inline V *prefix##_put_entry(prefix##_t *map, K key, sce_map_put_outcome_t *outcome) {                      \
        return (V *)sce_map_put_entry(map, &key, sizeof(K), outcome);                                                  \
    }                                                                                                                  \
    /* `sce_map_value_mut`: the value at `key`, written by `feed` (NULL for the map's own), or NULL. */                \
    static inline V *prefix##_value_mut(prefix##_t *map, const void *key, sce_map_feed_fn feed) {                      \
        return (V *)sce_map_value_mut(map, key, feed);                                                                 \
    }                                                                                                                  \
    static inline sce_map_remove_outcome_t prefix##_remove(prefix##_t *map, K key) {                                   \
        return sce_map_remove(map, &key, NULL);                                                                        \
    }                                                                                                                  \
    static inline prefix##_view_t prefix##_view(const prefix##_t *map) {                                               \
        prefix##_view_t view;                                                                                          \
        view.entries = map->slots;                                                                                     \
        view.first = map->core.head;                                                                                   \
        view.len = map->core.live;                                                                                     \
        return view;                                                                                                   \
    }

/* The type `prefix##_t` of a set of at most `B` keys of `K`, the map whose entries carry no
 * value, declared as `SCE_MAP_DECLARE` declares a map. */
#define SCE_SET_DECLARE(prefix, K, B, FEED)                                                                            \
    SCE_STATIC_ASSERT((B) >= 1u && (B) <= 0x7fffffffu, "a set's bound is at least 1 and twice it 32 bits");            \
    typedef struct {                                                                                                   \
        sce_map_links_t links;                                                                                         \
        K key;                                                                                                         \
    } prefix##_slot_t;                                                                                                 \
    typedef struct {                                                                                                   \
        sce_map_core_t core;                                                                                           \
        uint32_t index[2u * (B)];                                                                                      \
        prefix##_slot_t slots[B];                                                                                      \
    } prefix##_t;                                                                                                      \
    /* What a host reads, as a map's view is read. */                                                                  \
    typedef struct {                                                                                                   \
        const prefix##_slot_t *entries;                                                                                \
        uint32_t first;                                                                                                \
        size_t len;                                                                                                    \
    } prefix##_view_t;                                                                                                 \
    static inline const sce_map_layout_t *prefix##_layout(void) {                                                      \
        static const sce_map_layout_t layout = {(uint32_t)(B),                                                         \
                                                sizeof(prefix##_slot_t),                                               \
                                                offsetof(prefix##_slot_t, key),                                        \
                                                sizeof(K),                                                             \
                                                0u,                                                                    \
                                                0u,                                                                    \
                                                offsetof(prefix##_t, index),                                           \
                                                offsetof(prefix##_t, slots),                                           \
                                                0u,                                                                    \
                                                FEED,                                                                  \
                                                NULL,                                                                  \
                                                NULL};                                                                 \
        return &layout;                                                                                                \
    }                                                                                                                  \
    static inline void prefix##_start(prefix##_t *map, sce_map_seed_t seed) {                                          \
        sce_map_start(map, prefix##_layout(), seed);                                                                   \
    }                                                                                                                  \
    /* The start of the zeroed storage at `value` as an empty set of this type: what starts a                          \
     * map's value of this type in place. */                                                                           \
    static inline void prefix##_start_value(void *value, sce_map_seed_t seed) {                                        \
        sce_map_start(value, prefix##_layout(), seed);                                                                 \
    }                                                                                                                  \
    static inline sce_map_put_outcome_t prefix##_put(prefix##_t *map, K key) {                                         \
        return sce_map_put(map, &key, sizeof(K), NULL, 0u);                                                            \
    }                                                                                                                  \
    static inline sce_map_remove_outcome_t prefix##_remove(prefix##_t *map, K key) {                                   \
        return sce_map_remove(map, &key, NULL);                                                                        \
    }                                                                                                                  \
    static inline prefix##_view_t prefix##_view(const prefix##_t *map) {                                               \
        prefix##_view_t view;                                                                                          \
        view.entries = map->slots;                                                                                     \
        view.first = map->core.head;                                                                                   \
        view.len = map->core.live;                                                                                     \
        return view;                                                                                                   \
    }

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_MAP_H */
