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

    /// Put `index` on the ring. The ring never refuses: at most R indices
    /// circulate through it, and an enqueue that finds its entry unusable
    /// takes the next ticket.
    void enqueue(std::size_t index) noexcept {
        const std::uint64_t stored = static_cast<std::uint64_t>(index) ^ kIndexMask;
        for (;;) {
            const std::uint64_t tail = tail_.value.fetch_add(1, std::memory_order_acq_rel);
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
                    return;
                }
            }
        }
    }

    /// Take the oldest index off the ring, or `std::nullopt` when it is empty.
    std::optional<std::size_t> dequeue() noexcept {
        if (threshold_.value.load(std::memory_order_seq_cst) < 0) {
            return std::nullopt;
        }
        for (;;) {
            const std::uint64_t head = head_.value.fetch_add(1, std::memory_order_acq_rel);
            const std::uint64_t ticket_cycle = (head << 1) | kLowMask;
            if (const std::optional<std::size_t> taken = look(entry_at(map(head)), ticket_cycle)) {
                return taken;
            }

            const std::uint64_t tail = tail_.value.load(std::memory_order_acquire);
            if (at_or_before(tail, head + 1)) {
                catch_up(tail, head + 1);
                threshold_.value.fetch_sub(1, std::memory_order_acq_rel);
                return std::nullopt;
            }
            if (threshold_.value.fetch_sub(1, std::memory_order_acq_rel) <= 0) {
                return std::nullopt;
            }
        }
    }

private:
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
    /// that the next enqueue does not start a cycle ahead of what is read.
    void catch_up(std::uint64_t tail, std::uint64_t head) noexcept {
        while (!tail_.value.compare_exchange_weak(tail, head, std::memory_order_acq_rel, std::memory_order_acquire)) {
            head = head_.value.load(std::memory_order_acquire);
            tail = tail_.value.load(std::memory_order_acquire);
            if (!before(tail, head)) {
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
            // The index came off the free ring, so no other participant holds
            // this slot until the enqueue below hands it on.
            ::new (static_cast<void *>(std::addressof(queue.slots_[*index].value_))) T(std::move(value));
            queue.allocated_.enqueue(*index);
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
            Scq &queue = *queue_;
            const std::optional<std::size_t> index = queue.allocated_.dequeue();
            if (!index) {
                return std::nullopt;
            }
            // The index came off the allocated ring, so the slot holds a
            // pushed element and no other participant holds the slot until
            // the enqueue below hands it on.
            T &stored = queue.slots_[*index].value_;
            std::optional<T> value(std::move(stored));
            stored.~T();
            queue.free_.enqueue(*index);
            return value;
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

}  // namespace SCE::Forge::Queue
