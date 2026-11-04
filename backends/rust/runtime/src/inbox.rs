// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The inbox of an engine: the one part of it that a thread which does not own
//! the engine may use (docs/adr/0084, ARCHITECTURE.md "Engine Threading").
//!
//! What may be posted is closed ([`InboxInput`]). Each input carries what the
//! owner-thread operation of the same name takes, and the owner applies it by
//! calling that operation, at its next macrostep boundary, before the external
//! queue is consulted, in the order the inbox took the posts. A posted input and
//! a direct call therefore cannot differ in what they do.
//!
//! One inbox per profile:
//!
//! - `EngineInbox`, the `std` profile's: the engine's own, handed out by
//!   `Engine::inbox`. Unbounded, on the standard library's channel, whose send
//!   takes no lock. A post is refused only once the engine is gone.
//! - `BoundedInbox`, the bare-metal profile's: declared by the deployment in a
//!   `static`, of the capacity it names, and handed to the engine with
//!   `Engine::attach_inbox`. A post to a full one is refused. It is present on
//!   every target with compare-and-swap of a byte and of a pointer, so a
//!   bare-metal crate still compiles in a build that turns `std` on
//!   (docs/adr/0053).
//!
//! A refused post gives its input back ([`InboxRefusal`]), so nothing posted is
//! dropped without the poster being told.
//!
//! Neither inbox closes when a run ends. A post taken after the run reached a
//! top-level final state, or was stopped, is applied where the owner next
//! reaches the end of its loop, as the direct call made at that moment would
//! be: an external event is counted with the machine's unseen events
//! (`Engine::unseen_external_events`), and an ending finds its invocation over.
//! A closing gate would need the owner to wait for the posts in flight to
//! finish, which a bare-metal owner sharing one core with its posters cannot
//! do.
//!
//! A Mesh delivery is not an input here. The Rust Mesh endpoint and a child
//! session's router hold what arrives until the owner pulls it
//! (`Engine::apply_mesh_calls`, the endpoint crate's `apply_to`), so no Mesh
//! delivery has a caller on another thread, and the set is the inputs that
//! have one.

#[cfg(feature = "std")]
use std::sync::{mpsc, Arc, Mutex, PoisonError};

#[cfg(any(
    feature = "std",
    all(target_has_atomic = "8", target_has_atomic = "ptr")
))]
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::event::{EventMetadata, EventWithMetadata};
#[cfg(any(
    feature = "std",
    all(target_has_atomic = "8", target_has_atomic = "ptr")
))]
use crate::hal::Hal;
use crate::policy::StatePolicy;

/// One input a thread that does not own an engine may post to its inbox.
///
/// `E` is the machine's event type and `D` its payload type; [`InboxInputOf`]
/// names the input of a machine by its policy. Marked `non_exhaustive` because
/// the `std` profile adds arms the bare-metal profile does not have.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InboxInput<E, D = ()> {
    /// An external event, applied with `Engine::raise_external_with_meta`.
    /// [`InboxInput::external`] builds the one `Engine::raise_external` raises.
    ExternalEvent(EventWithMetadata<E, D>),
    /// An external event named by text, applied with
    /// `Engine::raise_external_by_name_with_meta`: delivered as the document's
    /// event of that name, or of the longest token prefix of it the document
    /// writes, when the input is applied, and dropped then if the document has
    /// none.
    #[cfg(feature = "std")]
    ExternalEventByName {
        /// The name the event arrived under.
        name: String,
        /// Its `_event` fields.
        metadata: EventMetadata,
    },
    /// The end of one start of a host-run invocation. With no `session_id` it
    /// ends one of the machine whose inbox this is, applied with
    /// `Engine::complete_host_invoke` or `Engine::fail_host_invoke`; with one it
    /// ends one of that session of the tree, applied with
    /// `Engine::end_host_invoke_in`. The token check those make is made when the
    /// input is applied, so an ending posted for a start that was cancelled or
    /// started again by then raises nothing, as a direct one would.
    #[cfg(feature = "std")]
    HostInvokeEnding {
        /// The session of the tree whose invocation ends; `None` for the
        /// machine whose inbox this is.
        session_id: Option<String>,
        /// The `type` the request named.
        processor_type: String,
        /// The invocation's id, as the request named it.
        invoke_id: String,
        /// The token the request carried.
        token: u64,
        /// `true` for a completion (`done.invoke`), `false` for a failure
        /// (`error.invoke`).
        done: bool,
        /// `_event.data` of the event the ending raises.
        data: String,
    },
}

/// The input of the inbox of an engine whose policy is `P`.
pub type InboxInputOf<P> = InboxInput<<P as StatePolicy>::Event, <P as StatePolicy>::Payload>;

impl<E, D: Default> InboxInput<E, D> {
    /// The external event `Engine::raise_external(event, data, origin)` raises,
    /// as an input: `data` as `_event.data` and `origin` as `_event.origin`.
    /// Both are dropped in the bare-metal profile, as that call drops them.
    pub fn external(event: E, data: &str, origin: &str) -> Self {
        let mut raised = EventWithMetadata::new(event);
        raised.metadata = EventMetadata::external("", origin);
        raised.set_event_data(data);
        InboxInput::ExternalEvent(raised)
    }
}

#[cfg(feature = "std")]
impl<E, D> InboxInput<E, D> {
    /// The external event named `name`, carrying `data` as `_event.data`: what
    /// `Engine::raise_external_by_name(name, data)` raises, as an input.
    pub fn external_by_name(name: &str, data: &str) -> Self {
        let mut metadata = EventMetadata::external("", "");
        metadata.data = crate::sce_string_from_str(data);
        InboxInput::ExternalEventByName {
            name: name.to_string(),
            metadata,
        }
    }

    /// `ending`, of the machine whose inbox this is when `session_id` is
    /// `None` and of that session of the tree otherwise, as an input.
    pub fn host_invoke_ending(
        session_id: Option<&str>,
        ending: crate::host_processor::HostInvokeEnding<'_>,
    ) -> Self {
        InboxInput::HostInvokeEnding {
            session_id: session_id.map(str::to_string),
            processor_type: ending.processor_type.to_string(),
            invoke_id: ending.invoke_id.to_string(),
            token: ending.token,
            done: ending.done,
            data: ending.data.to_string(),
        }
    }
}

/// Why an inbox did not take a post. The input comes back, so the poster
/// decides what becomes of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxRefusal<I> {
    /// A bounded inbox held its capacity.
    Full(I),
    /// The engine the inbox belonged to is gone, so nothing would apply it.
    EngineGone(I),
}

impl<I> InboxRefusal<I> {
    /// The input the inbox did not take.
    pub fn into_inner(self) -> I {
        match self {
            InboxRefusal::Full(input) | InboxRefusal::EngineGone(input) => input,
        }
    }
}

/// What an engine drains at each macrostep boundary: an inbox seen from its
/// owner.
pub(crate) trait InboxSource<I> {
    /// The oldest input posted and not yet taken, if any.
    fn take(&self) -> Option<I>;
    /// How many inputs one boundary takes at most, so posters that never stop
    /// cannot hold the owner at a boundary.
    fn per_boundary(&self) -> usize;
}

/// An inbox a deployment declared in a `static` and attached to the engine
/// whose policy is `P`, as the engine holds it.
pub(crate) type AttachedInbox<P> = &'static (dyn InboxSource<InboxInputOf<P>> + Sync);

// ─────────────────────────────────────────────────────────────────────
// The std profile: the engine's own inbox, unbounded
// ─────────────────────────────────────────────────────────────────────

/// A waker a host installs on an engine's inbox (`EngineInbox::wake_with`),
/// called on the posting thread after each post the inbox took.
#[cfg(feature = "std")]
pub type InboxWaker = Arc<dyn Fn() + Send + Sync>;

/// What an engine's inbox and every handle to it share.
#[cfg(feature = "std")]
struct Shared<I> {
    /// The channel's sending half. `Sync`, so every handle posts through this
    /// one without a lock.
    sender: mpsc::Sender<I>,
    /// Posts taken and not yet taken off by the owner: what a boundary drains,
    /// read once when it is reached.
    held: AtomicUsize,
    /// What `EngineInbox::wake_with` installed.
    waker: Mutex<Option<InboxWaker>>,
}

/// The engine's own hold on its inbox in the `std` profile: the receiving half
/// of the channel, which only the owner reads, and what the handles share.
/// Dropped with the engine, which is what refuses a post made after it.
#[cfg(feature = "std")]
pub(crate) struct OwnedInbox<I> {
    shared: Arc<Shared<I>>,
    receiver: mpsc::Receiver<I>,
}

#[cfg(feature = "std")]
impl<I> OwnedInbox<I> {
    /// An empty inbox.
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            shared: Arc::new(Shared {
                sender,
                held: AtomicUsize::new(0),
                waker: Mutex::new(None),
            }),
            receiver,
        }
    }
}

#[cfg(feature = "std")]
impl<I> InboxSource<I> for OwnedInbox<I> {
    fn take(&self) -> Option<I> {
        let input = self.receiver.try_recv().ok()?;
        self.shared.held.fetch_sub(1, Ordering::AcqRel);
        Some(input)
    }

    /// What the inbox held when the boundary was reached: a post made while
    /// the owner applies these waits for the next boundary.
    fn per_boundary(&self) -> usize {
        self.shared.held.load(Ordering::Acquire)
    }
}

/// The handle a thread that does not own an engine posts to its inbox through
/// (`Engine::inbox`), in the `std` profile.
///
/// `Send` and `Sync` whenever the machine's events and payloads are, which a
/// generated machine's are; cheap to clone, and every clone names the same
/// inbox. It stays valid after the engine is gone, and its posts are refused
/// from then on.
#[cfg(feature = "std")]
pub struct EngineInbox<P: StatePolicy> {
    shared: Arc<Shared<InboxInputOf<P>>>,
}

#[cfg(feature = "std")]
impl<P: StatePolicy> Clone for EngineInbox<P> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

#[cfg(feature = "std")]
impl<P: StatePolicy> EngineInbox<P> {
    /// A handle to `owned`.
    pub(crate) fn of(owned: &OwnedInbox<InboxInputOf<P>>) -> Self {
        Self {
            shared: Arc::clone(&owned.shared),
        }
    }

    /// Post `input`. Taken unless the engine is gone, and then the owner applies
    /// it at its next macrostep boundary. After a post the inbox took, the
    /// policy's HAL is woken (`Hal::wake`) and then the waker installed with
    /// [`wake_with`](Self::wake_with), both on this thread.
    ///
    /// # Errors
    ///
    /// [`InboxRefusal::EngineGone`], with the input, when the engine was dropped.
    pub fn post(&self, input: InboxInputOf<P>) -> Result<(), InboxRefusal<InboxInputOf<P>>> {
        self.shared
            .sender
            .send(input)
            .map_err(|refused| InboxRefusal::EngineGone(refused.0))?;
        self.shared.held.fetch_add(1, Ordering::AcqRel);
        <P::Hal as Hal>::wake();
        let waker = self
            .shared
            .waker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(waker) = waker {
            (*waker)();
        }
        Ok(())
    }

    /// Install `waker`, called on the posting thread after each post the inbox
    /// took, so a host that drives the engine itself (`step`, `tick`) learns
    /// there is something to apply. `None` removes it. One waker per inbox:
    /// installing through any handle replaces what any handle installed.
    ///
    /// The policy's HAL is woken as well; this is for a host whose machine's
    /// HAL it does not choose, as a generated `std` machine's is
    /// [`StdHal`](crate::StdHal), whose wake does nothing.
    pub fn wake_with(&self, waker: Option<InboxWaker>) {
        *self
            .shared
            .waker
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = waker;
    }
}

// ─────────────────────────────────────────────────────────────────────
// The bare-metal profile: an inbox the deployment declares, bounded
// ─────────────────────────────────────────────────────────────────────

/// An inbox of `N` inputs that a deployment declares in a `static` and hands
/// to the engine whose policy is `P` with `Engine::attach_inbox`: the bare-metal
/// profile's door for an interrupt handler or another task. Present in the
/// `std` profile too, as every name of the bare-metal profile is
/// (docs/adr/0053).
///
/// ```ignore
/// static INBOX: BoundedInbox<MachinePolicy, 8, BoardHal> = BoundedInbox::new();
///
/// engine.attach_inbox(&INBOX);
/// // in an interrupt handler:
/// let _ = INBOX.post(InboxInput::external(MachineEvent::ButtonPressed, "", ""));
/// ```
///
/// `N` is a power of two from 2 to 128; another value fails to compile where the
/// `static` is declared. The queue is Vyukov's bounded multi-producer queue
/// (`heapless::mpmc`): a post takes no lock and never waits, so an interrupt
/// handler may post. The owner takes at most `N` inputs at one boundary.
///
/// `W` is the HAL whose `Hal::wake` a post the inbox took calls, on the posting
/// thread: the waker the deployment installs. A generated bare-metal machine's
/// own HAL is `NoOpHal`, whose wake stops the program, so the deployment names
/// its board's HAL here.
///
/// The queue does not drop what it still holds when it is dropped. In a
/// `static`, which is what it is for, it is never dropped.
#[cfg(all(target_has_atomic = "8", target_has_atomic = "ptr"))]
pub struct BoundedInbox<P: StatePolicy, const N: usize, W> {
    queue: ::heapless::mpmc::MpMcQueue<InboxInputOf<P>, N>,
    /// Posts refused because the inbox was full.
    refused: AtomicUsize,
    waker: core::marker::PhantomData<fn() -> W>,
}

#[cfg(all(target_has_atomic = "8", target_has_atomic = "ptr"))]
impl<P: StatePolicy, const N: usize, W: Hal> BoundedInbox<P, N, W> {
    /// An empty inbox, for a `static`.
    pub const fn new() -> Self {
        Self {
            queue: ::heapless::mpmc::MpMcQueue::new(),
            refused: AtomicUsize::new(0),
            waker: core::marker::PhantomData,
        }
    }

    /// Post `input`. Taken unless the inbox holds `N` inputs, and then the
    /// owner applies it at its next macrostep boundary. After a post the inbox
    /// took, `W`'s `Hal::wake` is called on this thread.
    ///
    /// # Errors
    ///
    /// [`InboxRefusal::Full`], with the input, when the inbox holds `N` inputs;
    /// the refusal is counted ([`refused_posts`](Self::refused_posts)).
    pub fn post(&self, input: InboxInputOf<P>) -> Result<(), InboxRefusal<InboxInputOf<P>>> {
        if let Err(input) = self.queue.enqueue(input) {
            // A count that stops at its ceiling rather than wraps to zero, which
            // would read as no refusal at all.
            let _ = self
                .refused
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    Some(count.saturating_add(1))
                });
            return Err(InboxRefusal::Full(input));
        }
        W::wake();
        Ok(())
    }

    /// How many posts this inbox refused because it was full: what a poster
    /// that could not act on the refusal, an interrupt handler among them,
    /// leaves the deployment to read.
    pub fn refused_posts(&self) -> usize {
        self.refused.load(Ordering::Relaxed)
    }
}

#[cfg(all(target_has_atomic = "8", target_has_atomic = "ptr"))]
impl<P: StatePolicy, const N: usize, W: Hal> Default for BoundedInbox<P, N, W> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(target_has_atomic = "8", target_has_atomic = "ptr"))]
impl<P: StatePolicy, const N: usize, W> InboxSource<InboxInputOf<P>> for BoundedInbox<P, N, W> {
    fn take(&self) -> Option<InboxInputOf<P>> {
        self.queue.dequeue()
    }

    fn per_boundary(&self) -> usize {
        N
    }
}
