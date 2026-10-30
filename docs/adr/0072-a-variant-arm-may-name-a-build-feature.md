# ADR 0072 — A variant arm may name a build feature

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left, and a consumer's request (watching-zenoh).
- Amends: ADR 0057 (feature worlds), the codec variant surface (SCE Protocol-Synthesis RFC §5.B).
- Evidence: watching-zenoh's `request.scxml`. Zenoh 1.10.1 accepts only Query as a request body,
  while zenoh-pico accepts Put, Del and Query. In the zenoh-preset images the Put and Del decoders
  are reached only through the Request decode: about 1.1 KB.

## Context

A codec document's `<sce:variant>` names the bodies it dispatches to, one `<sce:arm>` each. A
consumer whose target never receives one of those bodies still compiles its decoder, its enum
variants and every match arm that names it. A whole document can already be left out of a build;
an arm could not.

## Decision

### 1. The surface

`<sce:arm feature="name">`, with the name in `[a-z][a-z0-9-]*`. The arm is compiled where the
feature is on. The default is on, so a build that names no feature keeps every arm. Refused:
- the attribute on `<sce:default>` (the XSD refuses it; without the `xsd` feature the parser
  does);
- on the arm with `default="true"`, or one an overlay makes the default, because the codec's
  `Default` value is built from it (`codec/variant-feature-on-required-arm`);
- a malformed name;
- a gated arm that is the only one holding a body lifetime, since a build without it would leave
  the lifetime unused.

### 2. A build without the feature refuses the arm's tag

The tag of an arm that is not built is refused by decode, with `CodecError::VariantArmNotBuilt`
(Rust) or `SCE_FORGE_CODEC_VARIANT_ARM_NOT_BUILT` (C). It does not go to `<sce:default>`. "This
build does not take that body" and "a tag the document does not know" are different facts. In the
consumer's document the catch-all is Query, so routing a Put frame there would decode it as a
Query without an error. A `<sce:flag value>` seeds `Default` and is not checked on decode, so the
catch-all's own decoder would not catch the mismatch either. The refusing arm also keeps the match
total, so a gated arm needs no catch-all.

### 3. The backends

- Rust: `#[cfg(feature = "name")]` on the arm's variant in every generated enum, its imports,
  its generic parameters and every match arm. A `#[cfg(not(..))]` arm refuses the tag.
- C: `#if SCE_FEATURE_<NAME>` around the arm's kind, union member and cases, with
  `#ifndef SCE_FEATURE_<NAME>` / `#define SCE_FEATURE_<NAME> 1`, so the arm is present unless
  the build defines the macro to 0. An `#else` case refuses the tag.
- C++, Go, Kotlin and Python keep the arm in every build, which is the default meaning.
  - Go's build tags are per file, and Kotlin and Python have no compile-time switch.
  - C++'s variant alternatives are indexed, and leaving one out shifts every index after it.
  - Each template says so where it loops over the arms.
- A codec's `sce:encode-may-fail` is decided over every arm, gated or not (ADR 0065), because it is
  one statement that holds in every feature world (ADR 0057).

## Consequences

- watching-zenoh declares `request-put-del` in its codecs crate, default on, and turns it off in
  the zenoh-preset images. A parent that embeds Request through `<sce:variant-dispatch>` is gated
  the same way.
- In a build that leaves out every other arm, a variant enum has one variant. The Rust template
  then allows `irrefutable_let_patterns` on the `if let` that decodes into it, only for a variant
  that has a gated arm.
- The AST and XSD gain an optional attribute, additively; both surfaces are pre-release.
- A variant with n distinct features writes its source accessor once per subset of them (2^n). The
  consumer's case has one.
