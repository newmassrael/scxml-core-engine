// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce/forge/broadcast_log_narrow.h: the narrow row of the runtime half of
// `sce:kind="broadcast-log"` (ADR 0062, "Stamp width") for C++: the log of
// sce/forge/broadcast_log.h for a target whose atomics are 32 bits wide or only loads
// and stores (Cortex-M0 and M3, RISC-V 32). It is the C++ counterpart of the Rust
// runtime's narrow row (backends/rust/forge-runtime/src/broadcast_log_core.rs, the lane
// `Narrow<B>`) and of the C11 one (sce/forge/broadcast_log_narrow.h of the C runtime),
// with the same memory orders, the same two fences and the same arithmetic, and unlike
// the C11 row it is not a second copy of the algorithm: it is the one `BroadcastLog` of
// broadcast_log.h over the lane `Narrow<Bits>` defined here, so that a change of an
// order in that text is a change of both rows.
//
// The stamp, the head and every word of an entry are `std::uint32_t` atomics. The row
// needs no 64-bit atomic. The one operation that needs a read-modify-write instruction,
// `publisher()` (an exchange on the flag that makes the publisher one), has the
// ownership forms beside it, `split()` and `publisher_unchecked()`, for a target that
// has none; a member of a class template is compiled only where it is used, so a log
// whose publisher is taken by either never references the exchange.
//
// What a narrow stamp changes is arithmetic, written once in broadcast_log.h in terms of
// the stamp space `M = 2^w`, `w` = `Bits`:
//
//   - The stored stamps and head are the low `w` bits of the count they stand for:
//     busy(n) = (2n + 1) mod M and done(n) = (2n + 2) mod M, so a stamp tells apart
//     K = M / 2 consecutive entries of one slot, and the head stores (n + 1) mod M.
//   - A sequence number is a count modulo M: `head()`, a reader's `cursor()`, the `seq`
//     of a `Read` and the number `publish` returns are residues, the same for every
//     handle, and they are not ordered across a wrap. A reader takes its distance from
//     the head as (head - cursor) mod M, which is the true distance while it is below M.
//   - The slot count divides M, so that the slot index `n mod Slots`, which a reader
//     knows only from a residue, is the publisher's: a power of two, at most K. The
//     class refuses another shape at compile time.
//   - The wrap bound. A reader gets the answers of the specification of ADR 0062
//     section 2 while, during each read, the newest publish begun is at most K past its
//     cursor. A deploy states it as the longest pause in publishes between two of a
//     reader's reads (`min_wrap_ops`), which must not exceed K - Slots; the generator
//     refuses a log whose bound does not cover it. A reader that breaks the bound is not
//     told: what it is answered is defined, not undefined (see `BroadcastLog::read`).
//
// kUnwritten. A count modulo M cannot tell a log that has published fewer than
// `kRetention` entries from one that has published more and wrapped, which
// `reader_oldest` and the bind of a deferred reader need to know. They ask the last slot:
// it is first written by the publish of entry `kRetention`, so a stamp of `kUnwritten`
// there means at most `kRetention` entries were published and the oldest retained is
// entry 0. The value is 1, which no write into the last slot produces (the last slot
// holds the entries n = Slots - 1 modulo Slots, stamped 2 * Slots - 1 and 0 modulo
// 2 * Slots, and 2 * Slots divides M), and every slot of a new log holds it.
//
// The stamp width is a template parameter, 32 in a row (`kNarrowStampBits`); a unit test
// uses 3, so that the head's wrap is a few publishes away, as the Rust lane's parameter
// does. A lane keeps `Bits` between 2 and 32: a word is 32 bits, so a stamp is at most
// that, and it needs a bit for busy against done and one for the lap.
//
// Interface: the one of broadcast_log.h, over `Entry` of `W` 32-bit words, with
// `NarrowBroadcastLog<W, Slots>` the log of a row.

#pragma once

#include <atomic>
#include <cstddef>
#include <cstdint>

#include "sce/forge/broadcast_log.h"

namespace SCE::Forge::Broadcast {

/// The width of the stamp a row keeps, in bits. The generator judges a narrow log
/// against it, and a generated header asserts that the runtime keeps the width it
/// was judged against.
inline constexpr unsigned kNarrowStampBits = 32;

/// The narrow lane: a stamp of `Bits` bits kept in a 32-bit atomic, over words of 32
/// bits. It needs no 64-bit atomic and no read-modify-write instruction. A lane is
/// what `BroadcastLog` reads (see `Wide`): `Raw`, `Cell`, `kBits`, `kUnwritten` and
/// `kLockFree`.
///
/// `Bits` is between 2 and 32. The slot count of a log over it is a power of two no
/// greater than `2^(Bits - 1)`, which `BroadcastLog` checks.
template <unsigned Bits> struct Narrow {
    static_assert(Bits >= 2, "a stamp needs a bit for busy against done and one for the lap");
    static_assert(Bits <= 32, "a narrow stamp is at most 32 bits: its words are 32-bit");

    using Raw = std::uint32_t;
    using Cell = std::atomic<Raw>;
    static constexpr unsigned kBits = Bits;
    // The stamp of a slot nothing has been written to: an odd value that no write into
    // the last slot produces (see the header).
    static constexpr Raw kUnwritten = 1;
    static constexpr bool kLockFree = Cell::is_always_lock_free;
};

/// The log of the narrow row: `Slots` slots of `W` 32-bit words over a stamp of
/// `Bits` bits (`kNarrowStampBits` unless a unit test names a narrower one).
template <std::size_t W, std::size_t Slots, unsigned Bits = kNarrowStampBits>
using NarrowBroadcastLog = BroadcastLog<W, Slots, Narrow<Bits>>;

}  // namespace SCE::Forge::Broadcast
