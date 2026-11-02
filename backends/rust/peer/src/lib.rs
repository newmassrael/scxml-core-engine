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
//! - the machine's outbound `record_list.write` and `record_list.supersede`, as the event's name
//!   and its JSON payload, are signed and remembered ([`Peer::outbound`]); the core says whether it
//!   took one, held it already (a machine built again from its host's log sends its writes again,
//!   and a write is signed once), found it is no event that is signed, or refused it and why;
//! - a round with the relay ([`Peer::start`]) is a sequence of request texts the core gives
//!   ([`Peer::poll_request`]) and response texts the host gives back ([`Peer::respond`]), or a
//!   failure of the transport ([`Peer::fail`]); its outcome ([`Peer::take_outcome`]) is the events
//!   for the machine in order, the refusals, whether the machine is to be built again, whether the
//!   round stalled, and what the group's device did with what was read;
//! - an administrator's acts ([`Peer::open_list`], [`Peer::grant`], [`Peer::add_member`],
//!   [`Peer::remove_member`]) write the entries of the membership log and make the group's change
//!   that goes with them in one call; the entries go to the relay with the next round, and the
//!   commit and the Welcome a change of the group makes are given to it after that round;
//! - what the device stands at in the list (its person's role, the members, whether it waits for its
//!   own chain, what the relay is still owed) is [`Peer::standing`];
//! - what the device keeps between runs is one text ([`Peer::save`], [`Peer::restore`]).
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
//! epochs, the group's state) are not in what [`Peer::save`] writes: the host gives them again
//! ([`Peer::set_identity`], [`Peer::set_administrator`], [`Peer::with_group`]) until the device's
//! sealed state is specified (ADR 0079 section 6).

#![forbid(unsafe_code)]

pub use sce_rust_mls::host::{Member, Step as GroupStep};
pub use sce_rust_mls::{Device, MlsError};
pub use sce_rust_relay::{CourierError, Kept, Publication, Round, RoundKind};
pub use sce_rust_trust::membership::{Hash, LogError, Profile, OWNER, READER, WRITER};

use sce_rust_mls::host::Issued;
use sce_rust_relay::client::{floor_event, keep};
use sce_rust_relay::{CourierState, LinkError, Response, RoundMachine, Step};
use sce_rust_trust::membership::{Body, Entry};
use sce_rust_trust::{Identity, PublicKey, Trust};
use serde_json::{json, Value};

/// The version of what [`Peer::save`] writes. Version 2 holds what the relay is owed; a save of
/// version 1, which owed nothing, is still read.
pub const SAVE_VERSION: u64 = 2;

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
    /// How many of the commits and Welcomes the administrator's acts made the relay is still owed
    /// once the exchange ended. They are given after each round, in order, and kept until the relay
    /// takes them; one it did not take is among the refusals, with why.
    pub owed: usize,
}

impl Outcome {
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

    /// Whether the host is to build its machine again and make a [`RoundKind::Resync`]: the core
    /// found operations the machine was told and is to be without, or was not told and is to be
    /// with ([`Round::evicted`], [`Round::restored`]).
    pub fn rebuild(&self) -> bool {
        self.rounds()
            .any(|round| !round.evicted.is_empty() || !round.restored.is_empty())
    }

    /// Whether the round stopped before the end of what the relay holds; a later round goes on
    /// from there.
    pub fn stalled(&self) -> bool {
        self.round.stalled
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
    /// A saved text is not what [`Peer::save`] writes, or does not hold for this list, and why.
    Unreadable(String),
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
    /// relay ([`Peer::restore`]).
    pub awaits_own_chain: bool,
    /// The members of the list now, each node with its role, in the order of the nodes.
    pub members: Vec<(u64, u8)>,
    /// The latest epoch the log begins, 0 when none.
    pub epoch: u32,
    /// The epochs whose keys the device holds: what it can open.
    pub keys: Vec<u32>,
    /// How many of the commits and Welcomes the administrator's acts made the relay is still owed.
    pub owed: usize,
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
    trust: Trust,
    /// What the relay's client keeps between exchanges; while one runs, its machine holds it.
    state: CourierState,
    running: Option<Box<Running>>,
    outcome: Option<Result<Outcome, CourierError>>,
    group: Option<Member>,
    /// The administrator's key the device holds, when it holds one.
    administrator: Option<Identity>,
    /// The commits and Welcomes the administrator's acts made that the relay has not taken, in the
    /// order they were made.
    owed: Vec<Publication>,
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
    Follow { first: Round, steps: Vec<GroupStep> },
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
        Purpose::Follow { first, .. } => state.hold(first.clone()),
        Purpose::Deliver { outcome } => outcome.hold_in(state),
        Purpose::Round | Purpose::Publish => {}
    }
}

impl Peer {
    /// The core of the device of whoever makes the list `list`: its trust takes the genesis it is
    /// first given (the one [`Self::open_list`] makes, or its owner made) and every entry after it
    /// ([`Trust::open`]).
    pub fn open(list: &str) -> Self {
        Self::over(Trust::open(list))
    }

    /// The core of a device invited to the list `list` with the hash of its genesis entry, which the
    /// person was told in a channel they trust: it takes that genesis and no other
    /// ([`Trust::invited`]).
    pub fn invited(list: &str, genesis: Hash) -> Self {
        Self::over(Trust::invited(list, genesis))
    }

    fn over(trust: Trust) -> Self {
        Self {
            trust,
            state: CourierState::default(),
            running: None,
            outcome: None,
            group: None,
            administrator: None,
            owed: Vec::new(),
        }
    }

    /// The device's person: the node the log names them by, and their key.
    pub fn set_identity(&mut self, node: u64, identity: Identity) {
        self.trust.set_identity(node, identity);
    }

    /// The device holds an administrator's key: the one the genesis names when the device opens the
    /// list ([`Self::open_list`]), and the one its administrator's acts are signed with. It is a
    /// secret, given again before [`Self::restore`] as the person's key is.
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
        }
        Ok(genesis.hash())
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
    pub fn grant(&mut self, node: u64, role: u8, key: PublicKey) -> Result<(), PeerError> {
        if !(READER..=OWNER).contains(&role) {
            return Err(PeerError::NoSuchRole(role));
        }
        let administrator = self
            .administrator
            .as_ref()
            .ok_or(PeerError::NoAdministrator)?;
        if self.keying() == Keying::Group && !self.trust.current_members().contains(&(node, key)) {
            return Err(PeerError::AdmittedThroughGroup(node));
        }
        self.trust
            .administer(administrator, self.trust.grant_body(node, role, key))
            .map_err(PeerError::Log)?;
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
    pub fn add_member(&mut self, node: u64, role: u8, key: PublicKey) -> Result<(), PeerError> {
        if !(READER..=OWNER).contains(&role) {
            return Err(PeerError::NoSuchRole(role));
        }
        let administrator = self
            .administrator
            .as_ref()
            .ok_or(PeerError::NoAdministrator)?;
        let member = self.group.as_mut().ok_or(PeerError::NoGroupDevice)?;
        let package = member
            .key_packages_of(node)
            .last()
            .cloned()
            .ok_or(PeerError::NoKeyPackage(node))?;
        let issued = member
            .add_member(administrator, node, role, key, &package)
            .map_err(PeerError::Group)?;
        self.owe(issued, Some(node));
        Ok(())
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
    /// relay ([`Self::restore`]): a send that would make a new operation is refused meanwhile.
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
        }
    }

    /// What the machine sent to `#hub` or `#people`, as the event's name and its JSON payload: a
    /// `record_list.write` or a `record_list.supersede` is signed and remembered, and the next
    /// round carries it to the relay; what is not signed is kept nowhere; what the trust will not
    /// sign is refused with the reason. The one place this is done is the relay's client
    /// ([`keep`]), which a host with a Mesh endpoint reaches through `Keep`.
    ///
    /// **A replay writes nothing twice.** An application that keeps its own log of commands and
    /// builds its machine again from it gives the core the machine's sends again: a write or a note
    /// the device signed already is [`Kept::Held`], the operation it holds with its number and its
    /// signature, and nothing new reaches the relay. A write under a stamp the device signed with
    /// another content is refused, since it would be the device signing two things for one write.
    /// After [`Self::restore`], a send the device holds no operation for is refused until a round
    /// has read the device's own chain back from the relay (see there).
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
    pub fn poll_request(&mut self) -> Option<String> {
        loop {
            let running = self.running.as_mut()?;
            match running.machine.poll() {
                Step::Request(request) => return Some(request.to_json()),
                Step::Done(result) => self.conclude(result),
            }
        }
    }

    /// The relay's response text to the request [`Self::poll_request`] gave. A text that is not a
    /// response of the protocol is taken as a failure of the transport.
    pub fn respond(&mut self, text: &str) -> Result<(), PeerError> {
        self.answer(Response::from_json(text).map_err(LinkError))
    }

    /// The transport did not carry the request or its response: the exchange ends with
    /// [`CourierError::Link`], and what it took in is handed over by the next round.
    pub fn fail(&mut self, reason: &str) -> Result<(), PeerError> {
        self.answer(Err(LinkError(reason.to_string())))
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

    /// What the device keeps, as one text: the membership log, what the trust holds
    /// ([`Trust::to_text`]), the relay's client's state, which an exchange that runs, or an outcome
    /// not taken, leaves as it would if it stopped here, and the commits and Welcomes the relay is
    /// still owed. A host that saves in the middle of a round and restarts from the text goes on
    /// where it was; what was handed over after the save is handed over again, and the machine's
    /// merge takes an event twice as once.
    pub fn save(&self) -> String {
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
        })
        .to_string()
    }

    /// Take back what [`Self::save`] wrote, into a core just made for the same list, after its
    /// person and the keys it needs were given again: each entry of the log is checked as it is
    /// appended, each operation and checkpoint the log verifies is kept ([`Trust::restore_from_text`],
    /// whose count is returned), the relay's client goes on from its state, and what the relay is
    /// owed is given after the next round. An exchange that ran is dropped.
    ///
    /// **A restored device signs nothing new until its own chain is read back.** The core cannot
    /// tell the last save from an older one (a backup, a copy of the store, a save the device ran
    /// past before it was stopped), and a device that published operations past what the text holds
    /// would sign its next one at a number it used already, which every host that holds both takes
    /// as the device signing two histories and freezes it for. So the trust waits
    /// ([`Trust::await_own_chain`], [`Self::awaits_own_chain`]): [`Self::outbound`] refuses, with that
    /// reason, a send that would make a new operation, and still gives what the device holds as
    /// [`Kept::Held`]. The next round reads the relay's heads, asks for the device's own operations
    /// the relay holds past what the core does (`Operations`, the device's node), takes them in and
    /// tells them to the machine, and the wait ends once the core holds the chain as far as the
    /// relay does ([`Trust::confirm_own_chain`]); the device then signs at the number after its
    /// last. The rule is the core's and not its host's, so that no host can forget it, and it asks
    /// the relay nothing the protocol did not already serve. A host that rebuilds its machine from
    /// its own log after a restore makes that round first, then gives the core the machine's sends,
    /// then makes a [`RoundKind::Resync`] to tell the rebuilt machine the list.
    pub fn restore(&mut self, text: &str) -> Result<usize, PeerError> {
        let unreadable = |why: &str| PeerError::Unreadable(why.to_string());
        let value: Value = serde_json::from_str(text).map_err(|_| unreadable("it is not JSON"))?;
        let owed = match value.get("version").and_then(Value::as_u64) {
            // A save of the core from before an administrator's act could owe the relay anything.
            Some(1) => Vec::new(),
            Some(SAVE_VERSION) => value
                .get("owed")
                .and_then(Value::as_array)
                .ok_or_else(|| unreadable("it holds no list of what the relay is owed"))?
                .iter()
                .map(Publication::from_value)
                .collect::<Option<Vec<Publication>>>()
                .ok_or_else(|| unreadable("what it says the relay is owed is not a publication"))?,
            _ => return Err(unreadable("it is not a version this core reads")),
        };
        let state = value
            .get("courier")
            .and_then(Value::as_str)
            .and_then(CourierState::from_text)
            .ok_or_else(|| unreadable("it holds no state of the relay's client"))?;
        let held = value
            .get("trust")
            .and_then(Value::as_str)
            .ok_or_else(|| unreadable("it holds nothing of the trust"))?;
        let entries = value
            .get("log")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("it holds no log"))?
            .iter()
            .map(|entry| entry.as_str().and_then(Entry::from_json))
            .collect::<Option<Vec<Entry>>>()
            .ok_or_else(|| unreadable("an entry of its log is not one"))?;
        for entry in &entries {
            self.trust.append(entry).map_err(|error| {
                PeerError::Unreadable(format!("an entry of its log does not hold: {error:?}"))
            })?;
        }
        let kept = self.trust.restore_from_text(held);
        self.trust.await_own_chain();
        self.running = None;
        self.outcome = None;
        self.state = state;
        self.owed = owed;
        Ok(kept)
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
        let state = std::mem::take(&mut self.state);
        self.running = Some(Box::new(Running {
            machine: make(state, self.trust.clone()),
            purpose,
        }));
        Ok(())
    }

    fn answer(&mut self, answer: Result<Response, LinkError>) -> Result<(), PeerError> {
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
            (Purpose::Round, Ok(round)) => {
                let steps = self
                    .group
                    .as_mut()
                    .map(|member| member.take_round(&round))
                    .unwrap_or_default();
                if self.group.as_ref().is_some_and(Member::owes_reread) {
                    let state = std::mem::take(&mut self.state);
                    let machine = RoundMachine::start(RoundKind::Reread, state, self.trust.clone());
                    self.running = Some(Box::new(Running {
                        machine,
                        purpose: Purpose::Follow {
                            first: round,
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
                }
            }
            (Purpose::Follow { first, mut steps }, Ok(again)) => {
                if let Some(member) = self.group.as_mut() {
                    steps.extend(member.take_round(&again));
                }
                Outcome {
                    round: first,
                    reread: Some(again),
                    steps,
                    owed: 0,
                }
            }
            // A failure of the second read is left to the next round, as the group's device leaves
            // it when its host has a link: the commit is still named and still lacking, and it is
            // asked for again.
            (Purpose::Follow { first, steps }, Err(_)) => Outcome {
                round: first,
                reread: None,
                steps,
                owed: 0,
            },
            (Purpose::Deliver { outcome }, Ok(_)) => {
                self.owed.remove(0);
                *outcome
            }
            // What the relay did not take stays owed, and the next round gives it again: a commit
            // the log names and the relay lacks is one no member can apply until it has it.
            (Purpose::Deliver { mut outcome }, Err(error)) => {
                outcome.round.refused.push(format!(
                    "a commit or Welcome the relay is owed was not given: {error:?}"
                ));
                outcome.owed = self.owed.len();
                self.outcome = Some(Ok(*outcome));
                return;
            }
            (Purpose::Publish, Ok(round)) => {
                self.outcome = Some(Ok(Outcome {
                    round,
                    reread: None,
                    steps: Vec::new(),
                    owed: self.owed.len(),
                }));
                return;
            }
            (Purpose::Round | Purpose::Publish, Err(error)) => {
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
                self.outcome = Some(Ok(outcome));
            }
        }
    }
}
