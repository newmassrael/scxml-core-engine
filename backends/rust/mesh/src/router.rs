// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The router core: one machine's Mesh endpoint, joining the send half
//! ([`crate::outbound`]) and the receive half ([`crate::inbound`]) to the
//! engine's host-processor surface.
//!
//! A `<send target="#peer">` reaches a host as a `HostSendRequest` whose
//! `event_data` is the text a local delivery would have carried in
//! `_event.data`; [`Router::send`] turns it into an envelope for that peer.
//! Envelope bytes a transport received go through [`Router::receive`], which
//! answers with the events the engine may now raise, in order.
//!
//! Sans-IO, like both halves: every call returns [`Effect`]s and the host
//! performs them — it owns the transport, the clock and the randomness an
//! envelope id needs. That is what lets the same core run under a test, a
//! WebSocket or an in-process loopback, and it is the shape SCE_MESH.md
//! §mesh-6.4 gives a transport: the transport moves bytes, the core decides.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use sce_forge_runtime::algorithm::AlgorithmError;
use sce_forge_runtime::codec::CodecError;
use sce_rust_runtime::{HostSendRequest, HostSendResponse};

use crate::generated::envelope::Envelope;
use crate::generated::pattern_kind::PatternKind;
use crate::generated::payload_codec::PayloadCodec;
use crate::inbound::{AdmitError, ConfigError, Delivery, Inbound, Outcome};
use crate::outbound::{Admitted, Outbound, RetryPolicy};
use crate::signal::{Binding, Signal};

/// What deployment says about one peer this machine talks to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeerConfig {
    /// The binding's transport kind (`"wss"`, `"custom_tcp"`, ...): the
    /// `transport` column of the §16.7 rows observed on it.
    pub transport: &'static str,
    /// deploy.yaml's `max_pending_per_target` (§10.10).
    pub max_pending: u32,
    /// deploy.yaml's `max_age_ms`, 0 for no bound (§10.10).
    pub max_age_ms: i64,
    /// deploy.yaml's `retry` block, if the binding has one (§10.10).
    pub retry: Option<RetryPolicy>,
    /// Whether envelopes TO this peer carry a `sequence_no`: the binding
    /// declares `ordering: required` on a transport that does not itself
    /// order (§10.6.3).
    pub stamp_sequence: bool,
    /// How envelopes FROM this peer are delivered (§10.5, §10.6).
    pub delivery: Delivery,
}

/// One thing the host must do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Hand `bytes` to the transport bound to `peer`.
    Transmit { peer: String, bytes: Vec<u8> },
    /// Raise `event` on the engine's external queue with `data` as
    /// `_event.data`; `source` is the machine that sent it.
    Deliver {
        event: String,
        data: String,
        source: String,
    },
    /// Raise `error.communication` with this §16.7 row. `peer` names the
    /// binding the row is about, when there is one.
    Raise {
        peer: Option<String>,
        signal: Signal,
    },
}

struct Peer {
    outbound: Outbound,
    config: PeerConfig,
    next_sequence: u64,
}

/// One machine's Mesh endpoint.
pub struct Router {
    machine: String,
    inbound: Inbound,
    peers: BTreeMap<String, Peer>,
}

impl Router {
    /// A router for `machine`, with the receive side's dedup window and gap
    /// timeout (deploy.yaml `window_size`, `gap_timeout_ms`).
    pub fn new(machine: &str, dedup_window: u32, gap_timeout_ms: i64) -> Result<Self, ConfigError> {
        Ok(Self {
            machine: machine.to_string(),
            inbound: Inbound::new(dedup_window, gap_timeout_ms)?,
            peers: BTreeMap::new(),
        })
    }

    /// Bind `peer` — the name a document writes as `#peer`.
    pub fn add_peer(&mut self, peer: &str, config: PeerConfig) {
        self.peers.insert(
            peer.to_string(),
            Peer {
                outbound: Outbound::new(config.max_pending, config.max_age_ms),
                config,
                next_sequence: 1,
            },
        );
    }

    /// Perform a `<send target="#peer">` the engine handed the host. `id` is
    /// the envelope id the host drew (a UUID v7), `now_ms` its monotonic
    /// clock.
    ///
    /// A Mesh target this router has no binding for is unreachable, and says
    /// so as §16.7 row 1: the router is registered, so this is not the
    /// `error.execution` of a processor nobody serves.
    pub fn send(
        &mut self,
        request: &HostSendRequest,
        id: [u8; 16],
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        let peer_name = mesh_peer(&request.target)
            .ok_or_else(|| RouterError::NotMeshTarget(request.target.clone()))?;
        let Some(peer) = self.peers.get_mut(peer_name) else {
            return Ok(alloc::vec![Effect::Raise {
                peer: Some(peer_name.to_string()),
                signal: Signal::TransportUnavailable,
            }]);
        };
        let bytes = Envelope {
            id: &id,
            source: &self.machine,
            event_type: &request.event_name,
            pattern: PatternKind::FireForget,
            // The payload is the sending engine's own `_event.data` text, so
            // it travels as JSON, or as nothing when there is none — the rule
            // the C++ core applies to a child's donedata
            // (sce/include/mesh/ChildSessionAdapter.h).
            datacontenttype: if request.event_data.is_empty() {
                PayloadCodec::None
            } else {
                PayloadCodec::Json
            },
            data: request.event_data.as_bytes(),
            sequence_no: peer.config.stamp_sequence.then_some(peer.next_sequence),
            ..Envelope::new()
        }
        .encode_to_vec()
        .map_err(RouterError::Encode)?;
        // §mesh-10.6.3: the counter advances once per envelope written, so
        // one that could not be written leaves no hole; and it has no wrap
        // guard, 2^64 sends being beyond any deployment's lifetime.
        if peer.config.stamp_sequence {
            peer.next_sequence = peer.next_sequence.wrapping_add(1);
        }
        Ok(match peer.outbound.admit(bytes, now_ms) {
            Admitted::Send(bytes) => alloc::vec![Effect::Transmit {
                peer: peer_name.to_string(),
                bytes,
            }],
            Admitted::Queued => Vec::new(),
            Admitted::Dropped(signal) => alloc::vec![Effect::Raise {
                peer: Some(peer_name.to_string()),
                signal,
            }],
        })
    }

    /// `peer`'s transport became ready: release what waited for it.
    pub fn peer_ready(&mut self, peer: &str, now_ms: i64) -> Result<Vec<Effect>, RouterError> {
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        let drained = bound
            .outbound
            .mark_ready(now_ms)
            .map_err(RouterError::Rule)?;
        let mut effects: Vec<Effect> = drained
            .send
            .into_iter()
            .map(|bytes| Effect::Transmit {
                peer: peer.to_string(),
                bytes,
            })
            .collect();
        effects.extend(drained.signals.into_iter().map(|signal| Effect::Raise {
            peer: Some(peer.to_string()),
            signal,
        }));
        Ok(effects)
    }

    /// `peer`'s transport stopped being ready.
    pub fn peer_not_ready(&mut self, peer: &str) -> Result<Vec<Effect>, RouterError> {
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        Ok(bound
            .outbound
            .mark_not_ready()
            .map(|signal| Effect::Raise {
                peer: Some(peer.to_string()),
                signal,
            })
            .into_iter()
            .collect())
    }

    /// The retry policy the host applies when a transmission to `peer`
    /// fails (see [`crate::outbound::Attempts`]).
    pub fn retry_policy(&self, peer: &str) -> Option<RetryPolicy> {
        self.peers.get(peer).and_then(|bound| bound.config.retry)
    }

    /// The `error.communication` a raised row is, as the engine receives it:
    /// the row rendered with the binding it was observed on, so its
    /// `target` and `transport` columns are the deployment's.
    ///
    /// A [`HostSendResponse`] because that is the engine's own shape for an
    /// event a host produces — and the one a send handler answers with, so
    /// a row [`Router::send`] observed reaches the document the way any
    /// other host-served send reports back.
    pub fn error_event(&self, peer: Option<&str>, signal: &Signal) -> HostSendResponse {
        let binding = peer.map(|peer| Binding {
            peer,
            transport: self.peers.get(peer).map(|bound| bound.config.transport),
        });
        HostSendResponse {
            event_name: "error.communication".to_string(),
            event_data: signal.event_data(binding),
        }
    }

    /// Envelope `bytes` a transport received from `peer`, at `now_ms`.
    ///
    /// What the document is told about is an effect; what only the host can
    /// act on is an error. An envelope that cannot be read is §16.7 row 4
    /// and one an ordered binding cannot place is row 11, so both are
    /// effects. A hold that is full or a sequence at the `u64` maximum is a
    /// limit of this receiver, which no row names and distribution may not
    /// invent one for (§mesh-16.7, "Scope of synthesis"), so it is returned.
    pub fn receive(
        &mut self,
        peer: &str,
        bytes: Vec<u8>,
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        let delivery = self
            .peers
            .get(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?
            .config
            .delivery;
        let raise = |signal| {
            Ok(alloc::vec![Effect::Raise {
                peer: Some(peer.to_string()),
                signal,
            }])
        };
        match self.inbound.admit(bytes, delivery, now_ms) {
            Ok(outcome) => Ok(effects_of(outcome, Some(peer))),
            // The bytes are the envelope's CBOR (§mesh-7.5); nothing in
            // them can be trusted to name a source.
            Err(AdmitError::Malformed(_)) => raise(Signal::EnvelopeCorrupt {
                source: None,
                codec: "cbor",
            }),
            Err(AdmitError::Unstamped) => raise(Signal::MissingSequence {
                source: peer.to_string(),
            }),
            Err(AdmitError::Rule(error)) => Err(RouterError::Rule(error)),
            Err(AdmitError::HoldFull) => Err(RouterError::HoldFull {
                peer: Some(peer.to_string()),
            }),
        }
    }

    /// End every ordering gap that has waited out its timeout by `now_ms`.
    pub fn tick(&mut self, now_ms: i64) -> Result<Vec<Effect>, RouterError> {
        match self.inbound.tick(now_ms) {
            Ok(outcome) => Ok(effects_of(outcome, None)),
            Err(AdmitError::Rule(error)) => Err(RouterError::Rule(error)),
            Err(AdmitError::HoldFull) => Err(RouterError::HoldFull { peer: None }),
            // A tick reads no bytes and stamps nothing.
            Err(AdmitError::Malformed(_)) | Err(AdmitError::Unstamped) => {
                unreachable!("Inbound::tick decodes no envelope")
            }
        }
    }
}

/// Why the router could not do what the host asked. None of these is a
/// §16.7 row: each is the host's to act on, not the document's.
#[derive(Debug, Clone, PartialEq)]
pub enum RouterError {
    /// The host handed [`Router::send`] a target that is not a Mesh peer
    /// reference (see [`mesh_peer`]).
    NotMeshTarget(String),
    /// The host named a peer it never bound with [`Router::add_peer`].
    UnknownPeer(String),
    /// A standard rule refused its input: a sequence at the `u64` maximum,
    /// or a clock reading that cannot be subtracted.
    Rule(AlgorithmError),
    /// A sender has more envelopes held ahead of sequence than the ordering
    /// rule's list carries. `peer` is the binding that delivered the one
    /// that overflowed; a tick, which delivers nothing, cannot name one.
    HoldFull { peer: Option<String> },
    /// The envelope could not be written within the bounds its document
    /// declares — an event whose data is longer than an envelope carries.
    Encode(CodecError),
}

/// The peer a `<send target>` names, when it names one: `#` followed by at
/// least one character, where `#_` stays reserved for the targets
/// §scxml-6.2.4 defines (`#_internal`, `#_parent`, `#_<invokeid>`, ...).
///
/// The same predicate as the C++ core's `SendHelper::isMeshTarget`
/// (sce/include/common/SendHelper.h); a target one core routes over Mesh
/// and the other does not would be the same document meaning two things.
pub fn mesh_peer(target: &str) -> Option<&str> {
    target
        .strip_prefix('#')
        .filter(|peer| !peer.is_empty() && !peer.starts_with('_'))
}

/// Turn what the receive half released into effects, in release order,
/// followed by the rows it observed.
fn effects_of(outcome: Outcome, peer: Option<&str>) -> Vec<Effect> {
    let mut effects: Vec<Effect> = outcome
        .released
        .iter()
        .map(|received| deliver(&received.envelope(), peer))
        .collect();
    effects.extend(outcome.signals.into_iter().map(|signal| Effect::Raise {
        peer: peer.map(str::to_string),
        signal,
    }));
    effects
}

/// The event an admitted envelope raises. A JSON or empty payload is the
/// `_event.data` text the sender's engine wrote. A payload in another codec
/// is bytes the engine's text surface cannot be handed, so it is §16.7 row 4
/// naming that codec rather than a string made from bytes that are not one.
/// `peer` is the binding it arrived on, when a receipt rather than a tick
/// released it.
fn deliver(envelope: &Envelope<'_>, peer: Option<&str>) -> Effect {
    let text = match envelope.datacontenttype {
        PayloadCodec::None => Ok(String::new()),
        PayloadCodec::Json => core::str::from_utf8(envelope.data)
            .map(str::to_string)
            .map_err(|_| "json"),
        PayloadCodec::Cbor => Err("cbor"),
        PayloadCodec::Typed => Err("typed"),
        PayloadCodec::Raw => Err("raw"),
    };
    match text {
        Ok(data) => Effect::Deliver {
            event: envelope.event_type.to_string(),
            data,
            source: envelope.source.to_string(),
        },
        Err(codec) => Effect::Raise {
            peer: peer.map(str::to_string),
            signal: Signal::EnvelopeCorrupt {
                source: Some(envelope.source.to_string()),
                codec,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A binding with ordering required on a transport that does not order:
    /// the sender stamps and the receiver orders.
    const ORDERED: PeerConfig = PeerConfig {
        transport: "wss",
        max_pending: 4,
        max_age_ms: 0,
        retry: None,
        stamp_sequence: true,
        delivery: Delivery {
            dedup: true,
            ordered: true,
        },
    };

    const UNORDERED: PeerConfig = PeerConfig {
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        ..ORDERED
    };

    /// Two routers bound to each other, the sender's transport ready.
    fn pair(config: PeerConfig) -> (Router, Router) {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        ecu.add_peer("hmi", config);
        hmi.add_peer("ecu", config);
        assert!(ecu.peer_ready("hmi", 0).unwrap().is_empty());
        (ecu, hmi)
    }

    fn events(effects: &[Effect]) -> Vec<&str> {
        effects
            .iter()
            .map(|effect| match effect {
                Effect::Deliver { event, .. } => event.as_str(),
                other => panic!("expected only deliveries, got {other:?}"),
            })
            .collect()
    }

    fn request(target: &str, event: &str, data: &str) -> HostSendRequest {
        HostSendRequest {
            processor_type: "sce:mesh".to_string(),
            event_name: event.to_string(),
            target: target.to_string(),
            send_id: "send.1".to_string(),
            event_data: data.to_string(),
            ..HostSendRequest::default()
        }
    }

    fn transmitted(effects: Result<Vec<Effect>, RouterError>) -> Vec<u8> {
        match effects.unwrap().as_slice() {
            [Effect::Transmit { peer, bytes }] if peer == "hmi" => bytes.clone(),
            other => panic!("expected one transmission to hmi, got {other:?}"),
        }
    }

    #[test]
    fn a_send_to_a_ready_peer_crosses_as_the_engines_event_data() {
        let (mut ecu, mut hmi) = pair(ORDERED);
        let bytes = transmitted(ecu.send(
            &request("#hmi", "speed.changed", r#"{"kph":42}"#),
            [1; 16],
            0,
        ));
        assert_eq!(
            hmi.receive("ecu", bytes, 0).unwrap(),
            alloc::vec![Effect::Deliver {
                event: "speed.changed".to_string(),
                data: r#"{"kph":42}"#.to_string(),
                source: "ecu".to_string(),
            }]
        );
    }

    #[test]
    fn a_send_with_no_data_travels_with_no_payload() {
        let (mut ecu, mut hmi) = pair(ORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "ping", ""), [2; 16], 0));
        let envelope =
            Envelope::decode(&mut sce_forge_runtime::codec::SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.datacontenttype, PayloadCodec::None);
        assert_eq!(
            hmi.receive("ecu", bytes.clone(), 0).unwrap(),
            alloc::vec![Effect::Deliver {
                event: "ping".to_string(),
                data: String::new(),
                source: "ecu".to_string(),
            }]
        );
    }

    #[test]
    fn an_ordered_binding_stamps_from_one_so_the_receiver_restores_send_order() {
        let (mut ecu, mut hmi) = pair(ORDERED);
        let sent: Vec<_> = ["a", "b", "c"]
            .iter()
            .zip(1u8..)
            .map(|(event, id)| transmitted(ecu.send(&request("#hmi", event, ""), [id; 16], 0)))
            .collect();
        let stamps: Vec<_> = sent
            .iter()
            .map(|bytes| {
                Envelope::decode(&mut sce_forge_runtime::codec::SceCursor::new(bytes))
                    .unwrap()
                    .sequence_no
            })
            .collect();
        assert_eq!(stamps, alloc::vec![Some(1), Some(2), Some(3)]);

        assert_eq!(
            events(&hmi.receive("ecu", sent[0].clone(), 0).unwrap()),
            ["a"]
        );
        assert!(hmi.receive("ecu", sent[2].clone(), 0).unwrap().is_empty());
        assert_eq!(
            events(&hmi.receive("ecu", sent[1].clone(), 0).unwrap()),
            ["b", "c"]
        );
    }

    #[test]
    fn an_unordered_binding_does_not_stamp() {
        let (mut ecu, _) = pair(UNORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "a", ""), [1; 16], 0));
        let envelope =
            Envelope::decode(&mut sce_forge_runtime::codec::SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.sequence_no, None);
    }

    #[test]
    fn an_unstamped_envelope_on_an_ordered_binding_is_missing_its_sequence() {
        let (mut ecu, _) = pair(UNORDERED);
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer("ecu", ORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "a", ""), [1; 16], 0));
        assert_eq!(
            hmi.receive("ecu", bytes, 0).unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("ecu".to_string()),
                signal: Signal::MissingSequence {
                    source: "ecu".to_string(),
                },
            }]
        );
    }

    #[test]
    fn a_resent_envelope_is_delivered_once() {
        let (mut ecu, mut hmi) = pair(UNORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "a", ""), [7; 16], 0));
        assert_eq!(
            events(&hmi.receive("ecu", bytes.clone(), 0).unwrap()),
            ["a"]
        );
        assert!(hmi.receive("ecu", bytes, 1).unwrap().is_empty());
    }

    #[test]
    fn a_peer_not_yet_ready_holds_the_send_until_it_is() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        ecu.add_peer("hmi", ORDERED);
        assert!(ecu
            .send(&request("#hmi", "a", ""), [1; 16], 0)
            .unwrap()
            .is_empty());
        transmitted(ecu.peer_ready("hmi", 10));
    }

    #[test]
    fn a_mesh_target_with_no_binding_is_unreachable_not_unserved() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        assert_eq!(
            ecu.send(&request("#nowhere", "a", ""), [1; 16], 0).unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("nowhere".to_string()),
                signal: Signal::TransportUnavailable,
            }]
        );
    }

    #[test]
    fn a_target_that_is_not_a_mesh_peer_is_refused_to_the_host() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        for target in ["hmi", "#", "#_internal", "#_parent", ""] {
            assert_eq!(
                ecu.send(&request(target, "a", ""), [1; 16], 0),
                Err(RouterError::NotMeshTarget(target.to_string()))
            );
        }
    }

    #[test]
    fn the_mesh_target_predicate_is_the_cpp_cores() {
        // Every row of SendHelper::isMeshTarget's contract.
        assert_eq!(mesh_peer("#hmi"), Some("hmi"));
        assert_eq!(mesh_peer("#h"), Some("h"));
        assert_eq!(mesh_peer("#h_1"), Some("h_1"));
        assert_eq!(mesh_peer("#"), None);
        assert_eq!(mesh_peer("#_internal"), None);
        assert_eq!(mesh_peer("#_scxml_session"), None);
        assert_eq!(mesh_peer("hmi"), None);
        assert_eq!(mesh_peer(""), None);
    }

    #[test]
    fn bytes_that_are_not_an_envelope_are_reported_as_corrupt_cbor() {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer("ecu", ORDERED);
        assert_eq!(
            hmi.receive("ecu", alloc::vec![0xFF], 0).unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("ecu".to_string()),
                signal: Signal::EnvelopeCorrupt {
                    source: None,
                    codec: "cbor",
                },
            }]
        );
    }

    #[test]
    fn a_payload_the_engine_cannot_be_handed_is_corrupt_in_its_own_codec() {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer("ecu", UNORDERED);
        for (id, codec, name) in [
            (1u8, PayloadCodec::Cbor, "cbor"),
            (2, PayloadCodec::Typed, "typed"),
            (3, PayloadCodec::Raw, "raw"),
        ] {
            let bytes = Envelope {
                id: &[id; 16],
                source: "ecu",
                event_type: "a",
                pattern: PatternKind::FireForget,
                datacontenttype: codec,
                data: &[0x01],
                ..Envelope::new()
            }
            .encode_to_vec()
            .unwrap();
            assert_eq!(
                hmi.receive("ecu", bytes, 0).unwrap(),
                alloc::vec![Effect::Raise {
                    peer: Some("ecu".to_string()),
                    signal: Signal::EnvelopeCorrupt {
                        source: Some("ecu".to_string()),
                        codec: name,
                    },
                }]
            );
        }
    }

    #[test]
    fn a_raised_row_carries_the_binding_it_was_observed_on() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        ecu.add_peer("hmi", ORDERED);
        let raised = ecu.send(&request("#nowhere", "a", ""), [1; 16], 0).unwrap();
        let [Effect::Raise { peer, signal }] = raised.as_slice() else {
            panic!("expected one raised row, got {raised:?}");
        };
        // An unbound target has no transport to name: its row says which
        // peer it was, and nothing a deployment did not declare.
        let event = ecu.error_event(peer.as_deref(), signal);
        assert_eq!(event.event_name, "error.communication");
        assert_eq!(
            event.event_data,
            r#"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"nowhere"}"#
        );

        ecu.peer_ready("hmi", 0).unwrap();
        let lost = ecu.peer_not_ready("hmi").unwrap();
        let [Effect::Raise { peer, signal }] = lost.as_slice() else {
            panic!("expected one raised row, got {lost:?}");
        };
        assert_eq!(
            ecu.error_event(peer.as_deref(), signal).event_data,
            r#"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"hmi","transport":"wss"}"#
        );
    }

    #[test]
    fn a_peer_the_host_never_bound_is_the_hosts_error() {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        assert_eq!(
            hmi.receive("ecu", alloc::vec![0xFF], 0),
            Err(RouterError::UnknownPeer("ecu".to_string()))
        );
        assert_eq!(
            hmi.peer_ready("ecu", 0),
            Err(RouterError::UnknownPeer("ecu".to_string()))
        );
    }
}
