# ADR 0126 — The writes before a variant's body are printed once over the fields they read, apart from the body

- Status: Accepted (2026-11-25), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored. Its figures are the consumer's measurement at the landing (section "Acceptance").
- Amends: ADR 0081 (the encode is planned once and each backend prints the plan): the plan gains the place of the
  head, and the Rust printer prints it as a function of its own.
- Related: ADR 0058 (a codec encodes any value that presents its fields), ADR 0061 (a codec's length and its window
  write are two readings of the one encode), ADR 0065 (what a codec's encode can refuse), ADR 0077 §7 (the wire
  writes every form of a value through one interface).
- Requested by: a downstream bare-metal consumer, 2026-11-25.

## Context

A Rust codec's encode is `{Codec}Wire::write_source<S, K>`, generic over the source `S` the value is read through
and the sink `K` (ADR 0058, ADR 0077 §7). Rust copies a generic function for each pair of types it is called with.
A consumer that hands the encode one type of value for each kind of message, so that a build links the body writers
of the kinds it sends and no other, gets one copy of `write_source` for each kind and each sink. Each copy holds the
whole frame: the head, which is the same for every kind, and the body of the one arm the type holds.

The consumer measured its release no_std image. Its writer of a network `Declare` is 1,578 bytes, and 600 to 770
of them are the same instruction sequence repeated: the header byte computed from the presence of the interest id
and of the extensions, the interest id, and the walk of the extension chain, once for each kind of declaration it
sends. Its `Push`, `Request`, `Response` and `Reply` writers have the same structure: a head of fields read off
the value, then the body of the arm it holds.

The duplication is not the head's fault but where it is written. The head reads the value's fields and nothing of
its arm, so what it does depends on the types of those fields and not on the type of the value.

## Decision

### 1. The plan says where the head ends

`EncodePlan::envelope` answers the place of the variant's body in the program where everything before it is read off
the value's fields alone: the body is the last write and not the first, and nothing derived, checked or written
before it, nor anything an arm is handed, reads which arm the value holds. It answers `None` where the codec has no
variant, writes nothing before the body or something after it, or reads the arm before it:

- a tag of the arm held in a field (`Derive::VariantTag`), or an input an arm says (`Measured::ArmSays`);
- a gate group (`Derive::ArmFlag`, and every read of a group's arm);
- an input the value says that a gate reads;
- a check of an input an arm says (`CheckSite::Arm`);
- bits of a carrier an arm is handed (`HandedFrom::Carrier`): the body would read a local of the head.

The answer is a fact of the plan, not of a backend (ADR 0081 §1), so every printer whose encode is copied for each
type of value reads the same one.

### 2. The Rust printer writes the head once, over the fields

For a codec the plan answers a place for, and that does not lower its source to its parts (whose `write_parts_in`
already writes the wire from one form), the printer writes:

- `write_source` (`write_source_in` where a parent may set a flag): each field the head reads, read once off the
  source and handed to `write_envelope`, then the body of the arm the value holds, as before;
- `write_envelope<K>`: the head, the same statements as before, each read of a field a read of its parameter. A
  parameter's type is the accessor's return type, written as the source trait writes it, so a list is
  `BoundedIter<impl Iterator<Item = impl EntrySource> + Clone, ..>` and an embedded body `&impl BodySource`; its
  type parameters are the types of the fields, not the type of the value. A list is an iterator and each read takes a
  copy of it, as each call of the accessor made a new one; every other field is a value that copies.

The flag inputs the encode is handed and the flags a parent sets are passed on to it. A head of more than seven
parameters carries `#[allow(clippy::too_many_arguments)]`, the one place the printer says so.

`write_envelope` is `#[inline(never)]`, as `write_parts_in` is and for the same reason: inlined into
`write_source`, it is copied into each copy of it. A build that asks every write inlined (`--inline-encode`) marks it
`#[inline(always)]`, and then shares nothing, as it asked.

Every value whose fields have the same types, whatever arm it holds, shares one copy of the head for each sink. Each
type of value keeps a `write_source` of its own, which is now the reads, one call and the body of its arm.

### 3. What it costs

Each frame pays one call and its arguments: the fields the head reads, the sink, and the inputs and flags it is
passed. For the consumer's `Declare` that is three values beside the sink, one of them an iterator of a few words
passed in memory; on Cortex-M that is about ten instructions for each frame, against a frame whose head alone walks
an extension chain. No loop and no write of the head changes.

### 4. The other printers

Go, Python, Kotlin, Swift and C11 do not copy a function for each type of value, and keep their encode. The C++
printer's `write_source` is a template and is copied as Rust's is; it reads the same `EncodePlan::envelope` in a
landing of its own, with its own goldens and measurement.

## Consequences

- A program that writes several types of value of one codec carries the head once for each sink; each type adds its
  reads, a call and its arm.
- The Rust goldens of `codec_zenoh_request` and `codec_zenoh_response`, whose heads read no arm, show the two
  functions. Every other golden is as it was: `codec_zenoh_push`, `codec_zenoh_oam` and the other codecs whose
  header holds the arm's tag, `codec_zenoh_declaration` and `codec_transport_envelope`, whose header holds an input an
  arm says, and `codec_variant_peek_basic` and `codec_zenoh_network_envelope`, which write nothing before the body.
- No public item changes: `write_envelope` is private, and `write_source` and `write_source_in` keep their
  signatures and their bytes.

## Alternatives considered

- **Lower the source to the codec's parts, as a chain entry does (`write_parts_in`).** The parts hold a list as a
  slice, which a lent source does not have: its entries are an iterator, and holding them as a slice needs storage
  the consumer does not have. Refused for any codec with a list.
- **Read the source through a trait object.** One copy of everything, and a virtual call for every field of every
  frame, the arm included. The head needs no dispatch at all, since its types are the fields'. Refused.
- **Mark `write_source` `#[inline(never)]` alone.** It keeps one copy for each type of value, which is the copy that
  is the problem. Refused.
- **A struct of the head's fields, generic over their types, in place of parameters.** The same copies, a type more
  to print and to name, and no fewer instructions. Refused.
- **Leave the consumer to call the codec with one type of value for every kind.** Its writer would then hold the body
  writer of every kind in every build, which is what its per-kind types exist to avoid.

## Measured

To be measured at the landing, by the consumer, on its release no_std image: the writer of a `Declare` and of each
message whose head reads no arm, before and after. The expectation is the duplicated head the consumer measured, 600
to 770 bytes for `Declare` less the shells each kind keeps, about 20 to 30 bytes each; a codec written with one type
of value gains nothing and pays the call.

## Order of the changes

1. This ADR, `EncodePlan::envelope`, and the Rust printer, with the two goldens it changes.
2. The main session runs the Rust golden tests and the Rust conformance lane (`forge_source_round_trip`,
   `forge_view_round_trip`, which write `codec_zenoh_request` and `codec_zenoh_response` from every form of their
   values) and the codec clippy check.
3. The consumer regenerates its codecs and measures its image.

## Acceptance

Accepted on all of these: the Rust goldens and conformance lane pass; `scripts/check_clippy_codec.sh` passes; the
consumer measures its writers at the landing.
