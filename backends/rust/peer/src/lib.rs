// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The peer core of a device that holds a shared list (ADR 0079): one library that composes the
//! device's trust (`sce-rust-trust`), the relay's client side (`sce-rust-relay`), the device of the
//! list's group (`sce-rust-mls`) and the state the device keeps, and that does no I/O itself.
//!
//! The machine that holds the list (the standard shared record list, generated in the
//! application's language) talks to the core only in the record list's own events, and the relay is
//! reached through whatever carries the relay protocol's texts. Every input and output is a text in
//! the form the Mesh and the relay protocol already use:
//!
//! - the machine's outbound `record_list.change`, as the event's name and its JSON payload, is
//!   signed whole, one change as one operation, and remembered ([`Peer::outbound`]); the core says
//!   whether it took one, held it already (a machine built again from its host's log sends its
//!   changes again, and a change is signed once), found it is no event that is signed, keeps it
//!   until the device may sign it (an invited device that has not read the log that lets it write,
//!   a restored one that has not read its own chain back), or refused it and why;
//! - a round with the relay ([`Peer::start`]) is a sequence of request texts the core gives
//!   ([`Peer::poll_request`]) and response texts the host gives back ([`Peer::respond`]), or a
//!   failure of the transport ([`Peer::fail`]); its outcome ([`Peer::take_outcome`]) is the events
//!   for the machine in order, the refusals, whether the machine is to be built again, whether the
//!   round stalled, and what the group's device did with what was read. A host that has a
//!   [`Transport`] (the relay's WebSocket binding, the deployment transport's adapter) gives the core
//!   and the transport to the one driver ([`drive`]) instead, with a deadline and a cancellation; the
//!   core stays the same exchange ([`Exchange`]), and the driver, not the core, does the waiting;
//! - an administrator's acts ([`Peer::open_list`], [`Peer::grant`], [`Peer::add_member`],
//!   [`Peer::remove_member`]) write the entries of the membership log and make the group's change
//!   that goes with them in one call; the entries go to the relay with the next round, and the
//!   commit and the Welcome a change of the group makes, and the keys of the epochs before it that
//!   a device admitted is given, are given to it after that round;
//! - a device logs in at a relay with its person's key ([`Peer::login`], ADR 0080): the same
//!   exchange over the same transport, made on a connection that presents no token, or one that
//!   expired, whose outcome holds the token the relay issued ([`Outcome::token`]); the host gives the
//!   token to its transport, which carries it, and the core holds none. At a relay that takes a
//!   creation with no token, the login of a list the relay does not hold creates it, under a name
//!   that certifies its genesis ([`Peer::certified_name`]): a list opened there, or one that lapsed
//!   there, which the next round gives back what the device holds;
//! - an administrator's device invites a device that is not in the log yet ([`Peer::invite`], ADR
//!   0080 section 4): a text for a QR code that holds the relay's address, the signed invitation and
//!   its secret, of which the core keeps the key the secret makes; the invited device joins with it
//!   ([`Peer::join`]), leaving its identity key and its key package with a proof; and the
//!   administrator's device admits only a key a proof under its invitation names
//!   ([`Peer::joined`], [`Peer::add_member`], [`Peer::grant`]), after the two people compare the
//!   fingerprint each device shows;
//! - what the device stands at in the list (its person's role, the members, whether it waits for its
//!   own chain, what the relay is still owed) is [`Peer::standing`];
//! - what the device keeps between runs, its secrets with the rest, is one state sealed under a key
//!   drawn for it ([`Peer::save_sealed`], [`Peer::restore_sealed`]), which the host wraps with its
//!   platform's key store.
//!
//! The core reads no clock, opens no socket, starts no thread and waits on nothing: a host may hold
//! a response for minutes, lose its connection, or be stopped by its operating system in the middle
//! of a round, and the core goes on from what it was given. Nothing the core does depends on the time
//! today. When it comes to (a write's stamp judged against the receiving clock, ADR 0034 section 3;
//! the instant at which the core next wants to be called, for a relay's allowance or an operation
//! parked for want of a key, ADR 0079 section 3), the time enters as an argument of the calls that
//! need it, read by the host and given with each call.
//!
//! The secrets of the device (the person's key, the administrator's key it holds, the keys of the
//! epochs, the group's state) are given once ([`Peer::set_identity`], [`Peer::set_administrator`],
//! [`Peer::with_group`]) and are kept, with everything else the device keeps, in the state the core
//! seals (ADR 0079 section 6): envelope encryption under a data key the core draws for each save,
//! which the host wraps with a key its platform's key store holds and does not export, and gives back
//! unwrapped to restore. The core calls no platform's API, and the host handles no protocol secret
//! in the clear.

#![forbid(unsafe_code)]

pub use sce_rust_mls::host::{Member, Step as GroupStep};
pub use sce_rust_mls::{Device, MlsError};
// The key a sealed state is sealed under, and why a sealed state was not made or not opened.
pub use sce_rust_trust::backup::{BackupError, DataKey};
// What runs the core over a transport: the one driver, on the system's clock or another, the
// exchange it carries, how it ended, and the transport's contract with what it is given.
pub use sce_rust_relay::{
    drive, drive_with, Clock, Driven, Exchange, Signal, SystemClock, Transport, TransportError,
};
pub use sce_rust_relay::{CourierError, Kept, Publication, Round, RoundKind, Token};
pub use sce_rust_trust::membership::{Hash, LogError, Profile, OWNER, READER, WRITER};

use std::collections::BTreeMap;

use sce_rust_mls::host::Issued;
use sce_rust_relay::client::{floor_event, keep};
use sce_rust_relay::logs::list_digest;
use sce_rust_relay::{
    Code, CourierState, JoinMachine, JoinStep, LinkError, LoginMachine, LoginStep, Record, Request,
    Response, RoundMachine, Step,
};
use sce_rust_trust::backup::{self, Contents};
use sce_rust_trust::creation;
use sce_rust_trust::encoding;
use sce_rust_trust::invitation::{self, Invitation, Ticket};
use sce_rust_trust::membership::{Body, Entry};
use sce_rust_trust::{Identity, PublicKey, Trust};
use serde_json::{json, Value};

/// The version of the state [`Peer::save_sealed`] seals. Version 3 is the first that is sealed and
/// holds the device's secrets with the rest. The texts of versions 1 and 2 held no secret, were
/// taken back only after the host gave every key again, and were never written outside this
/// repository's tests (ADR 0079 ships no device before its review); they are not read. Version 4
/// holds what the machine sent that waits to be signed ([`Peer::outbound`], [`Kept::Waiting`]),
/// and what became of the sends that waited that a round did not hand over; the version is bound
/// into the sealed state, so a core of version 3, which would open it and lose them, does not open
/// it. Version 5 holds the invitations an administrator's device made and the joins under them it
/// read ([`Peer::invite`], [`Peer::joined`]), which a core of version 4 would lose. States of
/// versions 3 and 4 were written by this repository's tests alone, and are not read.
pub const SAVE_VERSION: u64 = 5;

/// The names under which the sealed state keeps what the core holds beside the person's key and the
/// keys of the epochs ([`Trust::secrets`]).
const KEPT_STATE: &str = "peer/state";
const KEPT_ADMINISTRATOR: &str = "peer/administrator";
const KEPT_GROUP: &str = "peer/group";
const KEPT_GROUP_NODE: &str = "peer/group-node";
const KEPT_GROUP_KEY: &str = "peer/group-key";
const KEPT_GROUP_HELD: &str = "peer/group-held";

/// What a sealed state is bound to beside its list and its node: the core's state, at its version.
fn sealed_context() -> String {
    format!("sce-peer/state/{SAVE_VERSION}")
}

/// What a round, or a publication, came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The round: the events for the machine in order, what the relay was given, the refusals, the
    /// group's messages read, what the machine is to be built again without or with, and whether it
    /// stalled. A publication's round tells the machine nothing.
    pub round: Round,
    /// The second read from the start that the group's device owed, when it owed one (a commit the
    /// log names that it dropped at its limit): its events are for the machine too, after the
    /// round's.
    pub reread: Option<Round>,
    /// What the group's device did with what was read, in order.
    pub steps: Vec<GroupStep>,
    /// How many of the commits, Welcomes and keys given the administrator's acts made the relay is
    /// still owed once the exchange ended. They are given after each round, in order, and kept until
    /// the relay takes them; one it did not take is among the refusals, with why.
    pub owed: usize,
    /// The token a login earned ([`Peer::login`]), and when it expires by the relay's clock; `None`
    /// for every other exchange.
    pub token: Option<Token>,
}

impl Outcome {
    /// What a login that came to `token`, or a join (no token), came to: a round that told the
    /// machine nothing.
    fn of_exchange(token: Option<Token>, owed: usize) -> Self {
        Self {
            round: Round::default(),
            reread: None,
            steps: Vec::new(),
            owed,
            token,
        }
    }

    /// Every event for the machine, in order.
    pub fn events(&self) -> Vec<(String, String)> {
        self.rounds()
            .flat_map(|round| round.events.iter().cloned())
            .collect()
    }

    /// Every refusal, in order: each is a record that was not taken, and the rest went on.
    pub fn refused(&self) -> Vec<String> {
        self.rounds()
            .flat_map(|round| round.refused.iter().cloned())
            .collect()
    }

    /// What the machine sent that waited to be signed ([`Kept::Waiting`]) and was signed now, a new
    /// operation of the device's chain each, in the order it was sent, each as the machine sent it:
    /// the relay was given it.
    pub fn taken(&self) -> Vec<(String, String)> {
        self.rounds()
            .flat_map(|round| round.taken.iter().cloned())
            .collect()
    }

    /// What the machine sent that waited to be signed and that the device held already when the
    /// round came to it, in the order it was sent: a device restored from an older state kept it
    /// waiting, and the round read the device's own chain back from the relay, which held it.
    /// Nothing was signed for it and the relay was given nothing new; the host has nothing to do.
    pub fn held(&self) -> Vec<(String, String)> {
        self.rounds()
            .flat_map(|round| round.held.iter().cloned())
            .collect()
    }

    /// What the machine sent that waited to be signed and never will be, each with why: the log
    /// the round read says the device's person may not write. The machine took each in as made and
    /// no other copy will hold it, so the machine is built again ([`Self::rebuild`]); a host that
    /// keeps its own log of commands takes each out of that log, and may tell its person.
    pub fn dropped(&self) -> Vec<(String, String, String)> {
        self.rounds()
            .flat_map(|round| round.dropped.iter().cloned())
            .collect()
    }

    /// Whether the host is to build its machine again and make a [`RoundKind::Resync`]: the core
    /// found operations the machine was told and is to be without, or was not told and is to be
    /// with ([`Round::evicted`], [`Round::restored`]), or a send of the machine's own that it took
    /// in will never be signed ([`Self::dropped`]).
    pub fn rebuild(&self) -> bool {
        self.rounds().any(|round| {
            !round.evicted.is_empty() || !round.restored.is_empty() || !round.dropped.is_empty()
        })
    }

    /// Whether the round stopped before the end of what the relay holds; a later round goes on
    /// from there.
    pub fn stalled(&self) -> bool {
        self.round.stalled
    }

    /// The number the device's own chain is frozen from, when this exchange froze it
    /// ([`Round::conflicted`]): the relay answered `conflict` to an operation of the device's own,
    /// or a round read one that conflicts with it (ADR 0083 section 4). The device signs nothing
    /// more, every host that holds both freezes it at the same number, and the host tells its
    /// person that an administrator has to take the device off and grant a new one. `None` when
    /// nothing of the device's own froze.
    pub fn conflicted(&self) -> Option<u64> {
        self.rounds().find_map(|round| round.conflicted)
    }

    fn rounds(&self) -> impl Iterator<Item = &Round> {
        std::iter::once(&self.round).chain(self.reread.iter())
    }

    /// Keep what the outcome hands over in `state`, for the next round to hand over first: what an
    /// outcome not taken, or abandoned before it was given, leaves.
    fn hold_in(&self, state: &mut CourierState) {
        if let Some(reread) = &self.reread {
            state.hold(reread.clone());
        }
        state.hold(self.round.clone());
    }
}

/// Why the core did not do what its host asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerError {
    /// No request of an exchange waits for an answer: the host answered before it asked
    /// ([`Peer::poll_request`]), or answered twice.
    NothingAsked,
    /// The outcome of the last exchange was not taken ([`Peer::take_outcome`]). The next exchange
    /// is not begun until it is, so that nothing it hands over is lost.
    OutcomeNotTaken,
    /// The core holds no device of the list's group ([`Peer::with_group`]).
    NoGroupDevice,
    /// The group's device refused, and why.
    Group(MlsError),
    /// Bytes given as a sealed state are not one ([`Peer::save_sealed`]), or what it holds does not
    /// hold for this core, and why.
    Unreadable(String),
    /// A sealed state did not open under the key given, for this list and the node given: the key
    /// is not the one it was sealed under, the state was changed or cut, or it is another device's.
    /// A cipher cannot tell these apart. Nothing of it was taken.
    NotOpened,
    /// The operating system gave no randomness for a data key, and no weaker source is used.
    NoRandomness,
    /// An administrator's act and the core holds no administrator's key
    /// ([`Peer::set_administrator`]).
    NoAdministrator,
    /// The act needs the device's person and the core has none ([`Peer::set_identity`]).
    NoPerson,
    /// The list is open already: the core's log holds its genesis.
    Opened,
    /// The membership log refused the entry the act wrote, and why: an administrator's key the log
    /// does not name, a genesis that is not the one the device was invited with. Nothing of the act
    /// was written.
    Log(LogError),
    /// The device of this node is no member of the list now.
    NotAMember(u64),
    /// A role that is not one a grant gives ([`READER`] to [`OWNER`]; a device is taken off with
    /// [`Peer::remove_member`]).
    NoSuchRole(u8),
    /// The list's content is keyed by its group, and the grant would make the device of this node
    /// a member the group does not hold (a new member, or a new key): a device is admitted to such
    /// a list with its key package ([`Peer::add_member`]), so that the log and the group hold the
    /// same members.
    AdmittedThroughGroup(u64),
    /// No key package of the device of this node was read from the relay: the device leaves one
    /// ([`Peer::publish_key_package`]) and a round of the administrator's device reads it.
    NoKeyPackage(u64),
    /// The list's content is sealed under keys an administrator drew (`Trust::rotate_epoch`) and
    /// not under its group's: taking a member off it gives the members left a new key, which the
    /// core does not hand over, and taking one off with no new epoch would leave what is written
    /// after it readable by the device taken off.
    DrawnKeys,
    /// The log has granted the device of this node already: an invitation is for a node it never
    /// granted ([`Peer::invite`]).
    Granted(u64),
    /// An invitation that is not one, or one this core cannot make or join with, and why: a text
    /// that is not an invitation's, another list or genesis than the core's, another node than its
    /// person's, a lifetime past a day, a list not opened yet.
    Invitation(String),
    /// The device of this node was invited by this core, and no join under its invitation that a
    /// round read names the key given with a proof that holds: a key package or a key the relay
    /// made up, or a join not read yet ([`Peer::joined`]).
    NotJoined(u64),
}

/// A device that joined under an invitation this core made, as [`Peer::joined`] tells it: the key
/// it proved, and the digits the two people compare before the administrator's device admits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedDevice {
    pub key: PublicKey,
    pub fingerprint: String,
}

/// An invitation this core made: the key its secret makes, and when it expires.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Invited {
    key: PublicKey,
    expires_at_ms: u64,
}

/// A join a round read whose proof holds under an invitation this core made.
#[derive(Debug, Clone, PartialEq, Eq)]
struct JoinHeld {
    /// The key of the invitation it was made under.
    invitation_key: PublicKey,
    /// The identity key it proved.
    key: PublicKey,
    /// Its key package, empty for a device that left none.
    key_package: Vec<u8>,
}

/// The exchange of a challenge and a signed request under way instead of a round.
enum Asking {
    Login(Box<LoginMachine>),
    Join(Box<JoinMachine>),
}

impl Asking {
    fn is_asking(&self) -> bool {
        match self {
            Asking::Login(machine) => machine.is_asking(),
            Asking::Join(machine) => machine.is_asking(),
        }
    }

    fn respond(&mut self, answer: Result<Response, LinkError>) {
        match self {
            Asking::Login(machine) => machine.respond(answer),
            Asking::Join(machine) => machine.respond(answer),
        }
    }

    /// The request to put to the relay next, or what the exchange came to: the token of a login,
    /// none for a join.
    fn poll(&mut self) -> Result<Request, Result<Option<Token>, CourierError>> {
        match self {
            Asking::Login(machine) => match machine.poll() {
                LoginStep::Request(request) => Ok(request),
                LoginStep::Done(result) => Err(result.map(Some)),
            },
            Asking::Join(machine) => match machine.poll() {
                JoinStep::Request(request) => Ok(request),
                JoinStep::Done(result) => Err(result.map(|()| None)),
            },
        }
    }
}

/// What [`Peer::save_sealed`] gives: the device's state, sealed, and the key it was sealed under.
/// The host wraps the key with a key its platform's key store holds, keeps the wrapped key beside the
/// state, and lets go of this one; to restore, it unwraps the key and gives both back
/// ([`Peer::restore_sealed`]).
#[derive(Debug)]
pub struct SealedState {
    pub state: Vec<u8>,
    pub key: DataKey,
}

/// A sealed state that was not made or not opened, as the core says it.
fn sealing_error(error: BackupError) -> PeerError {
    match error {
        BackupError::Corrupt => PeerError::NotOpened,
        BackupError::NoRandomness => PeerError::NoRandomness,
        BackupError::Malformed | BackupError::TooCostly => {
            PeerError::Unreadable("it is not a sealed state".to_string())
        }
    }
}

/// What a sealed state holds that is no secret, read whole before the core takes any of it.
struct Saved {
    entries: Vec<Entry>,
    trust: String,
    courier: CourierState,
    owed: Vec<Publication>,
    invitations: BTreeMap<u64, Vec<Invited>>,
    joins: BTreeMap<u64, Vec<JoinHeld>>,
}

/// The invitations a core made, as its state holds them: each node in decimal, the key in the URL
/// alphabet and the expiry as text.
fn invitations_value(invitations: &BTreeMap<u64, Vec<Invited>>) -> Value {
    invitations
        .iter()
        .flat_map(|(node, invited)| {
            invited.iter().map(move |one| {
                json!({"node": node.to_string(), "key": one.key.to_text(),
                       "expires_at_ms": one.expires_at_ms.to_string()})
            })
        })
        .collect()
}

/// The joins a core holds, as its state holds them.
fn joins_value(joins: &BTreeMap<u64, Vec<JoinHeld>>) -> Value {
    joins
        .iter()
        .flat_map(|(node, held)| {
            held.iter().map(move |join| {
                json!({"node": node.to_string(),
                       "invitation_key": join.invitation_key.to_text(),
                       "key": join.key.to_text(),
                       "key_package": encoding::encode(&join.key_package)})
            })
        })
        .collect()
}

/// The rows of `name` in a saved state, each read by `read` with its node.
fn read_rows<T>(
    value: &Value,
    name: &str,
    read: impl Fn(&Value) -> Option<T>,
) -> Option<BTreeMap<u64, Vec<T>>> {
    let mut rows: BTreeMap<u64, Vec<T>> = BTreeMap::new();
    for row in value.get(name)?.as_array()? {
        let node = row.get("node")?.as_str()?.parse().ok()?;
        rows.entry(node).or_default().push(read(row)?);
    }
    Some(rows)
}

fn key_in(row: &Value, name: &str) -> Option<PublicKey> {
    PublicKey::from_text(row.get(name)?.as_str()?)
}

impl Saved {
    fn read(text: &str) -> Result<Self, PeerError> {
        let unreadable = |why: &str| PeerError::Unreadable(why.to_string());
        let value: Value = serde_json::from_str(text).map_err(|_| unreadable("it is not JSON"))?;
        if value.get("version").and_then(Value::as_u64) != Some(SAVE_VERSION) {
            return Err(unreadable("it is not a version this core reads"));
        }
        let owed = value
            .get("owed")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("it holds no list of what the relay is owed"))?
            .iter()
            .map(Publication::from_value)
            .collect::<Option<Vec<Publication>>>()
            .ok_or_else(|| unreadable("what it says the relay is owed is not a publication"))?;
        let courier = value
            .get("courier")
            .and_then(Value::as_str)
            .and_then(CourierState::from_text)
            .ok_or_else(|| unreadable("it holds no state of the relay's client"))?;
        let trust = value
            .get("trust")
            .and_then(Value::as_str)
            .ok_or_else(|| unreadable("it holds nothing of the trust"))?
            .to_string();
        let entries = value
            .get("log")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("it holds no log"))?
            .iter()
            .map(|entry| entry.as_str().and_then(Entry::from_json))
            .collect::<Option<Vec<Entry>>>()
            .ok_or_else(|| unreadable("an entry of its log is not one"))?;
        let invitations = read_rows(&value, "invitations", |row| {
            Some(Invited {
                key: key_in(row, "key")?,
                expires_at_ms: row.get("expires_at_ms")?.as_str()?.parse().ok()?,
            })
        })
        .ok_or_else(|| unreadable("the invitations it holds are not ones"))?;
        let joins = read_rows(&value, "joins", |row| {
            Some(JoinHeld {
                invitation_key: key_in(row, "invitation_key")?,
                key: key_in(row, "key")?,
                key_package: encoding::decode(row.get("key_package")?.as_str()?)?,
            })
        })
        .ok_or_else(|| unreadable("the joins it holds are not ones"))?;
        Ok(Self {
            entries,
            trust,
            courier,
            owed,
            invitations,
            joins,
        })
    }
}

/// What a device stands at in its list, for its host to show as it is: [`Peer::standing`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    /// The hash of the list's genesis, which a person is invited with; `None` while the core holds
    /// no entry of the log (a list not opened yet, or not read yet by an invited device).
    pub genesis: Option<Hash>,
    /// The device's person: their node and their public key.
    pub person: Option<(u64, PublicKey)>,
    /// The person's role now, 0 when they are no member.
    pub role: u8,
    /// Whether the core holds an administrator's key that the log names as one.
    pub administrator: bool,
    /// Whether the device signs nothing new until a round has read its own chain back from the
    /// relay ([`Peer::restore_sealed`]).
    pub awaits_own_chain: bool,
    /// The members of the list now, each node with its role, in the order of the nodes.
    pub members: Vec<(u64, u8)>,
    /// The latest epoch the log begins, 0 when none.
    pub epoch: u32,
    /// The epochs whose keys the device holds: what it can open.
    pub keys: Vec<u32>,
    /// How many of the commits, Welcomes and keys given the administrator's acts made the relay is
    /// still owed.
    pub owed: usize,
    /// How many sends of the machine wait to be signed ([`Kept::Waiting`]): what a host shows as
    /// not sent yet.
    pub waiting: usize,
}

/// What the list's content is sealed under, which decides how a member is admitted and taken off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Keying {
    /// Nothing yet: no epoch has begun, and the core holds no group.
    Clear,
    /// The list's MLS group: a member is admitted and taken off with a commit of the group.
    Group,
    /// Keys an administrator drew (`Trust::rotate_epoch`).
    Drawn,
}

/// The peer core of one device of one list.
///
/// One exchange with the relay runs at a time: a round ([`Self::start`]) or a publication
/// ([`Self::publish`]). Beginning one while another runs abandons the one that runs, which loses
/// nothing: what it took in and did not hand over is handed over by the next round, and the state
/// it leaves is the one the relay's client keeps after a completed step ([`CourierState`]).
pub struct Peer {
    /// The list's name: what the group's device and a sealed state are made for.
    list: String,
    trust: Trust,
    /// What the relay's client keeps between exchanges; while one runs, its machine holds it.
    state: CourierState,
    running: Option<Box<Running>>,
    outcome: Option<Result<Outcome, CourierError>>,
    group: Option<Member>,
    /// The administrator's key the device holds, when it holds one.
    administrator: Option<Identity>,
    /// The commits, Welcomes and keys given the administrator's acts made that the relay has not
    /// taken, in the order they were made.
    owed: Vec<Publication>,
    /// A login ([`Self::login`]) or a join ([`Self::join`]) under way: an exchange of its own, which
    /// keeps nothing of the relay's client and runs instead of a round.
    asking: Option<Asking>,
    /// The invitations this core made, by the node each is for ([`Self::invite`]).
    invitations: BTreeMap<u64, Vec<Invited>>,
    /// The joins a round read under them whose proof holds, by node ([`Self::joined`]).
    joins: BTreeMap<u64, Vec<JoinHeld>>,
}

/// An exchange with the relay under way.
struct Running {
    machine: RoundMachine,
    purpose: Purpose,
}

enum Purpose {
    /// A round the host began.
    Round,
    /// The second read from the start that the group's device owed after `first`, which ended.
    Follow {
        first: Box<Round>,
        steps: Vec<GroupStep>,
    },
    /// A publication.
    Publish,
    /// The first publication the relay is owed, given after a round that came to `outcome`.
    Deliver { outcome: Box<Outcome> },
}

impl Running {
    /// The state the relay's client is left with when the exchange stops here. A round that ended
    /// before a second read, or before what the relay is owed was given, hands over what it found
    /// by the next round, before what the exchange that stopped took in.
    fn into_state(self) -> CourierState {
        let Running { machine, purpose } = self;
        let mut state = machine.into_state();
        hold_ended(&purpose, &mut state);
        state
    }

    /// The state [`Self::into_state`] would leave, with the exchange left running.
    fn saved_state(&self) -> CourierState {
        let mut state = self.machine.abandoned_state();
        hold_ended(&self.purpose, &mut state);
        state
    }
}

/// Keep in `state` what the round that ended before the exchange of `purpose` found and has not
/// handed over.
fn hold_ended(purpose: &Purpose, state: &mut CourierState) {
    match purpose {
        Purpose::Follow { first, .. } => state.hold((**first).clone()),
        Purpose::Deliver { outcome } => outcome.hold_in(state),
        Purpose::Round | Purpose::Publish => {}
    }
}

impl Peer {
    /// The core of the device of whoever makes the list `list`: its trust takes the genesis it is
    /// first given (the one [`Self::open_list`] makes, or its owner made) and every entry after it
    /// ([`Trust::open`]).
    pub fn open(list: &str) -> Self {
        Self::over(list, Trust::open(list))
    }

    /// The name of a list to open at a relay that takes a creation with no token (ADR 0080 section
    /// 3): `n1.<salt>.<digest>`, which certifies `administrator`, the key the list's genesis names
    /// first ([`sce_rust_trust::creation`]), so that no device without that key creates a list
    /// under it, before or after the list lapses. The host makes the core with this name
    /// ([`Self::open`]), gives it that administrator's key ([`Self::set_administrator`]) and opens
    /// the list ([`Self::open_list`]); the core's login creates the list at such a relay
    /// ([`Self::login`]). `salt` is the host's (any 1 to 32 bytes; two lists of one administrator
    /// take two salts), and [`Self::draw_certified_name`] draws one. `None` for a salt that is empty
    /// or longer.
    pub fn certified_name(administrator: &PublicKey, salt: &[u8]) -> Option<String> {
        creation::list_name(salt, administrator)
    }

    /// [`Self::certified_name`] under a salt of 16 bytes drawn from the operating system, the only
    /// source it is drawn from.
    pub fn draw_certified_name(administrator: &PublicKey) -> Result<String, PeerError> {
        let salt = creation::draw_salt().map_err(|_| PeerError::NoRandomness)?;
        // A salt of 16 bytes is within what a name carries: this never refuses.
        creation::list_name(&salt, administrator).ok_or(PeerError::NoRandomness)
    }

    /// The core of a device invited to the list `list` with the hash of its genesis entry, which the
    /// person was told in a channel they trust: it takes that genesis and no other
    /// ([`Trust::invited`]).
    pub fn invited(list: &str, genesis: Hash) -> Self {
        Self::over(list, Trust::invited(list, genesis))
    }

    fn over(list: &str, trust: Trust) -> Self {
        Self {
            list: list.to_string(),
            trust,
            state: CourierState::default(),
            running: None,
            outcome: None,
            group: None,
            administrator: None,
            owed: Vec::new(),
            asking: None,
            invitations: BTreeMap::new(),
            joins: BTreeMap::new(),
        }
    }

    /// The device's person: the node the log names them by, and their key.
    pub fn set_identity(&mut self, node: u64, identity: Identity) {
        self.trust.set_identity(node, identity);
    }

    /// The device holds an administrator's key: the one the genesis names when the device opens the
    /// list ([`Self::open_list`]), and the one its administrator's acts are signed with. It is a
    /// secret, kept in the sealed state as the person's key is ([`Self::save_sealed`]).
    pub fn set_administrator(&mut self, administrator: Identity) {
        self.administrator = Some(administrator);
    }

    /// The machine is a hub's: it is told who ever wrote, not who writes now. Set before the first
    /// round.
    pub fn for_a_hub(mut self) -> Self {
        self.state = std::mem::take(&mut self.state).for_a_hub();
        self
    }

    /// The core holds `device`, the device of `node` in the list's MLS group: what a round reads of
    /// the group it takes in, in the order the log names the epochs, and it gives the trust the key
    /// of each epoch it reaches.
    pub fn with_group(mut self, device: Device, node: u64) -> Self {
        self.group = Some(Member::new(device, self.trust.clone(), node));
        self.note_keying();
        self
    }

    /// The trust of the device: the membership log, the chains, the checkpoints and the keys of the
    /// epochs, for what a host reads of them that the core does not tell (the heads, the chains of
    /// each device, an audit's record). It stays open to the host because a host reads it; it is not
    /// how a list is run. An entry appended through it is outside the core's administrator's acts,
    /// which alone keep a change of the log and the change of the group that goes with it together.
    pub fn trust(&self) -> &Trust {
        &self.trust
    }

    /// The device of the list's group, when the core holds one: what a host reads of it (its epoch,
    /// what it keeps for the host's store). Its changes are the administrator's acts of the core.
    pub fn group(&self) -> Option<&Member> {
        self.group.as_ref()
    }

    /// Open a new list as its administrator. The genesis names the administrator's key the device
    /// holds as the list's one administrator, with a quorum of one and `profile`, and a grant makes
    /// the device's person a member with the highest role ([`OWNER`]). When the core holds a device
    /// of the list's group, the group is made with that device alone in it, so that the list is
    /// keyed by its group from its first entry and every member after is admitted to both at once
    /// ([`Self::add_member`]). The entries go to the relay with the next round. What is returned is
    /// the hash of the genesis, which a person is invited with ([`Self::invited`]).
    ///
    /// **A list keyed by its group is sealed from its first write.** The epoch a group is made at
    /// has no commit, so no entry of the log can name it, and the log decides that content is
    /// sealed from the entry that begins the first epoch: a group of one would leave everything its
    /// device writes before it admits anyone in the relay's clear, for good. So the opening moves
    /// the group to its first epoch with a commit of the device's own leaf (an MLS self-update,
    /// which adds no one) and writes the entry that names it after the grant: the third entry of
    /// the log, before the device can sign anything. The relay is owed that commit, as it is the
    /// commit of every epoch ([`Outcome::owed`]). Naming the epoch the group is made at instead
    /// would take an entry of the log no commit stands behind, which every member's group checks
    /// the log's epochs against; the self-update is the commit MLS has for a group that changes no
    /// member. What the device writes before a device is admitted is sealed under the key of that
    /// epoch, which a device admitted later is given ([`Self::add_member`]).
    ///
    /// What can refuse is asked before anything is written (an administrator's key, a person, a log
    /// with no genesis, a group not made yet); a core invited to another genesis is refused by its
    /// log ([`LogError::NotTheGenesis`]) and writes nothing.
    pub fn open_list(&mut self, profile: Profile) -> Result<Hash, PeerError> {
        let administrator = self
            .administrator
            .as_ref()
            .ok_or(PeerError::NoAdministrator)?;
        let (node, key) = self.trust.person().ok_or(PeerError::NoPerson)?;
        if self.trust.genesis().is_some() {
            return Err(PeerError::Opened);
        }
        if self
            .group
            .as_ref()
            .is_some_and(|member| member.device().epoch().is_ok())
        {
            return Err(PeerError::Group(MlsError::InGroup));
        }
        let genesis = self
            .trust
            .administer(
                administrator,
                Body::Genesis {
                    administrators: vec![administrator.public()],
                    quorum: 1,
                    profile,
                },
            )
            .map_err(PeerError::Log)?;
        self.trust
            .administer(administrator, self.trust.grant_body(node, OWNER, key))
            .map_err(PeerError::Log)?;
        if let Some(member) = self.group.as_mut() {
            member
                .device_mut()
                .create_group()
                .map_err(PeerError::Group)?;
            self.note_keying();
            self.name_first_epoch()?;
        }
        Ok(genesis.hash())
    }

    /// The list is keyed by its group and the log names no epoch of it: the group moves to its
    /// first epoch with a commit of the device's own leaf, the entry that names it is written, and
    /// the relay is owed the commit ([`Self::open_list`]).
    fn name_first_epoch(&mut self) -> Result<(), PeerError> {
        let administrator = self
            .administrator
            .as_ref()
            .ok_or(PeerError::NoAdministrator)?;
        let member = self.group.as_mut().ok_or(PeerError::NoGroupDevice)?;
        let issued = member.update(administrator).map_err(PeerError::Group)?;
        self.owe(issued, None);
        Ok(())
    }

    /// Tell the trust when the list is keyed by a group the core's device is in, so that no write
    /// of the device's person is signed while the log names no epoch of it
    /// ([`Trust::keyed_by_a_group`]): what holds a list that an earlier core opened, whose log
    /// named no epoch at its opening, until its first member is admitted.
    fn note_keying(&self) {
        if self.keying() == Keying::Group {
            self.trust.keyed_by_a_group();
        }
    }

    /// Grant the device of `node`, which signs with `key`, the role `role` ([`READER`] to
    /// [`OWNER`]): a new member, or another role or key for one. A grant that takes writing away
    /// from a device that could write carries where its chain ends, as the log requires
    /// (`Trust::grant_body`). The entry goes to the relay with the next round.
    ///
    /// A list keyed by its group admits a device the group does not hold with this key only with
    /// its key package ([`Self::add_member`], [`PeerError::AdmittedThroughGroup`]): a grant alone
    /// would make the log's members differ from the group's, and every later commit would be
    /// refused for it. Another role for a member the group holds is a grant alone.
    ///
    /// A device this core invited ([`Self::invite`]) is granted only the key a join a round read
    /// proved under the invitation ([`Self::joined`]), and [`PeerError::NotJoined`] otherwise: on a
    /// list no group keys, the join leaves no key package, and its proof is what makes the key
    /// the device's.
    pub fn grant(&mut self, node: u64, role: u8, key: PublicKey) -> Result<(), PeerError> {
        if !(READER..=OWNER).contains(&role) {
            return Err(PeerError::NoSuchRole(role));
        }
        let administrator = self
            .administrator
            .as_ref()
            .ok_or(PeerError::NoAdministrator)?;
        let member_with_key = self.trust.current_members().contains(&(node, key));
        if self.keying() == Keying::Group && !member_with_key {
            return Err(PeerError::AdmittedThroughGroup(node));
        }
        if !member_with_key {
            self.joined_with(node, key)?;
        }
        self.trust
            .administer(administrator, self.trust.grant_body(node, role, key))
            .map_err(PeerError::Log)?;
        self.admitted(node);
        Ok(())
    }

    /// Admit the device of `node`, which signs with `key`, to the list and to its group with the
    /// role `role`, from the newest key package of the device a round read from the relay. One call,
    /// because the two are one change: the grant (when the log holds none for the device with this
    /// key; a member's role is changed with [`Self::grant`]) and the commit that adds the device
    /// must both be written or neither, or the log's members and the group's would differ. The
    /// commit is made first and nothing is written when the group refuses the key package
    /// (`sce_rust_mls::host`). The entries go to the relay with the next round, and the commit and
    /// the device's Welcome after it ([`Outcome::owed`]): the relay keeps a commit only once its log
    /// names it.
    ///
    /// **The device is given the list's history.** A Welcome gives the key of the epoch it begins
    /// and of none before, as MLS does, and a list's content is one list: a device that held only
    /// the writes of its own epochs would hold rows the others hold without the writes that made
    /// them, and the first device's writes before anyone was admitted would open for no one but it.
    /// So the act gives the device the key of every epoch before its own that the administrator's
    /// device holds, each sealed to the key the device signs with (`Trust::grant_key`, from the
    /// person's key), and the relay is owed them after the Welcome; the device's next round takes
    /// them in and opens what was sealed under them. A device taken off later keeps what it was
    /// given, as it keeps the keys of the epochs it was a member in; nothing written after its
    /// removal opens for it.
    ///
    /// A device this core invited ([`Self::invite`]) is admitted only from the key package of a
    /// join a round read whose proof holds under the invitation's key and names `key`
    /// ([`Self::joined`]), and [`PeerError::NotJoined`] otherwise: a relay knows the invitation's
    /// public key alone, and cannot make a proof for a key package of its own.
    pub fn add_member(&mut self, node: u64, role: u8, key: PublicKey) -> Result<(), PeerError> {
        if !(READER..=OWNER).contains(&role) {
            return Err(PeerError::NoSuchRole(role));
        }
        let administrator = self
            .administrator
            .as_ref()
            .ok_or(PeerError::NoAdministrator)?;
        // A device this core invited is admitted from the key package of its join alone, and with
        // the key its proof names: a key package the relay made up is under no proof that holds.
        let invited = self
            .joined_with(node, key)?
            .map(|join| join.key_package.clone());
        let member = self.group.as_mut().ok_or(PeerError::NoGroupDevice)?;
        let package = match invited {
            Some(package) if package.is_empty() => return Err(PeerError::NoKeyPackage(node)),
            Some(package) => package,
            None => member
                .key_packages_of(node)
                .last()
                .cloned()
                .ok_or(PeerError::NoKeyPackage(node))?,
        };
        let issued = member
            .add_member(administrator, node, role, key, &package)
            .map_err(PeerError::Group)?;
        let before: Vec<u32> = self
            .trust
            .epochs()
            .into_iter()
            .filter(|epoch| *epoch < issued.epoch)
            .collect();
        let history = member.history_grants(&key, &before);
        self.owe(issued, Some(node));
        self.owed.extend(
            history
                .into_iter()
                .map(|(epoch, grant)| Publication::KeyGrant { epoch, node, grant }),
        );
        self.admitted(node);
        Ok(())
    }

    /// Invite the device of `node`, which the log has never granted, to the list at the relay whose
    /// address is `relay` and whose name is `audience`, from `now_ms` (the host's clock) for
    /// `valid_ms` (at most a day, [`invitation::MOST_MS`]) (ADR 0080 section 4). A secret is drawn
    /// from the operating system, the key it makes is kept in the sealed state, and the invitation
    /// is signed with the administrator's key the device holds. What is returned is the text a host
    /// shows as a QR code: the relay's address, the invitation and the secret
    /// ([`sce_rust_trust::invitation::Ticket`]); whoever holds it may join once, so it is shown and
    /// not kept. The invitation is not withdrawn before it expires: its day is its bound.
    pub fn invite(
        &mut self,
        relay: &str,
        audience: &str,
        node: u64,
        now_ms: u64,
        valid_ms: u64,
    ) -> Result<String, PeerError> {
        let secret = invitation::draw_secret().map_err(|_| PeerError::NoRandomness)?;
        self.invite_under(relay, audience, node, now_ms, valid_ms, secret)
    }

    /// [`Self::invite`] with the secret `secret` the host gives: one it drew from a source of its
    /// own, or a recording's fixed one.
    pub fn invite_under(
        &mut self,
        relay: &str,
        audience: &str,
        node: u64,
        now_ms: u64,
        valid_ms: u64,
        secret: [u8; 32],
    ) -> Result<String, PeerError> {
        if valid_ms == 0 || valid_ms > invitation::MOST_MS {
            return Err(PeerError::Invitation(format!(
                "an invitation lives from a millisecond to a day, and not {valid_ms} ms"
            )));
        }
        let administrators = self.trust.administrators();
        let administrator = self
            .administrator
            .as_ref()
            .filter(|key| administrators.contains(&key.public()))
            .ok_or(PeerError::NoAdministrator)?;
        let genesis = self.trust.genesis().ok_or_else(|| {
            PeerError::Invitation("the list is not open: the log holds no genesis".to_string())
        })?;
        if self.trust.has_granted(node) {
            return Err(PeerError::Granted(node));
        }
        let expires_at_ms = now_ms.saturating_add(valid_ms);
        let key = invitation::key_of(&secret).public();
        let ticket = Ticket {
            relay: relay.to_string(),
            invitation: Invitation::sign(
                administrator,
                audience,
                &self.list,
                genesis,
                node,
                expires_at_ms,
                key,
            ),
            secret,
        };
        let text = ticket
            .to_text()
            .map_err(|error| PeerError::Invitation(format!("{error:?}")))?;
        self.forget_invitations(now_ms);
        self.invitations
            .entry(node)
            .or_default()
            .push(Invited { key, expires_at_ms });
        Ok(text)
    }

    /// Begin the exchange that leaves the device's identity key, and its key package when the core
    /// holds a device of the list's group, with the relay under the invitation `text` (ADR 0080
    /// section 4): a challenge for the invitation's list and node, and a join proved with the key
    /// its secret makes. The core is the one invited to the invitation's list with its genesis, and
    /// its person is the invitation's node. The relay's name is the one the invitation carries, and
    /// a relay that names itself otherwise is given nothing. Its outcome tells the machine nothing
    /// and holds no token: the device logs in ([`Self::login`]) once the administrator's device
    /// admits it. What is returned is the fingerprint the person compares with the one the
    /// administrator's device shows ([`Self::joined`]).
    pub fn join(&mut self, text: &str) -> Result<String, PeerError> {
        self.ready()?;
        let ticket =
            Ticket::from_text(text).map_err(|error| PeerError::Invitation(format!("{error:?}")))?;
        let (node, key) = self.trust.person().ok_or(PeerError::NoPerson)?;
        let invitation = &ticket.invitation;
        if invitation.list != self.list {
            return Err(PeerError::Invitation(format!(
                "the invitation is to the list `{}`, and this core holds `{}`",
                invitation.list, self.list
            )));
        }
        if self.trust.genesis_taken() != Some(invitation.genesis) {
            return Err(PeerError::Invitation(
                "the invitation names another genesis than the one this core takes".to_string(),
            ));
        }
        if invitation.node != node {
            return Err(PeerError::Invitation(format!(
                "the invitation is for node {}, and the device's person is node {node}",
                invitation.node
            )));
        }
        let fingerprint = invitation::fingerprint(&self.list, node, &invitation.key, &key);
        let key_package = match self.group.as_mut() {
            None => Vec::new(),
            Some(member) => member
                .device_mut()
                .key_package()
                .map_err(PeerError::Group)?,
        };
        if let Some(running) = self.running.take() {
            self.state = (*running).into_state();
        }
        self.asking = Some(Asking::Join(Box::new(JoinMachine::start(
            ticket,
            key,
            key_package,
        ))));
        Ok(fingerprint)
    }

    /// The devices that joined under an invitation this core made for `node`, as the rounds read
    /// them, each the key its proof names and the fingerprint the two people compare: what the
    /// administrator's device shows before it admits one ([`Self::add_member`], [`Self::grant`]).
    /// Empty when none was read, and once the node is admitted. Two are a relay that took two joins
    /// of one invitation, or a person who photographed the code and raced the device: the
    /// fingerprint tells which one is the device in front of the administrator.
    pub fn joined(&self, node: u64) -> Vec<JoinedDevice> {
        self.joins
            .get(&node)
            .map(|held| {
                held.iter()
                    .map(|join| JoinedDevice {
                        key: join.key,
                        fingerprint: invitation::fingerprint(
                            &self.list,
                            node,
                            &join.invitation_key,
                            &join.key,
                        ),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// For a node this core invited, the join a round read whose proof names `key`, and
    /// [`PeerError::NotJoined`] when there is none; `None` for a node it did not invite.
    fn joined_with(&self, node: u64, key: PublicKey) -> Result<Option<&JoinHeld>, PeerError> {
        if !self.invitations.contains_key(&node) {
            return Ok(None);
        }
        self.joins
            .get(&node)
            .and_then(|held| held.iter().find(|join| join.key == key))
            .map(Some)
            .ok_or(PeerError::NotJoined(node))
    }

    /// The device of `node` was admitted: its invitations and joins are done with.
    fn admitted(&mut self, node: u64) {
        self.invitations.remove(&node);
        self.joins.remove(&node);
    }

    /// Forget the invitations that expired a day or more before `now_ms` and that no join read
    /// names: no relay takes a join under one, and a join it took before the expiry has had a day
    /// to be read.
    fn forget_invitations(&mut self, now_ms: u64) {
        let joins = &self.joins;
        self.invitations.retain(|node, invited| {
            if !joins.contains_key(node) {
                invited
                    .retain(|one| one.expires_at_ms.saturating_add(invitation::MOST_MS) > now_ms);
            }
            !invited.is_empty()
        });
    }

    /// Keep the joins `round` read whose proof holds under an invitation this core made for their
    /// node; one under an invitation it made that does not hold is a refusal of the round. A join
    /// for a node this core did not invite (another administrator's) is not its to judge.
    fn take_joins(&mut self, round: &mut Round) {
        let mut refused = Vec::new();
        for record in &round.messages {
            let Record::KeyPackage {
                node,
                data,
                join: Some(join),
            } = record
            else {
                continue;
            };
            let Some(invited) = self.invitations.get(node) else {
                continue;
            };
            let package = if data.is_empty() {
                Some(Vec::new())
            } else {
                encoding::decode(data).filter(|bytes| !bytes.is_empty())
            };
            let holds = invited.iter().any(|one| one.key == join.invitation.key)
                && join.invitation.node == *node
                && join.invitation.list == self.list
                && package
                    .as_ref()
                    .is_some_and(|package| join.proof.holds(&join.invitation, package));
            let Some(package) = package.filter(|_| holds) else {
                refused.push(format!(
                    "a key package of node {node} under no invitation this device made, or whose \
                     proof does not hold under it, is not taken"
                ));
                continue;
            };
            let held = self.joins.entry(*node).or_default();
            if !held
                .iter()
                .any(|kept| kept.key == join.proof.key && kept.key_package == package)
            {
                held.push(JoinHeld {
                    invitation_key: join.invitation.key,
                    key: join.proof.key,
                    key_package: package,
                });
            }
        }
        round.refused.extend(refused);
    }

    /// Take the device of `node` off the list, ending its chain at the last operation of it the
    /// device has admitted (`Trust::removal_body`). On a list keyed by its group the same call moves
    /// the group to an epoch without the device, since a removal the group did not follow would
    /// leave what is written after it readable by the device, and a commit with no entry that names
    /// it is applied by no one; the commit is made first, the entries go to the relay with the next
    /// round and the commit after it ([`Outcome::owed`]). A list whose keys an administrator draws
    /// is refused ([`PeerError::DrawnKeys`]), and one with no keys yet is a grant of the log alone.
    pub fn remove_member(&mut self, node: u64) -> Result<(), PeerError> {
        let administrator = self
            .administrator
            .as_ref()
            .ok_or(PeerError::NoAdministrator)?;
        let removal = self
            .trust
            .removal_body(node)
            .ok_or(PeerError::NotAMember(node))?;
        match self.keying() {
            Keying::Clear => {
                self.trust
                    .administer(administrator, removal)
                    .map_err(PeerError::Log)?;
            }
            Keying::Drawn => return Err(PeerError::DrawnKeys),
            Keying::Group => {
                let member = self.group.as_mut().ok_or(PeerError::NoGroupDevice)?;
                let issued = member
                    .remove_member(administrator, node)
                    .map_err(PeerError::Group)?;
                self.owe(issued, None);
            }
        }
        Ok(())
    }

    /// Whether the device signs nothing new until a round has read its own chain back from the
    /// relay ([`Self::restore_sealed`]): a send that would make a new operation waits meanwhile
    /// ([`Kept::Waiting`]).
    pub fn awaits_own_chain(&self) -> bool {
        self.trust.awaits_own_chain()
    }

    /// What the device stands at in its list now.
    pub fn standing(&self) -> Standing {
        let person = self.trust.person();
        Standing {
            genesis: self.trust.genesis(),
            person,
            role: person.map_or(0, |(node, _)| self.trust.role(node)),
            administrator: self
                .administrator
                .as_ref()
                .is_some_and(|key| self.trust.administrators().contains(&key.public())),
            awaits_own_chain: self.awaits_own_chain(),
            members: self
                .trust
                .current_members()
                .into_iter()
                .map(|(node, _)| (node, self.trust.role(node)))
                .collect(),
            epoch: self.trust.epoch(),
            keys: self.trust.epochs(),
            owed: self.owed.len(),
            waiting: self.trust.waiting(),
        }
    }

    /// What the machine sent to `#people`, as the event's name and its JSON payload: a
    /// `record_list.change` is signed whole, as one operation, and remembered, and the next round
    /// carries it to the relay (docs/adr/0085, section 1); what is not signed is kept nowhere; what
    /// the trust will never sign is refused with the reason. The one place this is done is the
    /// relay's client ([`keep`]), which a host with a Mesh endpoint reaches through `Keep`.
    ///
    /// **A send the device may not sign yet waits, and is not lost.** A machine sends whenever its
    /// person edits, offline too, and right after its application starts, which for a device that
    /// keeps a sealed state is a restore. What the device may not sign yet is kept, in order, with
    /// every send after it ([`Kept::Waiting`], and the reason): an invited device that has not read
    /// the log that lets it write, one whose list keys its content under an epoch it holds no key
    /// of yet, one whose list is keyed by its group and whose log names no epoch of it yet (a list
    /// an earlier core opened, until its first member is admitted: no write leaves in the clear),
    /// a device restored from a sealed state before a round has read its own chain back
    /// (see [`Self::restore_sealed`]). The round that reads what it waits for signs it and gives it
    /// the relay ([`Outcome::taken`]); a round whose log says the person never may (a reader, a
    /// device taken off) refuses it then, and says so ([`Outcome::dropped`], [`Outcome::rebuild`]).
    /// What waits is in the sealed state. A send of another content under the dot of a change that
    /// waits is refused at once, as one under a dot the device signed is.
    ///
    /// **A replay writes nothing twice.** An application that keeps its own log of commands and
    /// builds its machine again from it gives the core the machine's sends again: a change the
    /// device signed already is [`Kept::Held`], the operation it holds with its number and its
    /// signature, and nothing new reaches the relay; one that waits is [`Kept::Waiting`] and waits
    /// once. A change under a dot the device signed with another content is refused, since it would
    /// be the device signing two things for one change. A send of a person the log the core holds
    /// does not let write is refused, since no host would admit it.
    pub fn outbound(&mut self, event: &str, data: &str) -> Kept {
        keep(&self.trust, event, data)
    }

    /// Begin a round of `kind`. Its requests are taken by [`Self::poll_request`]. A round that ends
    /// goes on to give the relay what the administrator's acts owe it, before its outcome is given.
    pub fn start(&mut self, kind: RoundKind) -> Result<(), PeerError> {
        self.begin(Purpose::Round, |state, trust| {
            RoundMachine::start(kind, state, trust)
        })
    }

    /// Begin the exchange that logs the device in at the relay named `audience` (ADR 0080 section
    /// 2): a challenge for its person's node, and the login, signed with the person's key for that
    /// audience and bringing the entry of the node's latest grant the core holds, in case the relay
    /// was not given it. `audience` is the name the device was configured with, shipped with the
    /// relay's address or carried in an invitation, and not the one the relay says: a relay that
    /// names itself otherwise is signed nothing, and the outcome says so. The exchange is carried
    /// as a round is, on a connection that presents no token or one that expired, and its outcome
    /// holds the token ([`Outcome::token`]), which the host gives its transport
    /// ([`Transport::set_token`]). The core holds no token, and logs in again only when its host
    /// asks: when a round's outcome is the relay's `unauthorized`, or before the token's time is
    /// out. A login begun while a round runs abandons the round, which loses nothing, as beginning
    /// any exchange does.
    ///
    /// **A list the relay does not hold is created by the login** when the relay takes a creation
    /// with no token (ADR 0080 section 3) and its challenge says the work it takes: the core gives
    /// the relay its log from the genesis to its person's grant, with the work and the login's
    /// signature, under the list's name, which must certify the genesis ([`Self::certified_name`]).
    /// That is how a device opens such a list at the relay ([`Self::open_list`], then this), and how
    /// a list that lapsed there is made again, by whichever of its devices logs in first, with the
    /// same genesis and name. The relay then holds the log as far as the grant and nothing more, and
    /// its heads say its history of the list began at another instant: the next round of every
    /// device of the list reads the relay from its start, and gives it back everything the device
    /// holds that it lacks (the log, every device's operations, the checkpoints and the proof of a
    /// conflict). The group's commits and Welcomes that only the relay held are not given back; a
    /// device that had not read one before the list lapsed is admitted again.
    pub fn login(&mut self, audience: &str) -> Result<(), PeerError> {
        self.ready()?;
        if self.trust.person().is_none() {
            return Err(PeerError::NoPerson);
        }
        if let Some(running) = self.running.take() {
            self.state = (*running).into_state();
        }
        self.asking = Some(Asking::Login(Box::new(LoginMachine::start(
            self.trust.clone(),
            audience,
        ))));
        Ok(())
    }

    /// Begin the exchange that gives the relay `publication`.
    pub fn publish(&mut self, publication: &Publication) -> Result<(), PeerError> {
        self.begin(Purpose::Publish, |state, trust| {
            RoundMachine::publish(publication, state, trust)
        })
    }

    /// Begin the exchange that leaves a key package of the group's device with the relay, for an
    /// administrator to add it with.
    pub fn publish_key_package(&mut self) -> Result<(), PeerError> {
        self.ready()?;
        let member = self.group.as_mut().ok_or(PeerError::NoGroupDevice)?;
        let package = member
            .device_mut()
            .key_package()
            .map_err(PeerError::Group)?;
        self.publish(&Publication::KeyPackage(package))
    }

    /// The next request to put to the relay, as the protocol's text; `None` when no exchange runs
    /// or the one that ran is over, and its outcome waits ([`Self::take_outcome`]). Asked again
    /// before it is answered, it is the same request.
    ///
    /// A host that carries the texts itself calls this, [`Self::respond`] and [`Self::fail`]; one
    /// that has a [`Transport`] gives the core to the driver instead ([`drive`], through the core's
    /// [`Exchange`]), which makes the same calls.
    pub fn poll_request(&mut self) -> Option<String> {
        Exchange::request(self).map(|request| request.to_json())
    }

    /// The relay's response text to the request [`Self::poll_request`] gave. A text that is not a
    /// response of the protocol is taken as a failure of the transport.
    pub fn respond(&mut self, text: &str) -> Result<(), PeerError> {
        self.give_answer(Response::from_json(text).map_err(LinkError))
    }

    /// The transport did not carry the request or its response: the exchange ends with
    /// [`CourierError::Link`], and what it took in is handed over by the next round.
    pub fn fail(&mut self, reason: &str) -> Result<(), PeerError> {
        self.give_answer(Err(LinkError(reason.to_string())))
    }

    /// The outcome of the exchange that ended, once.
    pub fn take_outcome(&mut self) -> Option<Result<Outcome, CourierError>> {
        self.outcome.take()
    }

    /// The word the machine is told that an instant is settled, when the checkpoints the device
    /// holds make a settlement of it.
    pub fn floor_event(&self, before: i64, digest: u32) -> Option<(String, String)> {
        floor_event(&self.trust, before, digest)
    }

    /// What the device keeps, sealed under a data key drawn for this save alone from the operating
    /// system, and that key ([`SealedState`]). The host wraps the key with its platform's key store
    /// and keeps the wrapped key beside the state; it never handles a protocol secret in the clear,
    /// and gives no key again to restore ([`Self::restore_sealed`]).
    ///
    /// The state is everything the device keeps: its person's key, the administrator's key it
    /// holds, the keys of the epochs it holds, the group's device (its MLS state and the messages
    /// it waits to use), the membership log, what the trust holds ([`Trust::to_text`]), the relay's
    /// client's state, which an exchange that runs, or an outcome not taken, leaves as it would if
    /// it stopped here, and the commits and Welcomes the relay is still owed. A host that saves in
    /// the middle of a round and restarts from the state goes on where it was; what was handed over
    /// after the save is handed over again, and the machine's merge takes an event twice as once.
    ///
    /// **Why a key for each save, and not one for the device.** A key seals one state, so no nonce is
    /// ever at risk; a key that leaked (a process's memory) opens one save, not every save before
    /// and after it; and a platform's key that the host rotates retires every save wrapped under
    /// the old one once it is deleted from the key store, where one data key for the device would
    /// be wrapped again and keep every old save readable. Wrapping 32 bytes at each save is what it
    /// costs, and with a platform key that wraps with its public half (the Secure Enclave's, an RSA
    /// key of the Android Keystore) a save in the background asks for no user's presence; only the
    /// restore does. `docs/SCE_SHARED_LIST_PEER_ABI.md` section 6 gives each platform's wrapping.
    pub fn save_sealed(&self) -> Result<SealedState, PeerError> {
        let key = DataKey::draw().map_err(sealing_error)?;
        let state = self.save_sealed_under(&key)?;
        Ok(SealedState { state, key })
    }

    /// What [`Self::save_sealed`] seals, under the key `key` the host gives: a key it drew from a
    /// source of randomness of its own, or, for a recording, a key fixed by a test. The same key and
    /// the same state give the same bytes (the nonce is a keyed hash of what is sealed), so a key a
    /// host gives twice gives away only whether two states were the same; a key is still meant for
    /// one state, for the reasons [`Self::save_sealed`] gives. [`PeerError::NoPerson`] when the
    /// device has no person: a sealed state is the state of one person's device.
    pub fn save_sealed_under(&self, key: &DataKey) -> Result<Vec<u8>, PeerError> {
        let mut contents = self
            .trust
            .secrets()
            .ok_or(PeerError::NoPerson)?
            .with(KEPT_STATE, self.state_text().into_bytes());
        if let Some(administrator) = &self.administrator {
            contents = contents.with_identity(KEPT_ADMINISTRATOR, administrator);
        }
        if let Some(member) = &self.group {
            let device = member.device();
            contents = contents
                .with(KEPT_GROUP, device.to_bytes().map_err(PeerError::Group)?)
                .with(KEPT_GROUP_NODE, member.node().to_be_bytes().to_vec())
                .with_identity(KEPT_GROUP_KEY, device.identity())
                .with(KEPT_GROUP_HELD, member.held_to_bytes());
        }
        backup::seal_state(key, &sealed_context(), &contents).map_err(sealing_error)
    }

    /// The part of the state that is no secret, as the text the sealed state holds it in.
    fn state_text(&self) -> String {
        let mut state = match &self.running {
            Some(running) => running.saved_state(),
            None => self.state.clone(),
        };
        if let Some(Ok(outcome)) = &self.outcome {
            outcome.hold_in(&mut state);
        }
        json!({
            "version": SAVE_VERSION,
            "log": self.trust.entries().iter().map(Entry::to_json).collect::<Vec<_>>(),
            "trust": self.trust.to_text(),
            "courier": state.to_text(),
            "owed": self.owed.iter().map(Publication::to_value).collect::<Vec<_>>(),
            "invitations": invitations_value(&self.invitations),
            "joins": joins_value(&self.joins),
        })
        .to_string()
    }

    /// Take back what [`Self::save_sealed`] sealed under `key` for the device of `node`, into a core
    /// just made for the same list (opened, or invited with the same genesis), with no key given
    /// again: the person's key, the administrator's key and the group's device come out of the state
    /// with the rest. Each entry of the log is checked as it is appended, the keys of the epochs are
    /// taken except those the list had destroyed, each operation and checkpoint the log verifies is
    /// kept ([`Trust::restore_from_text`], whose count is returned), the relay's client goes on from
    /// its state, and what the relay is owed is given after the next round. An exchange that ran is
    /// dropped.
    ///
    /// A state that does not open (another key, a changed or cut state, another list's or another
    /// node's) is [`PeerError::NotOpened`], bytes that are no sealed state are
    /// [`PeerError::Unreadable`], and the state is read whole before the core takes any of it, so
    /// either refusal changes nothing.
    ///
    /// **A restored device signs nothing new until its own chain is read back.** The core cannot
    /// tell the last save from an older one (a backup, a copy of the store, a save the device ran
    /// past before it was stopped): it keeps nothing across runs but what it is given, and no
    /// platform gives it a counter no one can turn back. An older state and its wrapped key open as
    /// well as the last. What an older state could do is what matters, and a device that published
    /// operations past what the state holds would sign its next one at a number it used already,
    /// which every host that holds both takes as the device signing two histories and freezes it
    /// for. So the trust waits
    /// ([`Trust::await_own_chain`], [`Self::awaits_own_chain`]): [`Self::outbound`] keeps a send
    /// that would make a new operation, with that reason ([`Kept::Waiting`]), and still gives what
    /// the device holds as [`Kept::Held`]. The next round reads the relay's heads, asks for the
    /// device's own operations the relay holds from where the two agree (`Operations`, the device's
    /// node, [`Trust::own_chain_walk`]), a reading at a time until one brings nothing the core did
    /// not hold, takes them in and tells them to the machine, and the wait ends once the core holds
    /// the chain as far as the relay does ([`Trust::confirm_own_chain`]). The heads alone are not
    /// enough: a relay's head of a device stops below the first conflict among its operations and
    /// at a gap, and a device that read back only as far as the head would sign again what it
    /// signed past it, one more operation at the relay at each restore. The device then signs what
    /// waited at the numbers after its last, gives it the relay in the same round, and signs what
    /// comes after as it comes. A send that waited and that the chain read back holds already is
    /// the operation it holds, signed once. The rule is the core's and not its host's, so that no
    /// host can forget it, and it asks the relay nothing the protocol did not already serve. A host
    /// that rebuilds its machine from its own log after a restore may give the core the machine's
    /// sends at once, and makes a [`RoundKind::Resync`] to tell the rebuilt machine the list. An
    /// older state may also hold the key of an epoch the list has had destroyed since; the same
    /// round reads the entry that destroys it, and the key is zeroed as the entry is taken.
    pub fn restore_sealed(
        &mut self,
        node: u64,
        sealed: &[u8],
        key: &DataKey,
    ) -> Result<usize, PeerError> {
        let contents = backup::open_state(key, &self.list, node, &sealed_context(), sealed)
            .map_err(sealing_error)?;
        let unreadable = |why: &str| PeerError::Unreadable(why.to_string());
        let saved = contents
            .extra(KEPT_STATE)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .ok_or_else(|| unreadable("it holds no state of the core"))
            .and_then(Saved::read)?;
        let administrator = match contents.extra(KEPT_ADMINISTRATOR) {
            None => None,
            Some(_) => Some(
                contents
                    .identity_named(KEPT_ADMINISTRATOR)
                    .ok_or_else(|| unreadable("its administrator's key is not one"))?,
            ),
        };
        let group = match contents.extra(KEPT_GROUP) {
            None => None,
            Some(bytes) => Some(self.group_of(&contents, bytes)?),
        };

        // Everything is read; the core takes it.
        self.trust.set_identity(contents.node, contents.identity());
        for entry in &saved.entries {
            self.trust.append(entry).map_err(|error| {
                PeerError::Unreadable(format!("an entry of its log does not hold: {error:?}"))
            })?;
        }
        contents.restore_epoch_keys(&self.trust);
        let kept = self.trust.restore_from_text(&saved.trust);
        self.trust.await_own_chain();
        if administrator.is_some() {
            self.administrator = administrator;
        }
        if group.is_some() {
            self.group = group;
        }
        self.note_keying();
        self.running = None;
        self.asking = None;
        self.outcome = None;
        self.state = saved.courier;
        self.owed = saved.owed;
        self.invitations = saved.invitations;
        self.joins = saved.joins;
        Ok(kept)
    }

    /// The group's device a sealed state holds, made again with the key it signed with and the
    /// messages it waited to use; `device` is its MLS state.
    fn group_of(&self, contents: &Contents, device: &[u8]) -> Result<Member, PeerError> {
        let unreadable = |why: &str| PeerError::Unreadable(why.to_string());
        let node = contents
            .extra(KEPT_GROUP_NODE)
            .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
            .map(u64::from_be_bytes)
            .ok_or_else(|| unreadable("it holds no node of the group's device"))?;
        let identity = contents
            .identity_named(KEPT_GROUP_KEY)
            .ok_or_else(|| unreadable("it holds no key of the group's device"))?;
        let held = contents
            .extra(KEPT_GROUP_HELD)
            .ok_or_else(|| unreadable("it holds nothing the group's device waited to use"))?;
        let device =
            Device::from_bytes(&self.list, node, identity, device).map_err(PeerError::Group)?;
        let mut member = Member::new(device, self.trust.clone(), node);
        member.take_held(held).map_err(PeerError::Group)?;
        Ok(member)
    }

    fn ready(&self) -> Result<(), PeerError> {
        if self.outcome.is_some() {
            Err(PeerError::OutcomeNotTaken)
        } else {
            Ok(())
        }
    }

    /// What the list's content is sealed under: its group's when the log names a commit or the
    /// core's device is in the group, keys an administrator drew when the log began an epoch
    /// otherwise, and nothing before either.
    fn keying(&self) -> Keying {
        let in_group = self
            .group
            .as_ref()
            .is_some_and(|member| member.device().epoch().is_ok());
        if self.trust.is_mls() || in_group {
            Keying::Group
        } else if self.trust.epoch() > 0 {
            Keying::Drawn
        } else {
            Keying::Clear
        }
    }

    /// The relay is owed what a change of the group made: its commit, and the Welcome of the device
    /// `added` when it added one. A removal adds no device, and its commit has no Welcome.
    fn owe(&mut self, issued: Issued, added: Option<u64>) {
        let Issued {
            epoch,
            commit,
            welcome,
            ..
        } = issued;
        self.owed.push(Publication::Commit { epoch, commit });
        if let (Some(node), Some(welcome)) = (added, welcome) {
            self.owed.push(Publication::Welcome {
                epoch,
                node,
                welcome,
            });
        }
    }

    fn begin(
        &mut self,
        purpose: Purpose,
        make: impl FnOnce(CourierState, Trust) -> RoundMachine,
    ) -> Result<(), PeerError> {
        self.ready()?;
        if let Some(running) = self.running.take() {
            self.state = (*running).into_state();
        }
        self.asking = None;
        let state = std::mem::take(&mut self.state);
        self.running = Some(Box::new(Running {
            machine: make(state, self.trust.clone()),
            purpose,
        }));
        Ok(())
    }

    fn give_answer(&mut self, answer: Result<Response, LinkError>) -> Result<(), PeerError> {
        if let Some(asking) = self.asking.as_mut() {
            if !asking.is_asking() {
                return Err(PeerError::NothingAsked);
            }
            asking.respond(answer);
            return Ok(());
        }
        let running = self.running.as_mut().ok_or(PeerError::NothingAsked)?;
        if !running.machine.is_asking() {
            return Err(PeerError::NothingAsked);
        }
        running.machine.respond(answer);
        Ok(())
    }

    /// The exchange that ran ended with `result`. A round the group's device took in and that left
    /// it owing a second read from the start (`sce_rust_mls::host::Member::follow`) goes on with
    /// that read; a round that ended goes on to give the relay what it is owed
    /// ([`Self::deliver`]); then its outcome is given.
    fn conclude(&mut self, result: Result<Round, CourierError>) {
        let Some(running) = self.running.take() else {
            return;
        };
        let Running { machine, purpose } = *running;
        self.state = machine.into_state();
        let ended = match (purpose, result) {
            (Purpose::Round, Ok(mut round)) => {
                self.take_joins(&mut round);
                let steps = self
                    .group
                    .as_mut()
                    .map(|member| member.take_round(&round))
                    .unwrap_or_default();
                if self.group.as_ref().is_some_and(Member::owes_reread) {
                    log::info!(list = list_digest(&self.list).as_str(); "reread_owed");
                    let state = std::mem::take(&mut self.state);
                    let machine = RoundMachine::start(RoundKind::Reread, state, self.trust.clone());
                    self.running = Some(Box::new(Running {
                        machine,
                        purpose: Purpose::Follow {
                            first: Box::new(round),
                            steps,
                        },
                    }));
                    return;
                }
                Outcome {
                    round,
                    reread: None,
                    steps,
                    owed: 0,
                    token: None,
                }
            }
            (Purpose::Follow { first, mut steps }, Ok(mut again)) => {
                self.take_joins(&mut again);
                if let Some(member) = self.group.as_mut() {
                    steps.extend(member.take_round(&again));
                }
                Outcome {
                    round: *first,
                    reread: Some(again),
                    steps,
                    owed: 0,
                    token: None,
                }
            }
            // A failure of the second read is left to the next round, as the group's device leaves
            // it when its host has a link: the commit is still named and still lacking, and it is
            // asked for again.
            (Purpose::Follow { first, steps }, Err(error)) => {
                note_failure(&self.list, "reread", &error);
                Outcome {
                    round: *first,
                    reread: None,
                    steps,
                    owed: 0,
                    token: None,
                }
            }
            (Purpose::Deliver { outcome }, Ok(_)) => {
                let given = self.owed.remove(0);
                log::info!(
                    list = list_digest(&self.list).as_str(),
                    publication = publication_word(&given),
                    left = self.owed.len();
                    "owed_given"
                );
                *outcome
            }
            // What the relay did not take stays owed, and the next round gives it again: a commit
            // the log names and the relay lacks is one no member can apply until it has it.
            (Purpose::Deliver { mut outcome }, Err(error)) => {
                log::info!(
                    list = list_digest(&self.list).as_str(),
                    publication = self.owed.first().map_or("none", publication_word),
                    owed = self.owed.len(),
                    failure = failure_word(&error);
                    "owed_kept"
                );
                outcome.round.refused.push(format!(
                    "a commit, Welcome or key the relay is owed was not given: {error:?}"
                ));
                outcome.owed = self.owed.len();
                note_ended(&self.list, "round", &outcome);
                self.outcome = Some(Ok(*outcome));
                return;
            }
            (Purpose::Publish, Ok(round)) => {
                let outcome = Outcome {
                    round,
                    reread: None,
                    steps: Vec::new(),
                    owed: self.owed.len(),
                    token: None,
                };
                note_ended(&self.list, "publication", &outcome);
                self.outcome = Some(Ok(outcome));
                return;
            }
            (purpose @ (Purpose::Round | Purpose::Publish), Err(error)) => {
                let exchange = if matches!(purpose, Purpose::Round) {
                    "round"
                } else {
                    "publication"
                };
                note_failure(&self.list, exchange, &error);
                self.outcome = Some(Err(error));
                return;
            }
        };
        self.deliver(ended);
    }

    /// A round ended and came to `outcome`: the first publication the relay is owed is given to it
    /// before the outcome is, and the outcome is given once none is owed, or the relay did not take
    /// one ([`Self::conclude`]).
    fn deliver(&mut self, mut outcome: Outcome) {
        match self.owed.first() {
            Some(publication) => {
                log::debug!(
                    list = list_digest(&self.list).as_str(),
                    publication = publication_word(publication),
                    owed = self.owed.len();
                    "owed_giving"
                );
                let state = std::mem::take(&mut self.state);
                let machine = RoundMachine::publish(publication, state, self.trust.clone());
                self.running = Some(Box::new(Running {
                    machine,
                    purpose: Purpose::Deliver {
                        outcome: Box::new(outcome),
                    },
                }));
            }
            None => {
                outcome.owed = 0;
                note_ended(&self.list, "round", &outcome);
                self.outcome = Some(Ok(outcome));
            }
        }
    }
}

/// The word a record names a publication by: its kind, and nothing of what it carries.
fn publication_word(publication: &Publication) -> &'static str {
    match publication {
        Publication::KeyPackage(_) => "key_package",
        Publication::Commit { .. } => "commit",
        Publication::Welcome { .. } => "welcome",
        Publication::KeyGrant { .. } => "key_grant",
        Publication::Checkpoint { .. } => "checkpoint",
    }
}

/// The word a record names the failure of an exchange by.
fn failure_word(error: &CourierError) -> &'static str {
    match error {
        CourierError::Link(_) => "link",
        CourierError::Relay { .. } => "relay",
        CourierError::Unexpected(_) => "unexpected",
    }
}

/// Log the failure of the `exchange` (a round, a publication, a login, a join, a second read) of
/// the core of `list`: the link's words or the relay's code and reason, which name no secret. An
/// answer that was not the answer to the request is named by its kind alone: what the core says of
/// it quotes what the relay gave, which may be a token or what a list holds.
fn note_failure(list: &str, exchange: &'static str, error: &CourierError) {
    let list = list_digest(list);
    match error {
        CourierError::Link(reason) => log::info!(
            list = list.as_str(),
            exchange = exchange,
            failure = "link",
            reason = reason.as_str();
            "exchange_failed"
        ),
        CourierError::Relay { code, reason } => {
            let retry_after_ms = match code {
                Code::RateLimited { retry_after_ms } => *retry_after_ms,
                _ => 0,
            };
            log::info!(
                list = list.as_str(),
                exchange = exchange,
                failure = "relay",
                code = code.tag(),
                retry_after_ms = retry_after_ms,
                reason = reason.as_str();
                "exchange_failed"
            );
        }
        CourierError::Unexpected(_) => log::info!(
            list = list.as_str(),
            exchange = exchange,
            failure = "unexpected";
            "exchange_failed"
        ),
    }
}

/// Log the end of the `exchange` of the core of `list` that came to `outcome`, in counts: the
/// events for the machine, what the relay was given, what of the sends that waited was signed,
/// found held or refused for good, the refusals, whether it stalled or the machine is to be built
/// again, what the relay is still owed, and whether a token came (never which).
fn note_ended(list: &str, exchange: &'static str, outcome: &Outcome) {
    log::info!(
        list = list_digest(list).as_str(),
        exchange = exchange,
        events = outcome.events().len(),
        pushed = outcome.round.pushed,
        taken = outcome.taken().len(),
        held = outcome.held().len(),
        dropped = outcome.dropped().len(),
        refused = outcome.refused().len(),
        stalled = outcome.stalled(),
        rebuild = outcome.rebuild(),
        owed = outcome.owed,
        earned_token = outcome.token.is_some();
        "exchange_ended"
    );
}

/// The core is an exchange the driver carries over any link ([`drive`]): the requests are the ones
/// [`Peer::poll_request`] gives, and an answer goes where [`Peer::respond`] and [`Peer::fail`] put
/// it. An exchange the core composes (a round, the second read it owes, what the relay is owed after
/// it) is one exchange to the driver, whose outcome is taken with [`Peer::take_outcome`].
impl Exchange for Peer {
    fn request(&mut self) -> Option<Request> {
        if let Some(asking) = self.asking.as_mut() {
            let exchange = match asking {
                Asking::Login(_) => "login",
                Asking::Join(_) => "join",
            };
            return match asking.poll() {
                Ok(request) => Some(request),
                Err(result) => {
                    self.asking = None;
                    let owed = self.owed.len();
                    let outcome = result.map(|token| Outcome::of_exchange(token, owed));
                    match &outcome {
                        Ok(ended) => note_ended(&self.list, exchange, ended),
                        Err(error) => note_failure(&self.list, exchange, error),
                    }
                    self.outcome = Some(outcome);
                    None
                }
            };
        }
        loop {
            let running = self.running.as_mut()?;
            match running.machine.poll() {
                Step::Request(request) => return Some(request),
                Step::Done(result) => self.conclude(result),
            }
        }
    }

    fn answer(&mut self, answer: Result<Response, LinkError>) {
        // The driver answers only the request it was given, which waits; an answer when none waits
        // is not taken, as `respond` says.
        let _ = self.give_answer(answer);
    }
}
