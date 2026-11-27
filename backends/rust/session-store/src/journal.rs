// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a server keeps the facts its keyed sessions made: the event journal (ADR 0114, decisions
//! D1 to D3, D5, D6 and D8; its protocol is docs/formal/EventJournal.tla).
//!
//! A journal is an append-only order of **entries**. An entry is a **fact** (its schema's id and
//! revision and its data's bytes, which the store never reads), the **stream** it belongs to, its
//! **position** and the moment it was **recorded**. The store decides what makes the journal one:
//!
//! * **An entry exists exactly when the state that made it does.** A session's facts are appended
//!   by the write that keeps its checkpoint ([`Write::journal`]), judged by the same fence and
//!   version and kept in the same transaction, so a write the store refuses leaves none of its
//!   entries and no position a reader could see, and a session's removal keeps what it made.
//! * **A stream is a session, and nobody else appends to it**: the machine and the key of the
//!   session whose write made the entry ([`Stream::Session`]). The one other kind of stream is
//!   **observed** ([`Stream::Observed`]): another system's facts, which only
//!   [`JournalStore::observe`] appends, one entry for each id the other system gave. A stream's
//!   kind is part of what names it, so nobody says it twice; and one name is never both a machine's
//!   and an observed stream's in one journal, so a name alone still says which it is: the store
//!   refuses a write that would append under an observed stream's name ([`ObservedStream`]) and an
//!   observation under a machine's ([`SessionsStream`]).
//! * **Positions are one sequence of the journal.** Each is above every position the store gave
//!   before, to any stream, and none is given twice, after a crash either; a gap is allowed and
//!   means nothing. A store that gave the last position there is refuses what would need another
//!   ([`NoPositionLeft`]).
//! * **A reader is told how far it may read.** Transactions may become visible out of the order of
//!   their positions: two that took 7 and 8 may commit 8 first, and a reader that moved its cursor
//!   to 8 would never see 7. Every read answers the journal's **horizon**
//!   ([`JournalPage::horizon`]), the position at or below which nothing can still become visible,
//!   and no read answers an entry above it. A store that writes one transaction at a time has its
//!   last position as its horizon; a store that cannot tell its horizon is no store of this
//!   contract. A store ends the transaction of a writer that is gone, as a database does the one of
//!   a connection that closed: one left open keeps the horizon below it, and nothing past it would
//!   ever be read.
//! * **Two times.** An entry's recorded moment is the store's clock as the write gave it
//!   ([`Write::now_ms`]), never below that of the entry committed before it, so it never goes back
//!   when the clock does. Its **valid time**, when its schema marks one, is when the fact holds in
//!   the world, before or after that; an entry without one holds from its recorded moment
//!   ([`JournalEntry::holds_from`]). An answer as of an earlier moment is asked by a position at or
//!   below the horizon and a valid time ([`AsOf`]) and never changes; a moment of the clock is
//!   first turned into a position ([`JournalStore::position_at`]).
//! * **Who a fact was said for is the store's to keep, never a writer's to choose** (ADR 0116,
//!   D7). A fact a session said in a run under a delegation names the instance whose event the run
//!   took in and the person its incarnation acted for ([`Fact::actor`]), which the host core read
//!   from the delegation its own server holds, never from a field a sender wrote; the store keeps
//!   it with the entry and answers it with every read ([`JournalEntry::actor`]). Another system's
//!   fact was said for no instance of this server, so an observation that names one is refused
//!   ([`ObservedActor`]).
//! * **Nothing is changed or removed.** A correction is an entry. The one replacement is of the
//!   bytes that hold an entry's data, made only from the bytes held, every other field kept
//!   ([`JournalStore::reseal_journaled`]): how a store that seals what it keeps seals them again
//!   under its active key (ADR 0109, D8).
//!
//! * [`crate::MemoryStore`] keeps the journal in the process, beside its sessions.
//! * `SqliteStore` (feature `sqlite`) keeps it in the session store's file (layout 8).
//! * [`crate::journal_conformance`] is what every implementation is held to.
//!
//! [`Write::journal`]: crate::Write::journal
//! [`Write::now_ms`]: crate::Write::now_ms

use std::borrow::Cow;
use std::error::Error;
use std::fmt;

use crate::membership::Person;
use crate::{rules, Failure, SessionStore};

/// What names a stream of the journal. The kind is part of the name: an entry a session made and
/// one another system's adapter observed are never of one stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stream<'a> {
    /// The facts of the session of `machine` that `key` names, which its writes append.
    Session { machine: &'a str, key: &'a str },
    /// The facts of another system that [`JournalStore::observe`] appends under `name`.
    Observed { name: &'a str },
}

impl Stream<'_> {
    /// The name, owned.
    pub fn to_name(self) -> StreamName {
        match self {
            Self::Session { machine, key } => StreamName::Session {
                machine: machine.to_string(),
                key: key.to_string(),
            },
            Self::Observed { name } => StreamName::Observed {
                name: name.to_string(),
            },
        }
    }
}

/// What names a stream, owned: the stream of an entry read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StreamName {
    Session { machine: String, key: String },
    Observed { name: String },
}

impl StreamName {
    /// The name, borrowed: what a read of the stream is asked by.
    pub fn stream(&self) -> Stream<'_> {
        match self {
            Self::Session { machine, key } => Stream::Session { machine, key },
            Self::Observed { name } => Stream::Observed { name },
        }
    }
}

/// What makes the bytes of an entry's data once its position is known: how a store that wraps
/// another and seals what it keeps (ADR 0109, D8) binds a record to its position, which the store
/// gives inside the transaction.
///
/// A store calls it while it holds the transaction the position was given in, and may call it for
/// a position whose transaction is then rolled back, so it makes bytes and does nothing else: it
/// calls no method of the store it was given to.
pub trait AtPosition: Send + Sync {
    /// The bytes kept as the data of the entry at `position`. A failure fails the write that gave
    /// it, which then leaves nothing of itself.
    fn bytes(&self, position: u64) -> Result<Vec<u8>, Failure>;
}

/// The bytes of a fact.
#[derive(Clone, Copy)]
pub enum FactData<'a> {
    /// The bytes themselves: the record's deterministic CBOR, as a session's write gives it.
    Bytes(&'a [u8]),
    /// The bytes made once the entry's position is known.
    AtPosition(&'a dyn AtPosition),
}

impl<'a> FactData<'a> {
    /// The bytes kept for the entry at `position`.
    pub fn at(&self, position: u64) -> Result<Cow<'a, [u8]>, Failure> {
        match *self {
            Self::Bytes(bytes) => Ok(Cow::Borrowed(bytes)),
            Self::AtPosition(made) => made.bytes(position).map(Cow::Owned),
        }
    }
}

impl fmt::Debug for FactData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bytes(bytes) => f.debug_tuple("Bytes").field(bytes).finish(),
            Self::AtPosition(_) => f.write_str("AtPosition(..)"),
        }
    }
}

/// Two facts' bytes are equal when they are the same bytes, or are made by the same maker.
impl PartialEq for FactData<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bytes(one), Self::Bytes(another)) => one == another,
            (Self::AtPosition(one), Self::AtPosition(another)) => {
                std::ptr::addr_eq(std::ptr::from_ref(*one), std::ptr::from_ref(*another))
            }
            _ => false,
        }
    }
}

impl Eq for FactData<'_> {}

/// A fact, as a write or an observation gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fact<'a> {
    /// The id of the schema its data is a record of.
    pub schema: &'a str,
    /// The schema's revision that wrote the data.
    pub revision: u32,
    pub data: FactData<'a>,
    /// When the fact holds from in the world, in milliseconds since the Unix epoch, before it for a
    /// negative number; `None` for a fact that holds from its recorded moment.
    pub valid_ms: Option<i64>,
    /// Who the fact was said for (ADR 0116, D7): the instance and the person of the delegation the
    /// run that said it was made under; `None` for a fact of a run made for nobody, and for every
    /// observed one.
    pub actor: Option<Actor<'a>>,
}

/// Who a fact was said for (ADR 0116, D1 and D7): the instance of an open machine whose event the
/// run that said it took in, and the person that instance's incarnation acted for, as the delegation
/// the server's store holds names them. The host core gives it, from the edge that took the event
/// in; no document writes it and none leaves it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor<'a> {
    /// The instance id of the open machine (ADR 0113, D1).
    pub instance: &'a str,
    /// The identity provider's issuer.
    pub issuer: &'a str,
    /// Who the person is to their issuer: unique within the issuer alone.
    pub subject: &'a str,
    /// When they proved it, in milliseconds since the Unix epoch.
    pub proved_ms: u64,
    /// How they proved it, as the port that judged the proof names it.
    pub method: &'a str,
}

impl Actor<'_> {
    /// Who, owned.
    pub fn to_name(self) -> ActorName {
        ActorName {
            instance: self.instance.to_string(),
            person: Person {
                issuer: self.issuer.to_string(),
                subject: self.subject.to_string(),
                proved_ms: self.proved_ms,
                method: self.method.to_string(),
            },
        }
    }
}

/// Who a fact was said for, owned: the actor of an entry read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorName {
    pub instance: String,
    pub person: Person,
}

impl ActorName {
    /// Who, borrowed: what a write that keeps the fact again gives.
    pub fn actor(&self) -> Actor<'_> {
        Actor {
            instance: &self.instance,
            issuer: &self.person.issuer,
            subject: &self.person.subject,
            proved_ms: self.person.proved_ms,
            method: &self.person.method,
        }
    }
}

/// The facts a write appends to its session's stream ([`Write::journal`]).
///
/// The stream's key is the key the write names. Its machine is told here, since a session store is
/// given a key and never the machine whose sessions it keeps.
///
/// [`Write::journal`]: crate::Write::journal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Journal<'a> {
    /// The machine whose session the write keeps or removes.
    pub machine: &'a str,
    /// The facts, in the order made, which is the order of their positions.
    pub facts: &'a [Fact<'a>],
}

/// An entry of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    /// Its place in the order of the journal, above every position given before it; never `0`.
    pub position: u64,
    /// When it was recorded, in milliseconds since the Unix epoch, never below the moment of the
    /// entry committed before it.
    pub recorded_ms: u64,
    pub stream: StreamName,
    /// The id the other system gave an entry of an observed stream; `None` for a session's.
    pub origin: Option<String>,
    pub schema: String,
    pub revision: u32,
    pub valid_ms: Option<i64>,
    /// Who its fact was said for ([`Fact::actor`]); `None` for one said for nobody, every observed
    /// one among them, and in a store that wraps another and seals what it keeps, where it is
    /// inside the sealed record.
    pub actor: Option<ActorName>,
    /// Its data's bytes, which the store keeps and never reads: the fact's, or, in a store that
    /// wraps another and seals what it keeps (ADR 0109, D8), their sealed record.
    pub data: Vec<u8>,
}

impl JournalEntry {
    /// When the fact holds from: its valid time, or its recorded moment where it has none.
    pub fn holds_from(&self) -> i64 {
        holds_from(self.valid_ms, self.recorded_ms)
    }

    /// The entry's place in an answer as of a moment: what the next page of one resumes after.
    pub fn cursor(&self) -> AsOfCursor {
        AsOfCursor {
            valid_ms: self.holds_from(),
            position: self.position,
        }
    }
}

/// When a fact of the valid time `valid_ms` recorded at `recorded_ms` holds from.
pub(crate) fn holds_from(valid_ms: Option<i64>, recorded_ms: u64) -> i64 {
    valid_ms.unwrap_or_else(|| i64::try_from(recorded_ms).unwrap_or(i64::MAX))
}

/// The moment an entry is recorded at: the write's `now_ms` as a store keeps it, unless the entry
/// committed before it was recorded later, at `last_ms`.
pub(crate) fn recorded(last_ms: u64, now_ms: u64) -> u64 {
    crate::moment(now_ms).max(last_ms)
}

/// What a read answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalPage {
    /// The entries, in the read's order, at most as many as its limit and none above the horizon. A
    /// page shorter than its limit is the last one up to the horizon.
    pub entries: Vec<JournalEntry>,
    /// The position at or below which nothing can still become visible, when the read was made. It
    /// never falls.
    pub horizon: u64,
}

/// A moment of both times: what the journal held at a position, of what holds by a valid time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsOf {
    /// The last position read; at or below the horizon.
    pub position: u64,
    /// The latest moment a fact read holds from.
    pub valid_ms: i64,
}

impl AsOf {
    /// Refused when the position is above `horizon`: what the journal held there can still change.
    pub fn admitted(&self, horizon: u64) -> Result<(), PastHorizon> {
        if !rules::settled(self.position, horizon) {
            return Err(PastHorizon {
                asked: self.position,
                horizon,
            });
        }
        Ok(())
    }

    /// Whether the answer holds `entry`, an entry of the stream asked.
    pub fn includes(&self, entry: &JournalEntry) -> bool {
        entry.position <= self.position && entry.holds_from() <= self.valid_ms
    }
}

/// A place in an answer as of a moment, whose order is by valid time and then position: a page
/// resumes after the cursor of the last entry of the one before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsOfCursor {
    pub valid_ms: i64,
    pub position: u64,
}

impl AsOfCursor {
    /// Before every entry, since no position is `0`.
    pub const START: AsOfCursor = AsOfCursor {
        valid_ms: i64::MIN,
        position: 0,
    };
}

/// What an observation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observation {
    /// The entry was kept, at `position`.
    Kept { position: u64 },
    /// An entry of the stream and origin id is kept at `position` already; nothing was appended,
    /// and what it holds was not compared with what was given.
    Repeated { position: u64 },
}

impl Observation {
    /// The position of the entry of the stream and origin id.
    pub fn position(&self) -> u64 {
        match *self {
            Self::Kept { position } | Self::Repeated { position } => position,
        }
    }
}

/// Why a store refused an answer as of a position: it is above the horizon, below which nothing can
/// still appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PastHorizon {
    pub asked: u64,
    pub horizon: u64,
}

impl fmt::Display for PastHorizon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "position {} is above the journal's horizon, {}: what the journal held there can still \
             change",
            self.asked, self.horizon
        )
    }
}

impl Error for PastHorizon {}

/// The answer of a refused [`JournalStore::as_of`].
#[derive(Debug)]
pub enum AsOfError {
    PastHorizon(PastHorizon),
    Failed(Failure),
}

impl fmt::Display for AsOfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PastHorizon(refused) => write!(f, "{refused}"),
            Self::Failed(failure) => write!(f, "{failure}"),
        }
    }
}

impl Error for AsOfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PastHorizon(refused) => Some(refused),
            Self::Failed(failure) => Some(failure),
        }
    }
}

impl From<PastHorizon> for AsOfError {
    fn from(refused: PastHorizon) -> Self {
        Self::PastHorizon(refused)
    }
}

impl From<Failure> for AsOfError {
    fn from(failure: Failure) -> Self {
        Self::Failed(failure)
    }
}

/// Why a store refused a write: its machine has the name of an observed stream of the journal,
/// which no session appends to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedStream {
    pub name: String,
}

impl fmt::Display for ObservedStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the journal holds an observed stream named '{}', which no session appends to",
            self.name
        )
    }
}

impl Error for ObservedStream {}

/// Why a store refused an observation: the sessions of a machine of the stream's name made entries
/// of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionsStream {
    pub name: String,
}

impl fmt::Display for SessionsStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the journal holds entries the sessions of the machine '{}' made, which no observation \
             appends to",
            self.name
        )
    }
}

impl Error for SessionsStream {}

/// Why a store refused an observation: it names who it was said for, and another system's fact was
/// said for no instance of this server (ADR 0116, D7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedActor;

impl fmt::Display for ObservedActor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "an observed fact is another system's and was said for no instance of this server, so \
             it names no one it was said for",
        )
    }
}

impl Error for ObservedActor {}

/// Why a store refused to give a position: it gave the last position there is, and it gives none
/// twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoPositionLeft;

impl fmt::Display for NoPositionLeft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the journal has given the last position there is, and it gives none twice")
    }
}

impl Error for NoPositionLeft {}

/// The refusal of a session store that keeps no journal: what it answers, as
/// [`WriteError::Failed`](crate::WriteError::Failed), a write that gives facts, which it would
/// otherwise lose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoJournal;

impl fmt::Display for NoJournal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "the store keeps no journal, so it keeps no write that appends to one (ADR 0114)",
        )
    }
}

impl Error for NoJournal {}

/// A session store that keeps a journal. See the module documentation for what it keeps and why.
///
/// A session's entries are appended by [`SessionStore::write`] and [`SessionStore::write_many`];
/// what is here reads the journal and keeps what another system's adapter observed. Every method is
/// one transaction. A handle may be shared between threads; whether handles on one store may live
/// in several processes is the implementation's to say.
///
/// Every read is a page: at most `limit` entries, in the read's order, none above the horizon it
/// answers. Reading on from the last entry of a page misses no entry and repeats none, whatever was
/// appended in between.
pub trait JournalStore: SessionStore {
    /// The entries past `position` and at or below the horizon, in the order of their positions:
    /// how a follower reads the journal, its cursor the position of the last entry it took in.
    fn after(&self, position: u64, limit: usize) -> Result<JournalPage, Failure>;

    /// The entries of `stream` past the position `after` and at or below the horizon, in the order
    /// of their positions.
    fn stream(&self, stream: Stream<'_>, after: u64, limit: usize) -> Result<JournalPage, Failure>;

    /// The largest position the horizon has passed whose recorded moment is at or before `ms`; `0`
    /// when there is none. An answer as of the position never changes; the position of a moment the
    /// store still records at may rise.
    fn position_at(&self, ms: u64) -> Result<u64, Failure>;

    /// The entries of `stream` at or below `as_of`'s position that hold from its valid time or
    /// before, ordered by when they hold from and then by position, past `after`:
    /// [`AsOfCursor::START`] for the first page, then [`JournalEntry::cursor`] of the last entry of
    /// the page before. The whole answer is the same whenever it is asked.
    ///
    /// Refused with [`PastHorizon`] when the position is above the horizon.
    fn as_of(
        &self,
        stream: Stream<'_>,
        as_of: &AsOf,
        after: AsOfCursor,
        limit: usize,
    ) -> Result<JournalPage, AsOfError>;

    /// Keep `fact`, which another system says under its id `origin`, as an entry of the observed
    /// stream `name`, recorded at `now_ms`; or answer the position of the entry the stream holds
    /// for that id, appending nothing ([`Observation::Repeated`]). The only way an observed stream
    /// is written.
    ///
    /// Refused ([`SessionsStream`]) when a machine of that name has entries in the journal, and
    /// ([`ObservedActor`]) when the fact names who it was said for ([`Fact::actor`]), whatever the
    /// stream holds.
    fn observe(
        &self,
        name: &str,
        origin: &str,
        fact: &Fact<'_>,
        now_ms: u64,
    ) -> Result<Observation, Failure>;

    /// Replace the data of the entry at `position` with `data` when it is `held`, every other field
    /// kept, and answer whether it did: `false`, writing nothing, when the store holds no such
    /// entry or its data is other. The caller gives bytes that hold the same data, as a store that
    /// seals what it keeps does when it seals an entry again under its active key (ADR 0109, D8):
    /// an answer already given does not change.
    fn reseal_journaled(&self, position: u64, held: &[u8], data: &[u8]) -> Result<bool, Failure>;

    /// The position through which the reader `reader` has taken in every entry it reads, as it last
    /// kept it ([`JournalStore::keep_read`]); `0` for a reader that kept none. A follower of the
    /// journal reads on from here when it starts (ADR 0114, D4), and one under a new name reads
    /// from the start.
    fn read_through(&self, reader: &str) -> Result<u64, Failure>;

    /// Keep that the reader `reader` has taken in every entry it reads through `position`, and
    /// answer the position kept: the greater of `position` and the one kept before, since what a
    /// reader has taken in stays taken in, and a mark that fell would read again only what it
    /// already took.
    ///
    /// Refused ([`PastHorizon`]) when `position` is above the horizon: an entry can still appear
    /// at or below it, and a reader that starts past it would never read that entry.
    fn keep_read(&self, reader: &str, position: u64) -> Result<u64, Failure>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(position: u64, recorded_ms: u64, valid_ms: Option<i64>) -> JournalEntry {
        JournalEntry {
            position,
            recorded_ms,
            stream: StreamName::Observed {
                name: "s".to_string(),
            },
            origin: Some("o".to_string()),
            schema: "schema".to_string(),
            revision: 1,
            valid_ms,
            actor: None,
            data: Vec::new(),
        }
    }

    #[test]
    fn a_moment_is_recorded_at_the_clock_unless_one_before_it_is_later() {
        assert_eq!(recorded(0, 1_000), 1_000);
        assert_eq!(recorded(2_000, 1_000), 2_000);
        assert_eq!(recorded(0, u64::MAX), crate::LATEST_MOMENT);
    }

    #[test]
    fn a_fact_with_no_valid_time_holds_from_its_recorded_moment() {
        assert_eq!(entry(1, 1_000, None).holds_from(), 1_000);
        assert_eq!(entry(1, 1_000, Some(-7)).holds_from(), -7);
        assert_eq!(holds_from(None, u64::MAX), i64::MAX);
    }

    #[test]
    fn an_answer_holds_what_is_at_or_before_both_its_moments() {
        let as_of = AsOf {
            position: 5,
            valid_ms: 1_000,
        };
        assert!(as_of.includes(&entry(5, 1_000, None)));
        assert!(!as_of.includes(&entry(6, 1_000, None)));
        assert!(!as_of.includes(&entry(5, 1_001, None)));
        assert!(as_of.includes(&entry(5, 1_001, Some(1_000))));
        assert_eq!(as_of.admitted(5), Ok(()));
        assert_eq!(
            as_of.admitted(4),
            Err(PastHorizon {
                asked: 5,
                horizon: 4
            })
        );
    }

    #[test]
    fn a_cursor_is_ordered_by_valid_time_and_then_position() {
        let early = entry(9, 0, Some(1)).cursor();
        let late = entry(2, 0, Some(2)).cursor();
        let tied = entry(10, 0, Some(1)).cursor();
        assert!(early < tied && tied < late);
        assert!(AsOfCursor::START < entry(1, 0, Some(i64::MIN)).cursor());
    }

    #[test]
    fn the_bytes_of_a_fact_are_equal_by_value_or_by_maker() {
        struct Says(&'static [u8]);
        impl AtPosition for Says {
            fn bytes(&self, _: u64) -> Result<Vec<u8>, Failure> {
                Ok(self.0.to_vec())
            }
        }
        let (one, another) = (Says(b"x"), Says(b"x"));
        assert_eq!(FactData::Bytes(b"x"), FactData::Bytes(b"x"));
        assert_ne!(FactData::Bytes(b"x"), FactData::Bytes(b"y"));
        assert_eq!(FactData::AtPosition(&one), FactData::AtPosition(&one));
        assert_ne!(FactData::AtPosition(&one), FactData::AtPosition(&another));
        assert_ne!(FactData::Bytes(b"x"), FactData::AtPosition(&one));
        let made = FactData::AtPosition(&one).at(1).expect("its bytes");
        assert_eq!(made.into_owned(), b"x".to_vec());
    }
}
