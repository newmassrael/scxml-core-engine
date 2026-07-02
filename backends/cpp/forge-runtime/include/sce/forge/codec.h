// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — codec cursor + sink + typed error contract.
//
// Mirrors `backends/rust/forge-runtime/src/codec.rs`. RFC §synth-5-B L494-519 pins
// the per-language cursor + need-more-bytes contract on decode so a
// truncated input never aborts. RFC §synth-5-B extends the contract to the
// write side: `SceSink` is the abstract base that codec `encode`
// bodies emit into; `VectorSink` (heap-backed, infallible) and
// `SpanSink` (caller-owned `uint8_t*` + cap, raises `BufferOverflow`
// at the cap) are the two concrete impls. Caller owns the destination
// storage, mirroring the borrow contract on the decode cursor side.

#pragma once

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

namespace SCE::Forge {

/// Typed codec error. Used by decode (returned via `std::nullopt`
/// sentinel + cursor side-flag) and encode (`std::optional<CodecError>`
/// return: `std::nullopt` = success, value = error).
///
/// The variant primitive intentionally does NOT need a typed
/// `UnknownVariantTag` — RFC §synth-5-B requires `<sce:default>` when arms
/// don't exhaust the tag domain (build-time `codec/variant-arm-unreachable`
/// otherwise), so the default arm catches every unmatched tag at runtime.
///
/// The TLV chain primitive emits on cpp
/// (it was originally MCU-only as a conservative scope choice; Zenoh
/// extension envelopes need server-class peers too). On reject-policy
/// overflow cpp collapses the failure to the truncation sentinel
/// `std::nullopt` — same convention as `VleWidthOverflow` (see also
/// the matching Kotlin runtime). The typed `TlvChainOverflow` enum
/// variant lives only on Rust / C11 / Go / Python runtimes that
/// construct it at the call site.
enum class CodecError : std::uint8_t {
    NeedMoreBytes = 1,
    /// A `vle_u<N>` field's continuation chain implies a value wider
    /// than the declared type. RFC §synth-5-B `codec/vle-width-overflow`.
    VleWidthOverflow = 2,
    /// RFC §synth-5-B encode-side counterpart to `NeedMoreBytes`: the
    /// destination sink reported insufficient remaining capacity for
    /// the next write. Only the bounded `SpanSink` (caller-owned
    /// `uint8_t*` + cap) can raise this; the heap-backed `VectorSink`
    /// grows on demand and is effectively infallible. Codec authors
    /// decide per call site whether the destination is bounded; a
    /// fixed-frame on-wire codec driving a DMA bounce buffer will run
    /// on `SpanSink` and surface overflow as a typed error rather
    /// than aborting.
    BufferOverflow = 3,
};

/// Read-only cursor over a borrowed input buffer. Decode bodies use
/// `peek_slice` to bounds-check + read fixed-offset bytes positionally,
/// then `advance` after the construction succeeds.
class SceCursor {
public:
    constexpr SceCursor(const std::uint8_t *data, std::size_t len) noexcept : data_(data), len_(len), pos_(0) {}

    [[nodiscard]] constexpr std::size_t remaining() const noexcept {
        return len_ - pos_;
    }

    /// Borrow the next `n` bytes without advancing. Returns `nullptr`
    /// when the cursor's tail is shorter than `n`. Pair the returned
    /// pointer with the requested length — the cursor does not surface
    /// a slice abstraction at this size class.
    [[nodiscard]] constexpr const std::uint8_t *peek_slice(std::size_t n) const noexcept {
        if (remaining() < n) {
            return nullptr;
        }
        return data_ + pos_;
    }

    /// Advance the cursor by `n` bytes. Returns `false` if `n` would
    /// overrun the buffer.
    constexpr bool advance(std::size_t n) noexcept {
        if (remaining() < n) {
            return false;
        }
        pos_ += n;
        return true;
    }

    /// Read a base-128 variable-length encoded unsigned value of up to
    /// `max_bits` payload width. The leading bytes carry 7 data bits in
    /// their low 7 with bit 7 as the continuation flag; the final byte
    /// (at shift `7 * (VLE_LEN - 1)`) carries a full 8 data bits with no
    /// continuation flag. LSB-first byte order. Returns `std::nullopt`
    /// on `NeedMoreBytes` and signals `VleWidthOverflow` via
    /// `last_vle_overflow()` flag — split from the std::optional return
    /// to keep the hot decode path branch-light. Canonical Zenoh ZInt
    /// wire format (RFC §synth-5-B Appendix B): a u64 caps at 9 bytes.
    std::optional<std::uint64_t> read_vle_u16() noexcept {
        return read_vle_inner(16);
    }

    std::optional<std::uint64_t> read_vle_u32() noexcept {
        return read_vle_inner(32);
    }

    std::optional<std::uint64_t> read_vle_u64() noexcept {
        return read_vle_inner(64);
    }

    [[nodiscard]] constexpr bool last_vle_overflow() const noexcept {
        return vle_overflow_;
    }

private:
    std::optional<std::uint64_t> read_vle_inner(std::uint32_t max_bits) noexcept {
        vle_overflow_ = false;
        // Canonical Zenoh ZInt: ceil((W-1)/7) bytes (u64 -> 9, not 10);
        // the final byte carries a full 8 data bits, no continuation.
        const std::uint32_t vle_len = (max_bits - 1 + 6) / 7;
        const std::uint32_t final_shift = 7 * (vle_len - 1);
        std::uint64_t value = 0;
        std::uint32_t shift = 0;
        for (std::uint32_t i = 0; i < vle_len; ++i) {
            const std::uint8_t *p = peek_slice(1);
            if (p == nullptr) {
                return std::nullopt;
            }
            (void)advance(1);
            if (shift == final_shift) {
                // Final byte: 8 data bits, continuation bit reused as
                // data. For a sub-octet tail (u16 / u32) refuse a value
                // that would overflow the remaining bits.
                const std::uint32_t allowed = max_bits - shift;
                if (allowed < 8 && static_cast<std::uint64_t>(*p) > (1ULL << allowed) - 1ULL) {
                    vle_overflow_ = true;
                    return std::nullopt;
                }
                value |= static_cast<std::uint64_t>(*p) << shift;
                return value;
            }
            value |= static_cast<std::uint64_t>(*p & 0x7F) << shift;
            if ((*p & 0x80) == 0) {
                return value;
            }
            shift += 7;
        }
        vle_overflow_ = true;
        return std::nullopt;
    }

    const std::uint8_t *data_;
    std::size_t len_;
    std::size_t pos_;
    bool vle_overflow_ = false;
};

/// RFC §synth-5-B string typing (`sce:type="string"`): validate that `[p, p+n)` is a well-formed
/// UTF-8 byte sequence. Returns `true` for valid UTF-8 (including the
/// empty range), `false` for any malformed sequence (invalid lead
/// byte, incomplete multi-byte sequence, overlong encoding, or
/// surrogate). The decoder uses the standard Unicode tables (RFC 3629
/// §4): 1-byte form 0x00..0x7F; 2-byte form 0xC2..0xDF + one continuation
/// (0x80..0xBF); 3-byte form 0xE0..0xEF + two continuations with
/// overlong / surrogate guards; 4-byte form 0xF0..0xF4 + three
/// continuations bounded to U+10FFFF.
///
/// `inline` so codec headers can reuse the validator without forcing
/// a separate translation unit on the cpp consumer (mirrors the
/// header-only sce_forge_runtime contract). Used by codec decode
/// bodies emitted from `sce:type="string"` fields — Cpp collapses
/// invalid UTF-8 to `std::nullopt` (the existing truncation sentinel)
/// rather than constructing a typed `CodecError::InvalidUtf8`, which
/// would otherwise force every String-bearing codec's signature to
/// surface the variant (Rust + Go + Python construct the variant
/// because their decode return types already distinguish error cases).
// ── Write-side sink ──────────────────────────────────────────────

/// Write-side cursor for codec emit. Object-oriented base class that
/// generated `encode` bodies accept by reference.
///
/// `SceSink` mirrors the read-side `SceCursor` borrow contract: callers
/// own the destination storage, and codec bodies append bytes to it
/// positionally without owning the buffer. The pure-virtual
/// `write_bytes` writes raw bytes; per-width helpers (`write_u8`,
/// `write_u16_le`, …) are provided as non-virtual default methods so
/// concrete sinks can override `write_bytes` once without re-stating
/// the per-width call surface.
///
/// Return-shape mirror: encode returns `std::optional<CodecError>` —
/// `std::nullopt` on success, populated variant on failure. This
/// mirrors decode's `std::optional<T>` (decode returns Optional<Value>;
/// encode returns Optional<Error>) and keeps the `CodecError` enum
/// pure (no `Ok` sentinel polluting the error type).
class SceSink {
public:
    virtual ~SceSink() = default;

    /// Append `n` bytes from `data` to the underlying storage.
    /// Concrete sinks raise `CodecError::BufferOverflow` when the
    /// destination has insufficient remaining capacity; growable sinks
    /// return `std::nullopt`.
    [[nodiscard]] virtual std::optional<CodecError> write_bytes(const std::uint8_t *data, std::size_t n) noexcept = 0;

    /// Bytes written by this sink instance since it wrapped the
    /// destination. Used by codec emit for offset-aware writes (DMA-
    /// aligned field padding, length-prefix back-patching). Distinct
    /// from "total bytes in destination" — a `VectorSink` over a
    /// `std::vector` that already had bytes returns the delta, not
    /// the absolute length, so codec emit positional math operates
    /// within its own encoding regardless of coalesced-send prefix
    /// state.
    [[nodiscard]] virtual std::size_t position() const noexcept = 0;

    /// Append a single byte. Default impl forwards to `write_bytes`.
    [[nodiscard]] std::optional<CodecError> write_u8(std::uint8_t v) noexcept {
        return write_bytes(&v, 1);
    }

    /// Append a little-endian `uint16_t` (2 bytes).
    [[nodiscard]] std::optional<CodecError> write_u16_le(std::uint16_t v) noexcept {
        std::uint8_t buf[2] = {
            static_cast<std::uint8_t>(v),
            static_cast<std::uint8_t>(v >> 8),
        };
        return write_bytes(buf, 2);
    }

    /// Append a big-endian `uint16_t` (2 bytes).
    [[nodiscard]] std::optional<CodecError> write_u16_be(std::uint16_t v) noexcept {
        std::uint8_t buf[2] = {
            static_cast<std::uint8_t>(v >> 8),
            static_cast<std::uint8_t>(v),
        };
        return write_bytes(buf, 2);
    }

    /// Append a little-endian `uint32_t` (4 bytes).
    [[nodiscard]] std::optional<CodecError> write_u32_le(std::uint32_t v) noexcept {
        std::uint8_t buf[4] = {
            static_cast<std::uint8_t>(v),
            static_cast<std::uint8_t>(v >> 8),
            static_cast<std::uint8_t>(v >> 16),
            static_cast<std::uint8_t>(v >> 24),
        };
        return write_bytes(buf, 4);
    }

    /// Append a big-endian `uint32_t` (4 bytes).
    [[nodiscard]] std::optional<CodecError> write_u32_be(std::uint32_t v) noexcept {
        std::uint8_t buf[4] = {
            static_cast<std::uint8_t>(v >> 24),
            static_cast<std::uint8_t>(v >> 16),
            static_cast<std::uint8_t>(v >> 8),
            static_cast<std::uint8_t>(v),
        };
        return write_bytes(buf, 4);
    }

    /// Append a little-endian `uint64_t` (8 bytes).
    [[nodiscard]] std::optional<CodecError> write_u64_le(std::uint64_t v) noexcept {
        std::uint8_t buf[8] = {
            static_cast<std::uint8_t>(v),       static_cast<std::uint8_t>(v >> 8),  static_cast<std::uint8_t>(v >> 16),
            static_cast<std::uint8_t>(v >> 24), static_cast<std::uint8_t>(v >> 32), static_cast<std::uint8_t>(v >> 40),
            static_cast<std::uint8_t>(v >> 48), static_cast<std::uint8_t>(v >> 56),
        };
        return write_bytes(buf, 8);
    }

    /// Append a big-endian `uint64_t` (8 bytes).
    [[nodiscard]] std::optional<CodecError> write_u64_be(std::uint64_t v) noexcept {
        std::uint8_t buf[8] = {
            static_cast<std::uint8_t>(v >> 56), static_cast<std::uint8_t>(v >> 48), static_cast<std::uint8_t>(v >> 40),
            static_cast<std::uint8_t>(v >> 32), static_cast<std::uint8_t>(v >> 24), static_cast<std::uint8_t>(v >> 16),
            static_cast<std::uint8_t>(v >> 8),  static_cast<std::uint8_t>(v),
        };
        return write_bytes(buf, 8);
    }

    /// Append `value` as a base-128 VLE of `max_bits` payload width. The
    /// write-side counterpart of SceCursor::read_vle_inner: leading
    /// bytes carry 7 data bits + a continuation flag; the final byte
    /// (after at most VLE_LEN-1 continuation bytes) carries a full 8 data
    /// bits with no flag, so a u64 caps at 9 bytes — canonical Zenoh
    /// ZInt (RFC §synth-5-B Appendix B). VLE_LEN = ceil((max_bits-1)/7).
    [[nodiscard]] std::optional<CodecError> write_vle_inner(std::uint64_t value, std::uint32_t max_bits) noexcept {
        const std::uint32_t cont_max = (max_bits - 1 + 6) / 7 - 1;
        std::uint64_t v = value;
        std::uint32_t n = 0;
        while (v >= 0x80 && n < cont_max) {
            if (auto _e = write_u8(static_cast<std::uint8_t>((v & 0x7F) | 0x80))) {
                return _e;
            }
            v >>= 7;
            ++n;
        }
        return write_u8(static_cast<std::uint8_t>(v));
    }

    [[nodiscard]] std::optional<CodecError> write_vle_u16(std::uint16_t v) noexcept {
        return write_vle_inner(v, 16);
    }

    [[nodiscard]] std::optional<CodecError> write_vle_u32(std::uint32_t v) noexcept {
        return write_vle_inner(v, 32);
    }

    [[nodiscard]] std::optional<CodecError> write_vle_u64(std::uint64_t v) noexcept {
        return write_vle_inner(v, 64);
    }
};

/// Heap-backed sink over a caller-owned `std::vector<uint8_t>&`. The
/// vector grows on demand, so `write_bytes` never returns
/// `BufferOverflow` — the implementation is effectively infallible.
/// The natural sink for std consumers and the engine behind the
/// generated `encode_to_vec()` facade.
class VectorSink final : public SceSink {
public:
    explicit VectorSink(std::vector<std::uint8_t> &dst) noexcept : dst_(dst), start_len_(dst.size()) {}

    [[nodiscard]] std::optional<CodecError> write_bytes(const std::uint8_t *data, std::size_t n) noexcept override {
        dst_.insert(dst_.end(), data, data + n);
        return std::nullopt;
    }

    [[nodiscard]] std::size_t position() const noexcept override {
        return dst_.size() - start_len_;
    }

private:
    std::vector<std::uint8_t> &dst_;
    std::size_t start_len_;
};

/// Bounded sink over a caller-owned raw byte buffer + capacity. Raises
/// `CodecError::BufferOverflow` when a write would exceed `cap`. The
/// natural sink for DMA / fixed-frame call sites where the destination
/// storage is owned by an upstream peripheral driver.
class SpanSink final : public SceSink {
public:
    SpanSink(std::uint8_t *buf, std::size_t cap) noexcept : buf_(buf), cap_(cap), pos_(0) {}

    [[nodiscard]] std::optional<CodecError> write_bytes(const std::uint8_t *data, std::size_t n) noexcept override {
        if (cap_ - pos_ < n) {
            return CodecError::BufferOverflow;
        }
        std::memcpy(buf_ + pos_, data, n);
        pos_ += n;
        return std::nullopt;
    }

    [[nodiscard]] std::size_t position() const noexcept override {
        return pos_;
    }

    /// Remaining capacity from current position to end of buffer.
    [[nodiscard]] std::size_t remaining() const noexcept {
        return cap_ - pos_;
    }

private:
    std::uint8_t *buf_;
    std::size_t cap_;
    std::size_t pos_;
};

[[nodiscard]] inline bool is_valid_utf8(const std::uint8_t *p, std::size_t n) noexcept {
    std::size_t i = 0;
    while (i < n) {
        const std::uint8_t b0 = p[i];
        if (b0 <= 0x7F) {
            i += 1;
        } else if (b0 >= 0xC2 && b0 <= 0xDF) {
            if (i + 1 >= n) {
                return false;
            }
            const std::uint8_t b1 = p[i + 1];
            if (b1 < 0x80 || b1 > 0xBF) {
                return false;
            }
            i += 2;
        } else if (b0 >= 0xE0 && b0 <= 0xEF) {
            if (i + 2 >= n) {
                return false;
            }
            const std::uint8_t b1 = p[i + 1];
            const std::uint8_t b2 = p[i + 2];
            // Per RFC 3629 §4: E0 disallows overlong (b1 < 0xA0); ED
            // disallows surrogate (b1 > 0x9F = 0xA0..0xBF excluded).
            const std::uint8_t b1_min = (b0 == 0xE0) ? 0xA0 : 0x80;
            const std::uint8_t b1_max = (b0 == 0xED) ? 0x9F : 0xBF;
            if (b1 < b1_min || b1 > b1_max) {
                return false;
            }
            if (b2 < 0x80 || b2 > 0xBF) {
                return false;
            }
            i += 3;
        } else if (b0 >= 0xF0 && b0 <= 0xF4) {
            if (i + 3 >= n) {
                return false;
            }
            const std::uint8_t b1 = p[i + 1];
            const std::uint8_t b2 = p[i + 2];
            const std::uint8_t b3 = p[i + 3];
            // F0 disallows overlong (b1 < 0x90); F4 caps at U+10FFFF
            // (b1 > 0x8F = 0x90..0xBF excluded).
            const std::uint8_t b1_min = (b0 == 0xF0) ? 0x90 : 0x80;
            const std::uint8_t b1_max = (b0 == 0xF4) ? 0x8F : 0xBF;
            if (b1 < b1_min || b1 > b1_max) {
                return false;
            }
            if (b2 < 0x80 || b2 > 0xBF) {
                return false;
            }
            if (b3 < 0x80 || b3 > 0xBF) {
                return false;
            }
            i += 4;
        } else {
            // Lead bytes 0x80..0xC1 (continuation in lead position +
            // overlong 1-byte) and 0xF5..0xFF (above U+10FFFF) reject.
            return false;
        }
    }
    return true;
}

}  // namespace SCE::Forge
