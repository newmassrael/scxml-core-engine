// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The kind catalog — what each `sce:kind` is FOR, for whoever has to
//! choose one from a prose specification.
//!
//! # Why this is a product surface and not a paragraph
//!
//! Choosing the kind is the first decision an author makes, and the
//! product cannot make it: the evidence is in prose, which no check here
//! reads. What the product CAN do is carry the knowledge the choice rests
//! on to wherever the author is. Until this module, that knowledge lived in
//! SCE_FORGE.md §4 and in the fixtures under `tests/forge/resources/` —
//! reachable by a session working inside this tree, and by nothing else.
//! An author reaching SCE through the authoring MCP from their own
//! environment had one sentence of server instructions and a kind
//! enumeration they could only discover by writing a wrong kind and
//! reading the XSD's refusal.
//!
//! So the catalog is emitted by the binary (`sce-codegen kinds`), from
//! this one table, with every example embedded at build time and held by
//! a test to what `check` accepts. The facts a kind's definition already
//! decides elsewhere — which backends emit it, whether it may be declared
//! inline — are derived from the functions that decide them, not restated.
//!
//! ⚠ What the catalog cannot do is choose. `choose_when` lists the evidence
//! a specification offers for a kind, and `distinct_from` names the
//! behaviour that separates it from the kinds it is confused with; when
//! the source states neither side of that behaviour, the source has not
//! determined the kind, and the owner is the one to ask.

use serde::Serialize;

use crate::forge::codegen_matrix::{self, EmitOutcome};
use crate::forge::model::{ForgeKind, SCE_NAMESPACE};
use crate::generator::Language;

/// Catalog schema version. Bumped only on a breaking shape change;
/// additive fields do not bump it (SCE_WIRE_CONTRACTS.md policy).
pub const KIND_CATALOG_SCHEMA_VERSION: u32 = 1;

/// Stability status of the kind-catalog wire surface. Pinned to the
/// `x-sce-schema-status` header of `schemas/sce-kind-catalog.v1.schema.json`
/// by [`tests::schema_file_declares_status`].
pub const KIND_CATALOG_SCHEMA_STATUS: &str = "pre-release";

/// What a document of a kind describes, at the coarsest cut an author can
/// make from a specification before reading any entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KindRole {
    /// Reacts over time: waits, sequences, fires, or announces.
    Behavior,
    /// Computes an answer from its inputs when called.
    Computation,
    /// Declares the shape of data: bytes on a wire, a value set, a payload.
    DataFormat,
    /// Describes the platform the behaviour runs on rather than the
    /// behaviour: endpoints, memory, execution contexts, containers.
    PlatformResource,
}

/// The behaviour that separates a kind from one it is confused with.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Distinction {
    #[serde(serialize_with = "serialize_kind")]
    pub kind: ForgeKind,
    pub because: &'static str,
}

/// Where each kind's example lives: `<EXAMPLE_DIR>/<sce:kind>.scxml`.
///
/// ⚠ Curated for the catalog, not borrowed from the test fixtures. A
/// fixture exists to exercise one path through the product and changes for
/// that reason — its comments describe the test, its names come from
/// whatever the test needed — while an example is what an author copies.
/// Every file here is held to `check --lint` and `pseudo` by
/// `sce-build/tests/every_kind_example_is_accepted.rs`, and a file no kind
/// claims fails there too.
pub const EXAMPLE_DIR: &str = "sce-build/kind-examples";

/// The kind's example document, or why there is none.
#[derive(Debug, Clone, Copy)]
pub enum Example {
    /// The text of `<EXAMPLE_DIR>/<kind>.scxml`, embedded at build time.
    Document(&'static str),
    /// No single document of this kind passes `check`, and why.
    Absent(&'static str),
}

/// One kind as an author reads it.
#[derive(Debug, Clone, Copy)]
pub struct KindGuide {
    pub role: KindRole,
    pub summary: &'static str,
    pub choose_when: &'static [&'static str],
    pub distinct_from: &'static [Distinction],
    pub notes: &'static [&'static str],
    pub example: Example,
}

/// A [`Distinction`] literal. A macro rather than a `const fn` because a
/// slice of function-call results is not promoted to `'static`, and a
/// slice of struct literals is.
macro_rules! apart {
    ($kind:expr, $because:expr $(,)?) => {
        Distinction {
            kind: $kind,
            because: $because,
        }
    };
}

/// The catalog entry for `kind`. Exhaustive, so a kind added to
/// [`ForgeKind`] does not build until an author can be told what it is for.
pub const fn guide(kind: ForgeKind) -> KindGuide {
    use ForgeKind as K;
    match kind {
        K::Statechart => KindGuide {
            role: KindRole::Behavior,
            summary: "A long-lived, event-driven state machine (W3C SCXML): started once, it \
                      waits in states and reacts to each event according to where it is, \
                      until it is stopped.",
            choose_when: &[
                "the text names modes, phases or states the behaviour stays in between events",
                "what happens on an event depends on what happened before it",
                "there are timeouts or delayed actions measured from an event",
                "the behaviour runs for an open-ended lifetime rather than returning a result",
            ],
            distinct_from: &[
                apart!(
                    K::Procedure,
                    "a procedure always runs to a <final> state and returns a result; a \
                     statechart has no required end and keeps its configuration between events",
                ),
                apart!(
                    K::Timer,
                    "a timer document is one periodic tick; a one-shot deadline that no \
                     state exit closes is a delayed <send> in a statechart",
                ),
                apart!(
                    K::Observer,
                    "an observer only announces threshold crossings; what the system does \
                     about them is a statechart receiving those events",
                ),
            ],
            notes: &[
                "the one kind a document may leave undeclared: a root with no sce:kind is \
                 read as a statechart, and the manifest's document_kind.declared says which \
                 happened",
                "transform, lookup, condition and codec may also be declared inline, on a \
                 <data> element inside a statechart",
            ],
            example: Example::Document(include_str!("../../kind-examples/statechart.scxml")),
        },
        K::Transform => KindGuide {
            role: KindRole::Computation,
            summary: "A pure formula: each typed output is one expression over the typed \
                      inputs.",
            choose_when: &[
                "the text gives a conversion or formula: scaling, offset, unit conversion, \
                 an arithmetic combination of inputs",
                "every output follows from the current inputs, with no waiting and no events",
            ],
            distinct_from: &[
                apart!(
                    K::Algorithm,
                    "loops, local variables, early returns or a byte buffer built from the \
                     input need an algorithm; a transform is one expression per output",
                ),
                apart!(
                    K::Interpolation,
                    "values read off a table of breakpoints are an interpolation, not a \
                     formula",
                ),
                apart!(
                    K::Lookup,
                    "a table from enumerated input values to outputs is a lookup",
                ),
            ],
            notes: &[
                "an output that reads previous(x) keeps values between activations; the \
                 manifest's holder then names the object the host must drive",
            ],
            example: Example::Document(include_str!("../../kind-examples/transform.scxml")),
        },
        K::Lookup => KindGuide {
            role: KindRole::Computation,
            summary: "A discrete mapping: each enumerated input value selects one output \
                      value, with a fallback for values the table does not list.",
            choose_when: &[
                "the text gives a table of codes and what each code maps to",
                "an input value selects an output with no computation between them",
            ],
            distinct_from: &[
                apart!(
                    K::Interpolation,
                    "when values between the listed keys must be interpolated, the table \
                     is an interpolation",
                ),
                apart!(
                    K::Enum,
                    "an enum declares a closed set of named values, one wire number each; \
                     a lookup maps keys onto outputs, several keys may share one, and a \
                     key it does not list still gets an answer",
                ),
                apart!(
                    K::Condition,
                    "a single true-or-false decision is a condition",
                ),
            ],
            notes: &[
                "without sce:default the first entry's value answers every key the table \
                 does not list; that fallback is a policy the source has to state",
            ],
            example: Example::Document(include_str!("../../kind-examples/lookup.scxml")),
        },
        K::Condition => KindGuide {
            role: KindRole::Computation,
            summary: "A named boolean expression over typed inputs, reusable as a guard.",
            choose_when: &[
                "the text states when something is allowed, enabled or permitted, as a \
                 combination of input conditions",
                "the result is exactly true or false, and the caller does not need to know \
                 why",
            ],
            distinct_from: &[
                apart!(
                    K::Validator,
                    "when the caller must learn which check failed, or a rate of change is \
                     judged, it is a validator",
                ),
                apart!(
                    K::Observer,
                    "a level that switches on at one threshold and off at another is an \
                     observer's hysteresis, not a condition",
                ),
            ],
            notes: &[
                "declared inline inside a statechart it is generated beside the machine, \
                 but naming it in a transition guard does not call it; the host supplies \
                 the value (SCE_FORGE.md 4.4)",
            ],
            example: Example::Document(include_str!("../../kind-examples/condition.scxml")),
        },
        K::Codec => KindGuide {
            role: KindRole::DataFormat,
            summary: "The byte layout of one frame, for encoding and decoding: each field's \
                      byte offset, bit size and byte order.",
            choose_when: &[
                "the text gives a message or frame layout: byte positions, bit widths, \
                 length fields, byte order",
                "the same bytes must be read and written identically by every consumer",
            ],
            distinct_from: &[
                apart!(
                    K::Algorithm,
                    "output whose length depends on the data (escaping, stuffing, \
                     run-length encoding) is built by an algorithm, not laid out by a codec",
                ),
                apart!(
                    K::EventSchema,
                    "the fields an SCXML event carries are an event-schema; a codec \
                     defines bytes on a wire",
                ),
            ],
            notes: &[
                "byte order defaults to big-endian; the source has to state the byte \
                 order rather than leave it to the default",
            ],
            example: Example::Document(include_str!("../../kind-examples/codec.scxml")),
        },
        K::Procedure => KindGuide {
            role: KindRole::Behavior,
            summary: "A run-to-completion sequence of steps with branching and retry: it \
                      sends requests, branches on the replies, and always ends in a <final> \
                      state with a result.",
            choose_when: &[
                "the text describes a request-and-response sequence that finishes: send, \
                 wait for success or failure, retry up to a limit, report the outcome",
                "each run starts fresh and nothing is kept between runs",
            ],
            distinct_from: &[
                apart!(
                    K::Statechart,
                    "a statechart is long-lived and keeps its state between events; a \
                     procedure is run, completes, and holds nothing across runs",
                ),
                apart!(
                    K::Algorithm,
                    "an algorithm is a synchronous function that never waits; a procedure \
                     waits for the replies to what it sends",
                ),
            ],
            notes: &[
                "its <send> elements name a service request through sce:service, \
                 sce:subfunc, sce:payload and sce:addr, which the runtime's client \
                 interface carries out (SCE_FORGE.md 4.5)",
            ],
            example: Example::Document(include_str!("../../kind-examples/procedure.scxml")),
        },
        K::Validator => KindGuide {
            role: KindRole::Computation,
            summary: "Range, rate-of-change and plausibility checks on inputs, reporting \
                      which check failed.",
            choose_when: &[
                "the text gives allowed ranges, a largest change per sample, or a \
                 plausibility rule across fields",
                "the caller must know why an input was rejected",
            ],
            distinct_from: &[
                apart!(
                    K::Condition,
                    "a condition yields true or false with no reason and remembers no \
                     previous value",
                ),
                apart!(
                    K::Filter,
                    "a filter changes the signal; a validator only judges it",
                ),
            ],
            notes: &["a rate-of-change rule keeps the previous accepted value between calls"],
            example: Example::Document(include_str!("../../kind-examples/validator.scxml")),
        },
        K::Filter => KindGuide {
            role: KindRole::Computation,
            summary: "Filtering of a sampled signal: moving average, low-pass (exponential \
                      smoothing), or debounce.",
            choose_when: &[
                "the text asks to smooth a signal, average it over a number of samples, or \
                 accept a change only after it repeats for a number of samples",
                "the output depends on recent samples, not only on the current one",
            ],
            distinct_from: &[
                apart!(
                    K::Validator,
                    "rejecting an implausible sample is a validator",
                ),
                apart!(
                    K::Transform,
                    "a formula over the current sample alone is a transform",
                ),
                apart!(
                    K::Observer,
                    "announcing that a value crossed a threshold is an observer, which may \
                     read the filter's output",
                ),
            ],
            notes: &[],
            example: Example::Document(include_str!("../../kind-examples/filter.scxml")),
        },
        K::Interpolation => KindGuide {
            role: KindRole::Computation,
            summary: "Table interpolation in one or two dimensions: output values given at \
                      axis breakpoints, interpolated between them.",
            choose_when: &[
                "the text gives a characteristic curve or map: values at listed breakpoints \
                 of one or two inputs",
                "values between breakpoints are interpolated, and values outside the table \
                 follow a stated rule",
            ],
            distinct_from: &[
                apart!(
                    K::Lookup,
                    "exact keys with no values in between are a lookup",
                ),
                apart!(K::Transform, "a closed-form formula is a transform",),
            ],
            notes: &[
                "outside the table the output is clamped unless the document says \
                 extrapolate or error; the source has to say which",
            ],
            example: Example::Document(include_str!("../../kind-examples/interpolation.scxml")),
        },
        K::Timer => KindGuide {
            role: KindRole::Behavior,
            summary: "One periodic timer: it fires an event every period, and may be \
                      restarted by an event and cancelled when a state is exited.",
            choose_when: &[
                "the text says something is sent or checked every fixed period: a \
                 heartbeat, a keep-alive, a polling tick",
                "a deadline is closed by leaving a state, so at most one expiry is observed",
            ],
            distinct_from: &[apart!(
                K::Statechart,
                "a one-shot timeout that no state exit closes is a delayed <send> in a \
                 statechart, cancelled when the awaited event arrives",
            )],
            notes: &[
                "periodic is the only schedule this kind has; there is no one-shot form \
                 (SCE_FORGE.md 4.10)",
            ],
            example: Example::Document(include_str!("../../kind-examples/timer.scxml")),
        },
        K::Observer => KindGuide {
            role: KindRole::Behavior,
            summary: "Threshold monitoring with hysteresis: an event when a value enters a \
                      band, and another when it leaves it at a separate level.",
            choose_when: &[
                "the text gives a warning or alarm level and a different level at which it \
                 clears",
                "the requirement is to announce the crossings, not to act on them",
            ],
            distinct_from: &[
                apart!(
                    K::Condition,
                    "a single comparison with no separate clearing level is a condition",
                ),
                apart!(
                    K::Statechart,
                    "what the system does after the announcement is a statechart that \
                     receives the event",
                ),
            ],
            notes: &[],
            example: Example::Document(include_str!("../../kind-examples/observer.scxml")),
        },
        K::Algorithm => KindGuide {
            role: KindRole::Computation,
            summary: "A pure synchronous function with bounded loops, local variables and \
                      early returns, including building a byte buffer from its input.",
            choose_when: &[
                "the text describes a computation as steps: iterate, accumulate, search, \
                 branch, return",
                "the output is built from input data of varying length: a checksum, \
                 escaping, framing",
            ],
            distinct_from: &[
                apart!(K::Transform, "one expression per output needs no algorithm",),
                apart!(
                    K::Procedure,
                    "an algorithm never waits for an event or a reply",
                ),
                apart!(K::Codec, "a fixed field layout is a codec",),
            ],
            notes: &[],
            example: Example::Document(include_str!("../../kind-examples/algorithm.scxml")),
        },
        K::Link => KindGuide {
            role: KindRole::PlatformResource,
            summary: "A byte-stream endpoint of the platform: its transport class, the \
                      codec that frames its messages, its backpressure policy and the pool \
                      its frames are staged in.",
            choose_when: &[
                "the text specifies how bytes enter and leave the device over a transport, \
                 and what happens when a sender outpaces the receiver",
            ],
            distinct_from: &[
                apart!(
                    K::Codec,
                    "the layout of the messages a link carries is a codec, which the link \
                     names as its framer",
                ),
                apart!(
                    K::Worker,
                    "the task that consumes what the link receives is a worker",
                ),
            ],
            notes: &[
                "names its framer codec and its stage pool by reference; each is a \
                 document of its own",
            ],
            example: Example::Document(include_str!("../../kind-examples/link.scxml")),
        },
        K::BufferPool => KindGuide {
            role: KindRole::PlatformResource,
            summary: "A fixed table of memory slots handed to DMA hardware: slot count, slot \
                      size, memory section, alignment and cache policy, with a lifecycle \
                      for each slot.",
            choose_when: &[
                "the text specifies where transfer buffers live in memory, how many there \
                 are and how large, for DMA or zero-copy I/O",
            ],
            distinct_from: &[apart!(
                K::BoundedCollection,
                "a bounded-collection holds typed elements; a buffer-pool holds raw byte \
                 slots that hardware reads and writes",
            )],
            notes: &[],
            example: Example::Document(include_str!("../../kind-examples/buffer-pool.scxml")),
        },
        K::Worker => KindGuide {
            role: KindRole::PlatformResource,
            summary: "A concurrent execution context that consumes what a link receives \
                      through a fixed-size inbox.",
            choose_when: &[
                "the text specifies a task that processes incoming messages independently \
                 of whoever sends them",
            ],
            distinct_from: &[
                apart!(
                    K::Statechart,
                    "the logic run for each message may be a statechart; the worker is the \
                     context it runs in",
                ),
                apart!(K::Link, "the endpoint a worker reads from is a link"),
            ],
            notes: &[
                "reads a link it imports with <sce:import kind=\"link\">; the import is \
                 followed wherever it points, so a check of the worker alone still \
                 reads the link, its framer and its buffer pool",
            ],
            example: Example::Document(include_str!("../../kind-examples/worker.scxml")),
        },
        K::BoundedCollection => KindGuide {
            role: KindRole::PlatformResource,
            summary: "A typed container whose capacity is fixed when the program is built \
                      and whose occupancy varies while it runs.",
            choose_when: &[
                "the text specifies a table of at most N entries that are added and \
                 removed at run time (subscriptions, sessions, pending requests) where \
                 memory cannot be allocated on demand",
            ],
            distinct_from: &[
                apart!(K::BufferPool, "raw byte slots for DMA are a buffer-pool",),
                apart!(
                    K::Lookup,
                    "a table whose contents are fixed when the program is built is a lookup",
                ),
            ],
            notes: &["names its element type by reference; the element is declared elsewhere"],
            example: Example::Document(include_str!(
                "../../kind-examples/bounded-collection.scxml"
            )),
        },
        K::Enum => KindGuide {
            role: KindRole::DataFormat,
            summary: "A closed set of named values, each with its own fixed wire number.",
            choose_when: &[
                "the text lists named values (status codes, modes, reasons) with the number \
                 each is encoded as",
            ],
            distinct_from: &[
                apart!(
                    K::Lookup,
                    "several keys sharing one output, or an answer for a key nobody listed, \
                     is a lookup",
                ),
                apart!(
                    K::EventSchema,
                    "the fields an event carries are an event-schema, which may use an enum \
                     as a field's type",
                ),
            ],
            notes: &[],
            example: Example::Document(include_str!("../../kind-examples/enum.scxml")),
        },
        K::EventSchema => KindGuide {
            role: KindRole::DataFormat,
            summary: "The typed payload of one named SCXML event: the fields its _event.data \
                      carries.",
            choose_when: &[
                "the text specifies what data accompanies a named event or notification",
                "machines that send and receive the event must agree on its fields",
            ],
            distinct_from: &[
                apart!(K::Codec, "the bytes of a message on a wire are a codec",),
                apart!(
                    K::Enum,
                    "the closed set of values one field may take is an enum",
                ),
            ],
            notes: &[
                "cannot be declared for the W3C built-in events: error.*, done.state.*, \
                 done.invoke.*",
            ],
            example: Example::Document(include_str!("../../kind-examples/event-schema.scxml")),
        },
    }
}

fn serialize_kind<S: serde::Serializer>(kind: &ForgeKind, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(kind.as_attr())
}

/// One entry on the wire: the guide, plus the facts other modules decide.
#[derive(Debug, Serialize)]
pub struct KindRecord {
    #[serde(serialize_with = "serialize_kind")]
    pub name: ForgeKind,
    pub role: KindRole,
    pub summary: &'static str,
    pub choose_when: &'static [&'static str],
    pub distinct_from: &'static [Distinction],
    pub notes: &'static [&'static str],
    /// Whether the kind may be declared on a `<data>` inside a statechart
    /// — [`ForgeKind::is_inline_eligible`].
    pub inline_eligible: bool,
    /// The backends that emit the kind — [`codegen_matrix::lookup`].
    pub backends: Vec<&'static str>,
    pub example: ExampleRecord,
}

/// [`Example`] on the wire, with the document's path in the SCE tree.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExampleRecord {
    /// A document `check --lint` accepts and `pseudo` renders.
    Document { path: String, text: &'static str },
    /// No single document of this kind passes `check`, and why.
    Absent { because: &'static str },
}

impl ExampleRecord {
    fn of(kind: ForgeKind, example: Example) -> Self {
        match example {
            Example::Document(text) => ExampleRecord::Document {
                path: format!("{EXAMPLE_DIR}/{}.scxml", kind.as_attr()),
                text,
            },
            Example::Absent(because) => ExampleRecord::Absent { because },
        }
    }
}

impl KindRecord {
    pub fn of(kind: ForgeKind) -> Self {
        let guide = guide(kind);
        KindRecord {
            name: kind,
            role: guide.role,
            summary: guide.summary,
            choose_when: guide.choose_when,
            distinct_from: guide.distinct_from,
            notes: guide.notes,
            inline_eligible: kind.is_inline_eligible(),
            backends: Language::ALL
                .iter()
                .filter(|lang| matches!(codegen_matrix::lookup(kind, **lang), EmitOutcome::Emit))
                .map(|lang| codegen_matrix::language_wire_name(*lang))
                .collect(),
            example: ExampleRecord::of(kind, guide.example),
        }
    }
}

/// How a document declares its kind. A `kind` attribute in any other
/// namespace than `namespace` is not a declaration, and the document is
/// read as `default`.
#[derive(Debug, Serialize)]
pub struct Declaration {
    pub attribute: &'static str,
    pub namespace: &'static str,
    pub element: &'static str,
    #[serde(serialize_with = "serialize_kind")]
    pub default: ForgeKind,
    /// How the document states why it is its kind.
    pub basis: BasisDeclaration,
}

/// `<sce:kind-basis>` as an author writes it
/// (docs/SCE_ACCEPTED_SUBSET.md §2.10.1): named here so a client outside
/// the tree has the grammar from the product that will check it.
#[derive(Debug, Serialize)]
pub struct BasisDeclaration {
    /// The root child that holds it.
    pub element: &'static str,
    /// What it contains: one or more evidence, any number rejected.
    pub evidence: &'static str,
    pub rejected: &'static str,
    /// A basis for a transform, as it sits directly under the root.
    /// `sce-build/tests/every_kind_example_is_accepted.rs` puts it into
    /// the transform example and requires `check` to accept the result.
    pub example: &'static str,
}

/// The example a [`BasisDeclaration`] carries.
pub const BASIS_EXAMPLE: &str = "<sce:kind-basis>\n  \
    <sce:evidence provenance=\"SPEC-7@2#4.1\">the output is one formula over the current \
    input</sce:evidence>\n  \
    <sce:rejected kind=\"interpolation\">the text gives a formula, not values at \
    breakpoints</sce:rejected>\n</sce:kind-basis>";

/// The catalog as `sce-codegen kinds` writes it.
#[derive(Debug, Serialize)]
pub struct KindCatalog {
    pub v: u32,
    pub generator: &'static str,
    pub declaration: Declaration,
    pub kinds: Vec<KindRecord>,
}

impl KindCatalog {
    /// Every kind, in [`ForgeKind::ALL_ATTR_NAMES`] order — or only `only`.
    pub fn new(only: Option<ForgeKind>) -> Self {
        let kinds = ForgeKind::ALL_ATTR_NAMES
            .iter()
            .filter_map(|name| ForgeKind::from_attr(name))
            .filter(|kind| only.is_none_or(|wanted| wanted == *kind))
            .map(KindRecord::of)
            .collect();
        KindCatalog {
            v: KIND_CATALOG_SCHEMA_VERSION,
            generator: crate::GENERATOR_COMMIT,
            declaration: Declaration {
                attribute: "kind",
                namespace: SCE_NAMESPACE,
                element: "scxml",
                default: ForgeKind::Statechart,
                basis: BasisDeclaration {
                    element: crate::forge::kind_basis::ELEMENT,
                    evidence: "one or more <sce:evidence>: what the specification states, as \
                               text; `provenance` is the optional compact anchor \
                               doc_id[@rev][#section[:position]]",
                    rejected: "any number of <sce:rejected kind=\"...\">: a kind considered \
                               and not chosen, and as text the behaviour that ruled it out; \
                               never the document's own kind",
                    example: BASIS_EXAMPLE,
                },
            },
            kinds,
        }
    }

    /// The single JSON line on stdout.
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("the catalog serialises; every field is owned or static")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../schemas/sce-kind-catalog.v1.schema.json"
        ))
        .expect("kind catalog schema is JSON")
    }

    fn violations(instance: &serde_json::Value) -> Vec<String> {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("kind catalog schema compiles");
        let outcome = validator.validate(instance);
        match outcome {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        }
    }

    #[test]
    fn schema_file_declares_status() {
        assert_eq!(
            schema()["x-sce-schema-status"].as_str(),
            Some(KIND_CATALOG_SCHEMA_STATUS),
            "schemas/sce-kind-catalog.v1.schema.json x-sce-schema-status disagrees with \
             KIND_CATALOG_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    #[test]
    fn schema_version_matches_producer_constant() {
        assert_eq!(
            schema()["properties"]["v"]["const"].as_u64(),
            Some(u64::from(KIND_CATALOG_SCHEMA_VERSION)),
        );
    }

    /// The whole catalog, as produced, is what the schema describes.
    #[test]
    fn the_catalog_validates_against_the_wire_schema() {
        let instance: serde_json::Value =
            serde_json::from_str(&KindCatalog::new(None).to_line()).expect("the line is JSON");
        let found = violations(&instance);
        assert!(found.is_empty(), "{found:?}");
    }

    /// The control is the valid catalog; the one change is a kind the
    /// product does not have.
    #[test]
    fn the_catalog_schema_rejects_a_kind_the_product_does_not_have() {
        let mut instance: serde_json::Value =
            serde_json::from_str(&KindCatalog::new(None).to_line()).expect("the line is JSON");
        assert!(violations(&instance).is_empty(), "control must be valid");
        instance["kinds"][0]["name"] = serde_json::json!("state-machine");
        assert!(
            !violations(&instance).is_empty(),
            "an unknown kind must be refused"
        );
    }

    /// The schema's kind list is the parser's, in the parser's order.
    #[test]
    fn the_schema_names_every_kind_the_parser_accepts() {
        let schema = schema();
        let declared: Vec<&str> = schema["definitions"]["kindName"]["enum"]
            .as_array()
            .expect("kindName.enum is an array")
            .iter()
            .map(|v| v.as_str().expect("a kind name is a string"))
            .collect();
        assert_eq!(declared, ForgeKind::ALL_ATTR_NAMES);
    }

    /// Every kind once, in declaration order; a filter keeps exactly one.
    #[test]
    fn the_catalog_lists_every_kind_once() {
        let names: Vec<&str> = KindCatalog::new(None)
            .kinds
            .iter()
            .map(|k| k.name.as_attr())
            .collect();
        assert_eq!(names, ForgeKind::ALL_ATTR_NAMES);
        let one = KindCatalog::new(Some(ForgeKind::Lookup));
        assert_eq!(one.kinds.len(), 1);
        assert_eq!(one.kinds[0].name, ForgeKind::Lookup);
    }

    /// An entry an author can act on: evidence to look for, a neighbour
    /// named with the behaviour that separates them, and no entry that
    /// separates a kind from itself or names a neighbour twice.
    #[test]
    fn every_entry_gives_evidence_and_a_distinction() {
        for record in KindCatalog::new(None).kinds {
            let name = record.name.as_attr();
            assert!(!record.summary.is_empty(), "{name}: summary");
            assert!(!record.choose_when.is_empty(), "{name}: choose_when");
            assert!(!record.distinct_from.is_empty(), "{name}: distinct_from");
            let mut seen = Vec::new();
            for d in record.distinct_from {
                assert_ne!(d.kind, record.name, "{name} distinguished from itself");
                assert!(!seen.contains(&d.kind), "{name} names {:?} twice", d.kind);
                assert!(!d.because.is_empty(), "{name}: empty distinction");
                seen.push(d.kind);
            }
        }
    }

    /// Each embedded example declares the kind it is the example of, in the
    /// SCE namespace.
    #[test]
    fn every_example_is_a_document_of_its_kind() {
        for record in KindCatalog::new(None).kinds {
            if let ExampleRecord::Document { path, text } = record.example {
                let read = crate::manifest::DocumentKind::of(text)
                    .unwrap_or_else(|| panic!("{path}: the root cannot be read"));
                assert_eq!(read.name, record.name.as_attr(), "{path}");
                // The statechart's too: an example is copied, and the
                // default is what an author must not come to rely on.
                assert!(read.declared, "{path}: an example must declare its kind");
            }
        }
    }
}
