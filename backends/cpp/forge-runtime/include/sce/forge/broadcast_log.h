// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce/forge/broadcast_log.h — the runtime half of `sce:kind="broadcast-log"`
// (SCE Protocol-Synthesis RFC §synth-5-S, ADR 0062) for C++: one publisher, any number of readers, and the oldest entry
// forgotten when the log is full. The same algorithm as the Rust runtime
// (backends/rust/forge-runtime/src/broadcast_log_core.rs), with the same memory orders
// and the same two fences, because the orders are part of the algorithm and not a
// choice: a sequentially consistent model cannot tell a correct one from a broken
// one, and the loom model of the Rust row found that a relaxed busy stamp and a
// relaxed second stamp load are both faults. Rust's Release / Acquire / Relaxed
// map to std::memory_order_release / acquire / relaxed one for one, and its
// `fence` to std::atomic_thread_fence.
//
// The contract is the sequential log of ADR 0062 section 2. `h` entries have been
// published and only the last `kRetention` are retained. A reader owns a cursor:
// `read` returns the entry at the cursor, or says the log has nothing new, or says
// how many entries the reader missed and moves it to the oldest retained.
// Publishing never fails and never waits, and neither does a read: both are
// wait-free, in a number of steps that depends on the width of an entry and on
// nothing the other side does.
//
// The protocol (ADR 0062 section 4). A slot holds a stamp and the entry's words.
// The publisher writes a slot as `stamp := busy(n)` (release), a release fence, the
// words (relaxed), `stamp := done(n)` (release), then `head := n + 1` (release). A
// reader holding cursor `r` loads `head` (acquire) and answers Empty at the head
// and Lagged outside the window; otherwise it loads the stamp of slot `r mod
// Slots` (acquire), copies the words (relaxed), executes an acquire fence, and
// loads the stamp again (acquire): if it is still `done(r)` the copy is entry `r`;
// if not, the publisher has reused the slot, and the reader reloads `head` and
// answers Lagged. This is a sequence lock in the form Boehm showed correct under
// the C++11 model (MSPC 2012): the words are atomic accesses, so there is no data
// race, and the two fences carry the order.
//
// The slot count is `kRetention + 1`. The slot being overwritten is in flux and
// cannot be promised to a reader; with `kRetention` slots a reader could find the
// entry the specification retains already being overwritten and be told it lagged
// by nothing (docs/formal/BroadcastLog.tla finds that state).
//
// One implementation, two rows. `BroadcastLog<W, Slots, L>` is written once over a
// lane `L`, which fixes what a row varies: the type a stamp, the head and a word
// are kept in (`L::Raw`, in a `std::atomic`), the width `w` of the stamp in bits
// (`L::kBits`), and the stamp of a slot nothing was written to (`L::kUnwritten`).
// `Wide`, below, is the 64-bit row and is the default lane, so `BroadcastLog<W,
// Slots>` is what it always was. The row for a target without lock-free 64-bit
// atomics is the lane `Narrow<Bits>` of sce/forge/broadcast_log_narrow.h, over the
// same text: a stamp of `Bits` bits and 32-bit words. Every stamp, the head and a
// sequence number are counts modulo `2^w`; the 64-bit lane is the case in which the
// count never reaches `2^w` (the publisher stops at `kHorizon`), and the arithmetic
// reduces to the identity. The orders and the fences are written once, so a change
// of one is a change of both rows.
//
// The 64-bit stamp is a sequence number and not a flag, so a reader comparing
// against `done(r)` cannot mistake a later lap for the entry it wants. It stays
// below `kHorizon`, 292 years at a billion publishes per second. A target without
// lock-free atomics of the lane's width has no log of that row: the header refuses
// to compile there.
//
// No heap, no threads, no global state: a log owns its storage, and where it
// lives is the caller's choice. The constructor is
// `constexpr`, so a log in static storage is constant-initialised and an interrupt
// handler may use it before any dynamic initialisation has run. An entry is `W`
// words of the lane, the wire form of the element (ADR 0049/0058) and never a value
// with ownership, so an entry has no destructor to run when it is overwritten.
//
// Interface, the same on every backend:
//
//   publisher()                  the one publisher, or std::nullopt while another
//                                exists. It gives its place back when it is
//                                destroyed, and its successor continues the
//                                sequence. It takes the place with an exchange.
//   split()                      the same place, taken with a load and a store for a
//                                target that has no read-modify-write instruction,
//                                from a log the caller holds exclusively.
//   publisher_unchecked()        the same place on the caller's word, for a log in
//                                static storage that is shared from the start.
//   publish(entry) -> seq        never fails and never waits.
//   reader_oldest() / reader_head() / reader_at(seq) / reader_deferred()
//                                a Reader value that owns a cursor and a total of
//                                the entries it has been told it missed. The
//                                deferred one has no cursor until its first read.
//   read(reader, out) -> Read    Item{seq}, Empty or Lagged{skipped >= 1}.
//   head()                       the number of entries published.

#pragma once

#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>
#include <optional>

#include "sce/forge/queue.h"

namespace SCE::Forge::Broadcast {

/// The most entries a log of the 64-bit row can publish: the stamps are `2n + 2`,
/// which must fit a `std::uint64_t`. Past it `publish` publishes nothing and
/// returns `kNoSequence`. A narrow row counts modulo its stamp space and has no
/// horizon.
inline constexpr std::uint64_t kHorizon = (UINT64_MAX - 2) / 2;

/// What `publish` returns when the log has published its horizon of entries. It
/// is not a sequence number: every sequence number is below the horizon, and below
/// the stamp space of a narrow row.
inline constexpr std::uint64_t kNoSequence = UINT64_MAX;

namespace detail {

/// The arithmetic of a stamp of `Bits` bits, `M = 2^Bits` values. Everything is
/// written once in terms of counts modulo `M`: the stored stamps and head are the
/// low `Bits` bits of the count they stand for, and a reader takes its distance
/// from the head modulo `M`, which is the true distance whenever that is below
/// `M`. At 64 bits the count never reaches `M`, and each function is the plain
/// arithmetic it would be with no wrap.
///
/// A stamp tells apart `K = M / 2` consecutive entries of one slot:
/// `done(n) = done(n')` for two entries of one slot exactly when `n = n' (mod K)`,
/// which holds because the slot count divides `K` in a narrow row (ADR 0062
/// section 4, "What the narrow and load/store-only rows landed").
template <unsigned Bits> struct StampArith {
    static_assert(Bits >= 2 && Bits <= 64,
                  "a stamp needs a bit for busy against done and one for the lap, and is at most 64 bits");

    /// The low `Bits` bits. The shift count is reduced so that the arm that is not
    /// taken at 64 bits is still a valid shift.
    static constexpr std::uint64_t kMask = Bits >= 64 ? UINT64_MAX : (std::uint64_t{1} << (Bits % 64)) - 1;

    /// The number of values a stamp takes, `2^Bits`; 0 at 64 bits, whose count is
    /// never taken modulo it.
    static constexpr std::uint64_t kSpace = kMask + 1;

    /// Half the stamp space, `K`: how many consecutive entries of one slot a stamp
    /// tells apart, and the most slots a narrow log keeps.
    static constexpr std::uint64_t kHalf = std::uint64_t{1} << (Bits - 1);

    /// The stamp a slot holds while the entry `n` is being written into it.
    static constexpr std::uint64_t busy(std::uint64_t n) noexcept {
        return (n * 2 + 1) & kMask;
    }

    /// The stamp a slot holds once the entry `n` is in it.
    static constexpr std::uint64_t done(std::uint64_t n) noexcept {
        return (n * 2 + 2) & kMask;
    }

    /// How far `cursor` is behind `head`, both counts modulo `2^Bits`: the true
    /// distance whenever that is below `2^Bits`.
    static constexpr std::uint64_t distance(std::uint64_t head, std::uint64_t cursor) noexcept {
        return (head - cursor) & kMask;
    }

    /// `cursor` moved `by` entries on, modulo `2^Bits`.
    static constexpr std::uint64_t advance(std::uint64_t cursor, std::uint64_t by) noexcept {
        return (cursor + by) & kMask;
    }

    /// How many entries a reader missed, from `behind`, how far behind the head it
    /// found itself, for a log of `slots` slots retaining `retention` entries. A lag
    /// is only answered once the publisher is `slots` entries or more ahead, so a
    /// `behind` below that came from a head that has wrapped since the reader's
    /// cursor was valid: one lap of the stamp space is added, the fewest it can be.
    /// The 64-bit stamp has no lap (`kSpace` is 0), and its `behind` is never below
    /// `slots` here.
    static constexpr std::uint64_t missed(std::uint64_t behind, std::uint64_t slots, std::uint64_t retention) noexcept {
        const std::uint64_t counted = behind < slots ? behind + kSpace : behind;
        return counted > retention ? counted - retention : 0;
    }

    /// Whether a log of `slots` slots is one this stamp can number: at least two, a
    /// power of two so that the slot index `n mod slots` stays the publisher's across
    /// the stamp's wrap (`slots` divides `2^Bits`), and at most half the stamp space
    /// so that a stamp tells apart more than one lap of a slot.
    static constexpr bool slots_fit(std::uint64_t slots) noexcept {
        return slots >= 2 && (slots & (slots - 1)) == 0 && slots <= kHalf;
    }
};

/// The stamp a slot of the 64-bit row holds while the entry `n` is being written
/// into it.
constexpr std::uint64_t busy(std::uint64_t n) noexcept {
    return StampArith<64>::busy(n);
}

/// The stamp a slot of the 64-bit row holds once the entry `n` is in it.
constexpr std::uint64_t done(std::uint64_t n) noexcept {
    return StampArith<64>::done(n);
}

/// Whether the entry `n` is past what the 64-bit stamps can number. `publish`
/// publishes nothing for such an entry.
constexpr bool past_horizon(std::uint64_t n) noexcept {
    return n >= kHorizon;
}

}  // namespace detail

/// The 64-bit lane: stamps, head and words are `std::atomic<std::uint64_t>`, and
/// the count never wraps (the publisher stops at `kHorizon`). It is the default
/// lane of `BroadcastLog`, and the row of a target with lock-free 64-bit atomics.
///
/// A lane states, for `BroadcastLog` to read: `Raw`, the type a cell holds and a
/// word of an entry is; `Cell`, the atomic of it; `kBits`, the stamp's width;
/// `kUnwritten`, the stamp of a slot nothing has been written to, which no write
/// into the last slot produces; and `kLockFree`, whether the cell is lock-free.
struct Wide {
    using Raw = std::uint64_t;
    using Cell = std::atomic<Raw>;
    static constexpr unsigned kBits = 64;
    // Neither stamp the protocol writes is 0: `busy(n)` is odd and `done(n)` is at
    // least 2.
    static constexpr Raw kUnwritten = 0;
    static constexpr bool kLockFree = Cell::is_always_lock_free;
};

/// What a read found.
enum class ReadKind : std::uint8_t {
    /// The entry `seq` was copied into the caller's buffer, and the reader now
    /// holds `seq + 1`.
    Item,
    /// The reader is at the head: nothing newer is published. Its cursor is
    /// unchanged.
    Empty,
    /// The reader had fallen behind the window and missed `skipped >= 1` entries.
    /// It now holds the oldest retained entry, so its next read returns that. The
    /// caller's buffer is unspecified.
    Lagged,
};

struct Read {
    ReadKind kind;
    /// The sequence number of the entry copied; meaningful for `Item`. A count
    /// modulo `2^w` in a narrow row, the count itself in the 64-bit one.
    std::uint64_t seq;
    /// How many entries the reader missed; meaningful for `Lagged`.
    std::uint64_t skipped;

    static constexpr Read item(std::uint64_t seq) noexcept {
        return Read{ReadKind::Item, seq, 0};
    }

    static constexpr Read empty() noexcept {
        return Read{ReadKind::Empty, 0, 0};
    }

    static constexpr Read lagged(std::uint64_t skipped) noexcept {
        return Read{ReadKind::Lagged, 0, skipped};
    }

    friend constexpr bool operator==(const Read &a, const Read &b) noexcept {
        return a.kind == b.kind && a.seq == b.seq && a.skipped == b.skipped;
    }

    friend constexpr bool operator!=(const Read &a, const Read &b) noexcept {
        return !(a == b);
    }
};

template <std::size_t W, std::size_t Slots, class L = Wide> class BroadcastLog;

/// A reader's position, owned by the reader and shared with nothing.
///
/// Opening a reader costs the log nothing: the publisher does not know how many
/// readers exist, which is what keeps publishing wait-free.
class Reader {
public:
    /// The sequence number this reader reads next, modulo `2^w` in a narrow row. A
    /// deferred reader that has not read yet has none: this is 0 and `is_bound` is
    /// false.
    constexpr std::uint64_t cursor() const noexcept {
        return cursor_;
    }

    /// Whether the reader has a cursor: false only for a deferred reader that has
    /// not read yet.
    constexpr bool is_bound() const noexcept {
        return !unbound_;
    }

    /// The entries this reader has been told it missed, summed over its reads.
    constexpr std::uint64_t lost() const noexcept {
        return lost_;
    }

private:
    template <std::size_t W, std::size_t Slots, class L> friend class BroadcastLog;

    constexpr explicit Reader(std::uint64_t cursor) noexcept : cursor_(cursor), lost_(0), unbound_(false) {}

    /// A reader with no cursor: its first `read` binds one.
    constexpr Reader() noexcept : cursor_(0), lost_(0), unbound_(true) {}

    std::uint64_t cursor_;
    std::uint64_t lost_;
    /// Set on a reader opened by `reader_deferred`, until its first read.
    bool unbound_;
};

/// A bounded log whose entries are `W` words of the lane `L`, kept in `Slots` slots
/// and retaining the last `Slots - 1` entries.
///
/// The lane is `Wide` (64-bit words and stamps) unless a row names another. A lane
/// narrower than 64 bits wraps its stamps, and two obligations follow that this
/// class checks where a log is built: the slot count is a power of two, so that the
/// slot index `n mod Slots`, which a reader knows only from a count modulo `2^w`, is
/// the publisher's; and it is at most half the stamp space, so that a stamp tells
/// apart more than one lap of a slot. What a deploy states for the pause it relies
/// on is the generator's to judge (ADR 0062 section 4).
template <std::size_t W, std::size_t Slots, class L> class BroadcastLog {
    static_assert(W >= 1, "an entry holds at least one word");
    static_assert(Slots >= 2, "a log retains at least one entry, so it has at least two slots");
    static_assert(L::kLockFree, "this lane's stamps and words need lock-free atomics, which the target does not have");
    static_assert(
        L::kBits >= 64 || detail::StampArith<L::kBits>::slots_fit(Slots),
        "a narrow stamp wraps, and the slot index n mod Slots stays continuous across the wrap only if Slots "
        "divides the stamp space, so Slots is a power of two; and a log keeps at most half the stamp space in "
        "slots, so that a stamp tells apart more than one lap of a slot");

    /// The arithmetic of this lane's stamp width, and the types it keeps.
    using Arith = detail::StampArith<L::kBits>;
    using Raw = typename L::Raw;
    using Cell = typename L::Cell;

public:
    /// The entries a reader may rely on.
    static constexpr std::uint64_t kRetention = Slots - 1;

    /// One entry: `W` words.
    using Entry = std::array<typename L::Raw, W>;

    constexpr BroadcastLog() noexcept : head_(0) {}

    BroadcastLog(const BroadcastLog &) = delete;
    BroadcastLog &operator=(const BroadcastLog &) = delete;

private:
    /// Keeps `Publisher` constructible only through the log.
    struct Token {
        explicit Token() = default;
    };

    struct Slot {
        Cell stamp{L::kUnwritten};
        Cell words[W]{};
    };

public:
    /// The publishing side. There is one at a time; it holds the log's one
    /// publisher place until it is destroyed.
    class Publisher {
    public:
        Publisher(Token, BroadcastLog &log) noexcept
            : log_(&log), next_(log.head_.value.load(std::memory_order_acquire)) {}

        Publisher(Publisher &&other) noexcept : log_(other.log_), next_(other.next_) {
            other.log_ = nullptr;
        }

        Publisher &operator=(Publisher &&) = delete;
        Publisher(const Publisher &) = delete;
        Publisher &operator=(const Publisher &) = delete;

        ~Publisher() {
            if (log_ != nullptr) {
                log_->publisher_taken_.store(false, std::memory_order_release);
            }
        }

        /// Publish `entry` and return the sequence number it took (modulo `2^w` in
        /// a narrow row). Never fails and never waits, except that a log of the
        /// 64-bit row which has published its horizon of entries publishes nothing
        /// more and returns `kNoSequence`; a narrow row has no horizon.
        std::uint64_t publish(const Entry &entry) noexcept {
            BroadcastLog &log = *log_;
            const std::uint64_t n = next_;
            if constexpr (L::kBits >= 64) {
                if (detail::past_horizon(n)) {
                    return kNoSequence;
                }
            }
            Slot &slot = log.slot_of(n);
            // Release, for the reader that sees this stamp and then reloads
            // `head`: it must find `head` at `n` or later, and the store of `head`
            // to `n` ended the previous publish, in this thread's order before
            // this one. A relaxed store here lets that reload return an older
            // `head`, and the reader then reports a lag that skipped nothing. The
            // fence below is still needed: a release store orders what came
            // before it, and this stamp must also come before the words.
            slot.stamp.store(static_cast<Raw>(Arith::busy(n)), std::memory_order_release);
            std::atomic_thread_fence(std::memory_order_release);
            for (std::size_t k = 0; k < W; ++k) {
                slot.words[k].store(entry[k], std::memory_order_relaxed);
            }
            slot.stamp.store(static_cast<Raw>(Arith::done(n)), std::memory_order_release);
            // The linearization point: a reader that loads this index (acquire)
            // sees the slot's stamp and words.
            const std::uint64_t next = Arith::advance(n, 1);
            log.head_.value.store(static_cast<Raw>(next), std::memory_order_release);
            next_ = next;
            return n;
        }

    private:
        BroadcastLog *log_;
        /// The sequence number of the next entry, modulo `2^w`: this side's own
        /// count, so publishing reads no shared state.
        std::uint64_t next_;
    };

    /// The number of entries published so far: modulo `2^w` in a narrow row, the
    /// count itself in the 64-bit one.
    std::uint64_t head() const noexcept {
        return head_.value.load(std::memory_order_acquire);
    }

    /// The publisher, or `std::nullopt` while another exists: a log written by two
    /// publishers is not this algorithm. Destroying it frees the place, and its
    /// successor continues the sequence. It takes the place with an exchange, which
    /// a target with no read-modify-write instruction does not have: such a target
    /// does not call this (a member of a class template is compiled only where it
    /// is used) and takes the place by `split` or `publisher_unchecked`.
    std::optional<Publisher> publisher() noexcept {
        if (publisher_taken_.exchange(true, std::memory_order_acquire)) {
            return std::nullopt;
        }
        return std::optional<Publisher>(std::in_place, Token{}, *this);
    }

    /// The publisher's place, taken from a log the caller holds exclusively (before
    /// it is shared with another thread) with a load and a store and no
    /// read-modify-write, or `std::nullopt` while a place is held. The caller
    /// guarantees that no other thread is taking the place, or reading it, at this
    /// moment: the check is not atomic with the store. The place is then one with
    /// `publisher`'s, and the publisher continues the sequence.
    std::optional<Publisher> split() noexcept {
        if (publisher_taken_.load(std::memory_order_acquire)) {
            return std::nullopt;
        }
        publisher_taken_.store(true, std::memory_order_release);
        return std::optional<Publisher>(std::in_place, Token{}, *this);
    }

    /// The publisher of a log that is shared from the start (a log in static
    /// storage), taken on the caller's word. The caller guarantees, for as long as
    /// the publisher exists:
    ///   - no other publisher of this log exists, whether from this function, from
    ///     `split` or from `publisher`: two publishers write the same slots and
    ///     break the stamp protocol silently, with readers told entries that were
    ///     never published or lags that skipped nothing;
    ///   - the last publish of any earlier publisher of this log happens before
    ///     this call (program order, a thread join, or a release store the caller
    ///     acquires), so that the sequence this publisher continues from `head` is
    ///     the one that was published.
    /// The place is marked taken, so a later `publisher` or `split` refuses.
    Publisher publisher_unchecked() noexcept {
        publisher_taken_.store(true, std::memory_order_release);
        return Publisher(Token{}, *this);
    }

    /// A reader at the oldest retained entry.
    ///
    /// The oldest retained entry is `max(0, h - kRetention)`. A 64-bit count does
    /// not wrap, so that is the head less the retention, no lower than 0: one load.
    /// A count modulo `2^w` cannot tell a log that has published fewer than
    /// `kRetention` entries from one that has published more and wrapped, so a
    /// narrow log says it itself: the last slot is first written by the publish of
    /// entry `kRetention`, and still holds `L::kUnwritten` before it. If it does,
    /// `h <= kRetention` when it was loaded and the oldest retained is entry 0,
    /// whose cursor is 0. If it does not, the publish of entry `kRetention` began,
    /// so `h >= kRetention`, and the oldest is `h - kRetention` of the head loaded
    /// after.
    Reader reader_oldest() const noexcept {
        if constexpr (L::kBits >= 64) {
            const std::uint64_t h = head();
            return Reader(h > kRetention ? h - kRetention : 0);
        } else {
            const bool untouched = last_slot_untouched();
            return Reader(untouched ? 0 : Arith::distance(head(), kRetention));
        }
    }

    /// A reader whose start is the oldest retained entry at its first read, and
    /// not at this call: it has no cursor until that read binds one, from the head
    /// that read loads, so a reader that waited is told of nothing the publisher
    /// did meanwhile, where `reader_oldest` would tell it of the lag. Opening it
    /// loads nothing and costs the log nothing.
    ///
    /// The first read binds as `reader_oldest` does, its loads being part of the
    /// read, and then goes on as any read does: the answer is an item, `Empty` for
    /// a log with nothing published, or `Lagged` if the publisher overtook the
    /// cursor inside that read, which takes the slot beyond the retention (ADR 0062
    /// section 2). In a narrow row the bind is two loads, the stamp of the last
    /// slot and the head, and the deploy's pause bound covers them as it covers the
    /// loads of a read, counted from the first of them.
    Reader reader_deferred() const noexcept {
        return Reader();
    }

    /// A reader at the head: it sees only what is published after this call.
    Reader reader_head() const noexcept {
        return Reader(head());
    }

    /// A reader at the sequence number `seq`, or at the head when `seq` is past
    /// it.
    ///
    /// In the 64-bit row `seq` is a count and is clamped to the head. A narrow log
    /// knows a position only modulo `2^w`, so `seq` is reduced to it and read as an
    /// entry the caller got from this log (a publish's return or an item's `seq`):
    /// fewer than `K = 2^(w-1)` entries behind the head it is taken as it is, and
    /// any other position (`K` or more behind, which is what one ahead of the head
    /// looks like) is the head.
    Reader reader_at(std::uint64_t seq) const noexcept {
        const std::uint64_t h = head();
        if constexpr (L::kBits >= 64) {
            return Reader(seq < h ? seq : h);
        } else {
            const std::uint64_t residue = seq & Arith::kMask;
            return Reader(Arith::distance(h, residue) < Arith::kHalf ? residue : h);
        }
    }

    /// Read the entry at `reader`'s cursor into `out`. A bounded number of steps
    /// whatever the publisher is doing: a copy that the publisher overwrote is not
    /// retried, it is the answer (`Lagged`). `out` is unspecified unless the
    /// answer is `Item`.
    ///
    /// Outside the bound. The answer is what ADR 0062 section 4 proves while the
    /// reader is less than `K = 2^(w-1)` publishes behind (counting the one in
    /// flight) during the call. A narrow reader cannot know it has broken that
    /// hypothesis; what it does when it has is defined, not undefined, and is the
    /// arithmetic of its cursor modulo `2^w`: its cursor is always right modulo
    /// `2^w` (`Empty` leaves it at the head's residue, `Lagged` at the oldest
    /// retained entry's); `skipped` is the distance taken modulo `2^w` less the
    /// retention, so it is short by a multiple of `2^w` when the reader was that
    /// many laps behind, and a reader exactly `2^w` behind is told `Empty`; and a
    /// copy that validated while the reader was `K` or more behind is the entry a
    /// later lap wrote, handed as the one wanted. The deploy states the bound it
    /// relies on, and the generator refuses a log whose bound does not cover it.
    Read read(Reader &reader, Entry &out) const noexcept {
        // The first read of a deferred reader binds its cursor at the head it loads
        // here, and then goes on as every read does.
        std::uint64_t h = 0;
        if (reader.unbound_) {
            h = bind(reader);
        } else {
            h = head_.value.load(std::memory_order_acquire);
        }
        const std::uint64_t r = reader.cursor_;
        const std::uint64_t behind = Arith::distance(h, r);
        if (behind == 0) {
            return Read::empty();
        }
        if (behind >= Slots) {
            return lagged(reader, behind);
        }
        // `r` was in the window when `head` was loaded, so its slot was filled
        // before that load. It holds `r` still unless the publisher has since
        // begun writing `r + Slots`.
        const Slot &slot = slot_of(r);
        const Raw first = slot.stamp.load(std::memory_order_acquire);
        if (first == Arith::done(r)) {
            for (std::size_t k = 0; k < W; ++k) {
                out[k] = slot.words[k].load(std::memory_order_relaxed);
            }
            // The fence keeps the copy before the second load, which is what makes
            // an unchanged stamp mean an unchanged copy. The load is an acquire
            // for the other reason: if it sees the stamp of a later lap, the
            // reload of `head` below must see the publish that began that lap. A
            // fence orders a later load only against reads before it, so this
            // load cannot rely on the fence for that, and a relaxed one reports a
            // lag that skipped nothing.
            std::atomic_thread_fence(std::memory_order_acquire);
            if (slot.stamp.load(std::memory_order_acquire) == first) {
                reader.cursor_ = Arith::advance(r, 1);
                return Read::item(r);
            }
        }
        // The slot was reused while it was being read, so the publisher has
        // published through `r + kRetention` and `head` shows it.
        const std::uint64_t reloaded = head_.value.load(std::memory_order_acquire);
        return lagged(reader, Arith::distance(reloaded, r));
    }

private:
    /// The slot that holds sequence number `seq`: the index is the publisher's
    /// whatever the lap, because the slot count divides the stamp space in a narrow
    /// row.
    Slot &slot_of(std::uint64_t seq) noexcept {
        return slots_[static_cast<std::size_t>(seq % Slots)];
    }

    const Slot &slot_of(std::uint64_t seq) const noexcept {
        return slots_[static_cast<std::size_t>(seq % Slots)];
    }

    /// Whether the last slot still holds `L::kUnwritten`, which only a narrow log
    /// asks (see `reader_oldest`).
    bool last_slot_untouched() const noexcept {
        return slots_[Slots - 1].stamp.load(std::memory_order_acquire) == L::kUnwritten;
    }

    /// Bind a deferred reader's cursor to the oldest retained entry, as
    /// `reader_oldest` does, and return the head the cursor was taken from, which is
    /// the load the read goes on with. A 64-bit count does not wrap, so one load of
    /// the head is all it needs; a narrow count asks the last slot first, then loads
    /// the head (the order the models have: `RDStart`, `RDHead`).
    std::uint64_t bind(Reader &reader) const noexcept {
        if constexpr (L::kBits >= 64) {
            const std::uint64_t h = head_.value.load(std::memory_order_acquire);
            reader.cursor_ = h > kRetention ? h - kRetention : 0;
            reader.unbound_ = false;
            return h;
        } else {
            const bool untouched = last_slot_untouched();
            const std::uint64_t h = head_.value.load(std::memory_order_acquire);
            reader.cursor_ = untouched ? 0 : Arith::distance(h, kRetention);
            reader.unbound_ = false;
            return h;
        }
    }

    /// Move `reader` to the oldest entry retained and say how far that was, from
    /// `behind`, how far behind the head the reader found itself.
    static Read lagged(Reader &reader, std::uint64_t behind) noexcept {
        // The count is the one `Arith::missed` gives: a `behind` below `Slots` came
        // from a head that has wrapped, and has a lap added. The cursor lands on the
        // oldest retained entry's residue whatever the number of laps; only the count
        // is then short.
        const std::uint64_t skipped = Arith::missed(behind, Slots, kRetention);
        reader.cursor_ = Arith::advance(reader.cursor_, skipped);
        reader.lost_ = skipped > UINT64_MAX - reader.lost_ ? UINT64_MAX : reader.lost_ + skipped;
        return Read::lagged(skipped);
    }

    /// The number of entries published, modulo `2^w`. Written only by the
    /// publisher, loaded by every reader, so it has a cache line of its own.
    ::SCE::Forge::Queue::detail::Padded<Cell> head_;
    /// Whether a `Publisher` exists.
    std::atomic<bool> publisher_taken_{false};
    Slot slots_[Slots];
};

}  // namespace SCE::Forge::Broadcast
