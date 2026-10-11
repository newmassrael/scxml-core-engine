// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The three symbols GenMC's interpreter cannot resolve in a C++ harness, defined
// once for every harness (include it after the standard headers, in one
// translation unit). genmc_cxx_shim.h declares the rest.
//
//   __dso_handle and __cxa_atexit: a global object with a destructor is
//     registered with `__cxa_atexit(destructor, object, &__dso_handle)`, and the
//     interpreter reports "Could not resolve external global address:
//     __dso_handle" and aborts (measured 2026-10-11). Nothing runs at exit here,
//     so registering nothing is exactly right.
//
//   memcpy and memset: GenMC promotes a copy it can see the types of to loads and
//     stores, and one it cannot, "both src and dst being opaque", it skips, which
//     makes the bytes copied invisible to it; a `memset` it cannot promote at all
//     ("Invalid call to memset()", measured 2026-10-11 on a value-initialised
//     `new (memory) Node()`). The queue moves an element into a slot with a
//     constructor, but libstdc++ and the compiler's own aggregate copies and
//     zero-initialisations reach these two, so they are written out by words,
//     which GenMC does see (the C harnesses do the same for `memcpy`, and say
//     why, in linked_segment_handover.c). How, and why not a byte loop, is below.

// THE C++ LSCQ is covered by lscq_segment_handover.cpp, and it needed two things the
// other harnesses do not. A build-time spin count (`SCE_QUEUE_SCQ_SPINS`, defined
// there, default 10000 in queue.h) for the reason that harness gives. And a
// strengthened copy of the headers (`/* CXX_STRONG_CAS */`): the first attempt ran
// against the shipped runtime and GenMC stopped with an INTERNAL FAILURE
// (`Internal check failed: it != result.end()` in `calculateReplaySchedule`) after
// two executions it had to KILL at the unroll bound, in the single-threaded start
// of the scenario, in a loop of three events that repeats (an acquire load, an
// acquire load and a compare-and-swap read). That loop is the SCQ's `catch_up`,
// which retries a `compare_exchange_weak`; GenMC makes each spurious failure an
// execution of its own, so the number of failures a retry can have is bounded only
// by the unroll. The failure was GenMC's reaction to the KILLs and not a bug of
// the runtime or a limit of the tool; rewriting the weak compare-and-swap to a
// strong one in a copy explored the whole scenario (927 executions, no KILL).

#ifndef SCE_FORGE_GENMC_CXX_SUPPORT_H
#define SCE_FORGE_GENMC_CXX_SUPPORT_H

#include <cstddef>
#include <cstdint>

// By blocks of eight words, then words, then bytes, when both ends are aligned for
// it. GenMC cuts a loop at its unroll bound (20 in the gate) and kills the
// execution, so a loop that runs more often than that kills the main thread before
// it creates a thread, and the run that reports no error describes a program with
// no concurrency in it: the gate's thread-create check caught exactly that, twice
// (measured 2026-10-11). First a byte loop over the 32 bytes of
// `new (memory) Node()`; then a word loop, which fits 160 bytes in the bound, over
// a segment of the LSCQ, whose cache-line-aligned members make it several hundred
// bytes and whose value-initialisation reaches `memset`. A loop iteration here is
// eight straight-line word stores, 64 bytes, so each loop fits 1,280 bytes in the
// bound; the word loop after it takes at most seven iterations and the byte loop
// at most seven. An object that needs more is a KILL the same check reports, and
// the remedy is a bigger block, not a bigger bound.
namespace genmc_cxx_support {

inline bool word_aligned(const void *a, const void *b) noexcept {
    return ((reinterpret_cast<std::uintptr_t>(a) | reinterpret_cast<std::uintptr_t>(b)) % sizeof(std::uint64_t)) == 0;
}

inline constexpr std::size_t kBlockWords = 8;
inline constexpr std::size_t kBlockBytes = kBlockWords * sizeof(std::uint64_t);

}  // namespace genmc_cxx_support

extern "C" {

void *__dso_handle = nullptr;

int __cxa_atexit(void (*)(void *), void *, void *) {
    return 0;
}

void *memcpy(void *destination, const void *source, std::size_t bytes) noexcept {
    std::size_t done = 0;
    if (genmc_cxx_support::word_aligned(destination, source)) {
        auto *to = static_cast<std::uint64_t *>(destination);
        const auto *from = static_cast<const std::uint64_t *>(source);
        std::size_t word = 0;
        for (; (word + 1) * genmc_cxx_support::kBlockWords <= bytes / sizeof(std::uint64_t); word++) {
            std::uint64_t *block_to = to + word * genmc_cxx_support::kBlockWords;
            const std::uint64_t *block_from = from + word * genmc_cxx_support::kBlockWords;
            block_to[0] = block_from[0];
            block_to[1] = block_from[1];
            block_to[2] = block_from[2];
            block_to[3] = block_from[3];
            block_to[4] = block_from[4];
            block_to[5] = block_from[5];
            block_to[6] = block_from[6];
            block_to[7] = block_from[7];
        }
        std::size_t i = word * genmc_cxx_support::kBlockWords;
        for (; i < bytes / sizeof(std::uint64_t); i++) {
            to[i] = from[i];
        }
        done = i * sizeof(std::uint64_t);
    }
    auto *to = static_cast<unsigned char *>(destination);
    const auto *from = static_cast<const unsigned char *>(source);
    for (std::size_t i = done; i < bytes; i++) {
        to[i] = from[i];
    }
    return destination;
}

void *memset(void *destination, int value, std::size_t bytes) noexcept {
    std::size_t done = 0;
    if (genmc_cxx_support::word_aligned(destination, destination)) {
        // Every byte of a word is the value, so the word is the byte repeated.
        std::uint64_t repeated = static_cast<unsigned char>(value);
        repeated |= repeated << 8;
        repeated |= repeated << 16;
        repeated |= repeated << 32;
        auto *to = static_cast<std::uint64_t *>(destination);
        std::size_t word = 0;
        for (; (word + 1) * genmc_cxx_support::kBlockWords <= bytes / sizeof(std::uint64_t); word++) {
            std::uint64_t *block = to + word * genmc_cxx_support::kBlockWords;
            block[0] = repeated;
            block[1] = repeated;
            block[2] = repeated;
            block[3] = repeated;
            block[4] = repeated;
            block[5] = repeated;
            block[6] = repeated;
            block[7] = repeated;
        }
        std::size_t i = word * genmc_cxx_support::kBlockWords;
        for (; i < bytes / sizeof(std::uint64_t); i++) {
            to[i] = repeated;
        }
        done = i * sizeof(std::uint64_t);
    }
    auto *to = static_cast<unsigned char *>(destination);
    for (std::size_t i = done; i < bytes; i++) {
        to[i] = static_cast<unsigned char>(value);
    }
    return destination;
}

}  // extern "C"

#endif  // SCE_FORGE_GENMC_CXX_SUPPORT_H
