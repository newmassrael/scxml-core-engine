// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
// received messages and the snapshot of its head (ES 201 873-4 §8.3.3).

#pragma once

#include <any>
#include <cstdint>
#include <deque>
#include <memory>
#include <optional>
#include <string>
#include <utility>

#include "saved/SavedJson.h"

namespace SCE::Test {

/// An instant of the logical clock `NOW`: microseconds since the test case
/// started (docs/SCE_TEST_PROFILE.md §3.3).
using Instant = std::int64_t;

/// A length of logical time, in microseconds.
using Duration = std::int64_t;

/// The writer of the canonical form of a value of `V` in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5): the `canonical` of the unit of a template
/// over its record.
template <typename V> using CanonicalWriter = SCE::Saved::JsonValue (*)(const V &);

namespace detail {

/// `T` itself, in a context no template argument is deduced from, so that a
/// writer given as a lambda converts to the function pointer the value's type
/// names rather than failing the deduction.
template <typename T> struct Undeduced {
    using type = T;
};

}  // namespace detail

/**
 * @brief A value a message carries, its type's name and its canonical form in
 *        the test log (docs/SCE_TEST_PROFILE.md §12.5)
 *
 * The value is read back by its type (as()); its canonical form is written by
 * the types its document declares (test/Canonical.h), which its C++ type alone
 * does not say. messageValue() makes one of a value, its type's name and the
 * writer of its form, which is what a generated send and a system adapter
 * give.
 *
 * Shared rather than copied because it rides on a `HostSendRequest`, whose
 * `value` holds it (`std::any`), and a request is copied; a value is never
 * changed once sent, so every copy reads the same record. The Rust sibling is
 * `sce_rust_runtime::test::MessageValue`.
 */
class MessageValue {
public:
    template <typename V>
    MessageValue(V value, std::string typeName, typename detail::Undeduced<CanonicalWriter<V>>::type canonical)
        : held_(std::make_shared<Held<V>>(std::move(value), canonical)), typeName_(std::move(typeName)) {}

    /// The value, when it is a `V`; none when it is of another type, which a
    /// template of `V` does not match.
    template <typename V> const V *as() const {
        const auto *held = dynamic_cast<const Held<V> *>(held_.get());
        return held == nullptr ? nullptr : &held->value;
    }

    /// The value as a `V`, which the caller holds it is, as a receive branch
    /// whose guard matched the message holds; std::bad_any_cast when it is of
    /// another type.
    template <typename V> const V &get() const {
        const V *value = as<V>();
        if (value == nullptr) {
            throw std::bad_any_cast();
        }
        return *value;
    }

    /// The name of the value's record as the types module of the suite's
    /// TTCN-3 export names it (docs/SCE_TEST_PROFILE.md §9.5): the unit name of
    /// its `event-schema` document. A value in TCI always carries its type, and
    /// so does a message of the test log.
    const std::string &typeName() const {
        return typeName_;
    }

    /// The value in the canonical form of the test log: built from texts, truth
    /// values, arrays and objects alone (test/Canonical.h), so that the JSON
    /// Canonicalization Scheme writes it as the same bytes on every runtime.
    SCE::Saved::JsonValue canonical() const {
        return held_->canonical();
    }

private:
    struct Base {
        virtual ~Base() = default;
        virtual SCE::Saved::JsonValue canonical() const = 0;
    };

    template <typename V> struct Held final : Base {
        Held(V held, CanonicalWriter<V> writer) : value(std::move(held)), write(writer) {}

        SCE::Saved::JsonValue canonical() const override {
            return write(value);
        }

        V value;
        CanonicalWriter<V> write;
    };

    std::shared_ptr<const Base> held_;
    std::string typeName_;
};

/// @p value as a message's value, of the record @p typeName, whose canonical
/// form @p canonical writes: the `TYPE` and the `canonical` of the unit of a
/// template over its record, as a generated send gives them
/// (`<template>::TYPE`, `<template>::canonical`).
template <typename V>
MessageValue messageValue(V value, std::string typeName,
                          typename detail::Undeduced<CanonicalWriter<V>>::type canonical) {
    return MessageValue(std::move(value), std::move(typeName), canonical);
}

/**
 * @brief A message on a port: the event name of its record and the record's value
 *
 * The value is of the record type the unit of its `event-schema` declares; a
 * template's matcher reads it as that type (MessageValue::as()). Matching is on
 * the value and never on an encoding (ADR 0091 C2), and so is the test log
 * (MessageValue::canonical()).
 */
struct Message {
    std::string event;
    MessageValue value;
};

/**
 * @brief One component port: its queue and its snapshot (-4 §8.3.3)
 *
 * Only the component that owns the port removes from it, so between the
 * snapshot and the removal of a branch the head is the message the snapshot
 * saw; what others append goes to the back. The snapshot records only whether
 * the queue held a message: a message that arrives in an empty queue after the
 * snapshot is not in it.
 */
class Port {
public:
    /// A message a connected port sent, or one from the SUT on a mapped port,
    /// appended in arrival order (-4 §8.3.3.1).
    void enqueue(Message message) {
        queue_.push_back(std::move(message));
    }

    /// `SNAP-VALUE`: the head of the queue, taken in a round's snapshot step
    /// (docs/SCE_TEST_PROFILE.md §3.2 step 3).
    void takeSnapshot() {
        snapshotHolds_ = !queue_.empty();
    }

    /// The message the last snapshot saw at the head, or none.
    const Message *snapshot() const {
        return snapshotHolds_ ? &queue_.front() : nullptr;
    }

    /// `remove-from-port` of a receive branch that was taken on the snapshot
    /// (-4 §9.37.1, P4 step 1). The caller holds that the snapshot saw a
    /// message, which a branch's guard established.
    Message removeHead() {
        Message head = std::move(queue_.front());
        queue_.pop_front();
        snapshotHolds_ = false;
        return head;
    }

    std::size_t size() const {
        return queue_.size();
    }

private:
    std::deque<Message> queue_;
    bool snapshotHolds_ = false;
};

}  // namespace SCE::Test
