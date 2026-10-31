// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce/forge/broadcast_log.h — the runtime half of `sce:kind="broadcast-log"`
// (SCE Protocol-Synthesis RFC §synth-5-S, ADR 0062) for C++: one publisher, any number of readers, and the oldest entry
// forgotten when the log is full. The same algorithm as the Rust runtime
// (backends/rust/forge-runtime/src/broadcast_log.rs), with the same memory orders
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
// The stamp is a 64-bit sequence number and not a flag, so a reader comparing
// against `done(r)` cannot mistake a later lap for the entry it wants. It stays
// below `kHorizon`, 292 years at a billion publishes per second. A target without
// lock-free 64-bit atomics has no broadcast log, as it has no 64-bit SCQ: the
// header refuses to compile there.
//
// No heap, no threads, no global state: a log owns its storage, and where it
// lives is the caller's choice. The constructor is
// `constexpr`, so a log in static storage is constant-initialised and an interrupt
// handler may use it before any dynamic initialisation has run. An entry is `W`
// 64-bit words, the wire form of the element (ADR 0049/0058) and never a value
// with ownership, so an entry has no destructor to run when it is overwritten.
//
// Interface, the same on every backend:
//
//   publisher()                  the one publisher, or std::nullopt while another
//                                exists. It gives its place back when it is
//                                destroyed, and its successor continues the
//                                sequence.
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

/// The most entries a log can publish: the stamps are `2n + 2`, which must fit a
/// `std::uint64_t`. Past it `publish` publishes nothing and returns
/// `kNoSequence`.
inline constexpr std::uint64_t kHorizon = (UINT64_MAX - 2) / 2;

/// What `publish` returns when the log has published its horizon of entries. It
/// is not a sequence number: every sequence number is below the horizon.
inline constexpr std::uint64_t kNoSequence = UINT64_MAX;

namespace detail {

/// The stamp a slot holds while the entry `n` is being written into it.
constexpr std::uint64_t busy(std::uint64_t n) noexcept {
    return 2 * n + 1;
}

/// The stamp a slot holds once the entry `n` is in it.
constexpr std::uint64_t done(std::uint64_t n) noexcept {
    return 2 * n + 2;
}

/// Whether the entry `n` is past what the stamps can number. `publish` publishes
/// nothing for such an entry.
constexpr bool past_horizon(std::uint64_t n) noexcept {
    return n >= kHorizon;
}

}  // namespace detail

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
    /// The sequence number of the entry copied; meaningful for `Item`.
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

template <std::size_t W, std::size_t Slots> class BroadcastLog;

/// A reader's position, owned by the reader and shared with nothing.
///
/// Opening a reader costs the log nothing: the publisher does not know how many
/// readers exist, which is what keeps publishing wait-free.
class Reader {
public:
    /// The sequence number this reader reads next. A deferred reader that has not
    /// read yet has none: this is 0 and `is_bound` is false.
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
    template <std::size_t W, std::size_t Slots> friend class BroadcastLog;

    constexpr explicit Reader(std::uint64_t cursor) noexcept : cursor_(cursor), lost_(0), unbound_(false) {}

    /// A reader with no cursor: its first `read` binds one.
    constexpr Reader() noexcept : cursor_(0), lost_(0), unbound_(true) {}

    std::uint64_t cursor_;
    std::uint64_t lost_;
    /// Set on a reader opened by `reader_deferred`, until its first read.
    bool unbound_;
};

/// A bounded log whose entries are `W` 64-bit words, kept in `Slots` slots and
/// retaining the last `Slots - 1` entries.
template <std::size_t W, std::size_t Slots> class BroadcastLog {
    static_assert(W >= 1, "an entry holds at least one word");
    static_assert(Slots >= 2, "a log retains at least one entry, so it has at least two slots");
    static_assert(std::atomic<std::uint64_t>::is_always_lock_free,
                  "the stamp is a 64-bit sequence number, which needs lock-free 64-bit atomics");

public:
    /// The entries a reader may rely on.
    static constexpr std::uint64_t kRetention = Slots - 1;

    /// One entry: `W` words.
    using Entry = std::array<std::uint64_t, W>;

    constexpr BroadcastLog() noexcept : head_(0) {}

    BroadcastLog(const BroadcastLog &) = delete;
    BroadcastLog &operator=(const BroadcastLog &) = delete;

private:
    /// Keeps `Publisher` constructible only through the log.
    struct Token {
        explicit Token() = default;
    };

    struct Slot {
        std::atomic<std::uint64_t> stamp{0};
        std::atomic<std::uint64_t> words[W]{};
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

        /// Publish `entry` and return the sequence number it took. Never fails
        /// and never waits, except that a log which has published its horizon of
        /// entries publishes nothing more and returns `kNoSequence`.
        std::uint64_t publish(const Entry &entry) noexcept {
            BroadcastLog &log = *log_;
            const std::uint64_t n = next_;
            if (detail::past_horizon(n)) {
                return kNoSequence;
            }
            Slot &slot = log.slot_of(n);
            // Release, for the reader that sees this stamp and then reloads
            // `head`: it must find `head` at `n` or later, and the store of `head`
            // to `n` ended the previous publish, in this thread's order before
            // this one. A relaxed store here lets that reload return an older
            // `head`, and the reader then reports a lag that skipped nothing. The
            // fence below is still needed: a release store orders what came
            // before it, and this stamp must also come before the words.
            slot.stamp.store(detail::busy(n), std::memory_order_release);
            std::atomic_thread_fence(std::memory_order_release);
            for (std::size_t k = 0; k < W; ++k) {
                slot.words[k].store(entry[k], std::memory_order_relaxed);
            }
            slot.stamp.store(detail::done(n), std::memory_order_release);
            // The linearization point: a reader that loads this index (acquire)
            // sees the slot's stamp and words.
            log.head_.value.store(n + 1, std::memory_order_release);
            next_ = n + 1;
            return n;
        }

    private:
        BroadcastLog *log_;
        /// The sequence number of the next entry: this side's own count, so
        /// publishing reads no shared state.
        std::uint64_t next_;
    };

    /// The number of entries published so far.
    std::uint64_t head() const noexcept {
        return head_.value.load(std::memory_order_acquire);
    }

    /// The publisher, or `std::nullopt` while another exists: a log written by two
    /// publishers is not this algorithm. Destroying it frees the place, and its
    /// successor continues the sequence.
    std::optional<Publisher> publisher() noexcept {
        if (publisher_taken_.exchange(true, std::memory_order_acquire)) {
            return std::nullopt;
        }
        return std::optional<Publisher>(std::in_place, Token{}, *this);
    }

    /// A reader at the oldest retained entry.
    Reader reader_oldest() const noexcept {
        const std::uint64_t h = head();
        return Reader(h > kRetention ? h - kRetention : 0);
    }

    /// A reader whose start is the oldest retained entry at its first read, and
    /// not at this call: it has no cursor until that read binds one, from the head
    /// that read loads, so a reader that waited is told of nothing the publisher
    /// did meanwhile, where `reader_oldest` would tell it of the lag. Opening it
    /// loads nothing and costs the log nothing.
    ///
    /// The first read binds as `reader_oldest` does, its load of `head` being the
    /// read's own, and then goes on as any read does: the answer is an item,
    /// `Empty` for a log with nothing published, or `Lagged` if the publisher
    /// overtook the cursor inside that read, which takes the slot beyond the
    /// retention (ADR 0062 section 2).
    Reader reader_deferred() const noexcept {
        return Reader();
    }

    /// A reader at the head: it sees only what is published after this call.
    Reader reader_head() const noexcept {
        return Reader(head());
    }

    /// A reader at the sequence number `seq`, or at the head when `seq` is past
    /// it.
    Reader reader_at(std::uint64_t seq) const noexcept {
        const std::uint64_t h = head();
        return Reader(seq < h ? seq : h);
    }

    /// Read the entry at `reader`'s cursor into `out`. A bounded number of steps
    /// whatever the publisher is doing: a copy that the publisher overwrote is not
    /// retried, it is the answer (`Lagged`). `out` is unspecified unless the
    /// answer is `Item`.
    Read read(Reader &reader, Entry &out) const noexcept {
        const std::uint64_t h = head_.value.load(std::memory_order_acquire);
        // The first read of a deferred reader binds its cursor at this head, the one
        // load of the read, and then goes on as every read does: a 64-bit count does
        // not wrap, so the oldest retained entry is at most `kRetention` behind it
        // and the slot is inside the window.
        if (reader.unbound_) {
            reader.cursor_ = h > kRetention ? h - kRetention : 0;
            reader.unbound_ = false;
        }
        const std::uint64_t r = reader.cursor_;
        if (r >= h) {
            return Read::empty();
        }
        if (h - r >= Slots) {
            return lagged(reader, h);
        }
        // `r` was in the window when `head` was loaded, so its slot was filled
        // before that load. It holds `r` still unless the publisher has since
        // begun writing `r + Slots`.
        const Slot &slot = slot_of(r);
        const std::uint64_t first = slot.stamp.load(std::memory_order_acquire);
        if (first == detail::done(r)) {
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
                reader.cursor_ = r + 1;
                return Read::item(r);
            }
        }
        // The slot was reused while it was being read, so the publisher has
        // published through `r + kRetention` and `head` shows it.
        return lagged(reader, head_.value.load(std::memory_order_acquire));
    }

private:
    /// The slot that holds sequence number `seq`.
    Slot &slot_of(std::uint64_t seq) noexcept {
        return slots_[static_cast<std::size_t>(seq % Slots)];
    }

    const Slot &slot_of(std::uint64_t seq) const noexcept {
        return slots_[static_cast<std::size_t>(seq % Slots)];
    }

    /// Move `reader` to the oldest entry retained when the head is `h`, and say
    /// how far that was.
    static Read lagged(Reader &reader, std::uint64_t h) noexcept {
        const std::uint64_t oldest = h > kRetention ? h - kRetention : 0;
        const std::uint64_t skipped = oldest > reader.cursor_ ? oldest - reader.cursor_ : 0;
        reader.cursor_ = oldest;
        reader.lost_ = skipped > UINT64_MAX - reader.lost_ ? UINT64_MAX : reader.lost_ + skipped;
        return Read::lagged(skipped);
    }

    /// The number of entries published. Written only by the publisher, loaded by
    /// every reader, so it has a cache line of its own.
    ::SCE::Forge::Queue::detail::Padded<std::atomic<std::uint64_t>> head_;
    /// Whether a `Publisher` exists.
    std::atomic<bool> publisher_taken_{false};
    Slot slots_[Slots];
};

}  // namespace SCE::Forge::Broadcast
