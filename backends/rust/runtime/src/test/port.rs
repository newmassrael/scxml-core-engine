// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
//! received messages and the snapshot of its head (ES 201 873-4 §8.3.3).

use std::any::Any;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use crate::json::Value;

/// An instant of the logical clock `NOW`: microseconds since the test case
/// started (docs/SCE_TEST_PROFILE.md §3.3).
pub type Instant = i64;

/// A length of logical time, in microseconds.
pub type Duration = i64;

/// A value a message carries, and its canonical form in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5).
///
/// The value is read back by its type ([`Message::value_as`]); its canonical
/// form is written by the types its document declares ([`super::canonical`]),
/// which its Rust type alone does not say. [`message_value`] makes one of a
/// value and the writer of its form, which is what a generated send and a
/// system adapter give; a host may implement it on a type of its own.
pub trait TestValue: Any + Send + Sync {
    /// The value, to be read back by its type.
    fn as_any(&self) -> &dyn Any;

    /// The value in the canonical form of the test log: built from texts,
    /// truth values, arrays and objects alone ([`super::canonical`]), so that
    /// the JSON Canonicalization Scheme writes it as the same bytes on every
    /// runtime. It runs while the run's state is held, so it reads the value
    /// and nothing else.
    fn canonical(&self) -> Value;
}

impl fmt::Debug for dyn TestValue {
    // The value's type is not known here, so only that there is one is shown.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestValue").finish_non_exhaustive()
    }
}

/// The value a message carries: a record of the type the event schema's unit
/// declares, read back by that type, with the writer of its canonical form.
///
/// Shared and not boxed because it rides on a `HostSendRequest`, which is
/// cloned (`HostSendRequest::value`); a value is never changed once sent, so a
/// clone of the request and the message it becomes read the same record.
pub type MessageValue = Arc<dyn TestValue>;

/// A value and the writer of its canonical form: what [`message_value`] makes.
struct WithCanonical<V> {
    value: V,
    canonical: fn(&V) -> Value,
}

impl<V: Any + Send + Sync> TestValue for WithCanonical<V> {
    fn as_any(&self) -> &dyn Any {
        &self.value
    }

    fn canonical(&self) -> Value {
        (self.canonical)(&self.value)
    }
}

/// `value` as a message's value, whose canonical form `canonical` writes: the
/// `canonical` of the unit of a template over its record, as a generated send
/// gives it (`<template>::canonical`).
pub fn message_value<V: Any + Send + Sync>(value: V, canonical: fn(&V) -> Value) -> MessageValue {
    Arc::new(WithCanonical { value, canonical })
}

/// A message on a port: the event name of its record and the record's value.
///
/// The value is of the record type the unit of its `event-schema` declares; a
/// template's matcher reads it as that type
/// ([`Message::value_as`]). Matching is on the value and never on an encoding
/// (ADR 0091 C2), and so is the test log ([`Message::canonical_value`]).
#[derive(Clone)]
pub struct Message {
    /// The event name of the message's record.
    pub event: String,
    /// The record's value, read back by its type.
    pub value: MessageValue,
}

impl Message {
    /// A message of the record `event` whose value is `value`, whose canonical
    /// form `canonical` writes ([`message_value`]).
    pub fn new<V: Any + Send + Sync>(
        event: impl Into<String>,
        value: V,
        canonical: fn(&V) -> Value,
    ) -> Self {
        Message {
            event: event.into(),
            value: message_value(value, canonical),
        }
    }

    /// The value, when it is a `V`; none when it is of another type, which a
    /// template of `V` does not match.
    pub fn value_as<V: Any>(&self) -> Option<&V> {
        self.value.as_any().downcast_ref::<V>()
    }

    /// The value in the canonical form of the test log.
    pub fn canonical_value(&self) -> Value {
        self.value.canonical()
    }
}

impl fmt::Debug for Message {
    // The value's type is not known here, so only the event is shown.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("event", &self.event)
            .finish_non_exhaustive()
    }
}

/// One component port: its queue and its snapshot (-4 §8.3.3).
///
/// Only the component that owns the port removes from it, so between the
/// snapshot and the removal of a branch the head is the message the snapshot
/// saw; what others append goes to the back. The snapshot records only whether
/// the queue held a message: a message that arrives in an empty queue after the
/// snapshot is not in it.
#[derive(Debug, Default)]
pub struct Port {
    queue: VecDeque<Message>,
    snapshot_holds: bool,
}

impl Port {
    /// A message a connected port sent, or one from the SUT on a mapped port,
    /// appended in arrival order (-4 §8.3.3.1).
    pub fn enqueue(&mut self, message: Message) {
        self.queue.push_back(message);
    }

    /// `SNAP-VALUE`: the head of the queue, taken in a round's snapshot step
    /// (docs/SCE_TEST_PROFILE.md §3.2 step 3).
    pub fn take_snapshot(&mut self) {
        self.snapshot_holds = !self.queue.is_empty();
    }

    /// The message the last snapshot saw at the head, or none.
    pub fn snapshot(&self) -> Option<&Message> {
        if self.snapshot_holds {
            self.queue.front()
        } else {
            None
        }
    }

    /// `remove-from-port` of a receive branch that was taken on the snapshot
    /// (-4 §9.37.1, P4 step 1).
    ///
    /// # Panics
    ///
    /// When the queue is empty. The caller holds that the snapshot saw a
    /// message, which a branch's guard established, so an empty queue here is a
    /// machine that took a receive branch its guard did not allow.
    pub fn remove_head(&mut self) -> Message {
        let head = self
            .queue
            .pop_front()
            .expect("a receive branch is taken only on a snapshot that saw a message");
        self.snapshot_holds = false;
        head
    }

    /// How many messages the queue holds.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue holds no message.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::canonical;

    /// A message's value is read back by its type, and by no other, and its
    /// canonical form is the one its writer gives, whichever clone reads it.
    #[test]
    fn a_message_value_is_read_by_its_type_and_written_by_its_writer() {
        let message = Message::new("number", 7_u32, |n| canonical::integer(*n));
        assert_eq!(message.value_as::<u32>(), Some(&7));
        assert_eq!(message.value_as::<i32>(), None);
        let copy = message.clone();
        assert_eq!(copy.canonical_value(), Value::Text("7".to_string()));
        assert_eq!(format!("{:?}", message.value), "TestValue { .. }");
    }
}
