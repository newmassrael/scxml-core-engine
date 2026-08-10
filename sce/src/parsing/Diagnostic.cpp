// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <array>
#include <cstdint>
#include <cstdio>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

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

// Content-addressed FNV-1a 64-bit id. Canonical key shape mirrors
// `compute_id` in `sce-build/src/forge/diagnostic.rs`:
//
//     code | stage | file_or_empty  (<0x1f> fragment)*
//
// The unit separator PRECEDES each fragment rather than joining them,
// which is what makes a zero-fragment diagnostic hash without any
// separator at all — the shape Rust's `for frag in key_fragments`
// loop produces, and the reason a C++ side that always wrote one
// separator could not match `XIncludeError::MissingHref` (a unit
// variant, zero fragments) even after the fragment values agreed.
//
// Returned string satisfies the schema's `^fnv1a:[0-9a-f]{16}$`.
// File-local: the only caller is `Diagnostic::beginRecord`, so no
// leaf can reach past its declared fragments and hash something else.
std::string computeFnv1aDiagnosticId(std::string_view code, std::string_view stage, std::string_view file,
                                     const std::vector<std::string> &keyFragments) {
    Fnv1a64 hasher;
    hasher.write(code);
    hasher.write("|");
    hasher.write(stage);
    hasher.write("|");
    hasher.write(file);
    constexpr char kUnitSeparator = static_cast<char>(0x1f);
    for (const auto &fragment : keyFragments) {
        hasher.write(std::string_view{&kUnitSeparator, 1});
        hasher.write(fragment);
    }

    std::array<char, 32> buffer{};
    const int written =
        std::snprintf(buffer.data(), buffer.size(), "fnv1a:%016lx", static_cast<unsigned long>(hasher.finish()));
    return std::string(buffer.data(), static_cast<std::size_t>(written));
}

// Open a v1 record with the envelope every leaf shares.
//
// Exists because these keys were spelled out in all eight `to_json()`
// overrides. Adding `generator` to the schema meant editing eight sites
// that each had to be found, and an override that had been missed would
// have emitted a record the shared schema rejects while every other
// subtype's fixture stayed green. One assembly point makes the required
// set a property of this function rather than of whoever remembered.
//
// `generator` is the commit the library was built from, supplied by
// `SCE_GIT_COMMIT` from `sce/CMakeLists.txt`; see there for why the git
// refs are configure-dependencies. `unknown` when the build had no
// checkout to read — the value the schema's pattern allows alongside a
// hex commit, and the honest answer rather than a fabricated one.
nlohmann::ordered_json beginDiagnosticRecord(std::string_view code, std::string_view stage, const std::string &id) {
#ifdef SCE_GIT_COMMIT
    constexpr std::string_view kGeneratorCommit = SCE_GIT_COMMIT;
#else
    // Reached only when this translation unit is compiled outside the
    // `sce_base` target that defines the macro. Answering `unknown` is
    // what the schema's pattern allows for a build with no commit to
    // report; fabricating one would put a wrong commit on the wire.
    constexpr std::string_view kGeneratorCommit = "unknown";
#endif

    nlohmann::ordered_json out;
    out["v"] = 1;
    out["id"] = id;
    out["generator"] = std::string{kGeneratorCommit};
    out["code"] = std::string{code};
    out["stage"] = std::string{stage};
    return out;
}

}  // namespace

nlohmann::ordered_json Diagnostic::beginRecord() const {
    const std::string_view file = location_.has_value() ? std::string_view{location_->file} : std::string_view{};
    const std::string id = computeFnv1aDiagnosticId(code(), stage(), file, keyFragments_);
    return beginDiagnosticRecord(code(), stage(), id);
}

void Diagnostic::appendLocation(nlohmann::ordered_json &out) const {
    if (!location_.has_value() || location_->file.empty()) {
        return;
    }
    nlohmann::ordered_json loc;
    loc["file"] = location_->file;
    if (location_->line.has_value()) {
        loc["line"] = *location_->line;
    }
    if (location_->col.has_value()) {
        loc["col"] = *location_->col;
    }
    out["location"] = std::move(loc);
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
    const auto canonical = nlohmann::json::parse(record.dump());
    return canonical.dump(-1, ' ', false);
}

}  // namespace SCE::parsing
