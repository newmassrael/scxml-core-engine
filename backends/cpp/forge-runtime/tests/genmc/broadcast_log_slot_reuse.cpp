// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/* THREADS: 2 */
/* REQUIRED: acquire release acquire_release all */

// What this harness is expected to catch, and what has not been measured. The four
// mutants listed above weaken every acquire of the runtime to relaxed (`acquire`),
// every release (`release`), the pair (`acquire_release`) and every ordering
// (`all`). Each leaves the reader's relaxed copy of the words unordered against the
// publisher's relaxed writes of them, so a reader can hand over a slot whose stamp
// reads `done` and whose words are the zeros the log started with or the words of a
// later entry; the harness judges every item whole and so reports it. They were
// chosen by reading the protocol and were NOT run on GenMC when this was written.
// The mutants that weaken the two fences alone (`fence_acquire`, `fence_release`)
// are not listed as required for the same reason: the gate prints each one that
// survives, and one that does not survive is promoted to the list by whoever reads
// that output.

// Layer 3 of the broadcast-log kind's verification (RFC §synth-5-S; ADR 0062 section 6): the C++
// bounded log (sce/forge/broadcast_log.h) under GenMC, with a slot that is reused.
// It is the C harness of the same name
// (backends/c/forge-runtime/tests/genmc/broadcast_log_slot_reuse.c) over the C++
// runtime as it ships, and the counterpart of the loom model
// `a_reader_the_publisher_laps_is_told_how_far`
// (backends/rust/forge-runtime/tests/loom_broadcast_log.rs).
//
// A log of two slots retains one entry. The publisher publishes three, so the slot
// of entry 0 is written again for entry 2, and a reader that starts at entry 0 is
// either handed it whole, told it is gone, or finds the log empty. The reader reads
// three times, so it also crosses the lag.
//
// Every answer is held to the specification of ADR 0062 section 2: an item is the
// whole entry at the cursor (every word is its sequence number plus one, so a mix of
// two entries and a slot nothing was written to both show), the cursor moves by
// exactly what the answer says, and a lag skips at least one entry.

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

void *read_three(void *) {
    blog::Reader reader = g_log.reader_at(0);
    for (unsigned i = 0; i < 3; i++) {
        read_and_check(reader);
    }
    return nullptr;
}

}  // namespace

int main() {
    pthread_t publisher;
    pthread_t reader;
    pthread_create(&publisher, nullptr, publish_three, nullptr);
    pthread_create(&reader, nullptr, read_three, nullptr);
    pthread_join(publisher, nullptr);
    pthread_join(reader, nullptr);

    // Everything is published: the window is the last entry, and a reader at the
    // oldest finds exactly it and then the head.
    blog::Reader late = g_log.reader_oldest();
    Log::Entry out{};
    blog::Read result = g_log.read(late, out);
    assert(result == blog::Read::item(kPublishes - 1));
    assert(out[0] == kPublishes && out[1] == kPublishes);
    result = g_log.read(late, out);
    assert(result == blog::Read::empty());
    (void)result;
    return 0;
}
