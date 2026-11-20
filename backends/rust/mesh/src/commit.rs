// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The committer of a session store (docs/adr/0109, D15; SCE_MESH.md section 10.11): the
//! checkpoints of many sessions committed together, and nothing released before its commit.
//!
//! Group commit, as a database writes the commits of many transactions in one flush of its log
//! and answers each after it (PostgreSQL's `commit_delay`, InnoDB's group commit). A session
//! seals its checkpoint (section 10.11, step 1) and hands the write to the committer
//! ([`Committer::write`]); the committer writes everything handed to it since its last commit in
//! one transaction of the store ([`SessionStore::write_many`]), one savepoint per write, so a write
//! whose version or fence the store refuses is answered refused and the others commit; and it
//! answers each write once the transaction is durable, which is when the session that handed it
//! tells its endpoint (step 3) and releases what the checkpoint holds. An acknowledgement of a
//! durable client's outbox (docs/adr/0109, D10) is handed the same way
//! ([`Committer::acknowledge`]) and joins the same batches.
//!
//! No thread of its own runs it: the first caller that finds no commit going on leads one. It
//! takes everything handed so far, writes it, answers it, and lets the next caller that finds its
//! write unanswered lead the next commit, which therefore starts as soon as the one before it
//! ended and takes what arrived meanwhile. Under load a commit holds as many writes as arrived
//! while the one before it was written; alone, a write is committed at once, as it would be
//! without the committer. So no delay is added, unless the deployment asks for one
//! (`sessions.commit_delay_us`): a leader then waits that long before it takes its batch, so that
//! writes arriving in that time join it.
//!
//! The durability of each write is the one it would have had alone: a write is answered only
//! once the transaction that holds it is durable, and a caller blocks until then, as it would on
//! the store. What a batch shares is the cost of making it durable.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use sce_rust_session_store::{
    Append, Batched, Change, Committed, Decided, Failure, SessionStore, Write, WriteError, Written,
};

/// The committer of one store. See the module documentation.
pub struct Committer {
    store: Arc<dyn SessionStore>,
    /// How long a leader waits before it takes its batch: deploy.yaml's
    /// `sessions.commit_delay_us`.
    delay: Duration,
    queue: Mutex<Queue>,
    /// Signalled whenever a commit ended: its answers are in the queue, and no commit is going on.
    ended: Condvar,
}

/// What the committer holds between its callers.
#[derive(Default)]
struct Queue {
    /// What was handed and is in no commit yet, each with its ticket, in the order handed.
    handed: Vec<(u64, Item)>,
    /// Whether a commit is going on.
    leading: bool,
    /// The ticket the next item handed takes.
    next_ticket: u64,
    /// The decision of each item a commit ended, by its ticket, until its caller takes it.
    answers: BTreeMap<u64, Decided>,
}

/// One item handed to the committer, owned: the caller that handed it waits, and the leader that
/// writes it may be another thread.
enum Item {
    Write {
        key: String,
        write: HeldWrite,
    },
    Acknowledge {
        client: String,
        incarnation: String,
        through: u64,
        now_ms: u64,
    },
}

/// A [`Write`] as the committer holds it.
struct HeldWrite {
    /// The state and earliest due of a write that keeps the session; `None` for a removal.
    keep: Option<(Vec<u8>, Option<u64>)>,
    expected_version: u64,
    fence: u64,
    outbox: Vec<HeldAppend>,
    notes_through: u64,
    now_ms: u64,
    sessions: Vec<String>,
}

/// An [`Append`] as the committer holds it.
struct HeldAppend {
    client: String,
    incarnation: String,
    sender: Option<String>,
    envelope: Vec<u8>,
}

impl HeldWrite {
    fn of(write: &Write<'_>) -> Self {
        HeldWrite {
            keep: match write.change {
                Change::Keep {
                    state,
                    earliest_due,
                } => Some((state.to_vec(), earliest_due)),
                Change::Remove => None,
            },
            expected_version: write.expected_version,
            fence: write.fence,
            outbox: write
                .outbox
                .iter()
                .map(|append| HeldAppend {
                    client: append.client.to_string(),
                    incarnation: append.incarnation.to_string(),
                    sender: append.sender.map(str::to_string),
                    envelope: append.envelope.to_vec(),
                })
                .collect(),
            notes_through: write.notes_through,
            now_ms: write.now_ms,
            sessions: write.sessions.iter().map(|id| id.to_string()).collect(),
        }
    }

    /// Its appends, as a [`Write`] names them.
    fn appends(&self) -> Vec<Append<'_>> {
        self.outbox
            .iter()
            .map(|append| Append {
                client: &append.client,
                incarnation: &append.incarnation,
                sender: append.sender.as_deref(),
                envelope: &append.envelope,
            })
            .collect()
    }

    /// The sessions a removal ends, as a [`Write`] names them.
    fn ended(&self) -> Vec<&str> {
        self.sessions.iter().map(String::as_str).collect()
    }

    /// The [`Write`] it is, with `appends` and `ended` as [`Self::appends`] and [`Self::ended`]
    /// gave them.
    fn written<'a>(&'a self, appends: &'a [Append<'a>], ended: &'a [&'a str]) -> Write<'a> {
        Write {
            change: match &self.keep {
                Some((state, earliest_due)) => Change::Keep {
                    state: state.as_slice(),
                    earliest_due: *earliest_due,
                },
                None => Change::Remove,
            },
            expected_version: self.expected_version,
            fence: self.fence,
            outbox: appends,
            notes_through: self.notes_through,
            now_ms: self.now_ms,
            sessions: ended,
        }
    }
}

/// The batch a commit writes, answered when it is dropped: with what the store decided once the
/// commit ended, or, when the leader unwound before it ended (a store that panicked), with a
/// failure for each item, so that no caller waits for a commit that will never end.
struct Leading<'a> {
    committer: &'a Committer,
    tickets: Vec<u64>,
    decided: Option<Vec<Decided>>,
}

impl Drop for Leading<'_> {
    fn drop(&mut self) {
        let decided = self.decided.take();
        let mut queue = lock(&self.committer.queue);
        queue.leading = false;
        match decided {
            Some(decided) => queue
                .answers
                .extend(self.tickets.iter().copied().zip(decided)),
            None => {
                for ticket in &self.tickets {
                    queue.answers.insert(
                        *ticket,
                        Err(WriteError::Failed(Failure::new(
                            "the commit that held this write did not end: its writer unwound",
                        ))),
                    );
                }
            }
        }
        drop(queue);
        self.committer.ended.notify_all();
    }
}

impl Committer {
    /// The committer of `store`, whose leader waits `delay` before it takes a batch: deploy.yaml's
    /// `sessions.commit_delay_us`, `Duration::ZERO` for none.
    pub fn new(store: Arc<dyn SessionStore>, delay: Duration) -> Self {
        Committer {
            store,
            delay,
            queue: Mutex::new(Queue::default()),
            ended: Condvar::new(),
        }
    }

    /// The store it commits to, which its callers read directly: a read waits for no commit.
    pub fn store(&self) -> &Arc<dyn SessionStore> {
        &self.store
    }

    /// Commit `write` to the session `key` names, with whatever else is handed meanwhile, and
    /// answer what [`SessionStore::write`] would have answered, once the transaction that holds
    /// it is durable. A write refused for its version or its fence is refused alone.
    pub fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        let item = Item::Write {
            key: key.to_string(),
            write: HeldWrite::of(write),
        };
        match self.commit(item)? {
            Committed::Written(written) => Ok(written),
            Committed::Acknowledged(_) => Err(WriteError::Failed(Failure::new(
                "the store answered a write as an acknowledgement",
            ))),
        }
    }

    /// Commit the acknowledgement of `client`'s outbox through `through` in `incarnation` at
    /// `now_ms`, with whatever else is handed meanwhile, and answer how many entries it deleted,
    /// once the transaction that holds it is durable (docs/adr/0109, D10).
    pub fn acknowledge(
        &self,
        client: &str,
        incarnation: &str,
        through: u64,
        now_ms: u64,
    ) -> Result<u64, Failure> {
        let item = Item::Acknowledge {
            client: client.to_string(),
            incarnation: incarnation.to_string(),
            through,
            now_ms,
        };
        match self.commit(item) {
            Ok(Committed::Acknowledged(deleted)) => Ok(deleted),
            Ok(Committed::Written(_)) => Err(Failure::new(
                "the store answered an acknowledgement as a write",
            )),
            Err(WriteError::Failed(failure)) => Err(failure),
            // An acknowledgement carries no version or fence; a store that refuses one for
            // either has failed to keep the contract.
            Err(WriteError::Conflict(conflict)) => Err(Failure::new(conflict)),
        }
    }

    /// How many items were handed and are in no commit yet: what the next commit takes.
    pub fn waiting(&self) -> usize {
        lock(&self.queue).handed.len()
    }

    /// Hand `item` over and wait for its decision: lead a commit whenever none is going on and
    /// `item` is not answered yet.
    fn commit(&self, item: Item) -> Decided {
        let mut queue = lock(&self.queue);
        let ticket = queue.next_ticket;
        queue.next_ticket += 1;
        queue.handed.push((ticket, item));
        loop {
            if let Some(decided) = queue.answers.remove(&ticket) {
                return decided;
            }
            if queue.leading {
                queue = self
                    .ended
                    .wait(queue)
                    .unwrap_or_else(PoisonError::into_inner);
                continue;
            }
            queue.leading = true;
            drop(queue);
            self.lead();
            queue = lock(&self.queue);
        }
    }

    /// Lead one commit: wait the delay, take everything handed, write it in one transaction and
    /// answer each item ([`Leading`]).
    fn lead(&self) {
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        let batch = core::mem::take(&mut lock(&self.queue).handed);
        let mut leading = Leading {
            committer: self,
            tickets: batch.iter().map(|(ticket, _)| *ticket).collect(),
            decided: None,
        };
        leading.decided = Some(self.write_batch(&batch));
    }

    /// Write `batch` in one transaction of the store, and the decision of each item, in order. A
    /// transaction the store could not make, or an answer that is not one per item, fails each
    /// item: nothing of the batch is known to be durable.
    fn write_batch(&self, batch: &[(u64, Item)]) -> Vec<Decided> {
        let appends: Vec<Vec<Append<'_>>> = batch
            .iter()
            .map(|(_, item)| match item {
                Item::Write { write, .. } => write.appends(),
                Item::Acknowledge { .. } => Vec::new(),
            })
            .collect();
        let ended: Vec<Vec<&str>> = batch
            .iter()
            .map(|(_, item)| match item {
                Item::Write { write, .. } => write.ended(),
                Item::Acknowledge { .. } => Vec::new(),
            })
            .collect();
        let given: Vec<Batched<'_>> = batch
            .iter()
            .zip(appends.iter().zip(&ended))
            .map(|((_, item), (appends, ended))| match item {
                Item::Write { key, write } => Batched::Write {
                    key: key.as_str(),
                    write: write.written(appends, ended),
                },
                Item::Acknowledge {
                    client,
                    incarnation,
                    through,
                    now_ms,
                } => Batched::Acknowledge {
                    client: client.as_str(),
                    incarnation: incarnation.as_str(),
                    through: *through,
                    now_ms: *now_ms,
                },
            })
            .collect();
        let reason = match self.store.write_many(&given) {
            Ok(decided) if decided.len() == given.len() => return decided,
            Ok(decided) => alloc::format!(
                "the store answered {} items of a batch of {}",
                decided.len(),
                given.len()
            ),
            Err(failure) => alloc::format!("the batch's transaction failed: {failure}"),
        };
        given
            .iter()
            .map(|_| Err(WriteError::Failed(Failure::new(reason.clone()))))
            .collect()
    }
}

/// `mutex`'s value, whatever a thread that panicked while it held it left: the queue is changed in
/// steps that leave it whole.
fn lock<V>(mutex: &Mutex<V>) -> MutexGuard<'_, V> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};
    use sce_rust_session_store::{
        AcquireError, Conflict, Due, Lease, LeaseError, MemoryStore, Stored, LATEST_MOMENT,
    };
    use std::sync::mpsc;
    use std::thread::JoinHandle;
    use std::time::Instant;

    /// A store in memory that notes the size of every commit, and, until the test opens it, holds
    /// its first commit, as a slow medium would; or, when `unwinds`, whose first commit panics.
    struct Gated {
        inner: MemoryStore,
        open: Mutex<bool>,
        opened: Condvar,
        unwinds: AtomicBool,
        /// How many items each commit took, in the order they began.
        batches: Mutex<Vec<usize>>,
    }

    impl Gated {
        fn over(inner: MemoryStore) -> Self {
            Gated {
                inner,
                open: Mutex::new(false),
                opened: Condvar::new(),
                unwinds: AtomicBool::new(false),
                batches: Mutex::new(Vec::new()),
            }
        }

        /// Let every commit, the held one included, end.
        fn open(&self) {
            *lock(&self.open) = true;
            self.opened.notify_all();
        }

        fn batches(&self) -> Vec<usize> {
            lock(&self.batches).clone()
        }
    }

    impl SessionStore for Gated {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(key)
        }

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            self.inner.write(key, write)
        }

        fn write_many(&self, batch: &[Batched<'_>]) -> Result<Vec<Decided>, Failure> {
            let first = {
                let mut batches = lock(&self.batches);
                batches.push(batch.len());
                batches.len() == 1
            };
            if first && self.unwinds.load(Ordering::SeqCst) {
                panic!("the medium's writer unwound");
            }
            if first {
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut open = lock(&self.open);
                while !*open {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        break;
                    }
                    open = self
                        .opened
                        .wait_timeout(open, left)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
            }
            self.inner.write_many(batch)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
        }

        fn acquire(
            &self,
            key: &str,
            owner: &str,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, AcquireError> {
            self.inner.acquire(key, owner, ttl_ms, now_ms)
        }

        fn renew(
            &self,
            key: &str,
            fence: u64,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, LeaseError> {
            self.inner.renew(key, fence, ttl_ms, now_ms)
        }

        fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
            self.inner.release(key, fence)
        }
    }

    /// A committer of `gated`, whose leader waits `delay`.
    fn committer(gated: &Arc<Gated>, delay: Duration) -> Arc<Committer> {
        let store: Arc<dyn SessionStore> = Arc::clone(gated) as Arc<dyn SessionStore>;
        Arc::new(Committer::new(store, delay))
    }

    /// The lease on `key`, granted to this process again whenever it asks.
    fn lease(store: &MemoryStore, key: &str) -> Lease {
        SessionStore::acquire(store, key, "p", LATEST_MOMENT, 0).expect("a lease")
    }

    /// A first write of the session `key` under `fence`, committed on a thread of its own.
    fn write_on_a_thread(
        committer: &Arc<Committer>,
        key: &'static str,
        fence: u64,
    ) -> JoinHandle<Result<Written, WriteError>> {
        let committer = Arc::clone(committer);
        std::thread::spawn(move || {
            committer.write(key, &Write::put(key.as_bytes(), None, 0, fence))
        })
    }

    /// Wait, at most five seconds, until `holds`; whether it did.
    fn eventually(holds: impl Fn() -> bool) -> bool {
        let until = Instant::now() + Duration::from_secs(5);
        while !holds() {
            if Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        true
    }

    /// docs/adr/0109, D15: while one commit is written, the writes handed meanwhile wait, and
    /// the next commit, which starts as soon as it ended, takes all of them in one transaction.
    /// A write in that batch under a fence another grant replaced is answered refused, alone:
    /// the write beside it is kept, and neither waits for the other to be judged. Red if the
    /// committer writes each item in a transaction of its own (`[1, 1, 1]`), or answers a whole
    /// batch with one item's refusal.
    #[test]
    fn a_fence_refused_inside_a_batch_refuses_only_its_item() {
        let store = MemoryStore::new();
        let gated = Arc::new(Gated::over(store.clone()));
        let committer = committer(&gated, Duration::ZERO);
        let first = lease(&store, "first");
        let kept = lease(&store, "kept");
        let fenced = lease(&store, "fenced");
        // The process asks again, as one that started again does: the earlier grant is fenced
        // out.
        let taken = lease(&store, "fenced");

        let holding = write_on_a_thread(&committer, "first", first.fence);
        assert!(
            eventually(|| gated.batches().len() == 1),
            "the first write began no commit"
        );
        let kept_write = write_on_a_thread(&committer, "kept", kept.fence);
        let fenced_write = write_on_a_thread(&committer, "fenced", fenced.fence);
        assert!(
            eventually(|| committer.waiting() == 2),
            "the writes handed while a commit is written wait for the next"
        );
        gated.open();

        holding
            .join()
            .expect("no writer panicked")
            .expect("the first write");
        let kept_written = kept_write
            .join()
            .expect("no writer panicked")
            .expect("the write beside the refused one is kept");
        match fenced_write.join().expect("no writer panicked") {
            Err(WriteError::Conflict(conflict)) => assert_eq!(
                conflict,
                Conflict::Fence {
                    presented: fenced.fence,
                    current: Some(taken.fence)
                }
            ),
            other => panic!("a write under a replaced fence gave {other:?}"),
        }
        assert_eq!(
            gated.batches(),
            vec![1, 2],
            "the second commit took both writes handed while the first was written"
        );
        assert_eq!(
            SessionStore::get(&store, "kept")
                .expect("a read")
                .map(|stored| stored.version),
            Some(kept_written.version)
        );
        assert_eq!(
            SessionStore::get(&store, "fenced").expect("a read"),
            None,
            "a refused write was kept"
        );
        assert_eq!(committer.waiting(), 0);
    }

    /// docs/adr/0109, D15, `sessions.commit_delay_us`: a leader waits its delay before it takes
    /// its batch, so a write handed during the wait joins the same commit. Red if the delay is
    /// not waited: the leader takes its own write alone and the second makes a commit of its
    /// own (`[1, 1]`).
    #[test]
    fn a_commit_waits_its_delay_and_takes_what_arrived_in_it() {
        let store = MemoryStore::new();
        let gated = Arc::new(Gated::over(store.clone()));
        gated.open();
        let committer = committer(&gated, Duration::from_millis(300));
        let a = lease(&store, "a");
        let b = lease(&store, "b");
        let first = write_on_a_thread(&committer, "a", a.fence);
        assert!(eventually(|| committer.waiting() == 1));
        let second = write_on_a_thread(&committer, "b", b.fence);
        first.join().expect("no writer panicked").expect("a");
        second.join().expect("no writer panicked").expect("b");
        assert_eq!(gated.batches(), vec![2]);
    }

    /// A commit whose writer unwound (a store that panicked) leaves no commit going on: the
    /// committer still commits the next write, and no caller waits for a commit that will never
    /// end. Red if a leader that unwinds leaves the committer marked as committing: the next
    /// write then waits forever, and is not answered within five seconds.
    #[test]
    fn a_commit_whose_writer_unwound_leaves_the_committer_working() {
        let store = MemoryStore::new();
        let gated = Arc::new(Gated::over(store.clone()));
        gated.open();
        gated.unwinds.store(true, Ordering::SeqCst);
        let committer = committer(&gated, Duration::ZERO);
        let a = lease(&store, "a");
        let b = lease(&store, "b");
        assert!(
            write_on_a_thread(&committer, "a", a.fence).join().is_err(),
            "the write whose commit unwound unwinds its caller"
        );
        let (answered, answer) = mpsc::channel();
        std::thread::spawn({
            let committer = Arc::clone(&committer);
            move || {
                let _ = answered.send(committer.write("b", &Write::put(b"b", None, 0, b.fence)));
            }
        });
        answer
            .recv_timeout(Duration::from_secs(5))
            .expect("a commit after one that unwound")
            .expect("the write after it is kept");
        assert_eq!(gated.batches(), vec![1, 1]);
    }
}
