// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/Uuid.h"

#include <chrono>
#include <random>

namespace SCE::uuid {

namespace {

// Thread-local PRNG: avoids cross-thread contention on the mesh hot path.
// Seeded once per thread from 8 random_device words (256 bits) via seed_seq
// to cover mt19937_64's full state space and prevent cross-thread sequence
// collisions that a single 32-bit seed would allow at ~2^16 threads.
std::mt19937_64& tls_rng() {
    thread_local std::mt19937_64 rng = [] {
        std::random_device rd;
        std::seed_seq seq{rd(), rd(), rd(), rd(), rd(), rd(), rd(), rd()};
        return std::mt19937_64{seq};
    }();
    return rng;
}

uint64_t now_unix_ms() {
    using namespace std::chrono;
    return static_cast<uint64_t>(
        duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count());
}

// Write 16 random bytes from rng into out.
void fill_random(Bytes& out, std::mt19937_64& rng) {
    uint64_t a = rng();
    uint64_t b = rng();
    for (int i = 0; i < 8; ++i) out[i]     = static_cast<uint8_t>(a >> (56 - i * 8));
    for (int i = 0; i < 8; ++i) out[8 + i] = static_cast<uint8_t>(b >> (56 - i * 8));
}

}  // namespace

Bytes v7() {
    // RFC 9562 §5.7 layout (big-endian):
    //   bytes 0-5  : unix_ts_ms (48 bits)
    //   byte 6 hi  : version = 0b0111
    //   byte 6 lo  : rand_a high nibble (intra-ms monotonic counter)
    //   byte 7     : rand_a low byte    (intra-ms monotonic counter)
    //   byte 8 hi  : variant = 0b10 (top 2 bits)
    //   bytes 8-15 : rand_b (62 bits, fully random)
    //
    // RFC 9562 §6.2 Method 1: rand_a (12 bits) carries a per-thread monotonic
    // counter so IDs generated in the same millisecond are strictly ordered.
    // On a new millisecond, the counter is seeded from PRNG to avoid leaking
    // call-rate information. On counter overflow, ms is advanced by one
    // (synthetic future timestamp) and counter reset.
    struct V7State {
        uint64_t last_ms = 0;
        uint16_t counter = 0;  // 12-bit monotonic counter (top 4 bits unused)
    };
    thread_local V7State state;

    auto& rng = tls_rng();

    uint64_t ts = now_unix_ms();
    // Wall-clock skew protection: never go backwards.
    if (ts < state.last_ms) ts = state.last_ms;

    if (ts == state.last_ms) {
        ++state.counter;
        if (state.counter > 0x0FFF) {
            // Counter overflow within a single ms (>4096 IDs/ms on this
            // thread). Borrow from the next ms; counter restarts random.
            ++ts;
            state.last_ms = ts;
            state.counter = static_cast<uint16_t>(rng() & 0x0FFF);
        }
    } else {
        state.last_ms = ts;
        state.counter = static_cast<uint16_t>(rng() & 0x0FFF);
    }
    const uint16_t rand_a = state.counter;

    Bytes out{};
    fill_random(out, rng);  // populates rand_b in bytes 8-15 (and overwritten 0-7 below)

    ts &= 0x0000'FFFF'FFFF'FFFFULL;
    out[0] = static_cast<uint8_t>(ts >> 40);
    out[1] = static_cast<uint8_t>(ts >> 32);
    out[2] = static_cast<uint8_t>(ts >> 24);
    out[3] = static_cast<uint8_t>(ts >> 16);
    out[4] = static_cast<uint8_t>(ts >> 8);
    out[5] = static_cast<uint8_t>(ts);

    // Version 7 (high nibble of byte 6) | rand_a high nibble (low nibble of byte 6).
    out[6] = static_cast<uint8_t>(0x70 | ((rand_a >> 8) & 0x0F));
    // rand_a low byte.
    out[7] = static_cast<uint8_t>(rand_a & 0xFF);
    // Variant 0b10 in top two bits of byte 8 (low 6 bits = random rand_b).
    out[8] = static_cast<uint8_t>(0x80 | (out[8] & 0x3F));
    return out;
}

Bytes v4() {
    // RFC 9562 §5.4 layout (big-endian):
    //   bytes 0-15 random, with version/variant overwrites at byte 6 / byte 8.
    Bytes out{};
    fill_random(out, tls_rng());
    out[6] = static_cast<uint8_t>(0x40 | (out[6] & 0x0F));  // version 4
    out[8] = static_cast<uint8_t>(0x80 | (out[8] & 0x3F));  // variant 0b10
    return out;
}

}  // namespace SCE::uuid
