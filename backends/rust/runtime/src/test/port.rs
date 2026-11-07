// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
//! received messages and the snapshot of its head (ES 201 873-4 §8.3.3).

use std::any::Any;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

/// An instant of the logical clock `NOW`: microseconds since the test case
/// started (docs/SCE_TEST_PROFILE.md §3.3).
pub type Instant = i64;

/// A length of logical time, in microseconds.
pub type Duration = i64;

/// The value a message carries: a record of the type the statechart generator
/// writes for its event schema, read back by that type.
///
/// Shared and not boxed because it rides on a `HostSendRequest`, which is
/// cloned (`HostSendRequest::value`); a value is never changed once sent, so a
/// clone of the request and the message it becomes read the same record.
pub type MessageValue = Arc<dyn Any + Send + Sync>;

/// A message on a port: the event name of its record and the record's value.
///
/// The value is the record type the statechart generator writes for the
/// `event-schema` that declares it; a template's matcher reads it as that type
/// ([`Message::value_as`]). Matching is on the value and never on an encoding
/// (ADR 0091 C2).
#[derive(Clone)]
pub struct Message {
    /// The event name of the message's record.
    pub event: String,
    /// The record's value, read back by its type.
    pub value: MessageValue,
}

impl Message {
    /// A message of the record `event` whose value is `value`.
    pub fn new<V: Any + Send + Sync>(event: impl Into<String>, value: V) -> Self {
        Message {
            event: event.into(),
            value: Arc::new(value),
        }
    }

    /// The value, when it is a `V`; none when it is of another type, which a
    /// template of `V` does not match.
    pub fn value_as<V: Any>(&self) -> Option<&V> {
        self.value.downcast_ref::<V>()
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
