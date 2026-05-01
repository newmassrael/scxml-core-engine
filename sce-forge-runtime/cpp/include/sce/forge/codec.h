// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — codec cursor + typed error contract.
//
// Mirrors `sce-forge-runtime/rust/src/codec.rs`. RFC §5.B L494-519 pins
// the per-language cursor + need-more-bytes contract on decode so a
// truncated input never aborts.
//
// B1-prep ships the minimum API the existing fixed-width codec fixtures
// need: `peek_slice`, `advance`, `remaining`. Streaming readers
// (`read_u8`, `read_vle_*`, `read_tag`) land alongside their first
// consumer in B1-α/β/δ. Encode-side cursor + `BufferOverflow` lands in
// B1-α (variable-length VLE encode is the first reachable consumer).

#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>

namespace SCE::Forge {

/// Typed decode error. `NeedMoreBytes` is the only reachable variant
/// while every codec field is fixed-width. B1-α adds `VleWidthOverflow`,
/// B1-β adds `UnknownVariantTag`.
enum class CodecError : std::uint8_t {
    NeedMoreBytes = 1,
};

/// Read-only cursor over a borrowed input buffer. Decode bodies use
/// `peek_slice` to bounds-check + read fixed-offset bytes positionally,
/// then `advance` after the construction succeeds.
class SceCursor {
public:
    constexpr SceCursor(const std::uint8_t* data, std::size_t len) noexcept
        : data_(data), len_(len), pos_(0) {}

    [[nodiscard]] constexpr std::size_t remaining() const noexcept {
        return len_ - pos_;
    }

    /// Borrow the next `n` bytes without advancing. Returns `nullptr`
    /// when the cursor's tail is shorter than `n`. Pair the returned
    /// pointer with the requested length — the cursor does not surface
    /// a slice abstraction at this size class.
    [[nodiscard]] constexpr const std::uint8_t* peek_slice(std::size_t n) const noexcept {
        if (remaining() < n) return nullptr;
        return data_ + pos_;
    }

    /// Advance the cursor by `n` bytes. Returns `false` if `n` would
    /// overrun the buffer.
    constexpr bool advance(std::size_t n) noexcept {
        if (remaining() < n) return false;
        pos_ += n;
        return true;
    }

private:
    const std::uint8_t* data_;
    std::size_t len_;
    std::size_t pos_;
};

}  // namespace SCE::Forge
