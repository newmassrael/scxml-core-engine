// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What is said to a relay and what it says back, as JSON texts.
//!
//! A request is made by a session (a device of one list, which a login token names), so it names no
//! list and no device. Every whole number of 64 bits is written as text, as everywhere in the list's
//! events, so that no client that reads JSON numbers as floats loses one. A relay never reads what
//! it holds: a record is an entry of the membership log, an operation as it travels, or a
//! checkpoint, each exactly as its maker wrote it.

use serde_json::{json, Value};

use sce_rust_trust::encoding;
use sce_rust_trust::invitation::{Invitation, Proof};
use sce_rust_trust::{Head, PublicKey};

/// The version of this protocol a client of this crate writes into every request, and the newest a
/// relay of it speaks. Version 2 (ADR 0083 section 4) answers an operation that conflicts with what
/// the relay holds of its author `conflict` ([`Code::Conflict`]), where version 1 answered it
/// `accepted`.
pub const VERSION: u64 = 2;

/// The oldest version a relay of this crate speaks: a request of it is answered as a relay of that
/// version answered it, for as long as a deployment keeps clients of it.
pub const OLDEST_VERSION: u64 = 1;

/// The first version whose clients are answered [`Code::Conflict`].
pub const CONFLICT_SINCE: u64 = 2;

/// Whether a relay of this crate speaks `version`.
pub fn speaks(version: u64) -> bool {
    (OLDEST_VERSION..=VERSION).contains(&version)
}

/// The version a relay answers the text `request` in: the one it names, when the relay speaks it,
/// and the oldest otherwise (a text that is no request of the protocol, or that names a version
/// the relay does not speak), which every client reads.
pub fn version_of(request: &str) -> u64 {
    serde_json::from_str::<Value>(request)
        .ok()
        .and_then(|value| value.get("v")?.as_u64())
        .filter(|version| speaks(*version))
        .unwrap_or(OLDEST_VERSION)
}

/// The version the text `value` names, when it is one this crate speaks.
fn version_in(value: &Value) -> Result<u64, String> {
    value
        .get("v")
        .and_then(Value::as_u64)
        .filter(|version| speaks(*version))
        .ok_or_else(|| {
            format!("it is not a version of the protocol this side speaks ({OLDEST_VERSION} to {VERSION})")
        })
}

/// One thing a relay holds, in the order it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// An entry of the membership log (`Entry::to_json`).
    Entry(String),
    /// An operation of a device's chain: the event and the data it travels with.
    Operation { event: String, data: String },
    /// A member's checkpoint (`Checkpoint::to_json`).
    Checkpoint(String),
    /// An MLS key package a device published for an administrator to add it with (`data` in the URL
    /// alphabet). The relay does not read it. One a device left under an administrator's invitation
    /// (ADR 0080 section 4) holds the invitation and the device's proof (`join`), and is the record
    /// that the invitation was used; its `data` is empty when the device leaves no key package (a
    /// list its administrator admits by a grant).
    KeyPackage {
        node: u64,
        data: String,
        join: Option<Box<Joined>>,
    },
    /// The MLS commit that begins `epoch`, which the membership log names by its hash: the relay
    /// keeps only the one the log names.
    Commit { epoch: u32, data: String },
    /// The Welcome for the device of `node` added by the commit of `epoch`. It is sealed to that
    /// device's key package, and the relay does not read it.
    Welcome { epoch: u32, node: u64, data: String },
    /// The key of `epoch`, given to the device of `node` by a member who holds it (the text
    /// `sce_rust_trust::Trust::grant_key` writes, its bytes in the URL alphabet): what a device
    /// admitted after the epoch opens what was sealed under it with. It is sealed to that device's
    /// key, and the relay does not read it.
    KeyGrant { epoch: u32, node: u64, data: String },
}

impl Record {
    pub fn to_value(&self) -> Value {
        match self {
            Record::Entry(entry) => json!({"k": "entry", "d": entry}),
            Record::Operation { event, data } => json!({"k": "op", "e": event, "d": data}),
            Record::Checkpoint(checkpoint) => json!({"k": "cp", "d": checkpoint}),
            Record::KeyPackage { node, data, join } => {
                let mut value = json!({"k": "kp", "n": node.to_string(), "d": data});
                if let Some(join) = join {
                    value["j"] = join.to_value();
                }
                value
            }
            Record::Commit { epoch, data } => json!({"k": "commit", "p": epoch, "d": data}),
            Record::Welcome { epoch, node, data } => {
                json!({"k": "welcome", "p": epoch, "n": node.to_string(), "d": data})
            }
            Record::KeyGrant { epoch, node, data } => {
                json!({"k": "key_grant", "p": epoch, "n": node.to_string(), "d": data})
            }
        }
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |name: &str| Some(value.get(name)?.as_str()?.to_string());
        let node = || text("n")?.parse().ok();
        let epoch = || u32::try_from(value.get("p")?.as_u64()?).ok();
        match value.get("k")?.as_str()? {
            "entry" => Some(Record::Entry(text("d")?)),
            "op" => Some(Record::Operation {
                event: text("e")?,
                data: text("d")?,
            }),
            "cp" => Some(Record::Checkpoint(text("d")?)),
            "kp" => Some(Record::KeyPackage {
                node: node()?,
                data: text("d")?,
                join: match value.get("j") {
                    None => None,
                    Some(join) => Some(Box::new(Joined::from_value(join)?)),
                },
            }),
            "commit" => Some(Record::Commit {
                epoch: epoch()?,
                data: text("d")?,
            }),
            "welcome" => Some(Record::Welcome {
                epoch: epoch()?,
                node: node()?,
                data: text("d")?,
            }),
            "key_grant" => Some(Record::KeyGrant {
                epoch: epoch()?,
                node: node()?,
                data: text("d")?,
            }),
            _ => None,
        }
    }
}

/// What a session asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// The entries of the membership log from position `from`.
    Entries { from: u64 },
    /// An entry of the membership log: taken when it is the genesis of a list the relay holds
    /// none of, or the next entry the administrators signed.
    Append { entry: String },
    /// An operation of some device's chain. Taken when it holds under the log the relay has.
    Submit { event: String, data: String },
    /// A member's checkpoint.
    Checkpoint { checkpoint: String },
    /// A key package of the session's own device, for an administrator to add it with. Any session
    /// may publish its own: a device is invited before the log grants it anything.
    PublishKeyPackage { data: String },
    /// The MLS commit that begins `epoch`: taken when the log names its hash for that epoch.
    PublishCommit { epoch: u32, data: String },
    /// The Welcome for the device of `node` that the commit of `epoch` added.
    PublishWelcome { epoch: u32, node: u64, data: String },
    /// The key of `epoch` given to the device of `node` ([`Record::KeyGrant`]): taken for an epoch
    /// the log has begun.
    PublishKeyGrant { epoch: u32, node: u64, data: String },
    /// Everything the relay holds after the first `cursor` records, in the order it arrived, at
    /// most `limit` of them.
    Feed { cursor: u64, limit: u32 },
    /// The operations of `node` numbered `from` to `to`.
    Operations { node: u64, from: u64, to: u64 },
    /// What the relay holds of each device's chain, and how long its log is.
    Heads,
    /// A challenge for the device of `node` of `list` to log in against (ADR 0080 section 2): asked
    /// on a connection that presents no token the relay takes, which names the list and the node
    /// itself.
    Challenge { list: String, node: u64 },
    /// The device of `node` of `list` logs in: the `nonce` and `expires_at_ms` of a challenge the
    /// relay gave, the device's `signature` of them (`sce_rust_trust::login`, in the URL alphabet),
    /// and `entries` of the log for the relay to append first, as `Append` would, so that a device
    /// brings a grant the relay was not given yet.
    Login(Login),
    /// A device that is not in the log yet leaves its key package under an administrator's
    /// invitation (ADR 0080 section 4): asked on a connection that presents no token the relay
    /// takes, against a challenge for the invitation's list and node.
    Join(Box<Join>),
    /// A device creates `list` at a relay that takes a creation from a connection with no token
    /// (ADR 0080 section 3): a login whose entries are the log from its genesis to the node's
    /// grant, and the counter of its work (`sce_rust_trust::creation`).
    Create(Box<Create>),
}

/// What a device creates a list with (`Request::Create`): the login of section 2, whose entries
/// begin with the list's genesis and reach the grant of the node that logs in, and the counter of
/// the work the relay asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Create {
    pub login: Login,
    pub work: u64,
}

/// What a device joins with (`Request::Join`): the invitation, its proof, and its key package in
/// the URL alphabet, none for a device that leaves none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Join {
    pub invitation: Invitation,
    pub proof: Proof,
    pub key_package: Option<String>,
}

/// What a key package left under an invitation holds beside it: the invitation and the device's
/// proof, which an administrator's device checks under the key it made for the invitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joined {
    pub invitation: Invitation,
    pub proof: Proof,
}

impl Joined {
    /// As a record writes it: the invitation, and the proof's identity key, nonce, expiry and
    /// signature.
    pub fn to_value(&self) -> Value {
        let mut value = json!({"invitation": self.invitation.to_value()});
        write_proof(&mut value, &self.proof);
        value
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            invitation: Invitation::from_value(value.get("invitation")?)?,
            proof: read_proof(value)?,
        })
    }
}

/// The members a proof is written as, into `value`: `key`, `nonce` and `proof` in the URL alphabet
/// and `expires_at_ms` as text.
fn write_proof(value: &mut Value, proof: &Proof) {
    value["key"] = json!(proof.key.to_text());
    value["nonce"] = json!(encoding::encode(&proof.nonce));
    value["expires_at_ms"] = json!(proof.expires_at_ms.to_string());
    value["proof"] = json!(encoding::encode(&proof.signature));
}

/// The proof [`write_proof`] wrote into `value`.
fn read_proof(value: &Value) -> Option<Proof> {
    let text = |name: &str| value.get(name)?.as_str();
    Some(Proof {
        key: PublicKey::from_text(text("key")?)?,
        nonce: encoding::decode(text("nonce")?)?,
        expires_at_ms: text("expires_at_ms")?.parse().ok()?,
        signature: encoding::decode(text("proof")?)?.try_into().ok()?,
    })
}

/// A login as the request `op` writes it: a login's own, and a creation's, which adds its work.
fn login_value(op: &str, login: &Login) -> Value {
    let mut value = json!({
        "op": op,
        "list": login.list,
        "node": login.node.to_string(),
        "nonce": login.nonce,
        "expires_at_ms": login.expires_at_ms.to_string(),
        "signature": login.signature,
    });
    if !login.entries.is_empty() {
        value["entries"] = json!(login.entries);
    }
    value
}

/// The login [`login_value`] wrote into `value`.
fn read_login(value: &Value) -> Result<Login, String> {
    let text_of = |name: &str| -> Result<String, String> {
        value
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("it has no `{name}` text"))
    };
    let number = |name: &str| -> Result<u64, String> {
        text_of(name)?
            .parse()
            .map_err(|_| format!("`{name}` is not a number"))
    };
    Ok(Login {
        list: text_of("list")?,
        node: number("node")?,
        nonce: text_of("nonce")?,
        expires_at_ms: number("expires_at_ms")?,
        signature: text_of("signature")?,
        entries: match value.get("entries") {
            None => Vec::new(),
            Some(entries) => entries
                .as_array()
                .ok_or("`entries` is not a list")?
                .iter()
                .map(|entry| entry.as_str().map(str::to_string))
                .collect::<Option<_>>()
                .ok_or("an entry is not text")?,
        },
    })
}

/// What a device logs in with (`Request::Login`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub list: String,
    pub node: u64,
    /// The challenge's nonce, in the URL alphabet, as the relay gave it.
    pub nonce: String,
    pub expires_at_ms: u64,
    /// The signature of the login, in the URL alphabet.
    pub signature: String,
    pub entries: Vec<String>,
}

impl Request {
    /// The word the protocol names the request by (its `"op"`): what a record of it says, since
    /// what it carries is the members' and no record's.
    pub fn op(&self) -> &'static str {
        match self {
            Request::Entries { .. } => "entries",
            Request::Append { .. } => "append",
            Request::Submit { .. } => "submit",
            Request::Checkpoint { .. } => "checkpoint",
            Request::PublishKeyPackage { .. } => "key_package",
            Request::PublishCommit { .. } => "commit",
            Request::PublishWelcome { .. } => "welcome",
            Request::PublishKeyGrant { .. } => "key_grant",
            Request::Feed { .. } => "feed",
            Request::Operations { .. } => "operations",
            Request::Heads => "heads",
            Request::Challenge { .. } => "challenge",
            Request::Login(_) => "login",
            Request::Join(_) => "join",
            Request::Create(_) => "create",
        }
    }

    /// What the request costs a session's allowance: a read is one, a write is dearer.
    pub fn is_write(&self) -> bool {
        matches!(
            self,
            Request::Append { .. }
                | Request::Submit { .. }
                | Request::Checkpoint { .. }
                | Request::PublishKeyPackage { .. }
                | Request::PublishCommit { .. }
                | Request::PublishWelcome { .. }
                | Request::PublishKeyGrant { .. }
        )
    }

    /// The request as a client of this crate writes it, in [`VERSION`].
    pub fn to_json(&self) -> String {
        self.to_json_in(VERSION)
    }

    /// The request as a client of `version` writes it: the same members in every version this
    /// crate speaks, under its number.
    pub fn to_json_in(&self, version: u64) -> String {
        let mut value = match self {
            Request::Entries { from } => json!({"op": "entries", "from": from.to_string()}),
            Request::Append { entry } => json!({"op": "append", "entry": entry}),
            Request::Submit { event, data } => {
                json!({"op": "submit", "event": event, "data": data})
            }
            Request::Checkpoint { checkpoint } => {
                json!({"op": "checkpoint", "checkpoint": checkpoint})
            }
            Request::PublishKeyPackage { data } => json!({"op": "key_package", "data": data}),
            Request::PublishCommit { epoch, data } => {
                json!({"op": "commit", "epoch": epoch, "data": data})
            }
            Request::PublishWelcome { epoch, node, data } => json!({
                "op": "welcome",
                "epoch": epoch,
                "node": node.to_string(),
                "data": data,
            }),
            Request::PublishKeyGrant { epoch, node, data } => json!({
                "op": "key_grant",
                "epoch": epoch,
                "node": node.to_string(),
                "data": data,
            }),
            Request::Feed { cursor, limit } => {
                json!({"op": "feed", "cursor": cursor.to_string(), "limit": limit})
            }
            Request::Operations { node, from, to } => json!({
                "op": "operations",
                "node": node.to_string(),
                "from": from.to_string(),
                "to": to.to_string(),
            }),
            Request::Heads => json!({"op": "heads"}),
            Request::Challenge { list, node } => {
                json!({"op": "challenge", "list": list, "node": node.to_string()})
            }
            Request::Login(login) => login_value("login", login),
            Request::Create(create) => {
                let mut value = login_value("create", &create.login);
                value["work"] = json!(create.work.to_string());
                value
            }
            Request::Join(join) => {
                let mut value = json!({
                    "op": "join",
                    "invitation": join.invitation.to_value(),
                });
                write_proof(&mut value, &join.proof);
                if let Some(key_package) = &join.key_package {
                    value["key_package"] = json!(key_package);
                }
                value
            }
        };
        value["v"] = json!(version);
        value.to_string()
    }

    /// The request a text of any version this crate speaks writes.
    pub fn from_json(text: &str) -> Result<Self, String> {
        Self::read(text).map(|(request, _)| request)
    }

    /// The request a text writes, and the version it was written in: what a relay answers it in.
    pub fn read(text: &str) -> Result<(Self, u64), String> {
        let value: Value = serde_json::from_str(text).map_err(|_| "it is not JSON".to_string())?;
        let version = version_in(&value)?;
        Self::from_value(&value).map(|request| (request, version))
    }

    fn from_value(value: &Value) -> Result<Self, String> {
        let text_of = |name: &str| -> Result<String, String> {
            value
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("it has no `{name}` text"))
        };
        let number = |name: &str| -> Result<u64, String> {
            text_of(name)?
                .parse()
                .map_err(|_| format!("`{name}` is not a number"))
        };
        let epoch = || -> Result<u32, String> {
            value
                .get("epoch")
                .and_then(Value::as_u64)
                .and_then(|epoch| u32::try_from(epoch).ok())
                .ok_or_else(|| "it has no `epoch` number".to_string())
        };
        match value.get("op").and_then(Value::as_str) {
            Some("entries") => Ok(Request::Entries {
                from: number("from")?,
            }),
            Some("append") => Ok(Request::Append {
                entry: text_of("entry")?,
            }),
            Some("submit") => Ok(Request::Submit {
                event: text_of("event")?,
                data: text_of("data")?,
            }),
            Some("checkpoint") => Ok(Request::Checkpoint {
                checkpoint: text_of("checkpoint")?,
            }),
            Some("key_package") => Ok(Request::PublishKeyPackage {
                data: text_of("data")?,
            }),
            Some("commit") => Ok(Request::PublishCommit {
                epoch: epoch()?,
                data: text_of("data")?,
            }),
            Some("welcome") => Ok(Request::PublishWelcome {
                epoch: epoch()?,
                node: number("node")?,
                data: text_of("data")?,
            }),
            Some("key_grant") => Ok(Request::PublishKeyGrant {
                epoch: epoch()?,
                node: number("node")?,
                data: text_of("data")?,
            }),
            Some("feed") => Ok(Request::Feed {
                cursor: number("cursor")?,
                limit: u32::try_from(
                    value
                        .get("limit")
                        .and_then(Value::as_u64)
                        .ok_or("it has no `limit`")?,
                )
                .map_err(|_| "`limit` is too large".to_string())?,
            }),
            Some("operations") => Ok(Request::Operations {
                node: number("node")?,
                from: number("from")?,
                to: number("to")?,
            }),
            Some("heads") => Ok(Request::Heads),
            Some("challenge") => Ok(Request::Challenge {
                list: text_of("list")?,
                node: number("node")?,
            }),
            Some("login") => Ok(Request::Login(read_login(value)?)),
            Some("create") => {
                let login = read_login(value)?;
                if login.entries.is_empty() {
                    return Err("a creation gives the list's entries from its genesis".to_string());
                }
                Ok(Request::Create(Box::new(Create {
                    login,
                    work: number("work")?,
                })))
            }
            Some("join") => Ok(Request::Join(Box::new(Join {
                invitation: value
                    .get("invitation")
                    .and_then(Invitation::from_value)
                    .ok_or("its `invitation` is not one")?,
                proof: read_proof(value).ok_or("its proof is not one")?,
                key_package: match value.get("key_package") {
                    None => None,
                    Some(_) => Some(text_of("key_package")?),
                },
            }))),
            _ => Err("it is not a request the relay knows".to_string()),
        }
    }
}

/// Why a relay did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Code {
    /// The token is not one this relay issued, is for another relay, or has expired.
    Unauthorized,
    /// The session's device may not do this: a reader writes, or a device that is no member reads.
    Forbidden,
    /// The session spent its allowance; it may ask again after the given milliseconds.
    RateLimited { retry_after_ms: u64 },
    /// The request or what it carries is larger than the relay takes.
    TooLarge,
    /// It does not hold: not a request, or an entry, operation or checkpoint that does not verify.
    Invalid,
    /// It names an entry of the membership log the relay does not hold yet: send the log first.
    Behind,
    /// The relay cannot keep what it was given, and takes nothing until its operator has looked.
    Unavailable,
    /// The operation `seq` of the device of `node` conflicts with what the relay holds of that
    /// device (another operation at that number, or one its chain does not follow): the relay kept
    /// it, as the proof every member is to hold (ADR 0034), takes it as no part of the device's
    /// chain, and holds the chain frozen from `frozen`, as every host that holds both does. `held`
    /// is what the relay held that it conflicts with, each operation as the event and the data it
    /// travels with, and `cursor` the number of records the relay holds now. The same is answered
    /// whoever gives it, the device's own session or a member's courier (ADR 0083 section 4); a
    /// client of a version before [`CONFLICT_SINCE`] is answered `accepted` for it instead.
    Conflict {
        node: u64,
        seq: u64,
        frozen: u64,
        cursor: u64,
        held: Vec<(String, String)>,
    },
}

impl Code {
    /// The word the protocol names the refusal by (`"refused"` of a response).
    pub fn tag(&self) -> &'static str {
        match self {
            Code::Unauthorized => "unauthorized",
            Code::Forbidden => "forbidden",
            Code::RateLimited { .. } => "rate_limited",
            Code::TooLarge => "too_large",
            Code::Invalid => "invalid",
            Code::Behind => "behind",
            Code::Unavailable => "unavailable",
            Code::Conflict { .. } => "conflict",
        }
    }
}

/// Operations as a response writes them: each `[event, data]`.
fn pairs_value(pairs: &[(String, String)]) -> Value {
    json!(pairs.iter().map(|(e, d)| json!([e, d])).collect::<Vec<_>>())
}

/// The operations [`pairs_value`] wrote, when `value` is that.
fn pairs_of(value: &Value) -> Option<Vec<(String, String)>> {
    value
        .as_array()?
        .iter()
        .map(|row| {
            Some((
                row.get(0)?.as_str()?.to_string(),
                row.get(1)?.as_str()?.to_string(),
            ))
        })
        .collect()
}

/// What a relay says back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// The entries from the position asked, and how long the log the relay holds is.
    Entries {
        entries: Vec<String>,
        len: u64,
    },
    /// `new` is false when the relay held the entry already.
    Appended {
        new: bool,
        len: u64,
    },
    /// `new` is false when the relay held the operation or checkpoint already; `cursor` is the
    /// number of records it holds now.
    Accepted {
        new: bool,
        cursor: u64,
    },
    Feed {
        records: Vec<Record>,
        next: u64,
        more: bool,
    },
    Operations {
        operations: Vec<(String, String)>,
    },
    /// What the relay holds of each device's chain, how long its log is and how many records it
    /// holds; and, for a list a device created with no token, the instant its history at this
    /// relay began (`since`), which changes when the list lapsed and was created again, and tells a
    /// courier to read the relay from its start (ADR 0080 section 3).
    Heads {
        heads: Vec<Head>,
        log_len: u64,
        cursor: u64,
        since: Option<u64>,
    },
    /// A challenge to log in against: the relay's name, the nonce (in the URL alphabet) and the
    /// instant it expires at by the relay's clock, whether the relay holds the list, and the
    /// difficulty of the work a creation of it would take, when the relay takes one.
    Challenge {
        audience: String,
        nonce: String,
        expires_at_ms: u64,
        held: bool,
        work: Option<u32>,
    },
    /// The token a login earned, and when it expires by the relay's clock.
    Token {
        token: String,
        expires_at_ms: u64,
    },
    Refused {
        code: Code,
        reason: String,
    },
}

impl Response {
    /// The word the protocol names the response by: its `"ok"`, or `"refused"`.
    pub fn tag(&self) -> &'static str {
        match self {
            Response::Entries { .. } => "entries",
            Response::Appended { .. } => "appended",
            Response::Accepted { .. } => "accepted",
            Response::Feed { .. } => "feed",
            Response::Operations { .. } => "operations",
            Response::Heads { .. } => "heads",
            Response::Challenge { .. } => "challenge",
            Response::Token { .. } => "token",
            Response::Refused { .. } => "refused",
        }
    }

    pub fn refused(code: Code, reason: impl Into<String>) -> Self {
        Response::Refused {
            code,
            reason: reason.into(),
        }
    }

    /// The response as a relay of this crate writes it to a client of [`VERSION`].
    pub fn to_json(&self) -> String {
        self.to_json_in(VERSION)
    }

    /// The response as a relay writes it to a client of `version`, under that number. A relay
    /// gives a client only what its version knows: no [`Code::Conflict`] before
    /// [`CONFLICT_SINCE`] (`Relay::call`).
    pub fn to_json_in(&self, version: u64) -> String {
        let mut value = match self {
            Response::Entries { entries, len } => {
                json!({"ok": "entries", "entries": entries, "len": len.to_string()})
            }
            Response::Appended { new, len } => {
                json!({"ok": "appended", "new": new, "len": len.to_string()})
            }
            Response::Accepted { new, cursor } => {
                json!({"ok": "accepted", "new": new, "cursor": cursor.to_string()})
            }
            Response::Feed {
                records,
                next,
                more,
            } => json!({
                "ok": "feed",
                "records": records.iter().map(Record::to_value).collect::<Vec<_>>(),
                "next": next.to_string(),
                "more": more,
            }),
            Response::Operations { operations } => json!({
                "ok": "operations",
                "operations": pairs_value(operations),
            }),
            Response::Heads {
                heads,
                log_len,
                cursor,
                since,
            } => {
                let mut value = json!({
                    "ok": "heads",
                    "heads": heads.iter()
                        .map(|h| json!([h.node.to_string(), h.seq.to_string(), encoding::encode(&h.id)]))
                        .collect::<Vec<_>>(),
                    "log_len": log_len.to_string(),
                    "cursor": cursor.to_string(),
                });
                if let Some(since) = since {
                    value["since"] = json!(since.to_string());
                }
                value
            }
            Response::Challenge {
                audience,
                nonce,
                expires_at_ms,
                held,
                work,
            } => {
                let mut value = json!({
                    "ok": "challenge",
                    "audience": audience,
                    "nonce": nonce,
                    "expires_at_ms": expires_at_ms.to_string(),
                    "held": held,
                });
                if let Some(bits) = work {
                    value["work"] = json!(bits);
                }
                value
            }
            Response::Token {
                token,
                expires_at_ms,
            } => json!({
                "ok": "token",
                "token": token,
                "expires_at_ms": expires_at_ms.to_string(),
            }),
            Response::Refused { code, reason } => {
                let mut value = json!({"refused": code.tag(), "reason": reason});
                match code {
                    Code::RateLimited { retry_after_ms } => {
                        value["retry_after_ms"] = json!(retry_after_ms.to_string());
                    }
                    Code::Conflict {
                        node,
                        seq,
                        frozen,
                        cursor,
                        held,
                    } => {
                        value["node"] = json!(node.to_string());
                        value["seq"] = json!(seq.to_string());
                        value["frozen"] = json!(frozen.to_string());
                        value["cursor"] = json!(cursor.to_string());
                        value["held"] = pairs_value(held);
                    }
                    _ => {}
                }
                value
            }
        };
        value["v"] = json!(version);
        value.to_string()
    }

    /// The response a text of any version this crate speaks writes.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|_| "it is not JSON".to_string())?;
        version_in(&value)?;
        let text_of = |name: &str| -> Result<String, String> {
            value
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("it has no `{name}` text"))
        };
        let number = |name: &str| -> Result<u64, String> {
            text_of(name)?
                .parse()
                .map_err(|_| format!("`{name}` is not a number"))
        };
        let flag = |name: &str| -> Result<bool, String> {
            value
                .get(name)
                .and_then(Value::as_bool)
                .ok_or_else(|| format!("it has no `{name}` flag"))
        };
        let list = |name: &str| -> Result<&Vec<Value>, String> {
            value
                .get(name)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("it has no `{name}` list"))
        };
        if let Some(code) = value.get("refused").and_then(Value::as_str) {
            let code = match code {
                "unauthorized" => Code::Unauthorized,
                "forbidden" => Code::Forbidden,
                "rate_limited" => Code::RateLimited {
                    retry_after_ms: number("retry_after_ms")?,
                },
                "too_large" => Code::TooLarge,
                "invalid" => Code::Invalid,
                "behind" => Code::Behind,
                "unavailable" => Code::Unavailable,
                "conflict" => Code::Conflict {
                    node: number("node")?,
                    seq: number("seq")?,
                    frozen: number("frozen")?,
                    cursor: number("cursor")?,
                    held: value
                        .get("held")
                        .and_then(pairs_of)
                        .ok_or("its `held` is not a list of operations")?,
                },
                other => return Err(format!("`{other}` is not a refusal the relay gives")),
            };
            return Ok(Response::Refused {
                code,
                reason: text_of("reason")?,
            });
        }
        match value.get("ok").and_then(Value::as_str) {
            Some("entries") => Ok(Response::Entries {
                entries: list("entries")?
                    .iter()
                    .map(|entry| entry.as_str().map(str::to_string))
                    .collect::<Option<_>>()
                    .ok_or("an entry is not text")?,
                len: number("len")?,
            }),
            Some("appended") => Ok(Response::Appended {
                new: flag("new")?,
                len: number("len")?,
            }),
            Some("accepted") => Ok(Response::Accepted {
                new: flag("new")?,
                cursor: number("cursor")?,
            }),
            Some("feed") => Ok(Response::Feed {
                records: list("records")?
                    .iter()
                    .map(Record::from_value)
                    .collect::<Option<_>>()
                    .ok_or("a record is not one the relay writes")?,
                next: number("next")?,
                more: flag("more")?,
            }),
            Some("operations") => Ok(Response::Operations {
                operations: value
                    .get("operations")
                    .ok_or("it has no `operations` list")
                    .and_then(|list| {
                        pairs_of(list).ok_or("an operation is not an event and its data")
                    })?,
            }),
            Some("heads") => Ok(Response::Heads {
                heads: list("heads")?
                    .iter()
                    .map(|row| {
                        Some(Head {
                            node: row.get(0)?.as_str()?.parse().ok()?,
                            seq: row.get(1)?.as_str()?.parse().ok()?,
                            id: encoding::decode(row.get(2)?.as_str()?)?.try_into().ok()?,
                        })
                    })
                    .collect::<Option<_>>()
                    .ok_or("a head is not a node, a number and an id")?,
                log_len: number("log_len")?,
                cursor: number("cursor")?,
                since: match value.get("since") {
                    None => None,
                    Some(_) => Some(number("since")?),
                },
            }),
            Some("challenge") => Ok(Response::Challenge {
                audience: text_of("audience")?,
                nonce: text_of("nonce")?,
                expires_at_ms: number("expires_at_ms")?,
                held: flag("held")?,
                work: match value.get("work") {
                    None => None,
                    Some(bits) => Some(
                        bits.as_u64()
                            .and_then(|bits| u32::try_from(bits).ok())
                            .ok_or("`work` is not a number of bits")?,
                    ),
                },
            }),
            Some("token") => Ok(Response::Token {
                token: text_of("token")?,
                expires_at_ms: number("expires_at_ms")?,
            }),
            _ => Err("it is not a response the relay gives".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_request_reads_back_as_it_was_written() {
        for request in [
            Request::Entries { from: 3 },
            Request::Append {
                entry: "{\"x\":1}".to_string(),
            },
            Request::Submit {
                event: "record_list.change".to_string(),
                data: "{}".to_string(),
            },
            Request::Checkpoint {
                checkpoint: "{}".to_string(),
            },
            Request::PublishKeyPackage {
                data: "AAAA".to_string(),
            },
            Request::PublishCommit {
                epoch: 4,
                data: "BBBB".to_string(),
            },
            Request::PublishWelcome {
                epoch: 4,
                node: 9,
                data: "CCCC".to_string(),
            },
            Request::PublishKeyGrant {
                epoch: 3,
                node: u64::MAX,
                data: "DDDD".to_string(),
            },
            Request::Feed {
                cursor: u64::MAX,
                limit: 50,
            },
            Request::Operations {
                node: 2,
                from: 1,
                to: 9,
            },
            Request::Heads,
            Request::Challenge {
                list: "home".to_string(),
                node: u64::MAX,
            },
            Request::Login(Login {
                list: "home".to_string(),
                node: 2,
                nonce: "AAAA".to_string(),
                expires_at_ms: 60_000,
                signature: "BBBB".to_string(),
                entries: Vec::new(),
            }),
            Request::Login(Login {
                list: "home".to_string(),
                node: 2,
                nonce: "AAAA".to_string(),
                expires_at_ms: 60_000,
                signature: "BBBB".to_string(),
                entries: vec!["{\"x\":1}".to_string()],
            }),
            Request::Join(Box::new(Join {
                invitation: invitation(),
                proof: proof(),
                key_package: Some("AAAA".to_string()),
            })),
            Request::Join(Box::new(Join {
                invitation: invitation(),
                proof: proof(),
                key_package: None,
            })),
            Request::Create(Box::new(Create {
                login: Login {
                    list: "n1.AQID.BBBB".to_string(),
                    node: 1,
                    nonce: "AAAA".to_string(),
                    expires_at_ms: 60_000,
                    signature: "BBBB".to_string(),
                    entries: vec!["{\"x\":1}".to_string(), "{\"x\":2}".to_string()],
                },
                work: u64::MAX,
            })),
        ] {
            // The word a record names a request by is the `op` its text carries.
            let written: Value = serde_json::from_str(&request.to_json()).unwrap();
            assert_eq!(written["op"].as_str(), Some(request.op()), "{request:?}");
            assert_eq!(Request::from_json(&request.to_json()), Ok(request));
        }
    }

    /// The creation of ADR 0080 section 3, as a relay in another language reads it: a login's
    /// members, the counter of the work as text, and the entries, which a creation cannot leave
    /// out.
    #[test]
    fn a_create_is_the_text_the_decision_gives() {
        let text = r#"{"v":1,"op":"create","list":"n1.AQID.BBBB","node":"1","nonce":"A","expires_at_ms":"1","work":"77","entries":["e"],"signature":"B"}"#;
        assert_eq!(
            Request::from_json(text),
            Ok(Request::Create(Box::new(Create {
                login: Login {
                    list: "n1.AQID.BBBB".to_string(),
                    node: 1,
                    nonce: "A".to_string(),
                    expires_at_ms: 1,
                    signature: "B".to_string(),
                    entries: vec!["e".to_string()],
                },
                work: 77,
            })))
        );
        for wrong in [
            r#"{"v":1,"op":"create","list":"l","node":"1","nonce":"A","expires_at_ms":"1","work":"77","signature":"B"}"#,
            r#"{"v":1,"op":"create","list":"l","node":"1","nonce":"A","expires_at_ms":"1","work":77,"entries":["e"],"signature":"B"}"#,
            r#"{"v":1,"op":"create","list":"l","node":"1","nonce":"A","expires_at_ms":"1","entries":["e"],"signature":"B"}"#,
        ] {
            assert!(Request::from_json(wrong).is_err(), "{wrong}");
        }
    }

    fn invitation() -> Invitation {
        Invitation::sign(
            &sce_rust_trust::Identity::from_seed([9; 32]),
            "relay.example",
            "home",
            [1; 32],
            2,
            86_400_000,
            sce_rust_trust::invitation::key_of(&[4; 32]).public(),
        )
    }

    fn proof() -> Proof {
        Proof::make(
            &[4; 32],
            &invitation(),
            &[6; 32],
            60_000,
            sce_rust_trust::Identity::from_seed([2; 32]).public(),
            b"package",
        )
    }

    /// The join of ADR 0080 section 4, as a relay in another language reads it: the invitation as
    /// an object, the proof's parts beside it, and no `key_package` for a device that leaves none.
    #[test]
    fn a_join_is_the_text_the_decision_gives() {
        let text = Request::Join(Box::new(Join {
            invitation: invitation(),
            proof: proof(),
            key_package: None,
        }))
        .to_json();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["op"], "join");
        assert_eq!(value["invitation"]["node"], "2", "a 64-bit number is text");
        assert_eq!(value["expires_at_ms"], "60000");
        assert!(value["proof"].is_string() && value["key"].is_string());
        assert!(value.get("key_package").is_none(), "{text}");
    }

    /// The texts of ADR 0080 section 2, as a relay in another language reads and writes them.
    #[test]
    fn a_challenge_and_a_login_are_the_texts_the_decision_gives() {
        assert_eq!(
            Request::from_json(r#"{"v":1,"op":"challenge","list":"home","node":"7"}"#),
            Ok(Request::Challenge {
                list: "home".to_string(),
                node: 7
            })
        );
        let answer = Response::Challenge {
            audience: "relay.example".to_string(),
            nonce: "AQID".to_string(),
            expires_at_ms: 60_000,
            held: true,
            work: None,
        }
        .to_json();
        let value: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(value["ok"], "challenge");
        assert_eq!(value["expires_at_ms"], "60000", "a 64-bit number is text");
        assert!(value.get("work").is_none(), "no work is asked of a login");
        let token: Value = serde_json::from_str(
            &Response::Token {
                token: "t".to_string(),
                expires_at_ms: 9,
            }
            .to_json(),
        )
        .unwrap();
        assert_eq!(
            token,
            json!({"v": VERSION, "ok": "token", "token": "t", "expires_at_ms": "9"})
        );
        assert!(
            Request::from_json(
                r#"{"v":1,"op":"login","list":"home","node":"7","nonce":"A","expires_at_ms":"1","signature":"B","entries":"x"}"#
            )
            .is_err(),
            "entries are a list"
        );
    }

    #[test]
    fn every_response_reads_back_as_it_was_written() {
        for response in [
            Response::Entries {
                entries: vec!["a".to_string(), "b".to_string()],
                len: 2,
            },
            Response::Appended { new: true, len: 4 },
            Response::Accepted {
                new: false,
                cursor: 9,
            },
            Response::Feed {
                records: vec![
                    Record::Entry("e".to_string()),
                    Record::Operation {
                        event: "record_list.change".to_string(),
                        data: "d".to_string(),
                    },
                    Record::Checkpoint("c".to_string()),
                    Record::KeyPackage {
                        node: 3,
                        data: "k".to_string(),
                        join: None,
                    },
                    Record::KeyPackage {
                        node: 2,
                        data: String::new(),
                        join: Some(Box::new(Joined {
                            invitation: invitation(),
                            proof: proof(),
                        })),
                    },
                    Record::Commit {
                        epoch: 2,
                        data: "m".to_string(),
                    },
                    Record::Welcome {
                        epoch: 2,
                        node: 3,
                        data: "w".to_string(),
                    },
                    Record::KeyGrant {
                        epoch: 1,
                        node: 3,
                        data: "g".to_string(),
                    },
                ],
                next: 3,
                more: true,
            },
            Response::Operations {
                operations: vec![("record_list.change".to_string(), "d".to_string())],
            },
            Response::Heads {
                heads: vec![Head {
                    node: 1,
                    seq: 2,
                    id: [3; 32],
                }],
                log_len: 5,
                cursor: 6,
                since: None,
            },
            Response::Heads {
                heads: Vec::new(),
                log_len: 2,
                cursor: 2,
                since: Some(1_800_000_000_000),
            },
            Response::Challenge {
                audience: "relay.example".to_string(),
                nonce: "AQID".to_string(),
                expires_at_ms: 60_000,
                held: false,
                work: Some(20),
            },
            Response::Token {
                token: "payload.tag".to_string(),
                expires_at_ms: 3_600_000,
            },
            Response::refused(Code::RateLimited { retry_after_ms: 40 }, "slow down"),
            Response::refused(Code::Behind, "send the log"),
            Response::refused(
                Code::Conflict {
                    node: u64::MAX,
                    seq: 7,
                    frozen: 6,
                    cursor: 12,
                    held: vec![
                        ("record_list.change".to_string(), "a".to_string()),
                        ("record_list.change".to_string(), "b".to_string()),
                    ],
                },
                "kept as proof",
            ),
        ] {
            // The word a record names a response by is its `ok`, or `refused`, and a refusal's
            // code is the word its text carries.
            let written: Value = serde_json::from_str(&response.to_json()).unwrap();
            match &response {
                Response::Refused { code, .. } => {
                    assert_eq!(response.tag(), "refused");
                    assert_eq!(written["refused"].as_str(), Some(code.tag()));
                }
                _ => assert_eq!(written["ok"].as_str(), Some(response.tag()), "{response:?}"),
            }
            assert_eq!(Response::from_json(&response.to_json()), Ok(response));
        }
    }

    #[test]
    fn what_is_not_a_request_is_refused_with_a_reason() {
        for text in [
            "not json",
            "{}",
            r#"{"v":0,"op":"heads"}"#,
            r#"{"v":3,"op":"heads"}"#,
            r#"{"v":"2","op":"heads"}"#,
            r#"{"v":1,"op":"fly"}"#,
            r#"{"v":1,"op":"entries","from":"x"}"#,
            r#"{"v":1,"op":"feed","cursor":"0","limit":-1}"#,
            r#"{"v":1,"op":"submit","event":"record_list.change"}"#,
        ] {
            assert!(Request::from_json(text).is_err(), "{text}");
        }
    }

    /// Every version this crate speaks, and one either side of them: a request and a response are
    /// written under the version asked and read back with it; a client of this crate writes the
    /// newest; a text of a version not spoken is read as nothing, and is answered in the oldest,
    /// which every client reads, as a text that is no request is.
    #[test]
    fn a_text_is_written_and_read_in_each_version_spoken_and_no_other() {
        assert_eq!((OLDEST_VERSION, CONFLICT_SINCE, VERSION), (1, 2, 2));
        let request = Request::Feed {
            cursor: 3,
            limit: 9,
        };
        let response = Response::Accepted {
            new: true,
            cursor: 4,
        };
        let version_written =
            |text: &str| serde_json::from_str::<Value>(text).unwrap()["v"].clone();
        assert_eq!(version_written(&request.to_json()), json!(VERSION));
        assert_eq!(version_written(&response.to_json()), json!(VERSION));
        for version in 0..=VERSION + 1 {
            let asked = request.to_json_in(version);
            let answered = response.to_json_in(version);
            assert_eq!(version_written(&asked), json!(version));
            assert_eq!(version_written(&answered), json!(version));
            if speaks(version) {
                assert_eq!(Request::read(&asked), Ok((request.clone(), version)));
                assert_eq!(Response::from_json(&answered), Ok(response.clone()));
                assert_eq!(version_of(&asked), version);
            } else {
                assert!(Request::read(&asked).is_err(), "{asked}");
                assert!(Response::from_json(&answered).is_err(), "{answered}");
                assert_eq!(version_of(&asked), OLDEST_VERSION, "{asked}");
            }
        }
        for unread in ["not json", "{}", r#"{"v":"2","op":"heads"}"#] {
            assert_eq!(version_of(unread), OLDEST_VERSION, "{unread}");
        }
    }

    /// The refusal `conflict` as a client in another language reads it: its numbers as text, and
    /// what the relay held as `[event, data]` pairs; one that leaves out what it held is not one.
    #[test]
    fn a_conflict_is_the_text_the_decision_gives() {
        let text = Response::refused(
            Code::Conflict {
                node: 1,
                seq: 3,
                frozen: 3,
                cursor: 8,
                held: vec![("record_list.change".to_string(), "{}".to_string())],
            },
            "why",
        )
        .to_json();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value,
            json!({"v": 2, "refused": "conflict", "reason": "why", "node": "1", "seq": "3",
                   "frozen": "3", "cursor": "8", "held": [["record_list.change", "{}"]]})
        );
        let mut without = value.clone();
        without.as_object_mut().unwrap().remove("held");
        assert!(Response::from_json(&without.to_string()).is_err());
    }
}
