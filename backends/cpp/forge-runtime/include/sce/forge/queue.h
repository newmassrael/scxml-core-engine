// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce/forge/queue.h — the runtime half of `sce:kind="queue"` (SCE
// Protocol-Synthesis RFC §synth-5-P) for C++. The same two algorithms as the
// Rust runtime (backends/rust/forge-runtime/src/queue), with the same memory
// orderings, because the ordering is part of each algorithm and not a choice:
//
//   Spsc<T, N>      the `bounded` row for one producer and one consumer: a
//                   Lamport ring, wait-free on both sides.
//   Scq<T, N, R>    the `bounded` row for any other cardinality: Nikolaev's
//                   SCQ data queue (DISC 2019), lock-free on both sides, in
//                   the form of the authors' single-width-CAS reference
//                   implementation (lfring_cas1.h, dual 2-clause BSD / MIT).
//
// What both keep: a linearizable FIFO in which the push of an element
// happens-before the pop that returns it, never more than N elements held and
// exactly N when nothing is running. The SCQ row may refuse a push while
// another participant's operation holds a slot (at most one per participant);
// the Lamport row never does.
//
// No heap, no threads, no global state (SCE_FORGE.md §2.1): a queue owns its
// storage, and where it lives is the caller's choice. The constructors are
// `constexpr`, so a queue in static storage is constant-initialised and an
// interrupt handler may use it before any dynamic initialisation has run.
//
// Interface, the same on every backend:
//
//   producer() / consumer()      a handle, or std::nullopt when every place of
//                                that side is taken. A side declared `one` has
//                                one place; an SCQ side has R. A handle gives
//                                its place back when it is destroyed.
//   try_push(T&&) -> PushStatus  Ok moves the element in. Full (and, for
//                                the `segmented` row, OutOfMemory) leaves the
//                                argument exactly as it was, which is how the
//                                element "comes back".
//   try_pop() -> std::optional<T>
//   capacity()                   a compile-time constant.
//
// The element type must be nothrow-move-constructible and nothrow-
// destructible: a throwing move between taking a slot's index and publishing
// the element would lose the index for good. Elements still queued when the
// queue is destroyed are destroyed with it.
//
// Memory order, in one place. A producer publishes a filled slot with a
// release (the Lamport row's store of its index; the SCQ row's compare-and-
// swap that puts the slot's index on the allocated ring) and a consumer takes
// it with an acquire on the same location before it reads the slot, so the
// write of an element happens-before its pop. The slot goes back the same
// way in the other direction, so the pop's read happens-before the slot's
// reuse. Rust's AcqRel / Acquire / SeqCst map to std::memory_order_acq_rel /
// acquire / seq_cst one for one.

#pragma once

#include <atomic>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <new>
#include <optional>
#include <type_traits>
#include <utility>

namespace SCE::Forge::Queue {

/// Why a push did not take its element. `OutOfMemory` is the `segmented`
/// storage mode's, reported when its injected allocator refuses a segment; a
/// `bounded` queue only ever reports `Full`.
enum class PushStatus : std::uint8_t { Ok, Full, OutOfMemory };

/// What an operation guarantees about how long it takes whatever the other
/// participants are doing, ordered by strength: a stronger guarantee compares
/// greater, so "at least `LockFree`" is `>= Progress::LockFree`. The `segmented`
/// storage mode holds its injected allocator to the progress the document
/// declared for it (`queue_segmented.h`).
enum class Progress : std::uint8_t { Blocking = 0, LockFree = 1, WaitFree = 2 };

template <typename T, std::size_t N, std::size_t R, std::size_t H, typename A, Progress Required> class Lscq;

namespace detail {

/// Keeps what it wraps on a cache line of its own, so two indices that two
/// cores update do not contend for one. It costs a cache line each and
/// changes nothing about correctness.
inline constexpr std::size_t kCacheLine = 64;

template <typename Atomic> struct alignas(kCacheLine) Padded {
    Atomic value;

    constexpr explicit Padded(typename Atomic::value_type initial) noexcept : value(initial) {}
};

/// The storage of one element. A union, so the slot holds no object until a
/// push constructs one in it and has a `constexpr` constructor, which is what
/// lets a queue be constant-initialised.
template <typename T> union Slot {
    char unused_;
    T value_;

    constexpr Slot() noexcept : unused_() {}

    // The queue destroys the element a slot holds, if any, before the slot
    // goes; the slot itself has nothing to do.
    ~Slot() {}

    Slot(const Slot &) = delete;
    Slot &operator=(const Slot &) = delete;
};

/// Put an element back into the object it was moved out of. `destination` was
/// moved from, so its object is destroyed and a new one is built in its place
/// from `source`; the element type is only required to be nothrow-move-
/// constructible, not assignable. A refused push uses it to leave the caller's
/// argument holding the element it passed, as the contract says.
template <typename T> void give_back(T &destination, T &&source) noexcept {
    destination.~T();
    ::new (static_cast<void *>(std::addressof(destination))) T(std::move(source));
}

/// Take one of `limit` places of a side, if one is free. A compare-and-swap
/// loop and not a fetch_add, so a refused request leaves the count as it was
/// and no concurrent request can be refused on account of it.
inline bool take_place(std::atomic<std::size_t> &alive, std::size_t limit) noexcept {
    std::size_t seen = alive.load(std::memory_order_relaxed);
    for (;;) {
        if (seen >= limit) {
            return false;
        }
        if (alive.compare_exchange_weak(seen, seen + 1, std::memory_order_acq_rel, std::memory_order_relaxed)) {
            return true;
        }
    }
}

}  // namespace detail

// ───────────────────────────── Lamport ring ─────────────────────────────

/// A bounded queue of exactly `N` elements for one producer and one consumer.
///
/// Lap indices, not counters. Each side's index runs over `0..2N` and the slot
/// it names is the index modulo `N`. Two laps are what tell a full ring from
/// an empty one without spending a slot: the indices are equal exactly when
/// the ring is empty and `N` apart exactly when it is full, so the ring holds
/// exactly the capacity declared. An index never exceeds `2N`, so nothing
/// wraps however long the queue runs.
template <typename T, std::size_t N> class Spsc {
    static_assert(N > 0, "a queue's capacity must be at least one element");
    static_assert(N <= SIZE_MAX / 2, "a queue's lap indices run over twice its capacity, which must fit in a size_t");
    static_assert(std::is_nothrow_move_constructible_v<T> && std::is_nothrow_destructible_v<T>,
                  "a queue's element must be nothrow-move-constructible and nothrow-destructible");

    /// The size of the lap-index space.
    static constexpr std::size_t kLaps = 2 * N;

public:
    /// The capacity this queue holds, exactly.
    static constexpr std::size_t kCapacity = N;

    constexpr Spsc() noexcept : head_(0), tail_(0) {}

    Spsc(const Spsc &) = delete;
    Spsc &operator=(const Spsc &) = delete;

    ~Spsc() {
        std::size_t head = head_.value.load(std::memory_order_acquire);
        const std::size_t tail = tail_.value.load(std::memory_order_acquire);
        while (head != tail) {
            slot(head).value_.~T();
            head = next(head);
        }
    }

    constexpr std::size_t capacity() const noexcept {
        return N;
    }

private:
    /// Keeps `Producer` and `Consumer` constructible only through the queue.
    struct Token {
        explicit Token() = default;
    };

public:
    /// The producing side. There is one at a time; it holds the queue's one
    /// producer place until it is destroyed.
    class Producer {
    public:
        Producer(Token, Spsc &queue) noexcept
            : queue_(&queue), tail_(queue.tail_.value.load(std::memory_order_acquire)),
              cached_head_(queue.head_.value.load(std::memory_order_acquire)) {}

        Producer(Producer &&other) noexcept
            : queue_(other.queue_), tail_(other.tail_), cached_head_(other.cached_head_) {
            other.queue_ = nullptr;
        }

        Producer &operator=(Producer &&) = delete;
        Producer(const Producer &) = delete;
        Producer &operator=(const Producer &) = delete;

        ~Producer() {
            if (queue_ != nullptr) {
                queue_->producer_claimed_.store(false, std::memory_order_release);
            }
        }

        /// Push `value`, or leave it untouched and report `Full` when the
        /// queue holds its capacity. Wait-free: a bounded number of steps
        /// whatever the consumer is doing.
        PushStatus try_push(T &&value) noexcept {
            Spsc &queue = *queue_;
            if (occupancy(cached_head_, tail_) == N) {
                // Acquire pairs with the consumer's release store of its
                // index: its read of the slot about to be reused happens-
                // before the write below.
                cached_head_ = queue.head_.value.load(std::memory_order_acquire);
                if (occupancy(cached_head_, tail_) == N) {
                    return PushStatus::Full;
                }
            }
            // The slot at `tail_` is outside [head, tail), so the consumer
            // does not read it until the store below publishes it.
            ::new (static_cast<void *>(std::addressof(queue.slot(tail_).value_))) T(std::move(value));
            tail_ = next(tail_);
            // Release: the write of the slot happens-before the pop of any
            // consumer that acquires this index.
            queue.tail_.value.store(tail_, std::memory_order_release);
            return PushStatus::Ok;
        }

        constexpr std::size_t capacity() const noexcept {
            return N;
        }

    private:
        Spsc *queue_;
        /// This side's own index; the shared one only publishes it.
        std::size_t tail_;
        /// The consumer's index as this side last read it.
        std::size_t cached_head_;
    };

    /// The consuming side. There is one at a time.
    class Consumer {
    public:
        Consumer(Token, Spsc &queue) noexcept
            : queue_(&queue), head_(queue.head_.value.load(std::memory_order_acquire)),
              cached_tail_(queue.tail_.value.load(std::memory_order_acquire)) {}

        Consumer(Consumer &&other) noexcept
            : queue_(other.queue_), head_(other.head_), cached_tail_(other.cached_tail_) {
            other.queue_ = nullptr;
        }

        Consumer &operator=(Consumer &&) = delete;
        Consumer(const Consumer &) = delete;
        Consumer &operator=(const Consumer &) = delete;

        ~Consumer() {
            if (queue_ != nullptr) {
                queue_->consumer_claimed_.store(false, std::memory_order_release);
            }
        }

        /// Pop the oldest element, or `std::nullopt` when the queue is empty.
        /// Wait-free: a bounded number of steps whatever the producer is doing.
        std::optional<T> try_pop() noexcept {
            Spsc &queue = *queue_;
            if (head_ == cached_tail_) {
                // Acquire pairs with the producer's release store of its
                // index: its write of the slot happens-before the read below.
                cached_tail_ = queue.tail_.value.load(std::memory_order_acquire);
                if (head_ == cached_tail_) {
                    return std::nullopt;
                }
            }
            // The slot at `head_` is inside [head, tail), so it holds a pushed
            // element, and the producer does not reuse it until the store
            // below returns it.
            T &stored = queue.slot(head_).value_;
            std::optional<T> value(std::move(stored));
            stored.~T();
            head_ = next(head_);
            // Release: the read of the slot happens-before the producer's
            // reuse of it, by any producer that acquires this index.
            queue.head_.value.store(head_, std::memory_order_release);
            return value;
        }

        constexpr std::size_t capacity() const noexcept {
            return N;
        }

    private:
        Spsc *queue_;
        std::size_t head_;
        std::size_t cached_tail_;
    };

    /// The producer, or `std::nullopt` while another is alive: a ring shared
    /// by two producers is not this algorithm.
    std::optional<Producer> producer() noexcept {
        if (producer_claimed_.exchange(true, std::memory_order_acq_rel)) {
            return std::nullopt;
        }
        return std::optional<Producer>(std::in_place, Token{}, *this);
    }

    /// The consumer, or `std::nullopt` while another is alive.
    std::optional<Consumer> consumer() noexcept {
        if (consumer_claimed_.exchange(true, std::memory_order_acq_rel)) {
            return std::nullopt;
        }
        return std::optional<Consumer>(std::in_place, Token{}, *this);
    }

private:
    /// The slot a lap index names.
    detail::Slot<T> &slot(std::size_t index) noexcept {
        return slots_[index >= N ? index - N : index];
    }

    /// The lap index after `index`.
    static constexpr std::size_t next(std::size_t index) noexcept {
        return index + 1 == kLaps ? 0 : index + 1;
    }

    /// How many elements lie between a consumer index and a producer index.
    static constexpr std::size_t occupancy(std::size_t head, std::size_t tail) noexcept {
        return tail >= head ? tail - head : tail + kLaps - head;
    }

    /// Lap index of the next slot the consumer pops. Written only by the
    /// consumer.
    detail::Padded<std::atomic<std::size_t>> head_;
    /// Lap index of the next slot the producer fills. Written only by the
    /// producer.
    detail::Padded<std::atomic<std::size_t>> tail_;
    std::atomic<bool> producer_claimed_{false};
    std::atomic<bool> consumer_claimed_{false};
    detail::Slot<T> slots_[N];
};

// ─────────────────────────────── SCQ ring ───────────────────────────────

/// How many operations a participant may be delayed between reading an entry
/// and acting on it before it could be misled: 2^(w-2) for a word of w bits,
/// independent of the capacity. An entry's cycle is compared with a ticket's
/// modulo 2^c, where c = w - 1 - log2(2R), so a delay is safe until the
/// entry's cycle has moved by half that range, 2^(c-1); each cycle takes 2R
/// operations; the product is 2^(w-2) (RFC §synth-5-P, Counter width).
inline constexpr std::uint64_t kWrapBoundOps = std::uint64_t{1} << 62;

namespace detail {

/// `x` is before `y` in the order that wraps: the signed difference. Tickets
/// and cycles are compared this way throughout. (The unsigned-to-signed
/// conversion is modular on every compiler this runtime targets and is
/// defined to be so from C++20.)
constexpr bool before(std::uint64_t x, std::uint64_t y) noexcept {
    return static_cast<std::int64_t>(x - y) < 0;
}

/// `x` is `y` or before it, in the same order.
constexpr bool at_or_before(std::uint64_t x, std::uint64_t y) noexcept {
    return static_cast<std::int64_t>(x - y) <= 0;
}

constexpr unsigned log2_exact(std::size_t power_of_two) noexcept {
    unsigned order = 0;
    while (power_of_two > 1) {
        power_of_two >>= 1;
        ++order;
    }
    return order;
}

/// One SCQ ring of `R` slots (`R` a power of two) and `2R` entries.
///
/// An entry is `cycle | IsSafe | index`: the index in the low log2(2R) bits,
/// the IsSafe bit above it, the cycle in the rest. An entry whose index bits
/// are all ones holds nothing.
template <std::size_t R> class Ring {
    static_assert(R > 0 && (R & (R - 1)) == 0, "the ring size must be a power of two");

    /// The number of entries, 2R; also the value of the IsSafe bit.
    static constexpr std::uint64_t kEntries = 2 * static_cast<std::uint64_t>(R);
    /// The bits an index occupies.
    static constexpr std::uint64_t kIndexMask = kEntries - 1;
    /// The index bits and the IsSafe bit: everything below the cycle.
    static constexpr std::uint64_t kLowMask = 2 * kEntries - 1;
    /// log2(2R).
    static constexpr unsigned kEntryOrder = log2_exact(R) + 1;
    /// Threshold after an enqueue: 3R - 1 failed dequeues are enough to show
    /// that an empty-looking ring is empty.
    static constexpr std::int64_t kThreshold = 3 * static_cast<std::int64_t>(R) - 1;
    /// How many entries one 64-byte cache line holds, as a power of two.
    /// Consecutive tickets are spread over lines by rotating the ticket's
    /// bits by this much; a ring smaller than a line does not rotate.
    static constexpr unsigned kLineShift = 3;
    /// How often a dequeue that found an entry still empty looks again before
    /// it makes the entry unusable for the enqueue that owns it. Bounded, so
    /// the operation stays lock-free.
    static constexpr unsigned kSpins = 10000;
    /// The tail's top bit, set once the ring is closed: an enqueue that takes a
    /// ticket from a closed tail gets the bit back and puts nothing on the ring.
    /// This is the finalize bit of Nikolaev's LSCQ, which a `segmented` queue uses
    /// to end a segment (`queue_segmented.h`); a ring nobody closes never has it.
    /// Tickets are the low 63 bits, which no run reaches (`kWrapBoundOps`).
    static constexpr std::uint64_t kFin = std::uint64_t{1} << 63;

public:
    /// The entry a ticket names: the ticket rotated so that consecutive
    /// tickets land on different cache lines.
    static constexpr std::size_t map(std::uint64_t ticket) noexcept {
        if constexpr (kEntryOrder >= kLineShift) {
            return static_cast<std::size_t>(((ticket & kIndexMask) >> (kEntryOrder - kLineShift)) |
                                            ((ticket << kLineShift) & kIndexMask));
        } else {
            return static_cast<std::size_t>(ticket & kIndexMask);
        }
    }

    /// The ticket `map` sends to this entry.
    static constexpr std::uint64_t unmap(std::uint64_t position) noexcept {
        if constexpr (kEntryOrder >= kLineShift) {
            return ((position & kIndexMask) >> kLineShift) | ((position << (kEntryOrder - kLineShift)) & kIndexMask);
        } else {
            return position & kIndexMask;
        }
    }

    /// A ring holding the indices 0..filled, oldest first.
    constexpr explicit Ring(std::size_t filled) noexcept : Ring(filled, std::make_index_sequence<2 * R>{}) {}

    Ring(const Ring &) = delete;
    Ring &operator=(const Ring &) = delete;

    /// Close the ring: an enqueue that takes its ticket after this one's
    /// fetch-and-or puts nothing on it. An enqueue that already holds a ticket
    /// may still complete; the dequeues that follow either find its entry or make
    /// it unusable, which sends that enqueue to the next ticket and so to a
    /// refusal.
    void close() noexcept {
        tail_.value.fetch_or(kFin, std::memory_order_acq_rel);
    }

    /// Put `index` on the ring, or return `false`, putting nothing, when the ring
    /// is closed. An open ring never refuses: at most R indices circulate through
    /// it, and an enqueue that finds its entry unusable takes the next ticket.
    bool enqueue(std::size_t index) noexcept {
        const std::uint64_t stored = static_cast<std::uint64_t>(index) ^ kIndexMask;
        for (;;) {
            // Looking first keeps a closed ring's tail from counting attempts
            // that cannot succeed; the ticket's own bit is what decides.
            if ((tail_.value.load(std::memory_order_acquire) & kFin) != 0) {
                return false;
            }
            const std::uint64_t tail = tail_.value.fetch_add(1, std::memory_order_acq_rel);
            if ((tail & kFin) != 0) {
                return false;
            }
            const std::uint64_t ticket_cycle = (tail << 1) | kLowMask;
            std::atomic<std::uint64_t> &slot = entry_at(map(tail));
            std::uint64_t seen = slot.load(std::memory_order_acquire);
            // The entry is ours to fill when it is empty, from an earlier
            // cycle, and either safe or not yet passed by the head.
            for (;;) {
                const std::uint64_t entry_cycle = seen | kLowMask;
                const bool usable =
                    before(entry_cycle, ticket_cycle) &&
                    (seen == entry_cycle || (seen == (entry_cycle ^ kEntries) &&
                                             at_or_before(head_.value.load(std::memory_order_acquire), tail)));
                if (!usable) {
                    break;
                }
                // On failure `seen` holds what the entry now is.
                if (slot.compare_exchange_weak(seen, ticket_cycle ^ stored, std::memory_order_acq_rel,
                                               std::memory_order_acquire)) {
                    if (threshold_.value.load(std::memory_order_seq_cst) != kThreshold) {
                        threshold_.value.store(kThreshold, std::memory_order_seq_cst);
                    }
                    return true;
                }
            }
        }
    }

    /// Take the oldest index off the ring, or `std::nullopt` when it is empty.
    /// The empty answer may come from the threshold alone, which does not look at
    /// a ticket an enqueue holds and has not yet filled.
    std::optional<std::size_t> dequeue() noexcept {
        return dequeue_by(true);
    }

    /// Take the oldest index off a closed ring, or `std::nullopt` once every
    /// ticket the ring ever granted has been consumed or made unusable. It
    /// ignores the threshold and walks the head up to the tail, which a closed
    /// ring no longer moves, so an enqueue still holding a ticket either finds its
    /// entry taken from it (and takes the next ticket, which is refused) or has
    /// already filled it, and then the walk returns its index. A segment is
    /// retired only after this answers `std::nullopt`: until then an element may
    /// still arrive in it.
    std::optional<std::size_t> dequeue_drained() noexcept {
        assert((tail_.value.load(std::memory_order_acquire) & kFin) != 0 &&
               "only a closed ring has a fixed tail to walk to");
        return dequeue_by(false);
    }

private:
    /// The dequeue; `by_threshold` says whether the threshold may end it early.
    std::optional<std::size_t> dequeue_by(bool by_threshold) noexcept {
        if (by_threshold && threshold_.value.load(std::memory_order_seq_cst) < 0) {
            return std::nullopt;
        }
        for (;;) {
            const std::uint64_t head = head_.value.fetch_add(1, std::memory_order_acq_rel);
            const std::uint64_t ticket_cycle = (head << 1) | kLowMask;
            if (const std::optional<std::size_t> taken = look(entry_at(map(head)), ticket_cycle)) {
                return taken;
            }

            const std::uint64_t tail = tail_.value.load(std::memory_order_acquire);
            if (at_or_before(tail & ~kFin, head + 1)) {
                catch_up(tail, head + 1);
                if (by_threshold) {
                    threshold_.value.fetch_sub(1, std::memory_order_acq_rel);
                }
                return std::nullopt;
            }
            if (by_threshold && threshold_.value.fetch_sub(1, std::memory_order_acq_rel) <= 0) {
                return std::nullopt;
            }
        }
    }

    template <std::size_t... P>
    constexpr Ring(std::size_t filled, std::index_sequence<P...>) noexcept
        : head_(0), threshold_(filled == 0 ? -1 : kThreshold), tail_(static_cast<std::uint64_t>(filled)),
          entries_{initial(P, filled)...} {}

    /// What the entry at `position` holds in a ring built with `filled`
    /// indices: the first `filled` tickets' entries hold index `ticket` in
    /// cycle 0, safe; the rest hold nothing, in the cycle before it.
    static constexpr std::uint64_t initial(std::size_t position, std::size_t filled) noexcept {
        const std::uint64_t ticket = unmap(position);
        return ticket < filled ? kEntries + ticket : ~std::uint64_t{0};
    }

    std::atomic<std::uint64_t> &entry_at(std::size_t position) noexcept {
        return entries_[position];
    }

    /// Look at the entry a dequeue's ticket names until it is settled: the
    /// index when the entry was this ticket's (and the entry is left empty in
    /// the same cycle), `std::nullopt` when the ticket found nothing and the
    /// caller goes on to the empty check.
    std::optional<std::size_t> look(std::atomic<std::uint64_t> &slot, std::uint64_t ticket_cycle) noexcept {
        unsigned spins = 0;
        for (;;) {
            std::uint64_t seen = slot.load(std::memory_order_acquire);
            for (;;) {
                const std::uint64_t entry_cycle = seen | kLowMask;
                if (entry_cycle == ticket_cycle) {
                    // The entry is this ticket's: take its index and leave the
                    // entry empty in the same cycle.
                    slot.fetch_or(kIndexMask, std::memory_order_acq_rel);
                    return static_cast<std::size_t>(seen & kIndexMask);
                }
                std::uint64_t replacement;
                if ((seen | kEntries) != entry_cycle) {
                    // It holds an index of another cycle: mark it unsafe, so
                    // no enqueue of this cycle's ticket fills the entry behind
                    // the head. Already unsafe: nothing to do.
                    replacement = seen & ~kEntries;
                    if (seen == replacement) {
                        return std::nullopt;
                    }
                } else {
                    // It is empty. An enqueue holding this cycle's ticket may
                    // be about to fill it; look a few times, then move the
                    // entry to this cycle so that enqueue fails.
                    if (++spins <= kSpins) {
                        break;  // look again from a fresh read
                    }
                    replacement = ticket_cycle ^ ((~seen) & kEntries);
                }
                if (!before(entry_cycle, ticket_cycle)) {
                    return std::nullopt;
                }
                if (slot.compare_exchange_weak(seen, replacement, std::memory_order_acq_rel,
                                               std::memory_order_acquire)) {
                    return std::nullopt;
                }
                // On failure `seen` holds what the entry now is.
            }
        }
    }

    /// Move `tail` back to `head` after dequeues overshot an empty ring, so
    /// that the next enqueue does not start a cycle ahead of what is read. `tail`
    /// is the word as read, closing bit included, and the word written keeps the
    /// bit: a ring that was closed stays closed.
    void catch_up(std::uint64_t tail, std::uint64_t head) noexcept {
        while (!tail_.value.compare_exchange_weak(tail, head | (tail & kFin), std::memory_order_acq_rel,
                                                  std::memory_order_acquire)) {
            head = head_.value.load(std::memory_order_acquire);
            tail = tail_.value.load(std::memory_order_acquire);
            if (!before(tail & ~kFin, head)) {
                break;
            }
        }
    }

    /// Next ticket a dequeue takes.
    Padded<std::atomic<std::uint64_t>> head_;
    /// How many failed dequeues remain before the ring may be called empty;
    /// negative means it is empty.
    Padded<std::atomic<std::int64_t>> threshold_;
    /// Next ticket an enqueue takes.
    Padded<std::atomic<std::uint64_t>> tail_;
    std::atomic<std::uint64_t> entries_[2 * R];
};

}  // namespace detail

/// A bounded queue of at most `N` elements for any number of producers and
/// any number of consumers, lock-free. Exactly `N` fit when nothing else is
/// running.
///
/// The elements live in an array of `N` slots. Two rings of indices make it a
/// data queue: the *free* ring holds the indices of the slots nobody is using
/// and starts with all `N`; the *allocated* ring holds the indices of the
/// slots that hold an element, in the order they were filled, and starts
/// empty. A push takes an index from the free ring (none: the queue is full),
/// writes its element into that slot and puts the index on the allocated ring;
/// a pop does the reverse. The rings carry only indices, so the structure holds
/// no pointer: it is position-independent, which is what lets a mesh channel
/// place it in shared memory.
///
/// `R` is the ring size: a power of two, at least `N`, and at least the most
/// producers or consumers that will work the queue at once. The algorithm's
/// empty test is justified for at most `R` enqueuers and at most `R`
/// dequeuers working one ring at once (Nikolaev 2019, section 5.1); beyond
/// that a completed push can leave the threshold at -1 with its element in
/// the ring and every pop reports the queue empty until another push
/// completes. So a queue hands out at most `R` producer handles and at most
/// `R` consumer handles at a time.
///
/// A push may be refused while others hold slots: a slot's index goes back to
/// the free ring only after the pop that took the element has read it, and a
/// push holds the index it took until it has published its element. Waiting
/// for the operation that holds the slot would not be lock-free, since that
/// participant may be stopped. The shortfall is at most one slot per other
/// participant.
///
/// The queue needs 64-bit atomics that are lock-free; on a target without them
/// it does not compile, and the generator refuses the row there.
template <typename T, std::size_t N, std::size_t R> class Scq {
    static_assert(N > 0, "a queue's capacity must be at least one element");
    static_assert(R > 0 && (R & (R - 1)) == 0, "the ring size must be a power of two");
    static_assert(R >= N, "the ring must have at least as many slots as the capacity");
    static_assert(R <= (std::size_t{1} << 30), "a ring this large leaves its entries too few bits for the cycle");
    static_assert(std::atomic<std::uint64_t>::is_always_lock_free,
                  "the SCQ ring's entries are 64-bit atomics, which this target does not have lock-free");
    static_assert(std::is_nothrow_move_constructible_v<T> && std::is_nothrow_destructible_v<T>,
                  "a queue's element must be nothrow-move-constructible and nothrow-destructible");

public:
    /// The capacity this queue holds, exactly.
    static constexpr std::size_t kCapacity = N;

    constexpr Scq() noexcept : allocated_(0), free_(N) {}

    Scq(const Scq &) = delete;
    Scq &operator=(const Scq &) = delete;

    ~Scq() {
        // No handle is alive, so every index on the allocated ring names a
        // slot that holds a pushed element nobody popped.
        while (const std::optional<std::size_t> index = allocated_.dequeue()) {
            slots_[*index].value_.~T();
        }
    }

    constexpr std::size_t capacity() const noexcept {
        return N;
    }

private:
    /// Keeps `Producer` and `Consumer` constructible only through the queue.
    struct Token {
        explicit Token() = default;
    };

public:
    /// A producing side. It holds one of the queue's `R` producer places until
    /// it is destroyed.
    class Producer {
    public:
        Producer(Token, Scq &queue) noexcept : queue_(&queue) {}

        Producer(Producer &&other) noexcept : queue_(other.queue_) {
            other.queue_ = nullptr;
        }

        Producer &operator=(Producer &&) = delete;
        Producer(const Producer &) = delete;
        Producer &operator=(const Producer &) = delete;

        ~Producer() {
            if (queue_ != nullptr) {
                queue_->producers_.fetch_sub(1, std::memory_order_release);
            }
        }

        /// Another producer of the same queue, or `std::nullopt` when all `R`
        /// places are taken.
        std::optional<Producer> try_clone() const noexcept {
            return queue_->producer();
        }

        /// Push `value`, or leave it untouched and report `Full` when the
        /// queue holds its capacity, or while other participants' operations
        /// hold the slots it is short of. Lock-free: some operation completes
        /// in a bounded number of steps whatever the other participants are
        /// doing.
        PushStatus try_push(T &&value) noexcept {
            Scq &queue = *queue_;
            const std::optional<std::size_t> index = queue.free_.dequeue();
            if (!index) {
                return PushStatus::Full;
            }
            // A queue handed out as `bounded` is never closed, so the put cannot
            // be refused.
            const bool taken = queue.put(*index, value);
            assert(taken && "a bounded queue's allocated ring is never closed");
            (void)taken;
            return PushStatus::Ok;
        }

        constexpr std::size_t capacity() const noexcept {
            return N;
        }

    private:
        Scq *queue_;
    };

    /// A consuming side. It holds one of the queue's `R` consumer places until
    /// it is destroyed.
    class Consumer {
    public:
        Consumer(Token, Scq &queue) noexcept : queue_(&queue) {}

        Consumer(Consumer &&other) noexcept : queue_(other.queue_) {
            other.queue_ = nullptr;
        }

        Consumer &operator=(Consumer &&) = delete;
        Consumer(const Consumer &) = delete;
        Consumer &operator=(const Consumer &) = delete;

        ~Consumer() {
            if (queue_ != nullptr) {
                queue_->consumers_.fetch_sub(1, std::memory_order_release);
            }
        }

        /// Another consumer of the same queue, or `std::nullopt` when all `R`
        /// places are taken.
        std::optional<Consumer> try_clone() const noexcept {
            return queue_->consumer();
        }

        /// Pop the oldest element, or `std::nullopt` when the queue is empty.
        /// Lock-free.
        std::optional<T> try_pop() noexcept {
            return queue_->pop_any();
        }

        constexpr std::size_t capacity() const noexcept {
            return N;
        }

    private:
        Scq *queue_;
    };

    /// A producer handle, or `std::nullopt` when `R` producer handles are
    /// already alive: the ring cannot be shared by more enqueuers than it has
    /// slots. Destroying a handle gives its place back.
    std::optional<Producer> producer() noexcept {
        if (!detail::take_place(producers_, R)) {
            return std::nullopt;
        }
        return std::optional<Producer>(std::in_place, Token{}, *this);
    }

    /// A consumer handle, or `std::nullopt` when `R` consumer handles are
    /// already alive. Destroying a handle gives its place back.
    std::optional<Consumer> consumer() noexcept {
        if (!detail::take_place(consumers_, R)) {
            return std::nullopt;
        }
        return std::optional<Consumer>(std::in_place, Token{}, *this);
    }

private:
    /// What a segment of a `segmented` queue does with an SCQ: the same two
    /// rings, without handles (the segmented queue counts its own participants)
    /// and with a close that ends the segment (`queue_segmented.h`).
    template <typename U, std::size_t M, std::size_t S, std::size_t H, typename A, Progress Required> friend class Lscq;

    /// Move `value` into slot `index`, which the caller took off the free ring,
    /// and put the index on the allocated ring. A closed allocated ring takes
    /// nothing: the value goes back into `value` and the slot goes back to the
    /// free ring, and the call returns `false`.
    bool put(std::size_t index, T &value) noexcept {
        // The index came off the free ring, so no other participant holds this
        // slot until the enqueue below hands it on.
        T &stored = *::new (static_cast<void *>(std::addressof(slots_[index].value_))) T(std::move(value));
        if (allocated_.enqueue(index)) {
            return true;
        }
        // The enqueue was refused, so the index is still ours and the slot still
        // holds the element. Put it back where it came from.
        detail::give_back(value, std::move(stored));
        stored.~T();
        [[maybe_unused]] const bool returned = free_.enqueue(index);
        assert(returned && "the free ring is never closed");
        return false;
    }

    /// Take the oldest element, or `std::nullopt` when the queue is empty.
    /// Lock-free.
    std::optional<T> pop_any() noexcept {
        const std::optional<std::size_t> index = allocated_.dequeue();
        if (!index) {
            return std::nullopt;
        }
        return take(*index);
    }

    /// Take the oldest element of a closed queue, or `std::nullopt` only when no
    /// element can ever arrive in it again (`Ring::dequeue_drained`). Lock-free.
    std::optional<T> pop_drained() noexcept {
        const std::optional<std::size_t> index = allocated_.dequeue_drained();
        if (!index) {
            return std::nullopt;
        }
        return take(*index);
    }

    /// The element in slot `index`, which the caller took off the allocated ring;
    /// the slot goes back to the free ring.
    std::optional<T> take(std::size_t index) noexcept {
        // The index came off the allocated ring, so the slot holds a pushed
        // element and no other participant holds the slot until the enqueue below
        // hands it on.
        T &stored = slots_[index].value_;
        std::optional<T> value(std::move(stored));
        stored.~T();
        [[maybe_unused]] const bool returned = free_.enqueue(index);
        assert(returned && "the free ring is never closed");
        return value;
    }

    /// Push `value`, or close the queue and leave `value` as it was. A queue
    /// with no free slot is closed, and so is one a close has already reached:
    /// after a refusal nothing more is ever pushed into it, which is what lets a
    /// segment list put every later element behind this segment's. Lock-free.
    bool push_or_close(T &value) noexcept {
        const std::optional<std::size_t> index = free_.dequeue();
        if (!index) {
            allocated_.close();
            return false;
        }
        return put(*index, value);
    }

    /// The indices of the slots that hold an element, oldest first.
    detail::Ring<R> allocated_;
    /// The indices of the slots nobody is using.
    detail::Ring<R> free_;
    /// How many producer handles are alive.
    std::atomic<std::size_t> producers_{0};
    /// How many consumer handles are alive.
    std::atomic<std::size_t> consumers_{0};
    detail::Slot<T> slots_[N];
};

// ───────────────────────── Vyukov intrusive list ─────────────────────────

/// The value of a link that names no node.
inline constexpr std::uint32_t kNil = UINT32_MAX;

/// The `intrusive` storage mode (RFC §synth-5-P): Vyukov's MPSC list, in which
/// each element carries its own link, the caller owns every element, and a push
/// cannot fail.
///
/// **The link is an index.** `Link` names a `std::uint32_t` member of `T`, which
/// the queue uses in place as the index of the node that follows; the nodes are
/// an array the caller owns, and a node is named by its position in it. An index
/// fits the member on a 64-bit target where a pointer would not, and means the
/// same in every process that maps the array.
///
/// **The algorithm.** `tail_` names the newest node. A push exchanges itself
/// onto `tail_` and then stores its own index into the old tail's link; a pop
/// walks from the oldest node. The caller gets back the very node it pushed and
/// may reuse it at once, which a queue with a dummy node cannot give; the list
/// keeps a *stub* node for it, and the pop of the last node puts the stub behind
/// it through the producers' own exchange, so that it has a successor and can be
/// returned. The stub is a node of the caller's array, set aside at
/// construction and never pushed by the caller.
///
/// **Progress.** Push is wait-free: one exchange and one store. Pop is
/// `blocking`: a producer stopped between its exchange and its store hides every
/// node pushed after it, and in that window `try_pop` returns empty while pushes
/// that completed sit behind it. With `ManyConsumers`, a test-and-set flag
/// serialises the pops, and a consumer that finds it taken waits for the holder.
///
/// Memory order: a push writes its node, exchanges with acq_rel and stores the
/// predecessor's link with release; a pop reads a link with acquire, so a node's
/// contents written before its push happen-before the pop that returns it.
template <typename T, std::uint32_t T::*Link, bool ManyConsumers> class IntrusiveMpsc {
    static_assert(std::atomic<std::uint32_t>::is_always_lock_free,
                  "the link is a 32-bit atomic, which the target must make lock-free");
    static_assert(sizeof(std::atomic<std::uint32_t>) == sizeof(std::uint32_t) &&
                      alignof(std::atomic<std::uint32_t>) == alignof(std::uint32_t),
                  "an atomic<uint32_t> has the layout of the member it is used in place of");

public:
    /// An empty list over `len` nodes at `nodes`, with node `stub` set aside.
    /// `nodes` must stay valid for the queue's life and a node must not be
    /// touched, but through the queue, while it is queued; `stub < len < kNil`.
    IntrusiveMpsc(T *nodes, std::uint32_t len, std::uint32_t stub) noexcept
        : tail_(stub), head_(stub), busy_(0), stub_(stub), nodes_(nodes), len_(len) {
        assert(stub < len && len < kNil);
        link(stub).store(kNil, std::memory_order_relaxed);
    }

    IntrusiveMpsc(const IntrusiveMpsc &) = delete;
    IntrusiveMpsc &operator=(const IntrusiveMpsc &) = delete;

    /// The number of nodes in the array, the stub included.
    constexpr std::uint32_t nodes() const noexcept {
        return len_;
    }

    /// The node that is the stub.
    constexpr std::uint32_t stub() const noexcept {
        return stub_;
    }

    /// Put node `index` on the queue. Wait-free; it cannot fail. `index` names a
    /// node that is not the stub and is not in the queue, and the caller has
    /// finished writing it: its contents are published to the consumer here.
    void push(std::uint32_t index) noexcept {
        assert(index != stub_);
        append(index);
    }

    /// Take the oldest node, or `std::nullopt` when the queue holds none the
    /// consumer can reach, which is also the answer while a producer is between
    /// its two steps. With many consumers the call first takes the flag; with one,
    /// only that consumer may call it.
    std::optional<std::uint32_t> try_pop() noexcept {
        if constexpr (ManyConsumers) {
            while (busy_.value.exchange(1, std::memory_order_acquire) != 0) {
            }
            const std::optional<std::uint32_t> popped = pop();
            busy_.value.store(0, std::memory_order_release);
            return popped;
        } else {
            return pop();
        }
    }

private:
    /// The link of node `index`, used in place.
    std::atomic<std::uint32_t> &link(std::uint32_t index) noexcept {
        assert(index < len_);
        return *reinterpret_cast<std::atomic<std::uint32_t> *>(&(nodes_[index].*Link));
    }

    /// Link `index` behind the newest node.
    void append(std::uint32_t index) noexcept {
        link(index).store(kNil, std::memory_order_relaxed);
        // After this exchange `index` is the newest node and `previous` is behind
        // it, whatever the other producers do. Until the store below the consumer
        // cannot reach `index`.
        const std::uint32_t previous = tail_.value.exchange(index, std::memory_order_acq_rel);
        link(previous).store(index, std::memory_order_release);
    }

    /// The walk, run by one consumer at a time.
    std::optional<std::uint32_t> pop() noexcept {
        std::uint32_t head = head_.value.load(std::memory_order_relaxed);
        std::uint32_t next = link(head).load(std::memory_order_acquire);
        if (head == stub_) {
            if (next == kNil) {
                return std::nullopt;
            }
            head_.value.store(next, std::memory_order_relaxed);
            head = next;
            next = link(head).load(std::memory_order_acquire);
        }
        if (next != kNil) {
            head_.value.store(next, std::memory_order_relaxed);
            return head;
        }
        // `head` has no successor: it is the newest node, or a producer is between
        // its exchange and its store.
        if (head != tail_.value.load(std::memory_order_acquire)) {
            return std::nullopt;
        }
        // It is the newest node. Put the stub behind it so that it has a
        // successor and can be returned.
        append(stub_);
        next = link(head).load(std::memory_order_acquire);
        if (next != kNil) {
            head_.value.store(next, std::memory_order_relaxed);
            return head;
        }
        return std::nullopt;
    }

    /// The newest node. Exchanged by every push and by the stub's.
    detail::Padded<std::atomic<std::uint32_t>> tail_;
    /// The oldest node not yet returned. Private to the one consumer, or to the
    /// holder of `busy_`.
    detail::Padded<std::atomic<std::uint32_t>> head_;
    /// The test-and-set flag that serialises many consumers. Unused with one.
    detail::Padded<std::atomic<std::uint32_t>> busy_;
    std::uint32_t stub_;
    T *nodes_;
    std::uint32_t len_;
};

}  // namespace SCE::Forge::Queue
