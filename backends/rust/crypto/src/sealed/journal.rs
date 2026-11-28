// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The event journal of ADR 0114 sealed (D8; ADR 0109, D8).
//!
//! - A stream is kept under its entries: a session's under the entry of its machine's name and the
//!   entry of its key, an observed stream's under the entry of its name, with the other system's id
//!   as its entry too.
//! - An entry's data is the record of the payload `[kind, first, second, schema, revision, data]`,
//!   or `[kind, first, second, schema, revision, data, valid_ms]` for a fact with a valid time, or
//!   `[kind, first, second, schema, revision, data, valid_ms, actor]` for a fact said for a person
//!   (ADR 0116, D7), `valid_ms` `null` where it has none and `actor` the array
//!   `[instance, issuer, subject, proved_ms, method]`, or
//!   `[kind, first, second, schema, revision, data, valid_ms, actor, read]` for a fact decided having
//!   read the journals its session follows (ADR 0116, D5), `actor` `null` where it names no one too
//!   and `read` the array of `[journal, through]` in the order the write gave them:
//!   `kind` is 0 for a session's, whose `first` and `second` are the machine and the key, and 1 for
//!   an observed one, whose are the stream's name and the origin id. It is sealed for the store
//!   [`STORE_JOURNAL`] and the row of the stream's entries and the position (each entry, a colon,
//!   and the position's decimal digits), so data moved to another position or another stream does
//!   not open.
//! - Its position and its recorded moment are the wrapped store's, which orders by them, in the
//!   clear; its schema, its revision, its valid time, who it was said for and what it was decided
//!   having read are inside the record alone, and the wrapped store is given none of them.
//!
//! The wrapped store gives a position inside the transaction that writes the entry, so the record
//! is made there, by the wrapped store's call ([`AtPosition`]).
//!
//! What it costs: the wrapped store cannot order by a valid time it does not hold, so an answer as
//! of a moment (`JournalStore::as_of`) opens every entry of the stream at or below the position
//! asked, for each page, and orders them itself. A page's cost grows with the stream up to that
//! position, its memory with the page alone. Every other read opens the entries it answers.

use std::collections::BTreeMap;

use sce_forge_runtime::cbor::{
    read_array_len, read_bytes, read_int_within, read_text, read_uint, read_uint_upto, take_null,
    write_array_head, write_bytes, write_int, write_null, write_text, write_uint,
};
use sce_forge_runtime::codec::{CodecError, SceCursor};
use sce_rust_session_store::{
    Actor, ActorName, AsOf, AsOfCursor, AsOfError, AtPosition, Fact, FactData, Failure, Journal,
    JournalEntry, JournalPage, JournalStore, Observation, ObservedActor, ObservedRead, Person,
    Read, SessionStore, Stream, StreamName,
};
use zeroize::Zeroizing;

use super::{decoded, encoded, Reseal, Resealed, SealError, Sealed};
use crate::cose::put;

/// The name the entries of a `JournalStore` are sealed for.
pub const STORE_JOURNAL: &str = "journal";

/// How many entries a re-seal pass, or an answer as of a moment, reads at a time.
const PAGE: usize = 256;

/// The kind a payload says a session's entry is of.
const KIND_SESSION: u64 = 0;
/// The kind a payload says an observed entry is of.
const KIND_OBSERVED: u64 = 1;

/// What an entry is of, in the clear.
#[derive(Clone, Copy)]
enum Named<'a> {
    Session { machine: &'a str, key: &'a str },
    Observed { name: &'a str, origin: &'a str },
}

/// What an entry's record holds.
struct Opened {
    stream: StreamName,
    origin: Option<String>,
    schema: String,
    revision: u32,
    valid_ms: Option<i64>,
    actor: Option<ActorName>,
    read: Vec<Read>,
    data: Vec<u8>,
}

impl Opened {
    /// What the entry is of.
    fn named(&self) -> Result<Named<'_>, SealError> {
        match (&self.stream, &self.origin) {
            (StreamName::Session { machine, key }, None) => Ok(Named::Session { machine, key }),
            (StreamName::Observed { name }, Some(origin)) => Ok(Named::Observed { name, origin }),
            _ => Err(SealError::Payload),
        }
    }
}

/// The payload of `data`, a fact of `named` with the schema, the revision, the valid time, who it
/// was said for and what it was decided having read given.
fn payload(
    named: Named<'_>,
    schema: &str,
    revision: u32,
    valid_ms: Option<i64>,
    actor: Option<Actor<'_>>,
    read: &[Read],
    data: &[u8],
) -> Zeroizing<Vec<u8>> {
    let (kind, first, second) = match named {
        Named::Session { machine, key } => (KIND_SESSION, machine, key),
        Named::Observed { name, origin } => (KIND_OBSERVED, name, origin),
    };
    let texts = first.len() + second.len() + schema.len();
    let actor_texts = actor.map_or(0, |actor| {
        actor.instance.len() + actor.issuer.len() + actor.subject.len() + actor.method.len()
    });
    let read_texts: usize = read.iter().map(|each| each.journal.len() + 12).sum();
    let items = match (valid_ms, actor, read.is_empty()) {
        (_, _, false) => 9,
        (_, Some(_), true) => 8,
        (Some(_), None, true) => 7,
        (None, None, true) => 6,
    };
    encoded(texts + actor_texts + read_texts + data.len() + 96, |w| {
        put(write_array_head(w, items));
        put(write_uint(w, kind));
        put(write_text(w, first));
        put(write_text(w, second));
        put(write_text(w, schema));
        put(write_uint(w, u64::from(revision)));
        put(write_bytes(w, data));
        match valid_ms {
            Some(valid_ms) => put(write_int(w, valid_ms)),
            None if items >= 8 => put(write_null(w)),
            None => {}
        }
        match actor {
            Some(actor) => {
                put(write_array_head(w, 5));
                put(write_text(w, actor.instance));
                put(write_text(w, actor.issuer));
                put(write_text(w, actor.subject));
                put(write_uint(w, actor.proved_ms));
                put(write_text(w, actor.method));
            }
            None if items == 9 => put(write_null(w)),
            None => {}
        }
        if items == 9 {
            put(write_array_head(w, read.len() as u64));
            for each in read {
                put(write_array_head(w, 2));
                put(write_text(w, &each.journal));
                put(write_uint(w, each.through));
            }
        }
    })
}

/// Who a fact was said for, as a payload holds it: `[instance, issuer, subject, proved_ms, method]`.
fn actor_of(c: &mut SceCursor<'_>) -> Result<ActorName, CodecError> {
    if read_array_len(c)? != 5 {
        return Err(CodecError::CborWrongLength);
    }
    let instance = read_text(c, None)?.to_string();
    let issuer = read_text(c, None)?.to_string();
    let subject = read_text(c, None)?.to_string();
    let proved_ms = read_uint(c)?;
    let method = read_text(c, None)?.to_string();
    Ok(ActorName {
        instance,
        person: Person {
            issuer,
            subject,
            proved_ms,
            method,
        },
    })
}

/// What a fact was decided having read, as a payload holds it: an array of `[journal, through]`.
fn read_of(c: &mut SceCursor<'_>) -> Result<Vec<Read>, CodecError> {
    let mut read = Vec::new();
    for _ in 0..read_array_len(c)? {
        if read_array_len(c)? != 2 {
            return Err(CodecError::CborWrongLength);
        }
        let journal = read_text(c, None)?.to_string();
        let through = read_uint(c)?;
        read.push(Read { journal, through });
    }
    Ok(read)
}

/// What the payload `plaintext` holds.
fn opened(plaintext: &[u8]) -> Result<Opened, SealError> {
    decoded(plaintext, |c| {
        let items = read_array_len(c)?;
        if !(6..=9).contains(&items) {
            return Err(CodecError::CborWrongLength);
        }
        let kind = read_uint_upto(c, KIND_OBSERVED)?;
        let first = read_text(c, None)?.to_string();
        let second = read_text(c, None)?.to_string();
        let schema = read_text(c, None)?.to_string();
        let revision = u32::try_from(read_uint_upto(c, u64::from(u32::MAX))?)
            .map_err(|_| CodecError::CborOutOfRange)?;
        let data = read_bytes(c, None)?.to_vec();
        // The valid time, written as `null` in a payload of eight or nine items whose fact has
        // none.
        let valid_ms = if items == 6 || (items >= 8 && take_null(c)?) {
            None
        } else {
            Some(read_int_within(c, i64::MIN, i64::MAX)?)
        };
        // Who it was said for, written as `null` in a payload of nine items whose fact names no one.
        let actor = if items == 8 || (items == 9 && !take_null(c)?) {
            Some(actor_of(c)?)
        } else {
            None
        };
        let read = if items == 9 { read_of(c)? } else { Vec::new() };
        let (stream, origin) = if kind == KIND_SESSION {
            (
                StreamName::Session {
                    machine: first,
                    key: second,
                },
                None,
            )
        } else {
            (StreamName::Observed { name: first }, Some(second))
        };
        Ok(Opened {
            stream,
            origin,
            schema,
            revision,
            valid_ms,
            actor,
            read,
            data,
        })
    })
}

/// What the entry at `position` of the stream kept under the entries `stream` is sealed for.
fn indexed(stream: Stream<'_>, position: u64) -> Vec<u8> {
    match stream {
        Stream::Session { machine, key } => format!("{machine}:{key}:{position}"),
        Stream::Observed { name } => format!("{name}:{position}"),
    }
    .into_bytes()
}

/// A fact as the wrapped store is given it: what it is of, the entries its stream is kept under,
/// and the fact, whose record is made once its position is known.
pub(super) struct SealedFact<'a, S> {
    sealed: &'a Sealed<S>,
    named: Named<'a>,
    stream: StreamName,
    fact: Fact<'a>,
}

impl<S: SessionStore> AtPosition for SealedFact<'_, S> {
    fn bytes(&self, position: u64) -> Result<Vec<u8>, Failure> {
        let data = self.fact.data.at(position)?;
        self.sealed.seal(
            STORE_JOURNAL,
            &indexed(self.stream.stream(), position),
            &payload(
                self.named,
                self.fact.schema,
                self.fact.revision,
                self.fact.valid_ms,
                self.fact.actor,
                self.fact.read,
                &data,
            ),
        )
    }
}

impl<S: SessionStore> SealedFact<'_, S> {
    /// The fact the wrapped store is given, which says nothing of its own.
    pub(super) fn given(&self) -> Fact<'_> {
        Fact {
            schema: "",
            revision: 0,
            data: FactData::AtPosition(self),
            valid_ms: None,
            actor: None,
            read: &[],
        }
    }
}

/// The facts of a write as the wrapped store is given them, with the entry of their machine.
pub(super) struct SealedJournal<'a, S> {
    machine: String,
    facts: Vec<SealedFact<'a, S>>,
}

impl<S: SessionStore> SealedJournal<'_, S> {
    /// The entry of the machine's name.
    pub(super) fn machine(&self) -> &str {
        &self.machine
    }

    /// The facts the wrapped store is given.
    pub(super) fn facts(&self) -> Vec<Fact<'_>> {
        self.facts.iter().map(SealedFact::given).collect()
    }
}

impl<S> Sealed<S> {
    /// The entries the wrapped store keeps `stream` under.
    fn kept_stream(&self, stream: Stream<'_>) -> StreamName {
        match stream {
            Stream::Session { machine, key } => StreamName::Session {
                machine: self.digest_of(machine),
                key: self.digest_of(key),
            },
            Stream::Observed { name } => StreamName::Observed {
                name: self.digest_of(name),
            },
        }
    }

    /// The facts a write of the session `key` appends, as the wrapped store is given them.
    pub(super) fn seal_journal<'a>(
        &'a self,
        key: &'a str,
        journal: &Journal<'a>,
    ) -> SealedJournal<'a, S> {
        let machine = journal.machine;
        let stream = self.kept_stream(Stream::Session { machine, key });
        SealedJournal {
            machine: self.digest_of(machine),
            facts: journal
                .facts
                .iter()
                .map(|fact| SealedFact {
                    sealed: self,
                    named: Named::Session { machine, key },
                    stream: stream.clone(),
                    fact: *fact,
                })
                .collect(),
        }
    }

    /// What the record of `stored`, an entry of the wrapped store, holds; refused when it was
    /// sealed for another stream, another origin id or another position.
    fn open_journaled(&self, stored: &JournalEntry) -> Result<Opened, Failure> {
        let at = indexed(stored.stream.stream(), stored.position);
        let plaintext = self.open(STORE_JOURNAL, &at, &stored.data)?;
        let held = opened(&plaintext)?;
        let origin = held.origin.as_deref().map(|origin| self.digest_of(origin));
        if self.kept_stream(held.stream.stream()) != stored.stream || origin != stored.origin {
            return Err(SealError::Refused.into());
        }
        Ok(held)
    }

    /// `stored`, an entry of the wrapped store, opened.
    fn opened_journaled(&self, stored: JournalEntry) -> Result<JournalEntry, Failure> {
        let held = self.open_journaled(&stored)?;
        Ok(JournalEntry {
            position: stored.position,
            recorded_ms: stored.recorded_ms,
            stream: held.stream,
            origin: held.origin,
            schema: held.schema,
            revision: held.revision,
            valid_ms: held.valid_ms,
            actor: held.actor,
            read: held.read,
            data: held.data,
        })
    }

    /// `page`, a page of the wrapped store, opened entry by entry.
    fn opened_page(&self, page: JournalPage) -> Result<JournalPage, Failure> {
        Ok(JournalPage {
            entries: page
                .entries
                .into_iter()
                .map(|stored| self.opened_journaled(stored))
                .collect::<Result<Vec<_>, _>>()?,
            horizon: page.horizon,
        })
    }
}

impl<S: JournalStore> Sealed<S> {
    /// Write every entry of the journal sealed under a key other than the active one again under
    /// the active one, each by the wrapped store's replacement from the bytes the pass read: one
    /// another process changed in between is left for a later pass. One that does not open is
    /// counted and left. An entry keeps its position, its moments and its stream, so no answer
    /// changes.
    ///
    /// The pass reads up to the horizon it is told, so over a store whose horizon may stand below
    /// its last position an entry sealed under the key before can still appear above it: a pass
    /// that answers nothing left is the last one once the horizon passed every position given
    /// while a process still sealed under that key.
    pub fn reseal_journal(&self) -> Result<Resealed, Failure> {
        let mut resealed = Resealed::default();
        let mut after = 0;
        loop {
            let page = self.inner.after(after, PAGE)?.entries;
            let Some(last) = page.last().map(|stored| stored.position) else {
                break;
            };
            for stored in page {
                let at = indexed(stored.stream.stream(), stored.position);
                match self.reseal_record(STORE_JOURNAL, &at, &stored.data)? {
                    Reseal::Current => {}
                    Reseal::Refused => resealed.refused += 1,
                    Reseal::Record(record) => resealed.count(self.inner.reseal_journaled(
                        stored.position,
                        &stored.data,
                        &record,
                    )?),
                }
            }
            after = last;
        }
        Ok(resealed)
    }
}

impl<S: JournalStore> JournalStore for Sealed<S> {
    fn after(&self, position: u64, limit: usize) -> Result<JournalPage, Failure> {
        self.opened_page(self.inner.after(position, limit)?)
    }

    fn stream(&self, stream: Stream<'_>, after: u64, limit: usize) -> Result<JournalPage, Failure> {
        let kept = self.kept_stream(stream);
        self.opened_page(self.inner.stream(kept.stream(), after, limit)?)
    }

    fn position_at(&self, ms: u64) -> Result<u64, Failure> {
        self.inner.position_at(ms)
    }

    /// Every entry of the stream at or below the position asked, opened a page at a time, of which
    /// the first `limit` past `after` are kept.
    fn as_of(
        &self,
        stream: Stream<'_>,
        as_of: &AsOf,
        after: AsOfCursor,
        limit: usize,
    ) -> Result<JournalPage, AsOfError> {
        let kept = self.kept_stream(stream);
        let mut page = self.inner.stream(kept.stream(), 0, PAGE)?;
        let horizon = page.horizon;
        as_of.admitted(horizon)?;
        let mut first: BTreeMap<AsOfCursor, JournalEntry> = BTreeMap::new();
        while let Some(last) = page.entries.last().map(|stored| stored.position) {
            for stored in page.entries {
                if stored.position > as_of.position {
                    break;
                }
                let entry = self.opened_journaled(stored)?;
                if as_of.includes(&entry) && entry.cursor() > after {
                    first.insert(entry.cursor(), entry);
                    if first.len() > limit {
                        first.pop_last();
                    }
                }
            }
            if last >= as_of.position {
                break;
            }
            page = self.inner.stream(kept.stream(), last, PAGE)?;
        }
        Ok(JournalPage {
            entries: first.into_values().collect(),
            horizon,
        })
    }

    fn observe(
        &self,
        name: &str,
        origin: &str,
        fact: &Fact<'_>,
        now_ms: u64,
    ) -> Result<Observation, Failure> {
        if fact.actor.is_some() {
            return Err(Failure::new(ObservedActor));
        }
        if !fact.read.is_empty() {
            return Err(Failure::new(ObservedRead));
        }
        let stream = Stream::Observed { name };
        let sealed = SealedFact {
            sealed: self,
            named: Named::Observed { name, origin },
            stream: self.kept_stream(stream),
            fact: *fact,
        };
        self.inner.observe(
            &self.digest_of(name),
            &self.digest_of(origin),
            &sealed.given(),
            now_ms,
        )
    }

    /// The entry at `position`, opened and held to `held`, sealed again with `data`.
    fn reseal_journaled(&self, position: u64, held: &[u8], data: &[u8]) -> Result<bool, Failure> {
        let Some(before) = position.checked_sub(1) else {
            return Ok(false);
        };
        let Some(stored) = self
            .inner
            .after(before, 1)?
            .entries
            .into_iter()
            .find(|stored| stored.position == position)
        else {
            return Ok(false);
        };
        let opened = self.open_journaled(&stored)?;
        if opened.data != held {
            return Ok(false);
        }
        let record = self.seal(
            STORE_JOURNAL,
            &indexed(stored.stream.stream(), position),
            &payload(
                opened.named()?,
                &opened.schema,
                opened.revision,
                opened.valid_ms,
                opened.actor.as_ref().map(ActorName::actor),
                &opened.read,
                data,
            ),
        )?;
        self.inner.reseal_journaled(position, &stored.data, &record)
    }

    /// The wrapped store's mark of the reader, kept under the entry of its name as a stream's is.
    fn read_through(&self, reader: &str) -> Result<u64, Failure> {
        self.inner.read_through(&self.digest_of(reader))
    }

    fn keep_read(&self, reader: &str, position: u64) -> Result<u64, Failure> {
        self.inner.keep_read(&self.digest_of(reader), position)
    }
}
