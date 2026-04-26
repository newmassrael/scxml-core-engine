// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/TemplateError.h"

#include <nlohmann/json.hpp>

#include <array>
#include <cstdint>
#include <cstdio>
#include <string>
#include <string_view>

namespace SCE::parsing {

namespace {

// FNV-1a 64-bit. Mirrors `sce-build/src/forge/diagnostic.rs::Fnv1a64`
// byte-for-byte: same OFFSET / PRIME constants, same byte-XOR-then-multiply
// inner loop. The id field has to round-trip the schema's
// `^fnv1a:[0-9a-f]{16}$` regex; a custom hash here would diverge from
// the Rust producer's id space silently. Keeping the algorithm
// arithmetic-identical means C++ ids share the Rust id namespace.
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

// Canonical key shape mirrors `compute_id` in
// `sce-build/src/forge/diagnostic.rs`:
//   code | stage | file_or_empty <0x1f> frag1 <0x1f> frag2 ...
//
// W1 only refits the Template family, whose Rust counterparts use
// structured `key_fragments` (template/param/declared etc.). The C++
// subtype shape is unchanged this milestone (per the RFC's "Do NOT
// change subtype shape" constraint), so the C++ side derives its key
// from the rendered message text — a single fragment. This produces
// schema-valid ids but they will not byte-match Rust's
// structured-key ids for the same logical error; consumers that
// correlate ids across the two sides are out-of-scope for W1 and
// would require lifting structured fields onto the C++ subtypes.
std::string computeFnv1aId(std::string_view code, std::string_view stage,
                           std::string_view file,
                           std::string_view messageFragment) {
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
    const int written = std::snprintf(buffer.data(), buffer.size(),
                                      "fnv1a:%016lx",
                                      static_cast<unsigned long>(hasher.finish()));
    return std::string(buffer.data(),
                       static_cast<std::size_t>(written));
}

// Every `xml/template-*` DiagnosticCode shares the `xml` Stage in the
// Rust authority (see `DiagnosticCode::stage()` in
// `sce-build/src/forge/diagnostic.rs`). Hard-coded here rather than
// derived by string-prefix-split because the prefix→stage table on
// the Rust side is not 1:1 (e.g. `cli/*` codes map to `cli`,
// `mesh/deploy-*` map to `mesh-deploy`). Future W milestones extending
// this file beyond the Template family will need their own stage
// mapping; deferred until those subtypes land.
constexpr std::string_view kTemplateStage = "xml";

}  // namespace

nlohmann::ordered_json TemplateError::to_json() const {
    const std::string_view codeStr = code();
    const std::string fileStr = location_.has_value()
                                    ? location_->file.string()
                                    : std::string{};
    const std::string_view messageView{what()};
    const std::string idStr =
        computeFnv1aId(codeStr, kTemplateStage, fileStr, messageView);

    nlohmann::ordered_json out;
    out["v"] = 1;
    out["id"] = idStr;
    out["code"] = codeStr;
    out["stage"] = kTemplateStage;
    out["message"] = std::string{messageView};

    if (location_.has_value()) {
        nlohmann::ordered_json loc;
        loc["file"] = location_->file.string();
        loc["line"] = location_->row;
        loc["col"] = location_->col;
        out["location"] = std::move(loc);
    }

    return out;
}

}  // namespace SCE::parsing
