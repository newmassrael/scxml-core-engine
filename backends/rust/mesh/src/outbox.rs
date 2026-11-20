// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The outboxes of a server's clients under `receipt: durable` (docs/adr/0109, D10; SCE_MESH.md
//! sections 10.11 and 22.3), as the process holds them: a cache of what the session store keeps.
//!
//! What a session of a keyed pool sends a client whose `https` accepting binding is durable is
//! appended to the client's outbox by the same store write that keeps the session's checkpoint
//! (`SessionStore::write`), so the client's mailbox holds nothing a crash of the server could lose,
//! and the session holds nothing for it once the write answered. [`Outboxes`] is what the pool and
//! the server share of it:
//!
//! - **A cache of the store.** A client's outbox is read from the store at its first use in the
//!   process, and kept in step after: the pool adds what each write appended
//!   ([`Outboxes::committed`]), and an acknowledgement deletes in the store first and then here
//!   ([`Outboxes::acknowledge`]). The server answers an exchange from it
//!   ([`Outboxes::response`]), and is woken when an append arrives ([`Outboxes::watch`]), which
//!   ends a held exchange.
//! - **The durable window.** The binding's `mailbox` bounds the client's outbox together with what
//!   the pool's sessions produced for it and no write appended yet. Every session's router takes a
//!   place for each envelope it writes to the client ([`Outboxes::window`], a
//!   [`crate::router::Window`]), and one that finds none is refused as SCE_MESH.md section 16.7
//!   row 9 at admit: a send that was never accepted is told, and nothing accepted is dropped. The
//!   place is given back when the envelope is acknowledged, or when the session that held it was
//!   dropped before any write appended it ([`Outboxes::unreserve`]).
//! - **Expiry.** An entry older than the binding's `retain_ms` is deleted by the store, which
//!   notes it for the session that sent it in the same transaction ([`Outboxes::expire`]).
//!
//! The numbering is the store's: an outbox numbers in the incarnation it was opened in, and a
//! restart keeps both, so a client acknowledges across a restart of its server.

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::{Mutex, MutexGuard, PoisonError};

use sce_rust_session_store::{Appended, Expiry, Failure, Outbox, SessionStore};

use crate::peers::{Machine, PeerLink, Receipt};
use crate::router::Window;

/// What an accepting binding under `receipt: durable` keeps of one client (docs/adr/0109, D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxBinding {
    /// deploy.yaml's `mailbox`: the durable window, the envelopes the client's outbox and what is
    /// produced for it and not yet appended hold at most.
    pub window: usize,
    /// deploy.yaml's `retain_ms`: how long an entry is kept unless the client acknowledges it.
    pub retain_ms: u64,
}

/// What the server is told when an append arrives for a client: the client's name.
pub type Wake = Arc<dyn Fn(&str) + Send + Sync>;

/// The outboxes of a server's durable clients. See the module documentation.
pub struct Outboxes {
    store: Arc<dyn SessionStore>,
    bindings: BTreeMap<String, OutboxBinding>,
    clients: Mutex<BTreeMap<String, Client>>,
    wake: Mutex<Option<Wake>>,
}

/// One client's outbox as the process holds it.
#[derive(Default)]
struct Client {
    /// What the store holds, once read.
    held: Option<Held>,
    /// The places taken by what the sessions produced for the client that no write appended yet.
    reserved: usize,
}

/// What the store holds of one client's outbox.
struct Held {
    incarnation: String,
    entries: VecDeque<Cached>,
}

/// An entry of an outbox: its number, when it was appended, and its envelope.
struct Cached {
    number: u64,
    at_ms: u64,
    envelope: Vec<u8>,
}

impl Held {
    fn of(outbox: Outbox) -> Self {
        Held {
            incarnation: outbox.incarnation,
            entries: outbox
                .entries
                .into_iter()
                .map(|entry| Cached {
                    number: entry.number,
                    at_ms: entry.at_ms,
                    envelope: entry.envelope,
                })
                .collect(),
        }
    }
}

/// What a response carries of an outbox (SCE_MESH.md section 22.3): the incarnation it numbers
/// in, the number of its first envelope (`None` when it carries none), and the envelopes as a
/// CBOR sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    pub incarnation: String,
    pub first: Option<u64>,
    pub body: Vec<u8>,
}

impl Outboxes {
    /// The outboxes of the clients `bindings` names, kept in `store`.
    pub fn new(store: Arc<dyn SessionStore>, bindings: BTreeMap<String, OutboxBinding>) -> Self {
        Outboxes {
            store,
            bindings,
            clients: Mutex::new(BTreeMap::new()),
            wake: Mutex::new(None),
        }
    }

    /// The outboxes of `machine`'s clients whose `https` accepting binding declares `receipt:
    /// durable`, kept in `store`; `None` when it has none.
    pub fn of_machine(machine: &Machine, store: Arc<dyn SessionStore>) -> Option<Arc<Self>> {
        let bindings: BTreeMap<String, OutboxBinding> = machine
            .peers
            .iter()
            .filter_map(|peer| match peer.link {
                PeerLink::HttpsAccept {
                    receipt: Receipt::Durable { .. },
                    mailbox,
                    retain_ms,
                    ..
                } => Some((
                    peer.name.to_string(),
                    OutboxBinding {
                        window: mailbox as usize,
                        retain_ms,
                    },
                )),
                _ => None,
            })
            .collect();
        (!bindings.is_empty()).then(|| Arc::new(Self::new(store, bindings)))
    }

    /// Whether `client`'s binding keeps a durable outbox.
    pub fn keeps(&self, client: &str) -> bool {
        self.bindings.contains_key(client)
    }

    /// Every client whose binding keeps a durable outbox, with what it keeps.
    pub fn bindings(&self) -> impl Iterator<Item = (&str, &OutboxBinding)> + '_ {
        self.bindings
            .iter()
            .map(|(client, binding)| (client.as_str(), binding))
    }

    /// Call `wake` with a client's name whenever an append to its outbox arrives, once the
    /// outbox holds it: the server ends a held exchange of the client with it.
    pub fn watch(&self, wake: Wake) {
        *lock(&self.wake) = Some(wake);
    }

    /// The window of `client`'s outbox, which every session's router takes a place in for each
    /// envelope it writes to the client (`Router::bound_window`); `None` when the client's
    /// binding keeps no outbox.
    pub fn window(self: &Arc<Self>, client: &str) -> Option<Arc<dyn Window>> {
        self.keeps(client).then(|| {
            let window: Arc<dyn Window> = Arc::new(ClientWindow {
                outboxes: Arc::clone(self),
                client: client.to_string(),
            });
            window
        })
    }

    /// Take a place for one more envelope to `client`, or answer how many the window holds
    /// when it holds its most. An outbox not read yet is read first, so that what a restart kept
    /// counts; one that cannot be read is answered as a full window, so that no envelope is
    /// accepted that the bound cannot be kept for.
    fn reserve(&self, client: &str) -> Result<(), u32> {
        let Some(binding) = self.bindings.get(client) else {
            return Ok(());
        };
        let full = |depth: usize| u32::try_from(depth).unwrap_or(u32::MAX);
        let mut clients = lock(&self.clients);
        let slot = clients.entry(client.to_string()).or_default();
        if slot.held.is_none() {
            // One the store holds none of is empty until the first append opens it, and is read
            // again then.
            match self.store.outbox(client) {
                Ok(outbox) => slot.held = outbox.map(Held::of),
                Err(_) => return Err(full(binding.window)),
            }
        }
        let depth = slot.held.as_ref().map_or(0, |held| held.entries.len()) + slot.reserved;
        if depth >= binding.window {
            return Err(full(depth));
        }
        slot.reserved += 1;
        Ok(())
    }

    /// Give back `places` taken for `client` by envelopes no write will append: their session
    /// was dropped before a durable write held them.
    pub fn unreserve(&self, client: &str, places: usize) {
        if let Some(held) = lock(&self.clients).get_mut(client) {
            held.reserved = held.reserved.saturating_sub(places);
        }
    }

    /// A write appended `appended` at `at_ms`, the envelopes `envelopes` in the same order: keep
    /// them for the clients' exchanges, give back the places they took when they were `reserved`
    /// at admit (an answer the host made in a session's place took none), and wake each client's
    /// held exchange.
    pub fn committed(
        &self,
        appended: &[Appended],
        envelopes: &[Vec<u8>],
        at_ms: u64,
        reserved: bool,
    ) {
        let mut woken: Vec<String> = Vec::new();
        {
            let mut clients = lock(&self.clients);
            for (kept, envelope) in appended.iter().zip(envelopes) {
                let client = clients.entry(kept.client.clone()).or_default();
                if reserved {
                    client.reserved = client.reserved.saturating_sub(1);
                }
                // The cache numbers on as the store does; an append that does not follow its
                // last entry (another process appended between) leaves the cache to be read
                // again, and one not read yet is read at its first use, this append among its
                // entries.
                let follows = client.held.as_ref().map(|held| {
                    held.incarnation == kept.incarnation
                        && held
                            .entries
                            .back()
                            .is_none_or(|last| last.number < kept.number)
                });
                match follows {
                    Some(true) => {
                        if let Some(held) = client.held.as_mut() {
                            held.entries.push_back(Cached {
                                number: kept.number,
                                at_ms,
                                envelope: envelope.clone(),
                            });
                        }
                    }
                    Some(false) => client.held = None,
                    None => {}
                }
                if !woken.contains(&kept.client) {
                    woken.push(kept.client.clone());
                }
            }
        }
        let wake = lock(&self.wake).clone();
        if let Some(wake) = wake {
            for client in &woken {
                wake(client);
            }
        }
    }

    /// The outbox of `client` for its exchange: read from the store when the process holds none,
    /// and opened in `incarnation` at `now_ms` when the store holds none either. Answers the
    /// incarnation it numbers in.
    pub fn open(&self, client: &str, incarnation: &str, now_ms: u64) -> Result<String, Failure> {
        let mut clients = lock(&self.clients);
        if let Some(held) = clients.get(client).and_then(|held| held.held.as_ref()) {
            return Ok(held.incarnation.clone());
        }
        let outbox = self.store.open_outbox(client, incarnation, now_ms)?;
        let held = Held::of(outbox);
        let opened = held.incarnation.clone();
        clients.entry(client.to_string()).or_default().held = Some(held);
        Ok(opened)
    }

    /// Delete from `client`'s outbox, in the store and then here, every entry numbered at most
    /// `through`, when `incarnation` is the one it numbers in (SCE_MESH.md section 22.3): the
    /// acknowledgement of the binding. The places the entries held in the window are free again.
    pub fn acknowledge(
        &self,
        client: &str,
        incarnation: &str,
        through: u64,
        now_ms: u64,
    ) -> Result<u64, Failure> {
        let deleted = self
            .store
            .acknowledge(client, incarnation, through, now_ms)?;
        if let Some(held) = lock(&self.clients)
            .get_mut(client)
            .and_then(|held| held.held.as_mut())
            .filter(|held| held.incarnation == incarnation)
        {
            while held
                .entries
                .front()
                .is_some_and(|entry| entry.number <= through)
            {
                held.entries.pop_front();
            }
        }
        Ok(deleted)
    }

    /// Whether `client`'s outbox holds an envelope: what ends a held exchange.
    pub fn holds(&self, client: &str) -> bool {
        lock(&self.clients)
            .get(client)
            .and_then(|held| held.held.as_ref())
            .is_some_and(|held| !held.entries.is_empty())
    }

    /// What a response to `client` carries (SCE_MESH.md section 22.3): the oldest envelopes of its
    /// outbox that fit in `most` bytes, and always the first, so that an envelope larger than the
    /// bound still leaves. `None` when the process holds no outbox of the client: one not opened.
    pub fn response(&self, client: &str, most: usize) -> Option<Carried> {
        let clients = lock(&self.clients);
        let held = clients.get(client)?.held.as_ref()?;
        let mut body = Vec::new();
        for entry in &held.entries {
            if !body.is_empty() && body.len().saturating_add(entry.envelope.len()) > most {
                break;
            }
            body.extend_from_slice(&entry.envelope);
        }
        Some(Carried {
            incarnation: held.incarnation.clone(),
            first: held.entries.front().map(|entry| entry.number),
            body,
        })
    }

    /// What the process holds of `client`'s outbox, its incarnation and its envelopes by number,
    /// oldest first: for a host that carries the binding's exchanges itself and frames them as
    /// it frames them. `None` when the process holds no outbox of the client: one not opened.
    pub fn entries(&self, client: &str) -> Option<(String, Vec<(u64, Vec<u8>)>)> {
        let clients = lock(&self.clients);
        let held = clients.get(client)?.held.as_ref()?;
        Some((
            held.incarnation.clone(),
            held.entries
                .iter()
                .map(|entry| (entry.number, entry.envelope.clone()))
                .collect(),
        ))
    }

    /// Expire `client`'s outbox at `now_ms`: the store deletes at most `limit` of its entries
    /// older than the binding's `retain_ms`, notes each for the session that sent it, and closes
    /// the outbox once it holds nothing and nothing touched it for as long (docs/adr/0109, D10).
    /// What the store deleted is let go here too.
    pub fn expire(&self, client: &str, now_ms: u64, limit: usize) -> Result<Expiry, Failure> {
        let Some(binding) = self.bindings.get(client) else {
            return Ok(Expiry::default());
        };
        let before = now_ms.saturating_sub(binding.retain_ms);
        let expiry = self.store.expire(client, before, limit)?;
        let mut clients = lock(&self.clients);
        if let Some(held) = clients.get_mut(client) {
            if expiry.closed {
                held.held = None;
            } else if let Some(cached) = held.held.as_mut() {
                cached.entries.retain(|entry| {
                    !expiry
                        .expired
                        .iter()
                        .any(|expired| expired.entry.number == entry.number)
                });
            }
        }
        Ok(expiry)
    }

    /// The retain of `client`'s binding, in milliseconds, when it keeps an outbox.
    pub fn retain_ms(&self, client: &str) -> Option<u64> {
        self.bindings.get(client).map(|binding| binding.retain_ms)
    }

    /// The earliest moment an entry the process holds comes to expire, in milliseconds since the
    /// Unix epoch: what a host's clock loop wakes for. Outboxes not read in this process are not
    /// counted; the host calls the pool's expiry on its own schedule besides.
    pub fn next_expiry_ms(&self) -> Option<u64> {
        let clients = lock(&self.clients);
        clients
            .iter()
            .filter_map(|(client, held)| {
                let retain = self.bindings.get(client)?.retain_ms;
                let first = held.held.as_ref()?.entries.front()?;
                Some(first.at_ms.saturating_add(retain))
            })
            .min()
    }
}

/// The window of one client's outbox, as a session's router takes places in it.
struct ClientWindow {
    outboxes: Arc<Outboxes>,
    client: String,
}

impl Window for ClientWindow {
    fn reserve(&self) -> Result<(), u32> {
        self.outboxes.reserve(&self.client)
    }
}

/// `mutex`'s value, whatever a thread that panicked while it held it left: every value here is
/// changed in steps that leave it whole.
fn lock<V>(mutex: &Mutex<V>) -> MutexGuard<'_, V> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
