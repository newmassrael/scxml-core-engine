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

/// Typed decode error. B1-β adds `UnknownVariantTag`.
enum class CodecError : std::uint8_t {
    NeedMoreBytes = 1,
    /// A `vle_u<N>` field's continuation chain implies a value wider
    /// than the declared type. RFC §5.B `codec/vle-width-overflow`.
    VleWidthOverflow = 2,
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

    /// Read a base-128 variable-length encoded unsigned value of up to
    /// `max_bits` payload width. Each byte carries 7 data bits in its
    /// low 7; bit 7 is the continuation flag. LSB-first byte order.
    /// Returns `std::nullopt` on `NeedMoreBytes` and signals
    /// `VleWidthOverflow` via `last_vle_overflow()` flag — split from
    /// the std::optional return to keep the hot decode path branch-light.
    /// Mirrors the Zenoh ZInt wire format (RFC §5.B Appendix B).
    std::optional<std::uint64_t> read_vle_u16() noexcept { return read_vle_inner(16); }
    std::optional<std::uint64_t> read_vle_u32() noexcept { return read_vle_inner(32); }
    std::optional<std::uint64_t> read_vle_u64() noexcept { return read_vle_inner(64); }

    [[nodiscard]] constexpr bool last_vle_overflow() const noexcept { return vle_overflow_; }

private:
    std::optional<std::uint64_t> read_vle_inner(std::uint32_t max_bits) noexcept {
        vle_overflow_ = false;
        const std::uint32_t max_bytes = (max_bits + 6) / 7;
        std::uint64_t value = 0;
        std::uint32_t shift = 0;
        for (std::uint32_t i = 0; i < max_bytes; ++i) {
            const std::uint8_t* p = peek_slice(1);
            if (p == nullptr) return std::nullopt;
            (void)advance(1);
            const std::uint64_t payload = static_cast<std::uint64_t>(*p & 0x7F);
            if (shift + 7 > max_bits) {
                const std::uint32_t allowed = max_bits - shift;
                const std::uint64_t max_payload = (1ULL << allowed) - 1ULL;
                if (payload > max_payload) {
                    vle_overflow_ = true;
                    return std::nullopt;
                }
            }
            value |= payload << shift;
            if ((*p & 0x80) == 0) {
                return value;
            }
            shift += 7;
        }
        vle_overflow_ = true;
        return std::nullopt;
    }

    const std::uint8_t* data_;
    std::size_t len_;
    std::size_t pos_;
    bool vle_overflow_ = false;
};

}  // namespace SCE::Forge
