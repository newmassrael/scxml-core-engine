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
use sce_rust_trust::Head;

/// The version of this protocol, written into every request and response.
pub const VERSION: u64 = 1;

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
    /// alphabet). The relay does not read it.
    KeyPackage { node: u64, data: String },
    /// The MLS commit that begins `epoch`, which the membership log names by its hash: the relay
    /// keeps only the one the log names.
    Commit { epoch: u32, data: String },
    /// The Welcome for the device of `node` added by the commit of `epoch`. It is sealed to that
    /// device's key package, and the relay does not read it.
    Welcome { epoch: u32, node: u64, data: String },
}

impl Record {
    pub fn to_value(&self) -> Value {
        match self {
            Record::Entry(entry) => json!({"k": "entry", "d": entry}),
            Record::Operation { event, data } => json!({"k": "op", "e": event, "d": data}),
            Record::Checkpoint(checkpoint) => json!({"k": "cp", "d": checkpoint}),
            Record::KeyPackage { node, data } => {
                json!({"k": "kp", "n": node.to_string(), "d": data})
            }
            Record::Commit { epoch, data } => json!({"k": "commit", "p": epoch, "d": data}),
            Record::Welcome { epoch, node, data } => {
                json!({"k": "welcome", "p": epoch, "n": node.to_string(), "d": data})
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
    /// Everything the relay holds after the first `cursor` records, in the order it arrived, at
    /// most `limit` of them.
    Feed { cursor: u64, limit: u32 },
    /// The operations of `node` numbered `from` to `to`.
    Operations { node: u64, from: u64, to: u64 },
    /// What the relay holds of each device's chain, and how long its log is.
    Heads,
}

impl Request {
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
        )
    }

    pub fn to_json(&self) -> String {
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
        };
        value["v"] = json!(VERSION);
        value.to_string()
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|_| "it is not JSON".to_string())?;
        if value.get("v").and_then(Value::as_u64) != Some(VERSION) {
            return Err(format!("it is not version {VERSION} of the protocol"));
        }
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
}

impl Code {
    fn tag(&self) -> &'static str {
        match self {
            Code::Unauthorized => "unauthorized",
            Code::Forbidden => "forbidden",
            Code::RateLimited { .. } => "rate_limited",
            Code::TooLarge => "too_large",
            Code::Invalid => "invalid",
            Code::Behind => "behind",
            Code::Unavailable => "unavailable",
        }
    }
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
    Heads {
        heads: Vec<Head>,
        log_len: u64,
        cursor: u64,
    },
    Refused {
        code: Code,
        reason: String,
    },
}

impl Response {
    pub fn refused(code: Code, reason: impl Into<String>) -> Self {
        Response::Refused {
            code,
            reason: reason.into(),
        }
    }

    pub fn to_json(&self) -> String {
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
                "operations": operations.iter().map(|(e, d)| json!([e, d])).collect::<Vec<_>>(),
            }),
            Response::Heads {
                heads,
                log_len,
                cursor,
            } => json!({
                "ok": "heads",
                "heads": heads.iter()
                    .map(|h| json!([h.node.to_string(), h.seq.to_string(), encoding::encode(&h.id)]))
                    .collect::<Vec<_>>(),
                "log_len": log_len.to_string(),
                "cursor": cursor.to_string(),
            }),
            Response::Refused { code, reason } => {
                let mut value = json!({"refused": code.tag(), "reason": reason});
                if let Code::RateLimited { retry_after_ms } = code {
                    value["retry_after_ms"] = json!(retry_after_ms.to_string());
                }
                value
            }
        };
        value["v"] = json!(VERSION);
        value.to_string()
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|_| "it is not JSON".to_string())?;
        if value.get("v").and_then(Value::as_u64) != Some(VERSION) {
            return Err(format!("it is not version {VERSION} of the protocol"));
        }
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
                operations: list("operations")?
                    .iter()
                    .map(|row| {
                        Some((
                            row.get(0)?.as_str()?.to_string(),
                            row.get(1)?.as_str()?.to_string(),
                        ))
                    })
                    .collect::<Option<_>>()
                    .ok_or("an operation is not an event and its data")?,
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
                event: "todo.write".to_string(),
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
        ] {
            assert_eq!(Request::from_json(&request.to_json()), Ok(request));
        }
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
                        event: "todo.write".to_string(),
                        data: "d".to_string(),
                    },
                    Record::Checkpoint("c".to_string()),
                    Record::KeyPackage {
                        node: 3,
                        data: "k".to_string(),
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
                ],
                next: 3,
                more: true,
            },
            Response::Operations {
                operations: vec![("todo.write".to_string(), "d".to_string())],
            },
            Response::Heads {
                heads: vec![Head {
                    node: 1,
                    seq: 2,
                    id: [3; 32],
                }],
                log_len: 5,
                cursor: 6,
            },
            Response::refused(Code::RateLimited { retry_after_ms: 40 }, "slow down"),
            Response::refused(Code::Behind, "send the log"),
        ] {
            assert_eq!(Response::from_json(&response.to_json()), Ok(response));
        }
    }

    #[test]
    fn what_is_not_a_request_is_refused_with_a_reason() {
        for text in [
            "not json",
            "{}",
            r#"{"v":2,"op":"heads"}"#,
            r#"{"v":1,"op":"fly"}"#,
            r#"{"v":1,"op":"entries","from":"x"}"#,
            r#"{"v":1,"op":"feed","cursor":"0","limit":-1}"#,
            r#"{"v":1,"op":"submit","event":"todo.write"}"#,
        ] {
            assert!(Request::from_json(text).is_err(), "{text}");
        }
    }
}
