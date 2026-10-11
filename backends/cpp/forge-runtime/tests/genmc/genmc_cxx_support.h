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
//     zero-initialisations reach these two, so they are written out byte by byte,
//     which GenMC does see (the C harnesses do the same for `memcpy`, and say
//     why, in linked_segment_handover.c).

#ifndef SCE_FORGE_GENMC_CXX_SUPPORT_H
#define SCE_FORGE_GENMC_CXX_SUPPORT_H

#include <cstddef>
#include <cstdint>

// By words, and not byte by byte, when both ends are aligned for it. GenMC cuts a
// loop at its unroll bound (20 in the gate) and kills the execution, so a byte
// loop over a 32-byte object is killed in its main thread before it creates a
// thread, and the run that reports no error describes a program with no
// concurrency in it: the gate's thread-create check caught exactly that
// (measured 2026-10-11, `new (memory) Node()` zero-initialising 32 bytes). A word
// loop covers 160 bytes inside the bound, the tail of at most 7 bytes covers the
// rest. A copy that is not aligned falls back to bytes, and a size that needs
// more than the bound is a KILL the same check reports.
namespace genmc_cxx_support {

inline bool word_aligned(const void *a, const void *b) noexcept {
    return ((reinterpret_cast<std::uintptr_t>(a) | reinterpret_cast<std::uintptr_t>(b)) % sizeof(std::uint64_t)) == 0;
}

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
        for (std::size_t i = 0; i < bytes / sizeof(std::uint64_t); i++) {
            to[i] = from[i];
        }
        done = bytes - bytes % sizeof(std::uint64_t);
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
        std::uint64_t word = static_cast<unsigned char>(value);
        word |= word << 8;
        word |= word << 16;
        word |= word << 32;
        auto *to = static_cast<std::uint64_t *>(destination);
        for (std::size_t i = 0; i < bytes / sizeof(std::uint64_t); i++) {
            to[i] = word;
        }
        done = bytes - bytes % sizeof(std::uint64_t);
    }
    auto *to = static_cast<unsigned char *>(destination);
    for (std::size_t i = done; i < bytes; i++) {
        to[i] = static_cast<unsigned char>(value);
    }
    return destination;
}

}  // extern "C"

#endif  // SCE_FORGE_GENMC_CXX_SUPPORT_H
