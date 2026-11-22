// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A map and a set (docs/adr/0108, D1 to D8) as a `sce-static` machine generated as C holds
// them (`sce/forge/map.h`): entry slots with a free list, a doubly linked order through
// them, and an open-addressing index of twice the bound, hashed by SipHash-1-3 under a seed
// each map is started with. The checks are the Rust runtime's
// (`backends/rust/portable-bytes/src/map.rs`), held of the C one: SipHash meets the
// reference vectors, keys are equal by their canonical bytes, a record key is its fields in
// order, a map keeps the order its keys were first put and answers `full` unchanged, a
// remove moves no entry, the order survives a long sequence of puts, removes and clears
// against a list of the entries in order, keys chosen to collide under one seed spread
// under another, and a rebuild of the tombstones keeps every lookup right.
//
// Each check names what it holds; the line that breaks it is the one in the header that
// spells what the check names.

#include <sce/forge/map.h>

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

// The buffers a machine declares for a text of one bound, as the generator writes them.
typedef struct {
    char data[4 + 1];
} sce_static_string_4_t;

typedef struct {
    char data[8 + 1];
} sce_static_string_8_t;

static sce_static_string_4_t word4(const char *text) {
    sce_static_string_4_t value;
    memset(&value, 0, sizeof(value));
    (void)strncpy(value.data, text, 4u);
    return value;
}

static sce_static_string_8_t word8(const char *text) {
    sce_static_string_8_t value;
    memset(&value, 0, sizeof(value));
    (void)strncpy(value.data, text, 8u);
    return value;
}

// A record of two keys, written field by field, as a generated record key is.
typedef struct {
    uint64_t low;
    uint64_t high;
} pair_t;

static void pair_map_key(const void *key, sce_map_sink_t *sink) {
    const pair_t *pair = (const pair_t *)key;
    sce_map_feed_uint(sink, pair->low);
    sce_map_feed_uint(sink, pair->high);
}

SCE_MAP_DECLARE(small_map, uint8_t, uint8_t, 3u, sce_map_key_uint8)
SCE_MAP_DECLARE(slots_map, uint16_t, uint16_t, 8u, sce_map_key_uint16)
SCE_MAP_DECLARE(order_map, uint16_t, uint32_t, 12u, sce_map_key_uint16)
SCE_MAP_DECLARE(spread_map, uint32_t, uint32_t, 16u, sce_map_key_uint32)
SCE_MAP_DECLARE(churn_map, uint32_t, uint32_t, 6u, sce_map_key_uint32)
SCE_MAP_DECLARE(pair_map, pair_t, uint32_t, 4u, pair_map_key)
SCE_MAP_DECLARE(amount_map, uint32_t, int32_t, 3u, sce_map_key_uint32)
SCE_MAP_DECLARE(narrow_map, sce_static_string_4_t, uint8_t, 2u, sce_map_key_text)
SCE_MAP_DECLARE(wide_map, sce_static_string_8_t, uint8_t, 4u, sce_map_key_text)
SCE_SET_DECLARE(tag_set, sce_static_string_8_t, 3u, sce_map_key_text)
// docs/adr/0108, D12: a map of maps two deep (a region's table keyed again by a face), and a
// map of sets.
SCE_MAP_DECLARE(face_map, uint32_t, uint32_t, 3u, sce_map_key_uint32)
SCE_MAP_NESTED_DECLARE(region_map, uint16_t, face_map, 2u, sce_map_key_uint16)
SCE_MAP_NESTED_DECLARE(world_map, uint8_t, region_map, 2u, sce_map_key_uint8)
SCE_SET_DECLARE(peer_set, uint32_t, 2u, sce_map_key_uint32)
SCE_MAP_NESTED_DECLARE(peers_map, uint16_t, peer_set, 2u, sce_map_key_uint16)

static int failures = 0;

static void expect(bool holds, const char *what) {
    if (!holds) {
        (void)fprintf(stderr, "FAIL: %s\n", what);
        failures++;
    }
}

static const sce_map_seed_t SEED = {UINT64_C(0x0123456789abcdef), UINT64_C(0xfedcba9876543210)};
static const sce_map_seed_t OTHER_SEED = {UINT64_C(0x0f1e2d3c4b5a6978), UINT64_C(0x8796a5b4c3d2e1f0)};

static uint64_t sip24(sce_map_seed_t key, const uint8_t *message, size_t len) {
    sce_siphash_t sip;
    sce_siphash_begin(&sip, key, 2u, 4u);
    sce_siphash_write(&sip, message, len);
    return sce_siphash_finish(&sip);
}

// The one implementation of the rounds, held to the SipHash-2-4 vectors of the reference: the
// paper's own (Appendix A, a message of the bytes 0 to 14) and the empty message of its
// reference implementation's table, both under the key of the bytes 0 to 15. SipHash-1-3 is
// the same code with other round counts.
static void siphash_meets_the_reference_vectors(void) {
    uint8_t key_bytes[16];
    uint8_t message[15];
    for (unsigned i = 0u; i < 16u; ++i) {
        key_bytes[i] = (uint8_t)i;
    }
    for (unsigned i = 0u; i < 15u; ++i) {
        message[i] = (uint8_t)i;
    }
    const sce_map_seed_t key = sce_map_seed_of_bytes(key_bytes);
    expect(sip24(key, message, 0u) == UINT64_C(0x726fdb47dd0e0e31), "SipHash-2-4 of the empty message");
    expect(sip24(key, message, sizeof(message)) == UINT64_C(0xa129ca6149be45e5), "SipHash-2-4 of the paper's message");
    // A message written in pieces hashes as the same message written whole.
    sce_siphash_t pieces;
    sce_siphash_begin(&pieces, key, 2u, 4u);
    sce_siphash_write(&pieces, message, 3u);
    sce_siphash_write(&pieces, message + 3, 8u);
    sce_siphash_write(&pieces, message + 11, 4u);
    expect(sce_siphash_finish(&pieces) == UINT64_C(0xa129ca6149be45e5), "a message written in pieces");
    // SipHash-1-3 is another function of the same key and message.
    sce_siphash_t sip13;
    sce_siphash_begin(&sip13, key, 1u, 3u);
    sce_siphash_write(&sip13, message, sizeof(message));
    expect(sce_siphash_finish(&sip13) != UINT64_C(0xa129ca6149be45e5), "SipHash-1-3 is not SipHash-2-4");
}

// Two values are one key exactly when they write the same bytes: an integer whatever its
// width, a text whatever holds it, and two texts whose bytes run together alike are not one
// key.
static void keys_are_equal_by_their_canonical_bytes(void) {
    const uint8_t seven8 = 7u;
    const int32_t seven32 = 7;
    const uint64_t seven64 = 7u;
    const uint16_t seven16 = 7u;
    const uint32_t eight = 8u;
    const int32_t minus_one = -1;
    const uint32_t most = UINT32_MAX;
    expect(sce_map_same_key(&seven8, sce_map_key_uint8, &seven32, sce_map_key_int32), "7u8 is 7i32");
    expect(sce_map_same_key(&seven64, sce_map_key_uint64, &seven16, sce_map_key_uint16), "7u64 is 7u16");
    expect(!sce_map_same_key(&seven64, sce_map_key_uint64, &eight, sce_map_key_uint32), "7 is not 8");
    expect(!sce_map_same_key(&minus_one, sce_map_key_int32, &most, sce_map_key_uint32), "-1 is not u32::MAX");
    const sce_static_string_8_t red = word8("red");
    const char *red_text = "red";
    const char *re_text = "re";
    const char *redd_text = "redd";
    expect(sce_map_same_key(&red, sce_map_key_text, &red_text, sce_map_key_text_at), "a buffer and its text");
    expect(!sce_map_same_key(&red, sce_map_key_text, &re_text, sce_map_key_text_at), "red is not re");
    expect(!sce_map_same_key(&red, sce_map_key_text, &redd_text, sce_map_key_text_at), "red is not redd");
    // A byte string is the key of its length and its bytes.
    const uint8_t two[] = {0x00u, 0xffu};
    const uint8_t three[] = {0x00u, 0xffu, 0x00u};
    const sce_forge_bytes_view_t two_view = {two, sizeof(two)};
    const sce_forge_bytes_view_t two_again = {two, sizeof(two)};
    const sce_forge_bytes_view_t three_view = {three, sizeof(three)};
    expect(sce_map_same_key(&two_view, sce_map_key_bytes_view, &two_again, sce_map_key_bytes_view), "two bytes");
    expect(!sce_map_same_key(&two_view, sce_map_key_bytes_view, &three_view, sce_map_key_bytes_view),
           "a trailing zero byte is another key");
    const bool yes = true;
    const bool no = false;
    expect(sce_map_same_key(&yes, sce_map_key_bool, &yes, sce_map_key_bool), "true is true");
    expect(!sce_map_same_key(&yes, sce_map_key_bool, &no, sce_map_key_bool), "true is not false");
    expect(sce_map_hash(SEED, &red, sce_map_key_text) == sce_map_hash(SEED, &red_text, sce_map_key_text_at),
           "a buffer and its text hash alike");
    expect(sce_map_hash(SEED, &seven8, sce_map_key_uint8) == sce_map_hash(SEED, &seven64, sce_map_key_uint64),
           "7u8 and 7u64 hash alike");
    // The probes a read makes hold the canonical bytes of the value they are made of.
    expect(sce_map_same_key(&seven16, sce_map_key_uint16, SCE_MAP_KEY_UINT(7), sce_map_key_uint64),
           "the unsigned probe of 7");
    expect(sce_map_same_key(&minus_one, sce_map_key_int32, SCE_MAP_KEY_INT(-1), sce_map_key_int64),
           "the signed probe of -1");
    expect(sce_map_same_key(&red, sce_map_key_text, SCE_MAP_KEY_TEXT("red"), sce_map_key_text_at),
           "the probe of a text");
    expect(sce_map_same_key(&two_view, sce_map_key_bytes_view, SCE_MAP_KEY_BYTES(two_again), sce_map_key_bytes_view),
           "the probe of a byte string");
}

// A record key is its fields in order: the halves swapped are another key.
static void a_record_key_is_its_fields_in_order(void) {
    static pair_map_t seen;
    pair_map_start(&seen, SEED);
    const pair_t one = {1u, 0u};
    const pair_t swapped = {0u, 1u};
    const pair_t eleven_one = {11u, 1u};
    const pair_t one_eleven = {1u, 11u};
    expect(pair_map_put(&seen, one, 1u) == SCE_MAP_PUT_INSERTED, "the first pair is inserted");
    expect(pair_map_put(&seen, swapped, 1u) == SCE_MAP_PUT_INSERTED, "the swapped pair is another key");
    expect(pair_map_put(&seen, eleven_one, 1u) == SCE_MAP_PUT_INSERTED, "11 and 1 are inserted");
    expect(pair_map_put(&seen, one_eleven, 1u) == SCE_MAP_PUT_INSERTED, "1 and 11 are another key");
    expect(pair_map_put(&seen, one, 2u) == SCE_MAP_PUT_REPLACED, "the first pair again is replaced");
    const sce_map_found_t found = sce_map_lookup(&seen, &one, NULL);
    expect(found.found && *(const uint32_t *)found.value == 2u, "the first pair holds its new value");
    const sce_map_found_t other = sce_map_lookup(&seen, &swapped, NULL);
    expect(other.found && *(const uint32_t *)other.value == 1u, "the swapped pair holds its own");
    expect(sce_map_count(&seen) == 4u, "four pairs are held");
}

// The entries of a `small_map_t` in order, written into `keys` and `values`: how many.
static size_t small_entries(const small_map_t *map, uint8_t *keys, uint8_t *values) {
    size_t n = 0u;
    for (uint32_t at = sce_map_first(map); at != SCE_MAP_NO_ENTRY; at = sce_map_next(map, at)) {
        keys[n] = *(const uint8_t *)sce_map_key_at(map, at);
        values[n] = *(const uint8_t *)sce_map_value_at(map, at);
        ++n;
    }
    return n;
}

// docs/adr/0108, D2 and D4: a put of a key that is present replaces its value where it
// stands, a removed key put again goes last, a full map answers `full` and is as it was, and
// a key it holds is replaced however full it is.
static void a_map_keeps_the_order_its_keys_were_first_put(void) {
    static small_map_t map;
    static small_map_t before;
    uint8_t keys[3];
    uint8_t values[3];
    small_map_start(&map, SEED);
    expect(small_map_put(&map, 1u, 10u) == SCE_MAP_PUT_INSERTED, "1 is inserted");
    expect(small_map_put(&map, 2u, 20u) == SCE_MAP_PUT_INSERTED, "2 is inserted");
    expect(small_map_put(&map, 3u, 30u) == SCE_MAP_PUT_INSERTED, "3 is inserted");
    expect(small_map_put(&map, 1u, 11u) == SCE_MAP_PUT_REPLACED, "1 again is replaced");
    expect(small_entries(&map, keys, values) == 3u && keys[0] == 1u && values[0] == 11u && keys[1] == 2u &&
               values[1] == 20u && keys[2] == 3u && values[2] == 30u,
           "a replaced key keeps its place");
    memcpy(&before, &map, sizeof(map));
    expect(small_map_put(&map, 4u, 40u) == SCE_MAP_PUT_FULL, "a fourth key is full");
    expect(memcmp(&map, &before, sizeof(map)) == 0, "a full map is as it was");
    expect(small_map_put(&map, 2u, 21u) == SCE_MAP_PUT_REPLACED, "a full map replaces a key it holds");
    expect(small_map_remove(&map, 1u) == SCE_MAP_REMOVE_REMOVED, "1 is removed");
    expect(small_map_remove(&map, 1u) == SCE_MAP_REMOVE_ABSENT, "1 again is absent");
    expect(small_map_put(&map, 1u, 12u) == SCE_MAP_PUT_INSERTED, "1 is inserted again");
    expect(small_entries(&map, keys, values) == 3u && keys[0] == 2u && values[0] == 21u && keys[1] == 3u &&
               values[1] == 30u && keys[2] == 1u && values[2] == 12u,
           "a removed key put again goes last");
    sce_map_clear(&map);
    expect(sce_map_count(&map) == 0u && sce_map_first(&map) == SCE_MAP_NO_ENTRY, "a cleared map is empty");
    expect(small_map_put(&map, 5u, 50u) == SCE_MAP_PUT_INSERTED, "a cleared map takes a key");
    expect(small_entries(&map, keys, values) == 1u && keys[0] == 5u && values[0] == 50u, "it holds that key alone");
}

// The position of `key` in `map`, walked in its order, or no entry.
static uint32_t slot_of(const slots_map_t *map, uint16_t key) {
    for (uint32_t at = sce_map_first(map); at != SCE_MAP_NO_ENTRY; at = sce_map_next(map, at)) {
        if (*(const uint16_t *)sce_map_key_at(map, at) == key) {
            return at;
        }
    }
    return SCE_MAP_NO_ENTRY;
}

// docs/adr/0108, D8: a remove moves no entry. Each other entry stays in the slot it was put
// in, and a walk by positions meets them in order.
static void a_remove_moves_no_entry(void) {
    static slots_map_t map;
    uint32_t before[8];
    slots_map_start(&map, SEED);
    for (uint16_t key = 0u; key < 8u; ++key) {
        expect(slots_map_put(&map, key, (uint16_t)(key * 10u)) == SCE_MAP_PUT_INSERTED, "a key below 8 is inserted");
    }
    for (uint16_t key = 0u; key < 8u; ++key) {
        before[key] = slot_of(&map, key);
    }
    expect(slots_map_remove(&map, 3u) == SCE_MAP_REMOVE_REMOVED, "3 is removed");
    expect(slots_map_remove(&map, 6u) == SCE_MAP_REMOVE_REMOVED, "6 is removed");
    for (uint16_t key = 0u; key < 8u; ++key) {
        if (key == 3u || key == 6u) {
            continue;
        }
        const uint32_t at = slot_of(&map, key);
        expect(at == before[key], "an entry another remove left stays in its slot");
        expect(at != SCE_MAP_NO_ENTRY && *(const uint16_t *)sce_map_value_at(&map, at) == key * 10u,
               "an entry another remove left keeps its value");
    }
    // The slot a remove gave back is the one the next put takes.
    expect(slots_map_put(&map, 100u, 1u) == SCE_MAP_PUT_INSERTED, "100 is inserted");
    expect(slot_of(&map, 100u) == before[6], "the slot the last remove gave back is taken");
}

// A deterministic source of operations: xorshift64.
static uint64_t xorshift(uint64_t *state) {
    uint64_t x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    return x;
}

// docs/adr/0108, D2 and D8: the order survives any sequence of puts, removes and clears. Many
// operations, drawn over a few keys so each is put, replaced and removed again and again, run
// on a map and on a list of its entries in order; after each the two hold the same entries in
// the same order, and each key the list holds is found and each it does not is not. Over a
// small bound, so the map is full often and its tombstones are rebuilt away often.
static void order_survives_any_sequence_of_puts_removes_and_clears(void) {
    enum { BOUND = 12, KEYS = 24, STEPS = 20000 };

    static order_map_t map;
    uint16_t model_keys[BOUND];
    uint32_t model_values[BOUND];
    size_t model_len = 0u;
    uint64_t source = UINT64_C(0x9e3779b97f4a7c15);
    bool kept = true;
    order_map_start(&map, SEED);
    for (int step = 0; step < STEPS && kept; ++step) {
        const uint64_t draw = xorshift(&source);
        const uint16_t key = (uint16_t)(draw % KEYS);
        const uint32_t value = (uint32_t)(draw >> 32);
        const uint64_t op = (draw >> 8) % 16u;
        size_t at = 0u;
        while (at < model_len && model_keys[at] != key) {
            ++at;
        }
        if (op <= 8u) {
            sce_map_put_outcome_t expected;
            if (at < model_len) {
                model_values[at] = value;
                expected = SCE_MAP_PUT_REPLACED;
            } else if (model_len == BOUND) {
                expected = SCE_MAP_PUT_FULL;
            } else {
                model_keys[model_len] = key;
                model_values[model_len] = value;
                ++model_len;
                expected = SCE_MAP_PUT_INSERTED;
            }
            kept = order_map_put(&map, key, value) == expected;
        } else if (op <= 14u) {
            sce_map_remove_outcome_t expected = SCE_MAP_REMOVE_ABSENT;
            if (at < model_len) {
                for (size_t i = at + 1u; i < model_len; ++i) {
                    model_keys[i - 1u] = model_keys[i];
                    model_values[i - 1u] = model_values[i];
                }
                --model_len;
                expected = SCE_MAP_REMOVE_REMOVED;
            }
            kept = order_map_remove(&map, key) == expected;
        } else {
            model_len = 0u;
            sce_map_clear(&map);
        }
        kept = kept && sce_map_count(&map) == model_len;
        size_t walked = 0u;
        for (uint32_t pos = sce_map_first(&map); kept && pos != SCE_MAP_NO_ENTRY; pos = sce_map_next(&map, pos)) {
            kept = walked < model_len && *(const uint16_t *)sce_map_key_at(&map, pos) == model_keys[walked] &&
                   *(const uint32_t *)sce_map_value_at(&map, pos) == model_values[walked];
            ++walked;
        }
        kept = kept && walked == model_len;
        for (uint16_t probe = 0u; kept && probe < KEYS; ++probe) {
            size_t held = 0u;
            while (held < model_len && model_keys[held] != probe) {
                ++held;
            }
            const sce_map_found_t found = sce_map_lookup(&map, &probe, NULL);
            kept = found.found == (held < model_len) &&
                   (!found.found || *(const uint32_t *)found.value == model_values[held]);
        }
        kept = kept && map.core.tombstones <= map.core.live;
        if (!kept) {
            (void)fprintf(stderr, "FAIL: the map and its reference part at step %d\n", step);
            failures++;
        }
    }
}

// The cell of the index of `cells` cells the key `key` starts its probe at, under `seed`.
static size_t home_of(sce_map_seed_t seed, uint32_t key, uint32_t cells) {
    return sce_map_home(sce_map_hash(seed, &key, sce_map_key_uint32), cells);
}

// docs/adr/0108, D8: the hash is keyed. Keys chosen so that all of them start their probe at
// one cell under one seed are found all the same, and under another seed they start at many
// cells: a peer that does not know a map's seed cannot choose keys that make every lookup a
// scan.
static void keys_chosen_to_collide_under_one_seed_spread_under_another(void) {
    enum { BOUND = 16, CHOSEN = 8 };

    const uint32_t cells = 2u * BOUND;
    static spread_map_t flooded;
    uint32_t colliding[CHOSEN];
    size_t chosen = 0u;
    for (uint32_t key = 0u; chosen < CHOSEN; ++key) {
        if (home_of(SEED, key, cells) == 0u) {
            colliding[chosen++] = key;
        }
    }
    // Under the seed they were chosen for, one chain, and every key found.
    spread_map_start(&flooded, SEED);
    for (uint32_t i = 0u; i < CHOSEN; ++i) {
        expect(spread_map_put(&flooded, colliding[i], i) == SCE_MAP_PUT_INSERTED, "a colliding key is inserted");
    }
    for (uint32_t i = 0u; i < CHOSEN; ++i) {
        const sce_map_found_t found = sce_map_lookup(&flooded, &colliding[i], NULL);
        expect(found.found && *(const uint32_t *)found.value == i, "a colliding key is found");
    }
    // Under another seed, they spread.
    size_t distinct = 0u;
    for (size_t i = 0u; i < CHOSEN; ++i) {
        bool seen_before = false;
        for (size_t j = 0u; j < i; ++j) {
            seen_before =
                seen_before || home_of(OTHER_SEED, colliding[j], cells) == home_of(OTHER_SEED, colliding[i], cells);
        }
        if (!seen_before) {
            ++distinct;
        }
    }
    expect(distinct >= 3u, "keys that collide under one seed start at three cells or more under another");
}

// docs/adr/0108, D8: a rebuild of the index keeps every lookup right. Filling and emptying the
// map again and again leaves tombstones that a rebuild clears once they outnumber the live
// entries; each key is found where it is held, under the seed, after every such rebuild.
static void a_rebuild_of_the_tombstones_keeps_every_lookup_right(void) {
    enum { BOUND = 6 };

    static churn_map_t map;
    churn_map_start(&map, OTHER_SEED);
    for (uint32_t round = 0u; round < 50u; ++round) {
        for (uint32_t key = 0u; key < BOUND; ++key) {
            expect(churn_map_put(&map, round * 100u + key, key) == SCE_MAP_PUT_INSERTED, "a round's key is inserted");
        }
        for (uint32_t key = 0u; key + 1u < BOUND; ++key) {
            expect(churn_map_remove(&map, round * 100u + key) == SCE_MAP_REMOVE_REMOVED, "a round's key is removed");
            expect(map.core.tombstones <= map.core.live, "the tombstones never outnumber the entries");
            for (uint32_t later = key + 1u; later < BOUND; ++later) {
                const uint32_t probe = round * 100u + later;
                const sce_map_found_t found = sce_map_lookup(&map, &probe, NULL);
                expect(found.found && *(const uint32_t *)found.value == later, "a later key is found");
            }
        }
        expect(churn_map_remove(&map, round * 100u + BOUND - 1u) == SCE_MAP_REMOVE_REMOVED, "the last key is removed");
        expect(sce_map_count(&map) == 0u, "the round leaves the map empty");
    }
}

// A set is the map whose entries carry no value: a key it holds is replaced and stays where
// it was, one past the bound is full, and a text set is probed by a text.
static void a_set_holds_keys_in_the_order_they_were_first_put(void) {
    static tag_set_t tags;
    const char *walked[3];
    size_t n = 0u;
    tag_set_start(&tags, SEED);
    expect(tag_set_put(&tags, word8("red")) == SCE_MAP_PUT_INSERTED, "red is inserted");
    expect(tag_set_put(&tags, word8("blue")) == SCE_MAP_PUT_INSERTED, "blue is inserted");
    expect(tag_set_put(&tags, word8("red")) == SCE_MAP_PUT_REPLACED, "red again is replaced");
    expect(sce_map_holds(&tags, SCE_MAP_KEY_TEXT("red"), sce_map_key_text_at), "the set holds red");
    expect(sce_map_count(&tags) == 2u, "the set holds two");
    expect(tag_set_remove(&tags, word8("red")) == SCE_MAP_REMOVE_REMOVED, "red is removed");
    expect(tag_set_put(&tags, word8("red")) == SCE_MAP_PUT_INSERTED, "red is inserted again");
    expect(tag_set_put(&tags, word8("green")) == SCE_MAP_PUT_INSERTED, "green is inserted");
    expect(tag_set_put(&tags, word8("pink")) == SCE_MAP_PUT_FULL, "pink is full");
    for (uint32_t at = sce_map_first(&tags); at != SCE_MAP_NO_ENTRY && n < 3u; at = sce_map_next(&tags, at)) {
        walked[n++] = ((const sce_static_string_8_t *)sce_map_key_at(&tags, at))->data;
    }
    expect(n == 3u && strcmp(walked[0], "blue") == 0 && strcmp(walked[1], "red") == 0 &&
               strcmp(walked[2], "green") == 0,
           "the set walks blue, red, green");
    // A host's view walks the same order through the entries' links.
    const tag_set_view_t view = tag_set_view(&tags);
    expect(view.len == 3u && view.first != SCE_MAP_NO_ENTRY &&
               strcmp(view.entries[view.first - 1u].key.data, "blue") == 0,
           "the view begins at blue");
}

// The reads an expression lowers to: `len`, `has`, `m[k]` and a read that finds nothing,
// which fails `out-of-range` and gives the zero value.
static void the_reads_of_an_expression_answer_what_the_map_holds(void) {
    static amount_map_t amounts;
    sce_forge_algorithm_failure_t failure = {0};
    amount_map_start(&amounts, SEED);
    expect(sce_map_count(&amounts) == 0u, "an empty map holds none");
    expect(!sce_map_holds(&amounts, SCE_MAP_KEY_UINT(7), sce_map_key_uint64), "an empty map holds no 7");
    const int32_t absent = SCE_MAP_VALUE_OF(
        amounts, sce_map_lookup_take(&failure, sce_map_lookup(&amounts, SCE_MAP_KEY_UINT(7), sce_map_key_uint64)));
    expect(failure.failed && failure.error == SCE_FORGE_ALGORITHM_OUT_OF_RANGE && absent == 0,
           "a read of an absent key fails out-of-range and gives zero");
    expect(amount_map_put(&amounts, 7u, 5) == SCE_MAP_PUT_INSERTED, "7 is inserted");
    expect(sce_map_count(&amounts) == 1u, "the map holds one");
    expect(sce_map_holds(&amounts, SCE_MAP_KEY_UINT(7), sce_map_key_uint64), "the map holds 7");
    expect(sce_map_holds(&amounts, SCE_MAP_KEY_INT(7), sce_map_key_int64), "the map holds a signed 7");
    sce_forge_algorithm_failure_t none = {0};
    const int32_t five = SCE_MAP_VALUE_OF(
        amounts, sce_map_lookup_take(&none, sce_map_lookup(&amounts, SCE_MAP_KEY_UINT(7), sce_map_key_uint64)));
    expect(!none.failed && five == 5, "the value at 7 is 5");
}

// docs/adr/0108, D2: a map taken whole into a place of a larger bound, its texts into larger
// bounds, holds the same entries in the same order under its own seed; one taken into itself
// is as it was.
static void a_map_assigned_into_a_larger_bound_keeps_its_entries_in_order(void) {
    static narrow_map_t small;
    static wide_map_t wide;
    narrow_map_start(&small, SEED);
    wide_map_start(&wide, OTHER_SEED);
    expect(narrow_map_put(&small, word4("b"), 2u) == SCE_MAP_PUT_INSERTED, "b is inserted");
    expect(narrow_map_put(&small, word4("a"), 1u) == SCE_MAP_PUT_INSERTED, "a is inserted");
    expect(wide_map_put(&wide, word8("stale"), 9u) == SCE_MAP_PUT_INSERTED, "the place holds an entry before");
    sce_map_assign(&wide, &small);
    const uint32_t first = sce_map_first(&wide);
    const uint32_t second = sce_map_next(&wide, first);
    expect(sce_map_count(&wide) == 2u && first != SCE_MAP_NO_ENTRY && second != SCE_MAP_NO_ENTRY,
           "the place holds the two entries");
    expect(strcmp(((const sce_static_string_8_t *)sce_map_key_at(&wide, first))->data, "b") == 0 &&
               *(const uint8_t *)sce_map_value_at(&wide, first) == 2u,
           "b comes first");
    expect(strcmp(((const sce_static_string_8_t *)sce_map_key_at(&wide, second))->data, "a") == 0 &&
               *(const uint8_t *)sce_map_value_at(&wide, second) == 1u,
           "a comes second");
    expect(sce_map_holds(&wide, SCE_MAP_KEY_TEXT("a"), sce_map_key_text_at), "the place finds a under its own seed");
    expect(!sce_map_holds(&wide, SCE_MAP_KEY_TEXT("stale"), sce_map_key_text_at), "what the place held is gone");
    sce_map_assign(&small, &small);
    expect(sce_map_count(&small) == 2u, "a map taken into itself is as it was");
}

// A walk of a copy meets the entries of the map as it was, whatever the map is written after.
static void a_copy_for_a_walk_holds_the_map_as_it_was(void) {
    static small_map_t map;
    static small_map_t walked;
    uint8_t keys[3];
    uint8_t values[3];
    small_map_start(&map, SEED);
    expect(small_map_put(&map, 1u, 10u) == SCE_MAP_PUT_INSERTED, "1 is inserted");
    expect(small_map_put(&map, 2u, 20u) == SCE_MAP_PUT_INSERTED, "2 is inserted");
    sce_map_copy_for_walk(&walked, &map);
    expect(small_map_remove(&map, 1u) == SCE_MAP_REMOVE_REMOVED, "1 is removed from the map");
    expect(small_entries(&walked, keys, values) == 2u && keys[0] == 1u && keys[1] == 2u, "the copy walks both entries");
}

// The seeds of one machine's maps are drawn apart from one another and from the seed they are
// drawn from.
static void the_seeds_of_a_machine_s_maps_are_drawn_apart(void) {
    const sce_map_seed_t first = sce_map_seed_derive(SEED, 0u);
    const sce_map_seed_t second = sce_map_seed_derive(SEED, 1u);
    const sce_map_seed_t again = sce_map_seed_derive(SEED, 0u);
    expect(first.k0 != second.k0 || first.k1 != second.k1, "two ordinals draw two seeds");
    expect(first.k0 != SEED.k0 || first.k1 != SEED.k1, "a drawn seed is not the seed it is drawn from");
    expect(first.k0 == again.k0 && first.k1 == again.k1, "one ordinal draws one seed");
    // A child's seed is drawn apart from the parent's own maps' and from another child's.
    const sce_map_seed_t child = sce_map_seed_of_child(SEED, "faces");
    const sce_map_seed_t sibling = sce_map_seed_of_child(SEED, "peers");
    const sce_map_seed_t child_again = sce_map_seed_of_child(SEED, "faces");
    expect(child.k0 != first.k0 || child.k1 != first.k1, "a child's seed is not the parent's first map's");
    expect(child.k0 != sibling.k0 || child.k1 != sibling.k1, "two invokes draw two seeds");
    expect(child.k0 == child_again.k0 && child.k1 == child_again.k1, "one invoke draws one seed");
    expect(child.k0 != SEED.k0 || child.k1 != SEED.k1, "a child's seed is not its parent's");
}

static bool same_seed(sce_map_seed_t a, sce_map_seed_t b) {
    return a.k0 == b.k0 && a.k1 == b.k1;
}

// docs/adr/0108, D12 item 2: a value written through `sce_map_value_mut` changes where it
// stands. The index, the order and the free list are as they were, a key the map does not
// hold answers NULL, and a probe of another width finds the key.
static void a_value_written_in_place_keeps_the_order_and_the_index(void) {
    static small_map_t map;
    static small_map_t before;
    uint8_t keys[3];
    uint8_t values[3];
    small_map_start(&map, SEED);
    expect(small_map_put(&map, 1u, 10u) == SCE_MAP_PUT_INSERTED, "1 is inserted");
    expect(small_map_put(&map, 2u, 20u) == SCE_MAP_PUT_INSERTED, "2 is inserted");
    expect(small_map_put(&map, 3u, 30u) == SCE_MAP_PUT_INSERTED, "3 is inserted");
    expect(small_map_remove(&map, 3u) == SCE_MAP_REMOVE_REMOVED, "3 is removed, its slot on the free list");
    memcpy(&before, &map, sizeof(map));
    const uint8_t two = 2u;
    uint8_t *held = small_map_value_mut(&map, &two, NULL);
    expect(held != NULL && *held == 20u, "the value of 2 is found to be written");
    if (held != NULL) {
        *held = 21u;
    }
    uint8_t *again = small_map_value_mut(&map, SCE_MAP_KEY_UINT(2), sce_map_key_uint64);
    expect(again == held, "a probe of another width finds the same value");
    expect(small_map_value_mut(&map, SCE_MAP_KEY_UINT(3), sce_map_key_uint64) == NULL, "a removed key answers NULL");
    expect(small_map_value_mut(&map, SCE_MAP_KEY_UINT(9), sce_map_key_uint64) == NULL, "an absent key answers NULL");
    expect(memcmp(&map.core, &before.core, sizeof(map.core)) == 0, "the order, the free list and the counts stay");
    expect(memcmp(map.index, before.index, sizeof(map.index)) == 0, "the index stays");
    expect(small_entries(&map, keys, values) == 2u && keys[0] == 1u && values[0] == 10u && keys[1] == 2u &&
               values[1] == 21u,
           "the written value is held where its key stands");
}

// docs/adr/0108, D2 and D12 item 4: `sce_map_put_entry` puts a key with the empty value and
// answers where it is held. An absent key goes last, a present key keeps its place and its
// value is made empty, and a full map answers `full`, NULL and is as it was.
static void an_entry_put_in_place_answers_what_a_put_answers(void) {
    static small_map_t map;
    static small_map_t before;
    uint8_t keys[3];
    uint8_t values[3];
    sce_map_put_outcome_t outcome = SCE_MAP_PUT_FULL;
    small_map_start(&map, SEED);
    expect(small_map_put(&map, 1u, 10u) == SCE_MAP_PUT_INSERTED, "1 is inserted");
    uint8_t *fresh = small_map_put_entry(&map, 2u, &outcome);
    expect(fresh != NULL && outcome == SCE_MAP_PUT_INSERTED && *fresh == 0u, "a new key holds the zero value");
    if (fresh != NULL) {
        *fresh = 20u;
    }
    expect(small_map_put(&map, 3u, 30u) == SCE_MAP_PUT_INSERTED, "3 is inserted");
    outcome = SCE_MAP_PUT_FULL;
    uint8_t *present = small_map_put_entry(&map, 1u, &outcome);
    expect(present != NULL && outcome == SCE_MAP_PUT_REPLACED && *present == 0u,
           "a present key is replaced by the zero value");
    if (present != NULL) {
        *present = 11u;
    }
    expect(small_entries(&map, keys, values) == 3u && keys[0] == 1u && values[0] == 11u && keys[1] == 2u &&
               values[1] == 20u && keys[2] == 3u && values[2] == 30u,
           "a key put in place goes last, and a replaced one keeps its place");
    memcpy(&before, &map, sizeof(map));
    outcome = SCE_MAP_PUT_INSERTED;
    expect(small_map_put_entry(&map, 4u, &outcome) == NULL && outcome == SCE_MAP_PUT_FULL, "a fourth key is full");
    expect(memcmp(&map, &before, sizeof(map)) == 0, "a full map is as it was");
}

// The value of `regions` at `region`, or NULL.
static face_map_t *faces_of(region_map_t *regions, uint16_t region) {
    return region_map_value_mut(regions, &region, NULL);
}

// The value of `faces` at `face`, read as an expression reads it: whether it is held, and
// what it is.
static bool face_holds(const face_map_t *faces, uint32_t face, uint32_t value) {
    const sce_map_found_t found = sce_map_lookup(faces, SCE_MAP_KEY_UINT(face), sce_map_key_uint64);
    return found.found && *(const uint32_t *)found.value == value;
}

// docs/adr/0108, D12: a map whose values are maps. An entry put in place holds a started
// empty map under the seed drawn from the outer map's, a nested put writes it in place and
// answers the inner map's outcome, the outer map's order and index stay, a present key is
// replaced by the empty map, an absent outer key is no inner map, and the value a read at an
// absent key gives is a started empty map, which a read through finds nothing in.
static void a_map_of_maps_writes_its_inner_maps_in_place(void) {
    static region_map_t regions;
    static region_map_t before;
    sce_map_put_outcome_t outcome = SCE_MAP_PUT_FULL;
    region_map_start(&regions, SEED);
    // The value a read at an absent key gives is started.
    const sce_map_found_t missing = sce_map_lookup(&regions, SCE_MAP_KEY_UINT(5), sce_map_key_uint64);
    expect(!missing.found && missing.value == (const void *)&regions.absent, "an absent region gives the absent map");
    expect(regions.absent.core.layout == face_map_layout(), "the absent value is a started face map");
    expect(!sce_map_holds(missing.value, SCE_MAP_KEY_UINT(1), sce_map_key_uint64) && sce_map_count(missing.value) == 0u,
           "has and len through the absent value find nothing");
    expect(same_seed(regions.absent.core.seed, sce_map_seed_of_inner(SEED)), "the absent map is seeded from the outer");
    // An entry put in place.
    face_map_t *faces = region_map_put_entry(&regions, 1u, &outcome);
    expect(faces != NULL && outcome == SCE_MAP_PUT_INSERTED, "region 1 is inserted");
    if (faces == NULL) {
        return;
    }
    expect(faces->core.layout == face_map_layout() && sce_map_count(faces) == 0u, "region 1 holds a started empty map");
    expect(same_seed(faces->core.seed, sce_map_seed_of_inner(SEED)), "region 1's map is seeded from the outer");
    expect(!same_seed(faces->core.seed, SEED), "an inner seed is not the outer seed");
    expect(face_map_put(faces, 10u, 100u) == SCE_MAP_PUT_INSERTED, "face 10 is put into region 1");
    expect(region_map_put_entry(&regions, 2u, &outcome) != NULL && outcome == SCE_MAP_PUT_INSERTED,
           "region 2 is inserted");
    // A nested put through the mutable lookup leaves the outer map as it was.
    memcpy(&before, &regions, sizeof(regions));
    face_map_t *found = faces_of(&regions, 1u);
    expect(found == faces, "the mutable lookup finds region 1's map where it stands");
    expect(face_map_put(found, 11u, 110u) == SCE_MAP_PUT_INSERTED, "face 11 is put in place");
    expect(face_map_put(found, 12u, 120u) == SCE_MAP_PUT_INSERTED, "face 12 is put in place");
    expect(face_map_put(found, 13u, 130u) == SCE_MAP_PUT_FULL, "a fourth face answers the inner map's full");
    expect(memcmp(&regions.core, &before.core, sizeof(regions.core)) == 0 &&
               memcmp(regions.index, before.index, sizeof(regions.index)) == 0,
           "a nested put leaves the outer order and index as they were");
    const sce_map_found_t through = sce_map_lookup(&regions, SCE_MAP_KEY_UINT(1), sce_map_key_uint64);
    expect(through.found && face_holds(through.value, 10u, 100u) && face_holds(through.value, 11u, 110u) &&
               face_holds(through.value, 12u, 120u),
           "a read through the outer map finds each face");
    expect(faces_of(&regions, 3u) == NULL, "an absent region is no inner map to write");
    // A full outer map refuses a third region and is as it was.
    memcpy(&before, &regions, sizeof(regions));
    expect(region_map_put_entry(&regions, 3u, &outcome) == NULL && outcome == SCE_MAP_PUT_FULL, "region 3 is full");
    expect(memcmp(&regions, &before, sizeof(regions)) == 0, "a full map of maps is as it was");
    // A present region put again holds the empty map, in its place.
    face_map_t *emptied = region_map_put_entry(&regions, 1u, &outcome);
    expect(emptied == faces && outcome == SCE_MAP_PUT_REPLACED, "region 1 again is replaced where it stands");
    expect(sce_map_count(faces) == 0u && faces->core.layout == face_map_layout() &&
               !sce_map_holds(faces, SCE_MAP_KEY_UINT(10), sce_map_key_uint64),
           "a replaced region holds a started empty map");
    expect(sce_map_first(&regions) != SCE_MAP_NO_ENTRY &&
               *(const uint16_t *)sce_map_key_at(&regions, sce_map_first(&regions)) == 1u,
           "the replaced region keeps its place first");
}

// docs/adr/0108, D12 items 4 and 7: a put of no value puts the empty map, a map of maps taken
// whole keys each inner map from its own seed, and the absent value two levels down is
// started under a seed drawn once per depth.
static void a_map_of_maps_is_started_and_taken_whole_at_every_depth(void) {
    static region_map_t regions;
    static region_map_t copy;
    static world_map_t world;
    static peers_map_t peers;
    sce_map_put_outcome_t outcome = SCE_MAP_PUT_FULL;
    const uint16_t one = 1u;
    region_map_start(&regions, SEED);
    expect(sce_map_put(&regions, &one, sizeof(one), NULL, 0u) == SCE_MAP_PUT_INSERTED, "a put of no value inserts");
    face_map_t *faces = faces_of(&regions, 1u);
    expect(faces != NULL && faces->core.layout == face_map_layout() && sce_map_count(faces) == 0u,
           "a put of no value holds the empty map");
    if (faces == NULL) {
        return;
    }
    expect(face_map_put(faces, 7u, 70u) == SCE_MAP_PUT_INSERTED, "face 7 is put");
    region_map_start(&copy, OTHER_SEED);
    sce_map_assign(&copy, &regions);
    const face_map_t *copied = faces_of(&copy, 1u);
    expect(copied != NULL && copied != faces && face_holds(copied, 7u, 70u), "the copy holds region 1's faces");
    expect(copied != NULL && same_seed(copied->core.seed, sce_map_seed_of_inner(OTHER_SEED)),
           "the copy's inner map is seeded from the copy");
    // Two levels down.
    world_map_start(&world, SEED);
    const sce_map_seed_t depth1 = sce_map_seed_of_inner(SEED);
    const sce_map_seed_t depth2 = sce_map_seed_of_inner(depth1);
    expect(world.absent.core.layout == region_map_layout() && world.absent.absent.core.layout == face_map_layout(),
           "the absent value is started at every depth");
    expect(same_seed(world.absent.core.seed, depth1) && same_seed(world.absent.absent.core.seed, depth2),
           "each depth's seed is drawn from the one above it");
    expect(!same_seed(depth1, depth2), "two depths are keyed apart");
    const sce_map_found_t region = sce_map_lookup(&world, SCE_MAP_KEY_UINT(4), sce_map_key_uint64);
    const sce_map_found_t face = sce_map_lookup(region.value, SCE_MAP_KEY_UINT(9), sce_map_key_uint64);
    expect(!region.found && !face.found && !sce_map_holds(face.value, SCE_MAP_KEY_UINT(1), sce_map_key_uint64),
           "a read two levels through absent keys finds nothing");
    region_map_t *inner = world_map_put_entry(&world, 1u, &outcome);
    expect(inner != NULL && outcome == SCE_MAP_PUT_INSERTED && inner->absent.core.layout == face_map_layout(),
           "a region put into the world holds its own started absent value");
    // A map of sets.
    peers_map_start(&peers, SEED);
    expect(peers.absent.core.layout == peer_set_layout(), "the absent value of a map of sets is a started set");
    peer_set_t *known = peers_map_put_entry(&peers, 3u, &outcome);
    expect(known != NULL && outcome == SCE_MAP_PUT_INSERTED, "an entry of a map of sets is inserted");
    if (known != NULL) {
        expect(peer_set_put(known, 42u) == SCE_MAP_PUT_INSERTED, "a peer is put into the set in place");
        expect(sce_map_holds(sce_map_lookup(&peers, SCE_MAP_KEY_UINT(3), sce_map_key_uint64).value,
                             SCE_MAP_KEY_UINT(42), sce_map_key_uint64),
               "a read through the map of sets finds the peer");
    }
}

int main(void) {
    siphash_meets_the_reference_vectors();
    keys_are_equal_by_their_canonical_bytes();
    a_record_key_is_its_fields_in_order();
    a_map_keeps_the_order_its_keys_were_first_put();
    a_remove_moves_no_entry();
    order_survives_any_sequence_of_puts_removes_and_clears();
    keys_chosen_to_collide_under_one_seed_spread_under_another();
    a_rebuild_of_the_tombstones_keeps_every_lookup_right();
    a_set_holds_keys_in_the_order_they_were_first_put();
    the_reads_of_an_expression_answer_what_the_map_holds();
    a_map_assigned_into_a_larger_bound_keeps_its_entries_in_order();
    a_copy_for_a_walk_holds_the_map_as_it_was();
    the_seeds_of_a_machine_s_maps_are_drawn_apart();
    a_value_written_in_place_keeps_the_order_and_the_index();
    an_entry_put_in_place_answers_what_a_put_answers();
    a_map_of_maps_writes_its_inner_maps_in_place();
    a_map_of_maps_is_started_and_taken_whole_at_every_depth();
    if (failures != 0) {
        (void)fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
