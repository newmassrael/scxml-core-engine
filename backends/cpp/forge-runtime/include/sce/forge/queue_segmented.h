// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Forge — the `segmented` storage mode of `sce:kind="queue"` (SCE
// Protocol-Synthesis RFC §synth-5-P) for C++: queues that grow by segments taken
// from an allocator the caller injects, and hold as many elements as that
// allocator allows.
//
//   - LinkedLamport   one producer, one consumer: linked Lamport rings. Pop is
//                     wait-free; push is wait-free but for the allocator.
//   - Lscq            any other cardinality: Nikolaev's LSCQ (DISC 2019), a list of
//                     SCQ rings reclaimed through a hazard-pointer domain. Both
//                     operations are lock-free but for the allocator.
//   - HazardDomain    the reclamation domain Lscq needs, sized from the number of
//                     participants and built by the caller.
//
// There is no global default allocator and no global domain (SCE_FORGE.md §2.1,
// C2): a queue borrows both, so the queue can be moved by whoever owns the
// pieces and a target with no heap hands it an arena.
//
// What a segmented queue needs of its allocator `A`:
//
//   static constexpr Progress kProgress;
//       what allocate and deallocate give. The queue's push is no stronger than
//       this, and the queue does not build over an allocator that gives less than
//       the document declared (`allocator-progress`), which is the compile-time
//       form of RFC §synth-5-P, *Segment allocator progress is declared in the
//       document*.
//   void *allocate(std::size_t size, std::size_t align) noexcept;
//       a block of `size` bytes aligned to `align` that no other live block
//       overlaps, or nullptr when it refuses. A refusal is the queue's
//       `PushStatus::OutOfMemory`.
//   void deallocate(void *block, std::size_t size, std::size_t align) noexcept;
//       gives a block back.
//
// Both may be called from several threads at once.

#pragma once
#ifndef SCE_FORGE_QUEUE_SEGMENTED_H
#define SCE_FORGE_QUEUE_SEGMENTED_H

#include <atomic>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <new>
#include <optional>
#include <type_traits>
#include <utility>

#include "sce/forge/queue.h"

namespace SCE::Forge::Queue {

namespace detail {

/// A block from `allocator` holding a `S` built from `args`, or nullptr when it
/// refuses. Nothing is built on a refusal.
template <typename S, typename A, typename... Args> S *place(A &allocator, Args &&...args) noexcept {
    void *block = allocator.allocate(sizeof(S), alignof(S));
    if (block == nullptr) {
        return nullptr;
    }
    return ::new (block) S(std::forward<Args>(args)...);
}

/// Destroy the `S` at `segment` and give its block back.
template <typename S, typename A> void release(A &allocator, S *segment) noexcept {
    segment->~S();
    allocator.deallocate(static_cast<void *>(segment), sizeof(S), alignof(S));
}

}  // namespace detail

// ───────────────────────────── Hazard pointers ─────────────────────────────

/// The header a retired object carries so the domain can chain it and, once no
/// hazard names it, hand it back to what allocated it. The object derives from
/// it as its only base: a hazard names the object by its address, and the scan
/// compares that address with the header's.
struct Retired {
    /// The next retired object on the domain's list. Written only by the domain,
    /// and named apart from the `next` a segment links its successor through.
    std::atomic<Retired *> chain{nullptr};
    /// What frees the object. Written by `HazardDomain::retire` before the
    /// object is published to the list, read by the scan after it took the list.
    void (*reclaim)(Retired *, void *) noexcept = nullptr;
    /// The value `reclaim` is called with: the allocator that gave the object.
    void *context = nullptr;
};

/// The reclamation domain of a `segmented` queue (RFC §synth-5-P, *Reclamation
/// domain*): hazard pointers, sized statically from the number of participants
/// and built by the caller.
///
/// A segment can still be read by one participant after another has taken it out
/// of the queue, so it cannot be freed at that moment. Each participant
/// publishes, in a slot of its own, the one segment it is about to touch
/// (`Hazard::protect`); the thread that unlinks a segment hands it to the domain
/// (`retire`); and the domain frees it only when no slot names it. A participant
/// that stalls therefore holds back the segments its slot names and nothing
/// else, which is why hazard pointers were chosen over epochs: the memory a
/// stalled participant keeps unreclaimed is bounded, and so the domain needs no
/// storage that grows.
///
/// The domain is a fixed array of `H` slots, and the retired segments wait on an
/// intrusive list threaded through the header each carries, so it needs neither a
/// heap nor a bound on the list.
///
/// Progress. `protect` and `retire` are lock-free: a retry means another
/// participant moved the pointer or pushed a node. The scan that frees nodes is
/// run by one thread at a time; a thread that finds a scan running does not wait
/// for it, it returns and leaves the nodes on the list, so no operation of a
/// queue waits for another participant on account of the domain.
///
/// Ordering. A hazard is published with a sequentially consistent store and
/// re-validated with a sequentially consistent load of the pointer it guards, and
/// the scan reads every hazard with a sequentially consistent load after the node
/// was unlinked. Either the scan sees the hazard and keeps the node, or the
/// participant's validation sees the pointer already moved and does not use it.
/// That holds when the location a participant validates against is written, with
/// a sequentially consistent operation, by whoever unlinks the node before the
/// node is retired; the queue using the domain has to arrange that (`Lscq`,
/// *Where a hazard is validated*). No fence is used: every operation in the
/// argument is itself sequentially consistent, which also keeps the header free
/// of the one primitive ThreadSanitizer does not model.
///
/// One domain per queue is the intended use. A domain several queues share holds
/// the nodes of all of them; what a node is freed with is the allocator it names,
/// which the queue borrows for as long as the domain may hold the node. A queue
/// that is destroyed flushes the domain (`flush`) before its allocator can go.
template <std::size_t H> class HazardDomain {
    static_assert(H > 0, "a domain needs at least one slot");

    /// How many retired objects wait before a scan runs: so the unreclaimed memory
    /// is bounded by the participants.
    static constexpr std::size_t kScanAt = 2 * H;

public:
    constexpr HazardDomain() noexcept : HazardDomain(std::make_index_sequence<H>{}) {}

    HazardDomain(const HazardDomain &) = delete;
    HazardDomain &operator=(const HazardDomain &) = delete;

    /// How many participants the domain serves.
    constexpr std::size_t slots() const noexcept {
        return H;
    }

    /// A participant's slot in the domain. It holds the slot until it is
    /// destroyed, and publishes what its owner is about to touch.
    class Hazard {
    public:
        Hazard(Hazard &&other) noexcept : domain_(other.domain_), index_(other.index_) {
            other.domain_ = nullptr;
        }

        Hazard &operator=(Hazard &&) = delete;
        Hazard(const Hazard &) = delete;
        Hazard &operator=(const Hazard &) = delete;

        ~Hazard() {
            if (domain_ != nullptr) {
                clear();
                domain_->taken_[index_].store(false, std::memory_order_release);
            }
        }

        /// Read `source` and protect what it points to: publish it, then check
        /// that `source` still holds it, and try again if not. The pointer
        /// returned is not freed while the hazard names it, unless it is null.
        template <typename T> T *protect(const std::atomic<T *> &source) noexcept {
            T *pointer = source.load(std::memory_order_acquire);
            for (;;) {
                // Both are sequentially consistent, so in the one order they share
                // the publication comes before the check, and no fence is needed
                // between.
                slot().store(static_cast<void *>(pointer), std::memory_order_seq_cst);
                T *again = source.load(std::memory_order_seq_cst);
                if (again == pointer) {
                    return pointer;
                }
                pointer = again;
            }
        }

        /// Stop protecting what the hazard names. Called when the participant is
        /// done with it, so it does not hold back the segment longer than it uses
        /// it.
        void clear() noexcept {
            slot().store(nullptr, std::memory_order_release);
        }

    private:
        friend class HazardDomain;

        Hazard(HazardDomain &domain, std::size_t index) noexcept : domain_(&domain), index_(index) {}

        std::atomic<void *> &slot() noexcept {
            return domain_->hazards_[index_].value;
        }

        HazardDomain *domain_;
        std::size_t index_;
    };

    /// A slot for one participant, or `std::nullopt` when all `H` are taken.
    /// Destroying the guard gives the slot back.
    std::optional<Hazard> acquire() noexcept {
        for (std::size_t index = 0; index < H; ++index) {
            bool expected = false;
            if (taken_[index].compare_exchange_strong(expected, true, std::memory_order_acq_rel,
                                                      std::memory_order_relaxed)) {
                return std::optional<Hazard>(Hazard(*this, index));
            }
        }
        return std::nullopt;
    }

    /// Hand `node`, already unlinked from every structure a participant can reach
    /// it through, to the domain. It is freed by `reclaim(node, context)` once no
    /// hazard names it, now or at a later retirement.
    ///
    /// `node` is the header of a live object no longer reachable except through a
    /// hazard already published; it is retired once; `reclaim` frees exactly that
    /// object; and `context` stays valid until it has.
    void retire(Retired *node, void (*reclaim)(Retired *, void *) noexcept, void *context) noexcept {
        node->reclaim = reclaim;
        node->context = context;
        // Counted before it is published, so a scan that frees it at once never
        // subtracts what was not yet added.
        const std::size_t waiting = waiting_.fetch_add(1, std::memory_order_acq_rel) + 1;
        push(node, node);
        if (waiting >= kScanAt) {
            scan();
        }
    }

    /// Free what can be freed now: the retired objects no hazard names, unless a
    /// scan is already running, which is then left to finish. Lock-free: it never
    /// waits for another scan.
    void collect() noexcept {
        scan();
    }

    /// Free every retired object no hazard names, waiting for a scan that is
    /// running instead of leaving the objects to it. A queue being destroyed calls
    /// it: no participant of that queue is alive, so none of its segments is
    /// named, and every one of them is freed before the queue's allocator can go.
    /// Blocking, which a destructor may be.
    void flush() noexcept {
        while (scanning_.exchange(true, std::memory_order_acquire)) {
            // Another thread is scanning; it holds the flag only for one pass.
        }
        scan_held();
    }

    /// How many retired objects are waiting to be freed. For tests and for a
    /// target that budgets: it is bounded by the slots.
    std::size_t waiting() const noexcept {
        return waiting_.load(std::memory_order_acquire);
    }

private:
    template <std::size_t... I>
    constexpr explicit HazardDomain(std::index_sequence<I...>) noexcept
        : hazards_{empty_hazard(I)...}, taken_{(static_cast<void>(I), false)...} {}

    static constexpr detail::Padded<std::atomic<void *>> empty_hazard(std::size_t) noexcept {
        return detail::Padded<std::atomic<void *>>(nullptr);
    }

    /// Push the chain `first ..= last` (linked through `chain`) onto the list.
    void push(Retired *first, Retired *last) noexcept {
        Retired *head = retired_.load(std::memory_order_relaxed);
        do {
            last->chain.store(head, std::memory_order_relaxed);
        } while (!retired_.compare_exchange_weak(head, first, std::memory_order_release, std::memory_order_relaxed));
    }

    /// Free every retired object no hazard names. A thread that finds a scan
    /// already running returns: the objects wait for the next one.
    void scan() noexcept {
        if (scanning_.exchange(true, std::memory_order_acquire)) {
            return;
        }
        scan_held();
    }

    /// The scan, run by the thread that holds `scanning_`; it gives the flag back.
    void scan_held() noexcept {
        Retired *list = retired_.exchange(nullptr, std::memory_order_acquire);
        // The objects on `list` were unlinked before they were retired, so a
        // participant that has not published a hazard for one by now will find
        // the pointer moved when it validates.
        void *named[H];
        for (std::size_t index = 0; index < H; ++index) {
            named[index] = hazards_[index].value.load(std::memory_order_seq_cst);
        }
        Retired *kept = nullptr;
        Retired *kept_last = nullptr;
        std::size_t freed = 0;
        while (list != nullptr) {
            Retired *node = list;
            list = node->chain.load(std::memory_order_relaxed);
            if (is_named(named, node)) {
                node->chain.store(kept, std::memory_order_relaxed);
                if (kept == nullptr) {
                    kept_last = node;
                }
                kept = node;
            } else {
                // No hazard names it, nothing else reaches it, and the retirement
                // recorded how to free it.
                node->reclaim(node, node->context);
                ++freed;
            }
        }
        if (kept != nullptr) {
            push(kept, kept_last);
        }
        if (freed > 0) {
            waiting_.fetch_sub(freed, std::memory_order_acq_rel);
        }
        scanning_.store(false, std::memory_order_release);
    }

    static bool is_named(void *const (&named)[H], const Retired *node) noexcept {
        for (void *hazard : named) {
            if (hazard == static_cast<const void *>(node)) {
                return true;
            }
        }
        return false;
    }

    /// The object each participant is currently touching, or null.
    detail::Padded<std::atomic<void *>> hazards_[H];
    /// Which slots a participant holds.
    std::atomic<bool> taken_[H];
    /// Retired objects not yet freed.
    std::atomic<Retired *> retired_{nullptr};
    /// How many are on `retired_`.
    std::atomic<std::size_t> waiting_{0};
    /// A scan is running.
    std::atomic<bool> scanning_{false};
};

// ──────────────────────── Linked Lamport rings ─────────────────────────────

/// The `segmented` row for one producer and one consumer (RFC §synth-5-P):
/// linked Lamport rings, wait-free on both sides, bounded only by the allocator
/// it is given.
///
/// The queue is a list of segments. A segment holds `N` slots that are filled
/// once, in order, and read once, in order: it is a Lamport ring that is never
/// lapped, so it needs neither the second lap that tells a full ring from an
/// empty one nor a free slot. The producer fills the newest segment and, when it
/// is full, takes a new one from the allocator and links it behind; the consumer
/// reads the oldest segment and, when it has read all `N`, follows the link and
/// gives the segment back. Each side keeps its own position and shares it with
/// the other only through the two things the other must see: how many slots of a
/// segment are filled, and the link to the next segment.
///
/// No reclamation domain. The producer never touches a segment again after it has
/// linked its successor, and the consumer gives a segment back only after it has
/// read that link, so the consumer frees it directly. Nothing else can hold a
/// pointer to it: with one producer and one consumer no third party exists.
///
/// Progress. Pop is wait-free. Push is wait-free but for the one step that can
/// take a segment: the allocator. The queue is therefore as strong as the
/// allocator the document declares (`A::kProgress`, held to `Required` by a
/// static_assert), and a push the allocator refuses leaves its element where it
/// was and reports `PushStatus::OutOfMemory`.
///
/// Ordering. The producer publishes a filled slot by storing the count of filled
/// slots with release, and the consumer reads that count with acquire before it
/// reads the slot, so the write of an element happens-before the pop that returns
/// it. The link is stored with release and read with acquire for the same reason.
/// Each position is private to one side, so it needs no ordering of its own.
///
/// A queue whose allocator refused the first segment is not `valid()` and hands
/// out no handle.
template <typename T, std::size_t N, typename A, Progress Required> class LinkedLamport {
    static_assert(N > 0, "a segment holds at least one element");
    static_assert(A::kProgress >= Required, "the allocator gives less progress than the document declared for it");
    static_assert(std::is_nothrow_move_constructible_v<T> && std::is_nothrow_destructible_v<T>,
                  "a queue's element must be nothrow-move-constructible and nothrow-destructible");

    /// One segment: `N` slots filled once.
    struct Segment {
        /// How many slots, from the first, hold an element the producer
        /// published. Written only by the producer.
        detail::Padded<std::atomic<std::size_t>> written{0};
        /// The segment after this one, or null while this is the newest. Written
        /// once, by the producer, when this segment is full and it takes the next.
        std::atomic<Segment *> next{nullptr};
        detail::Slot<T> slots[N];
    };

public:
    /// The elements one segment holds.
    static constexpr std::size_t kSegment = N;

    /// The progress the allocator gives, and so the most push can.
    static constexpr Progress kAllocatorProgress = A::kProgress;

    /// An empty queue over `allocator`, which it borrows for its whole life.
    explicit LinkedLamport(A &allocator) noexcept : LinkedLamport(allocator, detail::place<Segment>(allocator)) {}

    LinkedLamport(const LinkedLamport &) = delete;
    LinkedLamport &operator=(const LinkedLamport &) = delete;

    ~LinkedLamport() {
        // No handle is alive, so every segment from the head is ours, and its
        // slots from `read` up to `written` hold elements that were pushed and not
        // popped.
        Segment *segment = head_.value.load(std::memory_order_acquire);
        std::size_t read = head_read_.value.load(std::memory_order_acquire);
        while (segment != nullptr) {
            const std::size_t written = segment->written.value.load(std::memory_order_acquire);
            Segment *next = segment->next.load(std::memory_order_acquire);
            for (std::size_t index = read; index < written; ++index) {
                segment->slots[index].value_.~T();
            }
            detail::release(allocator_, segment);
            segment = next;
            read = 0;
        }
    }

    /// Whether the allocator gave the first segment.
    bool valid() const noexcept {
        return head_.value.load(std::memory_order_relaxed) != nullptr;
    }

private:
    /// Keeps `Producer` and `Consumer` constructible only through the queue.
    struct Token {
        explicit Token() = default;
    };

public:
    /// The producing side. There is one at a time.
    class Producer {
    public:
        Producer(Token, LinkedLamport &queue) noexcept : queue_(&queue) {}

        Producer(Producer &&other) noexcept : queue_(other.queue_) {
            other.queue_ = nullptr;
        }

        Producer &operator=(Producer &&) = delete;
        Producer(const Producer &) = delete;
        Producer &operator=(const Producer &) = delete;

        ~Producer() {
            if (queue_ != nullptr) {
                queue_->producer_taken_.store(false, std::memory_order_release);
            }
        }

        /// Push `value`, or leave it untouched and report `OutOfMemory` when the
        /// newest segment is full and the allocator will not give another.
        /// Wait-free when the allocator is.
        PushStatus try_push(T &&value) noexcept {
            LinkedLamport &queue = *queue_;
            // The tail and the filled count are this side's: only the producer
            // writes them.
            Segment *tail = queue.tail_.value.load(std::memory_order_relaxed);
            std::size_t written = tail->written.value.load(std::memory_order_relaxed);
            if (written == N) {
                Segment *fresh = detail::place<Segment>(queue.allocator_);
                if (fresh == nullptr) {
                    return PushStatus::OutOfMemory;
                }
                // The producer never touches `tail` again after this store.
                tail->next.store(fresh, std::memory_order_release);
                queue.tail_.value.store(fresh, std::memory_order_relaxed);
                tail = fresh;
                written = 0;
            }
            // Slot `written` is past every filled slot, so the consumer does not
            // read it until the store below publishes it.
            ::new (static_cast<void *>(std::addressof(tail->slots[written].value_))) T(std::move(value));
            tail->written.value.store(written + 1, std::memory_order_release);
            return PushStatus::Ok;
        }

        /// The elements one segment holds.
        constexpr std::size_t segment() const noexcept {
            return N;
        }

    private:
        LinkedLamport *queue_;
    };

    /// The consuming side. There is one at a time.
    class Consumer {
    public:
        Consumer(Token, LinkedLamport &queue) noexcept : queue_(&queue) {}

        Consumer(Consumer &&other) noexcept : queue_(other.queue_) {
            other.queue_ = nullptr;
        }

        Consumer &operator=(Consumer &&) = delete;
        Consumer(const Consumer &) = delete;
        Consumer &operator=(const Consumer &) = delete;

        ~Consumer() {
            if (queue_ != nullptr) {
                queue_->consumer_taken_.store(false, std::memory_order_release);
            }
        }

        /// Pop the oldest element, or `std::nullopt` when the queue is empty.
        /// Wait-free.
        std::optional<T> try_pop() noexcept {
            LinkedLamport &queue = *queue_;
            Segment *head = queue.head_.value.load(std::memory_order_relaxed);
            std::size_t read = queue.head_read_.value.load(std::memory_order_relaxed);
            if (read == N) {
                // Every element of the head segment is taken. Its successor exists
                // once the producer has linked it, and the producer never touches
                // this segment after that, so it is ours to give back.
                Segment *next = head->next.load(std::memory_order_acquire);
                if (next == nullptr) {
                    return std::nullopt;
                }
                detail::release(queue.allocator_, head);
                head = next;
                read = 0;
                queue.head_.value.store(head, std::memory_order_relaxed);
                queue.head_read_.value.store(0, std::memory_order_relaxed);
            }
            // Slot `read` is below the published count when the count is above it,
            // so it holds an element, and only this side reads it.
            const std::size_t written = head->written.value.load(std::memory_order_acquire);
            if (read >= written) {
                return std::nullopt;
            }
            T &stored = head->slots[read].value_;
            std::optional<T> value(std::move(stored));
            stored.~T();
            queue.head_read_.value.store(read + 1, std::memory_order_relaxed);
            return value;
        }

        /// The elements one segment holds.
        constexpr std::size_t segment() const noexcept {
            return N;
        }

    private:
        LinkedLamport *queue_;
    };

    /// The producer, or `std::nullopt` when it is already taken (or the queue is
    /// not valid). There is one, and it is not copyable. Destroying it gives it
    /// back.
    std::optional<Producer> producer() noexcept {
        bool expected = false;
        if (!valid() || !producer_taken_.compare_exchange_strong(expected, true, std::memory_order_acq_rel,
                                                                 std::memory_order_relaxed)) {
            return std::nullopt;
        }
        return std::optional<Producer>(std::in_place, Token{}, *this);
    }

    /// The consumer, or `std::nullopt` when it is already taken (or the queue is
    /// not valid). There is one, and it is not copyable. Destroying it gives it
    /// back.
    std::optional<Consumer> consumer() noexcept {
        bool expected = false;
        if (!valid() || !consumer_taken_.compare_exchange_strong(expected, true, std::memory_order_acq_rel,
                                                                 std::memory_order_relaxed)) {
            return std::nullopt;
        }
        return std::optional<Consumer>(std::in_place, Token{}, *this);
    }

private:
    LinkedLamport(A &allocator, Segment *first) noexcept
        : head_(first), head_read_(0), tail_(first), allocator_(allocator) {}

    /// The segment the consumer reads. Written only by the consumer.
    detail::Padded<std::atomic<Segment *>> head_;
    /// How many elements of the head segment the consumer has taken. Written only
    /// by the consumer.
    detail::Padded<std::atomic<std::size_t>> head_read_;
    /// The segment the producer fills. Written only by the producer.
    detail::Padded<std::atomic<Segment *>> tail_;
    /// Whether the producer / consumer handle is alive: there is one of each.
    std::atomic<bool> producer_taken_{false};
    std::atomic<bool> consumer_taken_{false};
    A &allocator_;
};

// ─────────────────────────────────── LSCQ ──────────────────────────────────

/// The `segmented` row for any cardinality other than one producer and one
/// consumer (RFC §synth-5-P): LSCQ, a list of SCQ rings, lock-free on both sides
/// and bounded only by the allocator it is given.
///
/// The algorithm is Nikolaev's LSCQ (DISC 2019, section 6): a Michael-Scott list
/// whose nodes are SCQ rings instead of single elements. Producers push into the
/// newest ring (`tail`); consumers pop from the oldest (`head`). A ring that
/// cannot take another element is closed: nothing is ever pushed into it again.
/// The producer that found it closed takes a segment from the allocator, puts its
/// element in it before anyone can see it, and links it behind the closed one
/// with a compare-and-swap on that ring's `next`; a producer that loses that race
/// takes its element back, frees its segment and pushes into the winner's. A
/// consumer that finds the oldest ring empty and closed moves `head` to its
/// successor and retires it.
///
/// Each ring is the SCQ data queue of `queue.h`, which keeps its elements in a
/// slot array and moves their indices through two rings. A segment is therefore
/// allocated whole, and an element is written into its slot once and read out of
/// it once.
///
/// Closing. The close is a bit in the allocated ring's tail, set with a
/// fetch-and-or, and an enqueue takes its ticket with a fetch-and-add that returns
/// the bit: a ticket granted before the close can still be used, one after it is
/// refused. That is what makes the order across segments exact. Without it a
/// producer could pass a "not closed" check, stall, and push into a segment a
/// consumer had already emptied and retired, and the element would be lost.
///
/// Retiring a ring waits for the elements in flight. A consumer sees a ring empty
/// through SCQ's threshold, which does not look at a ticket an enqueue holds and
/// has not yet filled. So the consumer that finds the oldest ring empty and closed
/// pops it once more with `pop_drained`, which walks the head to the closed tail
/// and either takes the element such an enqueue filled or makes its entry
/// unusable, and only when that answers empty does it move `head`.
///
/// Reclamation. A segment can be read by a participant after another has moved
/// `head` past it, so it is freed through the hazard-pointer domain the caller
/// builds (`HazardDomain`); each handle holds one slot of it and protects the one
/// segment it is touching. A segment is retired only by the consumer whose
/// compare-and-swap moved `head` past it; a stalled producer that still holds the
/// segment keeps only that segment alive.
///
/// Where a hazard is validated. A hazard protects a segment only if the location
/// it is validated against is the one the unlinking thread writes before the scan
/// runs. A consumer validates against `head`, which the retiring consumer writes.
/// A producer validates against `tail`, which that consumer does not write: left
/// alone, `tail` can still name a segment that `head` has left and that has been
/// retired (the producer that linked its successor has not yet moved `tail`), so
/// a producer could publish its hazard after the scan read its slot, find `tail`
/// unchanged, and use the segment once the scan freed it. So a consumer moves
/// `tail` off the segment, to its successor, before it moves `head` past it, and
/// every write of `head` and `tail` is sequentially consistent. Then `tail` does
/// not name a segment by the time it is retired, and any producer whose check
/// still saw it there published its hazard before the retirement, which the scan
/// reads after it.
///
/// Progress. Push and pop are lock-free: a retry means another participant moved
/// `head`, `tail` or a ring, and the helping step lets a push finish what a
/// stalled one started. A push is as strong as the allocator it calls
/// (`A::kProgress`, held to `Required` by a static_assert); a refusal leaves the
/// element where it was and reports `PushStatus::OutOfMemory`.
///
/// Sizes. Each ring is an `Scq<T, N, R>`: `N` elements a segment, `R` slots in
/// its index rings, a power of two at least `N` and at least the most producers
/// or consumers that work the queue at once. The domain has `H` slots, one per
/// live handle, both sides together.
///
/// A queue whose allocator refused the first segment is not `valid()` and hands
/// out no handle.
template <typename T, std::size_t N, std::size_t R, std::size_t H, typename A, Progress Required> class Lscq {
    static_assert(N > 0, "a segment holds at least one element");
    static_assert(R > 0 && (R & (R - 1)) == 0, "the ring size must be a power of two");
    static_assert(R >= N, "the ring must have at least as many slots as the segment");
    static_assert(A::kProgress >= Required, "the allocator gives less progress than the document declared for it");
    static_assert(std::atomic<std::uint64_t>::is_always_lock_free,
                  "the SCQ ring's entries are 64-bit atomics, which this target does not have lock-free");

public:
    using Domain = HazardDomain<H>;

private:
    /// One segment: a ring and the link to the next. The retirement header is its
    /// only base, so a hazard that names the segment names the header too.
    struct Segment : Retired {
        /// The segment after this one, or null while this is the newest. Written
        /// once, by the producer whose compare-and-swap links a successor.
        std::atomic<Segment *> next{nullptr};
        Scq<T, N, R> ring;
    };

    /// Free a retired segment: what the domain calls once no hazard names it.
    static void reclaim(Retired *node, void *context) noexcept {
        A &allocator = *static_cast<A *>(context);
        detail::release(allocator, static_cast<Segment *>(node));
    }

public:
    /// The elements one segment holds.
    static constexpr std::size_t kSegment = N;

    /// The progress the allocator gives, and so the most push can.
    static constexpr Progress kAllocatorProgress = A::kProgress;

    /// An empty queue over `allocator` and `domain`, which it borrows for its
    /// whole life.
    Lscq(A &allocator, Domain &domain) noexcept : Lscq(allocator, domain, detail::place<Segment>(allocator)) {}

    Lscq(const Lscq &) = delete;
    Lscq &operator=(const Lscq &) = delete;

    ~Lscq() {
        // No handle is alive, so every segment from the head is ours alone, and
        // destroying one destroys the elements still in it.
        Segment *segment = head_.value.load(std::memory_order_acquire);
        while (segment != nullptr) {
            Segment *next = segment->next.load(std::memory_order_acquire);
            detail::release(allocator_, segment);
            segment = next;
        }
        // The segments consumers retired are on the domain's list and may still be
        // waiting for a scan; they must be freed before the allocator they name
        // can go.
        domain_.flush();
    }

    /// Whether the allocator gave the first segment.
    bool valid() const noexcept {
        return head_.value.load(std::memory_order_relaxed) != nullptr;
    }

private:
    /// Keeps `Producer` and `Consumer` constructible only through the queue.
    struct Token {
        explicit Token() = default;
    };

public:
    /// A producing side. It holds one producer place and one slot of the domain
    /// until it is destroyed.
    class Producer {
    public:
        Producer(Token, Lscq &queue, typename Domain::Hazard hazard) noexcept
            : queue_(&queue), hazard_(std::move(hazard)) {}

        Producer(Producer &&other) noexcept : queue_(other.queue_), hazard_(std::move(other.hazard_)) {
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

        /// Another producer of the same queue, or `std::nullopt` when every place
        /// or slot is taken.
        std::optional<Producer> try_clone() const noexcept {
            return queue_->producer();
        }

        /// Push `value`, or leave it untouched and report `OutOfMemory` when the
        /// newest segment is closed and the allocator will not give another.
        /// Lock-free when the allocator is.
        PushStatus try_push(T &&value) noexcept {
            Lscq &queue = *queue_;
            for (;;) {
                Segment *tail = hazard_.protect(queue.tail_.value);
                Segment *next = tail->next.load(std::memory_order_acquire);
                if (next != nullptr) {
                    // A producer linked a successor and has not moved `tail`:
                    // finish its step.
                    Segment *expected = tail;
                    queue.tail_.value.compare_exchange_strong(expected, next, std::memory_order_seq_cst,
                                                              std::memory_order_seq_cst);
                    continue;
                }
                if (tail->ring.push_or_close(value)) {
                    hazard_.clear();
                    return PushStatus::Ok;
                }
                // The segment is closed. Put the element in a segment of our own
                // before anyone can see it, then try to link it behind this one.
                Segment *fresh = detail::place<Segment>(queue.allocator_);
                if (fresh == nullptr) {
                    hazard_.clear();
                    return PushStatus::OutOfMemory;
                }
                [[maybe_unused]] const bool pushed = fresh->ring.push_or_close(value);
                assert(pushed && "a new ring is open and has a free slot, so it takes an element");
                Segment *expected = nullptr;
                if (tail->next.compare_exchange_strong(expected, fresh, std::memory_order_acq_rel,
                                                       std::memory_order_acquire)) {
                    Segment *old_tail = tail;
                    queue.tail_.value.compare_exchange_strong(old_tail, fresh, std::memory_order_seq_cst,
                                                              std::memory_order_seq_cst);
                    hazard_.clear();
                    return PushStatus::Ok;
                }
                // Another producer linked first. Take our element back out of the
                // segment nobody saw, free it, and push behind theirs.
                std::optional<T> back = fresh->ring.pop_any();
                assert(back.has_value() && "a segment holding one element gives it back");
                detail::give_back(value, std::move(*back));
                detail::release(queue.allocator_, fresh);
            }
        }

    private:
        Lscq *queue_;
        typename Domain::Hazard hazard_;
    };

    /// A consuming side. It holds one consumer place and one slot of the domain
    /// until it is destroyed.
    class Consumer {
    public:
        Consumer(Token, Lscq &queue, typename Domain::Hazard hazard) noexcept
            : queue_(&queue), hazard_(std::move(hazard)) {}

        Consumer(Consumer &&other) noexcept : queue_(other.queue_), hazard_(std::move(other.hazard_)) {
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

        /// Another consumer of the same queue, or `std::nullopt` when every place
        /// or slot is taken.
        std::optional<Consumer> try_clone() const noexcept {
            return queue_->consumer();
        }

        /// Pop the oldest element, or `std::nullopt` when the queue is empty.
        /// Lock-free.
        std::optional<T> try_pop() noexcept {
            Lscq &queue = *queue_;
            for (;;) {
                Segment *head = hazard_.protect(queue.head_.value);
                if (std::optional<T> value = head->ring.pop_any()) {
                    hazard_.clear();
                    return value;
                }
                Segment *next = head->next.load(std::memory_order_acquire);
                if (next == nullptr) {
                    hazard_.clear();
                    return std::nullopt;
                }
                // The segment is closed. An enqueue that took a ticket before the
                // close may still be filling its entry, so the empty answer above
                // does not yet show the segment holds nothing and never will.
                if (std::optional<T> value = head->ring.pop_drained()) {
                    hazard_.clear();
                    return value;
                }
                // Take `tail` off the segment before it is unlinked from `head`: a
                // producer validates its hazard on the segment against `tail`, so
                // `tail` must stop naming the segment before the segment can be
                // retired (the class comment, *Where a hazard is validated*). The
                // successor exists, so `tail` may legally move to it; if another
                // participant already moved it, the exchange fails and reads that
                // move.
                Segment *tail_expected = head;
                queue.tail_.value.compare_exchange_strong(tail_expected, next, std::memory_order_seq_cst,
                                                          std::memory_order_seq_cst);
                Segment *expected = head;
                if (queue.head_.value.compare_exchange_strong(expected, next, std::memory_order_seq_cst,
                                                              std::memory_order_seq_cst)) {
                    // This consumer moved `head` past the segment, so it alone
                    // retires it. Its own hazard goes first, or the scan the
                    // retirement may start would keep the segment for it.
                    hazard_.clear();
                    queue.domain_.retire(static_cast<Retired *>(head), &Lscq::reclaim,
                                         static_cast<void *>(std::addressof(queue.allocator_)));
                }
            }
        }

    private:
        Lscq *queue_;
        typename Domain::Hazard hazard_;
    };

    /// A producer handle, or `std::nullopt` when `R` producers are already alive,
    /// the domain has no free slot, or the queue is not valid. Destroying a handle
    /// gives its place back.
    std::optional<Producer> producer() noexcept {
        if (!valid() || !detail::take_place(producers_, R)) {
            return std::nullopt;
        }
        std::optional<typename Domain::Hazard> hazard = domain_.acquire();
        if (!hazard) {
            producers_.fetch_sub(1, std::memory_order_release);
            return std::nullopt;
        }
        return std::optional<Producer>(std::in_place, Token{}, *this, std::move(*hazard));
    }

    /// A consumer handle, or `std::nullopt` when `R` consumers are already alive,
    /// the domain has no free slot, or the queue is not valid. Destroying a handle
    /// gives its place back.
    std::optional<Consumer> consumer() noexcept {
        if (!valid() || !detail::take_place(consumers_, R)) {
            return std::nullopt;
        }
        std::optional<typename Domain::Hazard> hazard = domain_.acquire();
        if (!hazard) {
            consumers_.fetch_sub(1, std::memory_order_release);
            return std::nullopt;
        }
        return std::optional<Consumer>(std::in_place, Token{}, *this, std::move(*hazard));
    }

private:
    Lscq(A &allocator, Domain &domain, Segment *first) noexcept
        : head_(first), tail_(first), allocator_(allocator), domain_(domain) {
        // A hazard names a segment by its address and the scan compares it with
        // the header's; the header is the segment's only base, which every ABI
        // lays out at offset zero.
        assert(first == nullptr || static_cast<void *>(static_cast<Retired *>(first)) == static_cast<void *>(first));
    }

    /// The oldest segment. Moved forward by the consumer that finds it empty and
    /// closed.
    detail::Padded<std::atomic<Segment *>> head_;
    /// The newest segment, or the one before it while a producer is between
    /// linking a successor and moving `tail`.
    detail::Padded<std::atomic<Segment *>> tail_;
    /// How many producer and consumer handles are alive.
    std::atomic<std::size_t> producers_{0};
    std::atomic<std::size_t> consumers_{0};
    A &allocator_;
    Domain &domain_;
};

}  // namespace SCE::Forge::Queue

#endif  // SCE_FORGE_QUEUE_SEGMENTED_H
