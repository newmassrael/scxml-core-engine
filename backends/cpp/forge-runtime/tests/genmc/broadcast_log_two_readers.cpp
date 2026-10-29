// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 3 */
/* REQUIRED: acquire release acquire_release all */

// What this harness is expected to catch, and what has not been measured: the same
// four mutants as broadcast_log_slot_reuse.cpp, for the same reason, and with the
// same caveat. They were chosen by reading the protocol and were NOT run on GenMC
// when this was written; the two fence mutants are left unlisted and the gate
// prints each one that survives.

// Layer 3 of the broadcast-log kind's verification (RFC §synth-5-S; ADR 0062 section 6): the C++
// bounded log (sce/forge/broadcast_log.h) under GenMC with two readers. It is the C
// harness of the same name
// (backends/c/forge-runtime/tests/genmc/broadcast_log_two_readers.c) over the C++
// runtime as it ships, and the counterpart of the loom model
// `two_readers_each_hold_their_own_cursor`
// (backends/rust/forge-runtime/tests/loom_broadcast_log.rs), the model that found
// that a relaxed busy stamp is a fault: a reader that meets the stamp of a later lap
// reloads `head` and must find the publish that began that lap, or it reports a lag
// that skipped nothing.
//
// A log of two slots retains one entry; the publisher publishes three, so the slot
// of entry 0 is written again for entry 2. Two readers, each with its own cursor
// from the oldest retained entry, read twice while that happens. Every answer is
// held to the specification of ADR 0062 section 2: an item is the whole entry at the
// cursor (every word is its sequence number plus one), the cursor moves by exactly
// what the answer says, a lag skips at least one entry, and one reader's reads move
// nothing of the other's.

#include <array>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <utility>

#include <pthread.h>

#include <sce/forge/broadcast_log.h>

#include "genmc_cxx_support.h"

namespace {

namespace blog = ::SCE::Forge::Broadcast;

using Log = blog::BroadcastLog<2, 2>;

constexpr std::uint64_t kPublishes = 3;

Log g_log;

void *publish_three(void *) {
    auto publisher = g_log.publisher();
    if (!publisher) {
        return nullptr;
    }
    for (std::uint64_t n = 0; n < kPublishes; n++) {
        const std::uint64_t seq = publisher->publish(Log::Entry{n + 1, n + 1});
        assert(seq == n);
        (void)seq;
    }
    return nullptr;
}

// One read, held to the specification.
void read_and_check(blog::Reader &reader) {
    const std::uint64_t before = reader.cursor();
    const std::uint64_t lost_before = reader.lost();
    Log::Entry out{UINT64_MAX, UINT64_MAX};
    const blog::Read result = g_log.read(reader, out);
    if (result.kind == blog::ReadKind::Item) {
        assert(result.seq == before);
        assert(out[0] == before + 1);
        assert(out[1] == before + 1);
        assert(reader.cursor() == before + 1);
        assert(reader.lost() == lost_before);
    } else if (result.kind == blog::ReadKind::Empty) {
        assert(reader.cursor() == before);
    } else {
        assert(result.skipped >= 1);
        assert(reader.cursor() == before + result.skipped);
        assert(reader.lost() == lost_before + result.skipped);
        assert(reader.cursor() <= kPublishes);
    }
    (void)lost_before;
}

void *read_twice(void *) {
    blog::Reader reader = g_log.reader_oldest();
    for (unsigned i = 0; i < 2; i++) {
        read_and_check(reader);
    }
    return nullptr;
}

}  // namespace

int main() {
    pthread_t publisher;
    pthread_t first;
    pthread_t second;
    pthread_create(&publisher, nullptr, publish_three, nullptr);
    pthread_create(&first, nullptr, read_twice, nullptr);
    pthread_create(&second, nullptr, read_twice, nullptr);
    pthread_join(publisher, nullptr);
    pthread_join(first, nullptr);
    pthread_join(second, nullptr);
    return 0;
}
