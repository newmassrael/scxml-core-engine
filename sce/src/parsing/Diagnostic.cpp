// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <array>
#include <cstdint>
#include <cstdio>
#include <string>
#include <string_view>

namespace SCE::parsing {

namespace {

// FNV-1a 64-bit. Mirrors `sce-build/src/forge/diagnostic.rs::Fnv1a64`
// byte-for-byte: same OFFSET / PRIME constants, same byte-XOR-then-
// multiply inner loop. Hosted on `Diagnostic.cpp` so every concrete
// subtype's `to_json()` impl shares one canonical id helper —
// extracted from `TemplateError.cpp`'s anonymous namespace when the
// W3 `XIncludeError` family arrived as the second consumer.
class Fnv1a64 {
public:
    static constexpr uint64_t kOffset = 0xcbf29ce484222325ULL;
    static constexpr uint64_t kPrime = 0x00000100000001b3ULL;

    void write(std::string_view bytes) noexcept {
        for (unsigned char b : bytes) {
            state_ ^= static_cast<uint64_t>(b);
            state_ *= kPrime;
        }
    }

    uint64_t finish() const noexcept {
        return state_;
    }

private:
    uint64_t state_{kOffset};
};

}  // namespace

std::string
computeFnv1aDiagnosticId(std::string_view code, std::string_view stage,
                         std::string_view file,
                         std::string_view messageFragment) {
    // Canonical key shape mirrors `compute_id` in
    // `sce-build/src/forge/diagnostic.rs`:
    //   code | stage | file_or_empty <0x1f> messageFragment
    //
    // The Rust authority feeds *structured* `key_fragments` after the
    // unit-separator byte; the C++ side currently sends a single
    // rendered-message fragment. Schema-valid on both sides; not
    // byte-equivalent for the same logical error. Lifting structured
    // fields onto C++ subtypes is W3+ scope (RFC §1 Q5).
    Fnv1a64 hasher;
    hasher.write(code);
    hasher.write("|");
    hasher.write(stage);
    hasher.write("|");
    hasher.write(file);
    constexpr char kUnitSeparator = static_cast<char>(0x1f);
    hasher.write(std::string_view{&kUnitSeparator, 1});
    hasher.write(messageFragment);

    std::array<char, 32> buffer{};
    const int written = std::snprintf(
        buffer.data(), buffer.size(), "fnv1a:%016lx",
        static_cast<unsigned long>(hasher.finish()));
    return std::string(buffer.data(), static_cast<std::size_t>(written));
}

std::string Diagnostic::to_canonical_json_string() const {
    // `to_json()` returns `nlohmann::ordered_json` — keys are emitted
    // in producer insertion order, which is convenient for human
    // reading but unstable as a byte-diff target. Re-parse through
    // the default `nlohmann::json` (std::map-backed) to alphabetise
    // keys, then dump with `dump(-1, ' ', false)`:
    //   indent = -1 → no whitespace between tokens
    //   indent_char = ' ' → unused (indent = -1)
    //   ensure_ascii = false → preserve UTF-8 bytes verbatim
    // The result is the canonical NDJSON record body the W1 byte-
    // diff parity test pins against Rust's --error-format=json
    // output. §wire-W2 deliverable item #3.
    const auto record = to_json();
    const auto canonical =
        nlohmann::json::parse(record.dump());
    return canonical.dump(-1, ' ', false);
}

}  // namespace SCE::parsing
