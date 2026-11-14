# ADR 0100 — A codec dispatches on a masked tag, refuses by rule, and states its fixed values, its value ranges and where a list ends

- Status: Accepted (2026-11-14). Decided by the owner on 2026-11-14 under the standing rule of 2026-10-26 and
  2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored. The owner chose each
  element of "Decision" and its spelling; this record states the reasoning, checks the census it answers against the
  tree, and decides the interactions the choices leave open.
- Date: 2026-11-14
- Evidence: the consumer census `zenoh-as-sce-codec-documents.md`, kept in the consumer's tree: a classification of
  every wire element of zenoh 1.10.2 (101 rows) by whether it can be written as an SCE positional codec document,
  read against SCE at `7e396e7ecc`. It finds every message skeleton writable today and the language blocked at five
  places (L1 to L5) and two minor ones (L6, L7). Its sections 1, 3, 6 and 7 are the input of this record.
- Amends:
  - SCE_FORGE.md §4.6, §4.6.2 and §4.6.3: the grammar of a variant, a predicate, a flag input, a chain and a list;
  - ADR 0058 §1: a fixed value, the flags a masked tag covers, and a dispatched flag inside a tag are derived elements;
  - ADR 0065 §1 (two new places an encode refuses) and §6: R3's set of tags a default arm holds excludes every tag a
    refusal in force covers; R4c's gate group widens from a disjunction to any formula over one-bit flags; R7's
    non-empty chain has a fewest of 0 where it may be empty at the end of its scope; R2 gains an exact count;
  - ADR 0072: a refusal names a build feature as an arm does;
  - ADR 0081 §1: the plan reads one statement of each codec's tag partition and value rules (§7 below);
  - `schemas/sce-forge-ext.xsd`: the one-bit lock on `<sce:flag-input width>`, which the xsd deferred "to a reachable
    consumer"; this census is that consumer.
- Scope:
  - the model and parser (`sce-build/src/forge/model.rs`, `parser.rs`), the cross-codec validators of `generator.rs`,
    `codec_roles.rs`, `codec_totality.rs`, `encode_plan.rs`, and one new module, `codec_constraints.rs` (§7);
  - the seven codec backends, Rust, C, C++, Go, Kotlin, Python and Swift: their printers or templates
    (`generator/rust_source.rs`, `generator/cpp_source.rs`, `tools/codegen/templates/forge/{c,go,kotlin,python}/`,
    `generator/swift_codec.rs`) and their forge runtimes' codec errors;
  - `schemas/sce-forge-ext.xsd`, `apis/forge-ast.v1.schema.json` (`pre-release`), `sce-build/src/forge/diagnostic.rs`
    with `schemas/sce-diagnostic.v1.schema.json`, SCE_ERROR_CONTRACT.md's catalog and docs/SCE_ACCEPTED_SUBSET.md's
    appendix;
  - the cross-backend codec conformance corpus: `tests/forge/conformance/binary_model.py`, `fixtures.json`,
    `numerical_reference.json`, and the fixtures under `tests/forge/resources/`.
- Related: ADR 0010 decision 2 (byte parity), ADR 0049 (a decoded value is a view whose lists are walked), ADR 0057
  (every world of the features is a valid machine), ADR 0077 (a declared bound is part of a field's type), ADR 0078 (a
  consumer follows SCE one landing at a time), ADR 0095 (where a fixed field's bits lie), ADR 0097 (generated code is a
  build output). Separate from this record, and not decided here: the narrow-width VLE reader (a `vle` field of a type
  narrower than 64 bits bounds the value, while its wire is a 64-bit ZInt of up to nine bytes; being fixed on its
  own), the span and walk generation mode (ADR 0103), and the consumer's codec binding (ADR 0102).

## Context

### What the census asked and found

The consumer generates its zenoh codecs from SCE codec documents and asked whether every zenoh 1.10.2 frame can be
one. Its answer: the skeleton of every message can (header flags, fields gated on a flag, a flag the parent passes in,
lengths and `length-1` lengths, VLE integers, a Z-terminated extension chain, the payload chosen by a shared-memory
marker in the chain), and the language blocks at:

- **L1**, an extension's body chosen by its identity (`id | M | enc`, the header without the continuation flag Z)
  while an identity no arm names is still read by its encoding. Every one of the 41 typed extensions needs it. Two
  extensions of one carrier can share an id and differ by encoding (Init QoS and QoSLink), so the identity is the
  seven bits, not the id.
- **L2**, refusing what the protocol refuses: an unknown extension with M set, the reserved encoding `0b11`, an
  unknown message id. A variant must reach an arm for every tag, so a document cannot refuse one.
- **L3**, a bit range with a fixed value: a message id, which today is a value field every builder writes by hand and
  no decode checks.
- **L4**, a Z-terminated chain that is empty when its scope is: the authentication extension's method list.
- **L5**, a list that ends where the next byte names no arm: the network messages of a `Frame`, followed in the same
  batch by the next transport message.
- **L6**, a value constraint that is not a layout: a zero budget, an all-zero timestamp id, a zero length.
- **L7**, a list of a constant count: the eight priority pairs of the Join QoS body.

SCE's own tree shows L2 as well. `tests/forge/resources/codec_zenoh_network_envelope.scxml` routes every network
message id outside 0x19 to 0x1f to its OAM arm through `<sce:default>`, and its comment records that upstream refuses
such an id: the catch-all exists because the language has no refusal.

### What the census says of SCE, checked

Each statement the census makes about SCE was read against the tree at `eaec7ae62e` (main moved from `7e396e7ecc`
by two commits that touch only `.claude/hooks/`, so every line it cites still holds). The xsd is
`schemas/sce-forge-ext.xsd`, the language SCE_FORGE.md §4.6.

Held as stated:

- the predicate grammar is `carrier.flag`, `<chain>.has(v)`, a bare flag input, a leading `!` on one clause, and `||`;
  there is no `&&` (xsd `sce:present-if`; SCE_FORGE.md §4.6.2, "`!` negates one clause, and the grammar has no `&&`");
- a flag input is one bit wide: the parser refuses another width ("v1 fixes flag-input width at 1", `parser.rs`);
- `<sce:flag value=>` seeds the carrier's `Default` and is not checked on decode (xsd; ADR 0072 §2);
- flags of one carrier may not overlap, so a flag over the seven identity bits cannot sit beside `ext_id`, `M` and
  `enc`;
- a variant's tag is a whole own field, a multi-bit flag of one (`CodecVariant::tag_flag`), a peek-byte flag
  (`uint8` only), or the caller's (no `tag=`); without `<sce:default>` the arms must cover the tag's domain
  (`parse_codec_variant`, `codec/variant-arm-unreachable`), and the runtimes have no unknown-tag error for that reason
  (`backends/rust/forge-runtime/src/codec.rs`, the `CodecError` doc; the Python runtime says the same);
- a chain closed by an entry flag holds one entry or more (SCE_FORGE.md §4.6.2; ADR 0065 R7, `Bounded<L, MAX, 1>`);
- a `<sce:repeat>` ends on a count field or at the end of its scope (`until-eof`);
- the encode plan finds a chain's continuation flag among the entry codec's own fields (`encode_plan.rs`,
  `list_write`), so a peek-byte variant, which has none, cannot be a chain's entry that ends the chain;
- ADR 0058 §1 derives `sce:length-from`, and the xsd's "author-trust on encode" on it is stale.

Corrected:

1. **Dispatch at a variant arm.** The census (§6.2.1, §10) says it "found no statement" that `<sce:variant-dispatch>`
   applies where the import is a variant arm, and leaves it open. The build states it, and refuses it: an arm or a
   `<sce:default>` whose body is a caller-tag variant is `codec/variant-arm-body-caller-tag-unsupported`
   (`validate_cross_codec_variant_arm_not_caller_tag` in `generator.rs`), with or without a dispatch on the import.
   The dispatch is read only where the import is embedded (`validate_cross_codec_variant_dispatch` and
   `collect_parent_tag_derivations` walk `Embed` fields only). L1 is blocked, and more firmly than the census says.
2. **One document for a strict and a lenient build "the way an arm's build feature already does"** (§6.2.2). An arm's
   feature leaves the arm out of a build only on Rust, C and C++. Go, Kotlin and Python keep every arm in every build
   (ADR 0072 §3), and Swift refuses a featured arm by name (`swift_codec::variant_unlowered`). The same holds for a
   featured refusal (§2.4 below): the lenient build is reachable on Rust, C and C++.
3. **`<sce:flag value=>` is "a constant for the carrier's `Default` only"** (§3). On decode, yes. The build also holds
   it against a parent's peek-byte arm: an arm's body must declare `value=` on the flag the peek reads, equal to the
   arm's tag (`codec/variant-arm-inner-mid-undeclared`, `codec/variant-arm-mid-mismatch`). A fixed value (§3 below)
   satisfies both rules.
4. **P1 with P2.** The census proposes the masked tag (P1) and the dispatch at an arm (P2) together, with the default
   arm dispatched on `header.enc`. That flag lies inside the P1 tag, so two declarations would derive it: the tag held
   by the default arm, and the arm the dispatched body holds. The census does not say which one does. §1.2 decides it.
5. **"The exhaustiveness rule does not apply to such an element"** (P6). The element of the list is another
   document, and its own parse applies the rule today (`parse_codec_variant`). The census does not say where the
   exception is decided. §5.2 decides it.
6. The census cites `generator.rs:17118` for the dispatch validator. That line is inside the validator's doc comment;
   the function is `validate_cross_codec_variant_dispatch`, at line 17136.
7. Not a census claim, but read by its verdicts across backends: the xsd's `<sce:tlv-chain>` banner still says a
   chain is MCU-class, generated only for Rust and C11. Every backend generates one (`tests/forge/expected/` holds
   chain goldens for C, C++, Go, Kotlin, Python and Rust; Swift lowers one, `swift_codec::chain_unlowered`). The
   comment is stale, and the xsd step of this record corrects it with the `sce:length-from` one.

Not established by the census and not by this reading: whether a peek-byte variant can be the element of a
`<sce:repeat>` today. No rule refuses it and no fixture holds one; step 4 of "Landing order" adds the fixture.

## The textbook structure

A codec document is a data description in the sense of DFDL (OGF GFD.207, *Data Format Description Language*),
Kaitai Struct and ASN.1's abstract syntax with its encoding rules (ITU-T X.680, X.690): a grammar of the wire, from
which a parser and a printer are derived. Each block the census found is a construct those languages have and SCE
does not:

- **A choice keyed by a computed value.** DFDL chooses a branch by `dfdl:choiceDispatchKey`, an expression over what
  was read; Kaitai's `switch-on` takes any expression, a mask included (`switch-on: header & 0x7f`), with `_` as the
  default case. The key is a function of the field, not the field: SCE's `entry-id-except` already computes such a key
  for a chain. A key that is the field with named flags cleared is the masked key, stated by the flags it leaves out
  so that the mask is not written twice (§1.1).
- **Refusal as part of the grammar.** A grammar's language is the set of strings it accepts, and a parser refuses the
  rest. DFDL states what is not a valid instance with `dfdl:assert` and `dfdl:discriminator`; Kaitai with `valid:`
  (`eq`, `min`, `max`, `any-of`, `expr`), raising a typed validation error; ASN.1 distinguishes an extensible type
  (`...`, unknown additions accepted) from one that is not. A variant whose every tag must reach an arm cannot state a
  protocol's language: what it refuses is part of it. The tag space is then a partition into named arms, refused
  tags and the catch-all, and exhaustiveness is the statement that the three cover it (§2).
- **A fixed value is a derived element.** Kaitai's `contents:` (a magic signature) and XML Schema's `fixed` on an
  element, which DFDL validates, state a value the parser checks and the printer writes. It is the limit case of
  ADR 0058's derived element: determined by the declaration alone, so no value presents it (§3). X.680's single-value
  subtype constraint is the same statement on a type.
- **Value constraints are subtypes.** ASN.1's value-range subtype `INTEGER (1..MAX)` and Kaitai's `valid: {min, max}`
  restrict a value type without changing its layout. They refuse on decode, and an encoder over the unrestricted type
  refuses too, until the restriction is in the value's type (ADR 0065 §6's refinement program) (§6).
- **A list ended by lookahead is an LL(1) sequence.** A peek-byte variant is an LL(1) choice: one byte of lookahead
  selects the alternative (Aho, Lam, Sethi and Ullman, *Compilers*, 2nd ed., §4.4). The Kleene star of such a choice
  ends where the next byte is not in the choice's FIRST set, and it is deterministic exactly when that FIRST set and
  the FOLLOW set of the list (what can come after it) are disjoint. DFDL's `dfdl:occursCountKind="parsed"` and the
  upstream decoder reach the same end by speculation and rewind; the LL(1) statement needs neither, and the build can
  check the disjointness (§5).
- **A gate is a proposition.** A field present under a condition is the guarded alternative of a grammar; the
  condition is a propositional formula over what was read. A closed boolean algebra over the atoms SCE already reads
  (flags, inputs, chain membership) and equality of a multi-bit value against a constant keeps every gate analysable:
  its satisfying assignments are finite, which is what ADR 0065 R4's gate groups enumerate (§1.3).

## Decision

### 1. Dispatch (L1)

#### 1.1 A tag is a whole carrier with named flags cleared

`<sce:variant tag="header" tag-except="header.Z">` dispatches on the field `header` with the bits of the flags
`tag-except` names cleared. The attribute has the grammar of `entry-id-except` (SCE_FORGE.md §4.6.2): a
whitespace-separated list of `<field>.<flag>`, each a flag of the field `tag` names. The bits are read from the field's
own flag layout and are resolved by the same function that resolves `entry-id-except`, shared and not copied.

- `tag` names a whole own field that is a `<sce:flags>` carrier: not a flag (`header.ext_id`), not a peek-byte flag,
  not absent (caller-tag).
- The discriminant is `header & ~E`, `E` the union of the excepted flags' bits. The domain is the values of the
  carrier's width with every bit of `E` clear, `2^(w - |E|)` of them, and the exhaustiveness rule (§2.2) reads that
  domain.
- An arm's `value`, a refusal's `mask` and `value`, and an `<sce:arm default="true">`'s value have no bit of `E` set;
  one that does is refused with the rule and the value without the bit, as `entry-id-except` refuses a `has(..)`
  value.
- **The excepted flags keep their roles.** A flag a chain ends on (`entry-flag-name`) is excepted, so the chain still
  ends on `header.Z` and an entry's identity does not depend on where in the chain it stands. A chain whose entry
  codec is such a variant and that ends on a flag the tag covers would change an entry's identity with its place, and
  is refused (`codec/constraint-on-derived-element`, §9).
- **The covered flags are derived from the arm** (ADR 0058 §1, an own-field variant's tag). A known arm writes its
  value into the covered bits; the default arm writes the tag it holds. No value presents a covered flag. A one-flag
  gate (`header.M`) on a covered flag is a check of the gate against the tag, not a second derivation (ADR 0058 §1:
  two kinds of declaration that name one element are not both derivations). `fixed` (§3) on a covered flag is
  `codec/constraint-on-derived-element`, and `value=` on one is `validation/attribute-rule-violated`: the arm states
  those bits, and the `default="true"` arm's value is their `Default`.

A variant without `tag-except` dispatches on its whole field, as today.

#### 1.2 A dispatch on an import holds wherever the import is used

`<sce:variant-dispatch flag="carrier.flag"/>` on an `<sce:import>` applies to every use of that import in the importing
codec: an `<sce:embed>`, an `<sce:arm>` and the `<sce:default>`. Two uses that need different flags import the same
document under two aliases. A list or a chain of a caller-tag codec is not changed by this record.

- One validator judges every use, with the codes the embed has today: the carrier and the flag exist on the importing
  codec (`codec/variant-dispatch-flag-not-resolved`), the flag declares no `value=`
  (`codec/variant-dispatch-flag-has-static-value`), its width holds the body's arms
  (`codec/variant-dispatch-bit-width-mismatch`), and the carrier is declared before the first use
  (`codec/variant-dispatch-carrier-after-embed`; a variant is the codec's suffix, so every carrier precedes an arm). A
  caller-tag body with no dispatch and no `default="true"` arm is
  `codec/variant-dispatch-arms-not-distinguishable-without-default` at an arm as at an embed.
- `codec/variant-arm-body-caller-tag-unsupported` is retired: its refusal is replaced by the rule above.
- **A dispatched flag outside the variant's own tag** is derived as an embed's is today: the encode sets the carrier's
  flag from the arm the body holds, the arm's tag masked to the flag's bits, or the tag the body's catch-all holds.
- **A dispatched flag inside the variant's own tag** (the census's `header.enc` under `tag="header"
  tag-except="header.Z"`) is not a second derivation. Its bits are a function of the tag. The decode hands the body
  `(header >> bit) & mask`. The encode derives nothing from the body: the body's arm is held to the one the tag's bits
  name.
  - For a known arm the tag is a constant, so the arm the body holds is known at build. The arm's body is that one
    arm's type, and a known arm whose bits name no arm of its body (and whose body has no catch-all) is
    `codec/variant-dispatch-arm-tag-unmatched`.
  - For the default arm the tag is the value it holds. The default arm's value is the pair of its tag and its body,
    built by one checked constructor that refuses a body whose arm the tag's bits do not name, as ADR 0065 R3's
    `{Codec}UnknownTag::new` refuses a tag an arm names. It is a refinement, not an encode refusal (ADR 0081 §1.2 says
    how each backend holds one).
  - The body's caller-tag variant must cover the values the flag takes in the tags that reach that use. For the
    default arm these are the tags no arm names and no refusal in force covers; the check is made in the world where
    no refusal's feature is on, whose set holds every other world's (ADR 0057 §1, monotonicity). A value it does not
    cover is `codec/variant-dispatch-arm-tag-unmatched`.

#### 1.3 A predicate is a formula

`sce:present-if` takes a boolean algebra:

```
predicate  := disjunct ( "||" disjunct )*
disjunct   := unary ( "&&" unary )*
unary      := "!" unary | primary
primary    := "(" predicate ")" | comparison | atom
comparison := operand ( "==" | "!=" ) integer
atom       := carrier "." flag | input | chain ".has(" integer ")"
operand    := carrier "." flag | input
```

- `!` binds tightest, then a comparison, then `&&`, then `||`. Integers are decimal, `0x` or `0b`, as elsewhere in a
  codec. Whitespace between tokens is not significant.
- An atom that is a flag or an input is a truth value only when it is one bit wide; a multi-bit flag or input is read
  only through a comparison, a one-bit one never through a comparison (one spelling for one test), and the constant of
  a comparison fits the operand's width. Each is `codec/predicate-operand-width`.
- Every predicate the old grammar accepts (a disjunction of clauses, each optionally negated) parses to a formula of
  the same truth table.
- Two predicates are the same predicate (the co-gating rule of a gated list and its count) when they have the same
  truth table over the atoms they read, as they are equal "by meaning" today.
- **Roles on encode** (ADR 0058 §1, ADR 0065 R4):
  - the one-flag form `c.f` or `!c.f` stays a derivation of the flag from the presence;
  - a formula whose atoms are all one-bit flags of the codec's own carriers that no other gate reads is a gate group
    whose arms are the formula's satisfying assignments of those flags, each arm named after the flags it holds set.
    This is R4c, widened from a disjunction to any formula; the flags are derived from the arm;
  - every other formula (a comparison, an input, a chain's membership, a flag another gate reads) is a check: the
    encode refuses a presence that disagrees with it (`PresentIfMismatch`), and the codec declares
    `sce:encode-may-fail="true"`, as SCE_FORGE.md already requires for a disjunction over inputs and a flag wider than
    one bit.

#### 1.4 A flag input may be wider than one bit

`<sce:flag-input name="enc" width="2"/>`: `width` is 1 to 64.

- The input is a parameter of `decode` and `encode` of the smallest unsigned type that holds the width, the type a
  multi-bit flag's accessor already returns on each backend. Its value is its low `width` bits, as a one-bit input's
  is today.
- The parent binds it as it binds a one-bit one: `<sce:flag-bind input="enc" source="header.enc"/>` names a flag of a
  carrier of the parent, or a flag input of the parent, of the same width (`codec/flag-bind-width-mismatch`).
- `<sce:entry-flag-bind>` stays one bit: adjacency is a truth value.
- A multi-bit input is read in a predicate through a comparison (§1.3), and is said by a value (ADR 0065 R4a) only
  through a body or an arm that it is handed to and that says it, never through a gate.

#### 1.5 The extension entry, written whole

The entry codec of a network message's extension chain, with the typed bodies the census needs, the unknown identities
read by their encoding, and the refusals of §2:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="little" name="push_ext_entry">
  <sce:import src="ext_qos.scxml" kind="codec" as="qos"/>
  <sce:import src="ext_timestamp.scxml" kind="codec" as="timestamp"/>
  <sce:import src="ext_node_id.scxml" kind="codec" as="node_id"/>
  <sce:import src="ext_body_by_enc.scxml" kind="codec" as="unknown_body">
    <sce:variant-dispatch flag="header.enc"/>
  </sce:import>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="ext_id" bit="0" width="4"/>
      <sce:flag name="M" bit="4"/>
      <sce:flag name="enc" bit="5" width="2"/>
      <sce:flag name="Z" bit="7"/>
    </sce:flags>
    <sce:variant tag="header" tag-except="header.Z">
      <sce:arm value="0x21" type="qos" default="true"/>   <!-- z64, id 1 -->
      <sce:arm value="0x42" type="timestamp"/>            <!-- zbuf, id 2 -->
      <sce:arm value="0x33" type="node_id"/>              <!-- z64, mandatory, id 3 -->
      <sce:refuse mask="0x60" value="0x60"/>              <!-- the reserved encoding -->
      <sce:refuse mask="0x10" value="0x10" feature="strict-extensions"/>
      <sce:default type="unknown_body"/>
    </sce:variant>
  </datamodel>
</scxml>
```

`ext_body_by_enc` is a caller-tag variant with an arm for each of the encodings unit, z64 and zbuf. The reserved
encoding never reaches it: the first refusal is in force in every world, so its arms cover what the default arm can
be handed (§1.2). The message's chain is `<sce:tlv-chain type="push_ext_entry" terminate-on="entry-flag"
entry-flag-name="Z" entry-id="header" entry-id-except="header.Z">`, unchanged.

### 2. Refusal (L2)

#### 2.1 The element

`<sce:refuse mask="M" value="V" [feature="name"]/>`, zero or more, inside a `<sce:variant>` after its `<sce:arm>`
children and before its `<sce:default>`, on every kind of tag: an own field (masked by §1.1 or not), a flag, a
peek-byte flag, the caller's.

- `M` and `V` are integers inside the tag's domain (for a caller-tag variant, the width of the flag the importing
  codec dispatches on, checked there). `feature` is an arm's feature name (ADR 0072, `[a-z][a-z0-9-]*`).
- A refusal covers the tags `t` with `t & M == V`. A refusal that covers no tag outside the arms is
  `codec/variant-refuse-unreachable`; a `V` with a bit outside `M` covers none.

#### 2.2 Decode

A decode reads the tag `t` and, in order:

1. an arm names `t`: that arm, or `VariantArmNotBuilt` where the arm's feature is off (ADR 0072);
2. a refusal in force covers `t`: the frame is refused with `TagRefused`, which carries the ordinal of the first such
   refusal among the variant's `<sce:refuse>` children in document order, from 0 (at most 256 per variant);
3. otherwise the `<sce:default>`.

A refusal is in force in every build where it names no feature, and where its feature is on where it names one.

**Exhaustiveness is a partition.** Without a `<sce:default>`, the arms and the refusals that name no feature cover the
domain (`codec/variant-arm-unreachable`, as today, its message naming the tags left). A refusal that names a feature
does not count: in a world where its feature is off its tags need a destination, so a variant with such a refusal and
no default is refused by the same code, naming the world. With a `<sce:default>`, where the arms and the refusals that
name no feature already cover the domain, the default reaches no tag and is `codec/variant-default-unreachable`.

#### 2.3 Encode: no value holds a refused tag

A known arm's tag is never refused (arms come first). The default arm's tag is ADR 0065 R3's refinement, the tags
inside the tag's bits that no arm names; it now also excludes every tag a refusal in force covers. Its set therefore
depends on the build: on Rust, `{Codec}UnknownTag::new(tag)` returns `None` for a covered tag, and its `TagSpace` is
one constant per world, written under the feature's `cfg`, so `UnknownTag::from_unknown`'s inclusion is still a
constant decided where the call is compiled. A refused tag is not an encode refusal, and no codec's
`sce:encode-may-fail` moves for one. A backend holds the default arm's tag as ADR 0081 §1.2 prints R3 for it; one
that still holds it as a number writes what it holds, as it does today for a known arm's tag in the default arm, the
defect R3 names, until it prints R3.

#### 2.4 One document, a strict and a lenient build

`feature=` gates a refusal as it gates an arm (ADR 0072): the refusal is an element present in a world where its
feature is on (ADR 0057 §1), and the default is on, so a build that names no feature refuses. A participant keeps the
default; an analyzer that must show what a participant refuses turns the feature off and receives those tags in its
`<sce:default>`, from the same document.

- Rust: `#[cfg(feature = "name")]` on the refusal's match arm, and on the term of `TagSpace` it removes.
- C and C++: the refusal's case under `#if SCE_FEATURE_<NAME>`, with the macro defined to 1 unless the build defined
  it, as an arm's is.
- Go, Kotlin, Python and Swift keep the refusal in every build, the default meaning, as Go, Kotlin and Python keep a
  featured arm (ADR 0072 §3). A lenient build is not reachable on them; each template says so where it loops over the
  refusals.

The feature narrows what decodes where an arm's feature widens it. Under Cargo's union of features (ADR 0053), a
crate built once for a participant and an analyzer is strict for both, so the two keep separate builds of the codec
crate. ADR 0057 §1 allows only this direction: a negated condition, a refusal present where a feature is off, is not
in the model.

### 3. A fixed value (L3)

`fixed="0x1D"` on a `<sce:flag>`, or on a whole field (`<data>` or `<sce:field>`) of an unsigned integer or enum
type, of any `sce:bit-size` that is a number of bits or `vle`.

- The encode writes the value; the decode reads the element and refuses any other value with `FixedValueMismatch`.
- The value has no member for it on any backend: not in the source interface, the builder, the view, the owned
  mirror, the parts or a struct. It is a derived element in ADR 0058's sense, derived from the declaration alone, and
  neither a refusal nor a check of the encode.
- The value fits the element's width; an enum's is one of its variants (`validation/attribute-rule-violated`).
- `fixed` and `value=` on one flag are two statements of one constant and are refused
  (`validation/attribute-rule-violated`). `value=` keeps its meaning: it seeds `Default` and is not checked.
- An element another declaration derives (a length or count field, a variant's tag or a flag it covers, a dispatched
  flag, a chain's continuation flag, the flag of a one-flag gate) takes no `fixed`, and no predicate reads a fixed
  flag, whose truth would be a constant of the document: `codec/constraint-on-derived-element`.
- A peek-byte parent's arm whose body fixes the flag the peek reads: the fixed value satisfies
  `codec/variant-arm-inner-mid-undeclared`, and `codec/variant-arm-mid-mismatch` holds it equal to the arm's tag, as it
  holds `value=` today.

A fixed byte string (Kaitai's `contents:`) is not decided here; no consumer needs one.

### 4. A chain that may be empty at the end of its scope (L4)

`<sce:tlv-chain terminate-on="entry-flag" empty-at-end="true">`: where the chain's scope ends at the byte the chain
starts at, the chain holds no entry. The scope is what the codec reads: the frame, or the bytes a parent's
`sce:length-from` gives it.

- Anywhere else the chain reads as today: one entry or more, ending on an entry whose flag is clear, and a scope that
  ends after an entry whose flag is set is a frame cut short.
- Only on `terminate-on="entry-flag"` (a chain that ends with its frame may already be empty), refused elsewhere
  (`validation/attribute-rule-violated`). The chain is the last element of its codec, so "the scope ends where the
  chain starts" has one meaning; a chain followed by another element is `codec/chain-empty-at-end-not-last`.
- The encode writes no byte for an empty list. ADR 0065 R7's list type has a fewest of 0 for such a chain
  (`Bounded<L, MAX>` on Rust, not `Bounded<L, MAX, 1>`), and its view holds a `ListView` that may be empty.
- An empty chain holds no entry, so `has(..)` over it is false; a chain gated off holds none either, as today.

### 5. A list ended by lookahead (L5)

#### 5.1 The element

`<sce:repeat id="messages" type="network_message" until="unmatched" max-count="N"/>`. `until` takes the one value
`unmatched` and excludes `count` and `until-eof`; `max-count` is required, as on every list (ADR 0077 §5). The element
is an imported codec whose `<sce:variant>` dispatches on a peek-byte flag and has no `<sce:default>`
(`codec/repeat-until-unmatched-element` otherwise).

Before each element the decode peeks one byte, consuming nothing:

1. the scope has ended: the list ends;
2. an arm of the element names the byte's tag: the element is read (or refused with `VariantArmNotBuilt`); past
   `max-count` such an element is refused with `TooManyElements`, before it is read (ADR 0077 §2);
3. a refusal of the element in force covers the tag: the frame is refused with `TagRefused`. A byte the element's
   grammar refuses is a malformed element, not the start of what follows;
4. otherwise the list ends before that byte, which is left for what follows.

The list may be empty. Its view (ADR 0049) walks with the same rule.

#### 5.2 Where exhaustiveness is decided

The element is another document, whose own parse applies the exhaustiveness rule today. The rule for a peek-byte
variant with no `<sce:default>` moves from its parse to its uses, as the rule for a caller-tag variant already sits
with its importer:

- as an embed, an arm, a counted or `until-eof` list's element, or a chain's entry, it is judged where it is used,
  and a variant that leaves tags with no arm and no refusal is `codec/variant-arm-unreachable` there, naming the use;
- as the element of an `until="unmatched"` list it is not: there a tag no arm names is where the list ends.

The codec's own decode, which the list calls, answers a tag no arm names and no refusal covers with `TagUnmatched`,
having consumed nothing. That is the one outcome the list reads as its end, so the list does not test the arms a
second time. It reaches a host only through a direct call of that codec's decode. Its encode is unchanged: a value
holds one of the arms.

#### 5.3 The list is deterministic

The build checks that the list's FIRST set (the tags the element's arms name and its refusals cover) and its FOLLOW
set (the tags that can begin what comes next) are disjoint, where the next element is known: a peek-byte variant of the
same codec (its arms' tags), a field with a `fixed` value at its first byte, or, where the list is the last element of
its codec, the next element of the importing codec, judged where the import is resolved. An overlap is
`codec/repeat-until-unmatched-ambiguous`, naming the tags in both. Where the next element is not known (a field of any
value, the end of a codec decoded by its host) nothing is checked, and the conformance law of ADR 0058 §5 (a decoded
frame encodes to itself) holds for the frames the decode produces.

For the census's `Frame`: the network ids are 0x19 to 0x1f and the transport ids 0x00 to 0x07, read from the same five
bits, so the batch's next transport message ends the list.

### 6. Value constraints (L6, L7)

- **`sce:range="min..max"`** on an integer field, a `<sce:flags>` carrier included, of any `sce:bit-size`. Both ends
  are inclusive; either may be left out (`1..` is "at least 1", `..100` "at most 100"), as ASN.1's `MIN` and `MAX`.
  Integers are decimal, `0x` or `0b`, negative on a signed type. Refused (`validation/attribute-rule-violated`): a
  malformed range, `min > max`, an end outside the type, a range that admits every value of the type. Not on an enum
  (its set is closed already), a `bool`, a real, or a derived element (`codec/constraint-on-derived-element`); not
  beside `fixed`.
- **`sce:nonzero="true"`** on a byte string (`tail`, `length-ref` or a fixed `sce:length`): the value has a byte that
  is not zero, so an all-zero string and the empty one are refused.
- **A literal count**: `<sce:repeat count="8">`, a positive integer in place of a count field. The list holds exactly
  that many entries; no `max-count` is written beside it (a second statement of the bound,
  `validation/attribute-rule-violated`). Its type is ADR 0065 R2's bounded list with a fewest equal to its most: an
  array where the backend has one (`[E; 8]` on Rust, `std::array` on C++, an array member on C11, `[8]E` on Go), a list
  made by a checked constructor elsewhere (ADR 0081 §2). The decode reads exactly that many; the encode is total.

A decode refuses a value outside its range with `ValueOutOfRange` and a byte string `sce:nonzero` refuses with
`BytesAllZero`, judged when the element is read. The encode refuses the same values with the same errors, before the
first byte is written. Those are two new places an encode refuses (ADR 0065 §1), so a codec that holds either states
`sce:encode-may-fail="true"`, and one that does not is `codec/undeclared-encode-refusal`. Taking them into the value's
type (a non-zero integer, a ranged one) is a step of ADR 0065 §6's refinement program, not of this record; where it
lands, the declaration it makes untrue is removed with it.

### 7. One statement of what a decode refuses and where a list ends

`sce-build/src/forge/codec_constraints.rs` (new) is the one place that turns these declarations into sets and
predicates, beside `bit_layout.rs` (ADR 0095) and read as it is:

- the tag partition of each variant: the discriminant's mask and domain (§1.1), the named arms, each refusal with its
  ordinal and feature, the default arm's admitted set per world (§2.3), and the bits a dispatched flag inside the tag
  takes (§1.2);
- the stop rule of an `until="unmatched"` list and its FIRST and FOLLOW sets (§5);
- each element's fixed value, range and non-zero rule (§3, §6);
- whether a chain may be empty (§4).

The parser and validators, `codec_roles` (a fixed element and a covered flag are derived; an excepted flag keeps its
role), `codec_totality` (the refusal kinds `ValueOutOfRange` and `BytesAllZero`; a formula's check), the encode plan
of ADR 0081 (the derivations and refinements above), every backend's decode and every printer read it. A backend that
computes a mask, a set or a stop rule from the model is a defect, as ADR 0081 §1 says of the plan. The plan's checks
gain no kind of their own beyond `ValueOutOfRange` and `BytesAllZero`; the gate checks reuse `PresentIfMismatch`.

### 8. The runtimes' errors

| Error | Raised | Rust `CodecError` | C status | C++ | Go | Kotlin `CodecError` | Python | Swift `CodecError` |
|---|---|---|---|---|---|---|---|---|
| a refused tag | decode | `TagRefused { rule: u8 }` | `SCE_FORGE_CODEC_TAG_REFUSED`, the ordinal in `cursor->refused_rule` | `CodecError::TagRefused` on the cursor, the ordinal in `last_refused_rule()` | `*codec.TagRefusedError{Rule}`, `errors.Is(err, codec.ErrTagRefused)` | `TagRefused(rule: Int)` on `SceCursor.lastRefusal` | `TagRefused(CodecError)` with `rule` | `.tagRefused(rule: UInt8)` |
| a tag no arm names, where a list ends | decode | `TagUnmatched` | `SCE_FORGE_CODEC_TAG_UNMATCHED` | `CodecError::TagUnmatched` | `codec.ErrTagUnmatched` | `TagUnmatched` | `TagUnmatched` | `.tagUnmatched` |
| a fixed value differs | decode | `FixedValueMismatch` | `SCE_FORGE_CODEC_FIXED_VALUE_MISMATCH` | `CodecError::FixedValueMismatch` | `codec.ErrFixedValueMismatch` | `FixedValueMismatch` | `FixedValueMismatch` | `.fixedValueMismatch` |
| a value outside its range | decode and encode | `ValueOutOfRange` | `SCE_FORGE_CODEC_VALUE_OUT_OF_RANGE` | `CodecError::ValueOutOfRange` | `codec.ErrValueOutOfRange` | `ValueOutOfRange` | `ValueOutOfRange` | `.valueOutOfRange` |
| an all-zero byte string | decode and encode | `BytesAllZero` | `SCE_FORGE_CODEC_BYTES_ALL_ZERO` | `CodecError::BytesAllZero` | `codec.ErrBytesAllZero` | `BytesAllZero` | `BytesAllZero` | `.bytesAllZero` |

- The C statuses take the next free numbers after `SCE_FORGE_CODEC_CAPACITY_EXCEEDED = 22`, in the order of the
  table. C++ and Kotlin return their one sentinel and record the reason on the cursor, as they do for
  `VariantArmNotBuilt` and `LengthOutOfRange` (ADR 0077 §1).
- Python's decode answers every refusal with `None`, as it does today, so a Python caller does not see which refusal
  it was. Giving Python a channel for the reason is not this record's.
- The refusal carries the rule's ordinal, not its mask and value: Rust's `CodecError` grows from one byte to two, and
  the codec states each ordinal's rule in a generated table (`{Codec}::REFUSALS` on Rust, its spelling elsewhere).
- Each runtime declares an error only with the feature that raises it, so no error is one nothing returns (the rule
  the Swift runtime states).

### 9. Diagnostics

Each new code has a golden, a spec anchor naming SCE_FORGE.md §4.6 and this record's section, a row in
docs/SCE_ACCEPTED_SUBSET.md's appendix (`acceptance_doc_covers_every_code`), SCE_ERROR_CONTRACT.md §5's catalog and
`schemas/sce-diagnostic.v1.schema.json`'s enum. None carries a fix: each repair is the author's choice. All are in the
`validation` stage.

| Code | Refuses | Section |
|---|---|---|
| `codec/variant-refuse-unreachable` | a `<sce:refuse>` that covers no tag outside the arms, naming the rule and why (`V` outside `M`, or every covered tag named) | 2.1 |
| `codec/variant-default-unreachable` | a `<sce:default>` that no tag reaches, the arms and the refusals that name no feature covering the domain | 2.2 |
| `codec/variant-dispatch-arm-tag-unmatched` | a dispatched flag inside the tag whose bits, in a known arm's tag or in a tag that reaches the default arm, name no arm of the body | 1.2 |
| `codec/constraint-on-derived-element` | `fixed` or `sce:range` on an element another declaration derives, a predicate that reads a fixed flag, or a chain that ends on a flag its entry's tag covers, naming the other declaration | 1.1, 3, 6 |
| `codec/predicate-operand-width` | a multi-bit flag or input read as a truth value, a one-bit one compared, or a constant past the operand's width | 1.3 |
| `codec/chain-empty-at-end-not-last` | `empty-at-end` on a chain followed by another element of its codec | 4 |
| `codec/repeat-until-unmatched-element` | `until="unmatched"` over an element that is not a peek-byte variant without `<sce:default>` | 5.1 |
| `codec/repeat-until-unmatched-ambiguous` | an `until="unmatched"` list whose FIRST and FOLLOW sets share a tag, naming the tags | 5.3 |

Retired: `codec/variant-arm-body-caller-tag-unsupported` (§1.2), with its appendix and catalog rows. The diagnostic
schema is `pre-release`, so a code may be removed (SCE_ERROR_CONTRACT.md §8.1). Removed without a code: the
one-bit lock on a flag input's width.

Existing codes keep their classes: a malformed or out-of-width `mask`, `value`, `fixed`, `sce:range` or count, an
attribute on an element it does not apply to, and a bit of `tag-except` named in a value are
`validation/attribute-rule-violated`, as the same refusals of `entry-id-except` are; a flag `tag-except` names that the
field lacks is the refusal `entry-id-except` gives the same mistake; a non-exhaustive variant (at its parse or at a use,
§5.2) is `codec/variant-arm-unreachable`; a feature name is refused as an arm's is; a construct a backend does not
print yet is `generate/unsupported-feature` on that backend, by name, never generated wrong.

### 10. Wire surfaces (SCE_WIRE_CONTRACTS.md)

No status flips; every surface stays `pre-release`.

- **Authoring grammar** (`schemas/sce-forge-ext.xsd`): `tag-except` on `<sce:variant>`; `<sce:refuse>` (`mask`,
  `value` required, `feature`) between the arms and `<sce:default>`; `fixed` on `<sce:flag>` and as `sce:fixed` on a
  field; `empty-at-end` on `<sce:tlv-chain>`; `until` on `<sce:repeat>`, and `count` admitting a literal;
  `sce:range` and `sce:nonzero`; `<sce:flag-input width>` 1 to 64. The xsd's comments on `sce:length-from`
  ("author-trust on encode") and on the chain's backends (MCU-class) are corrected in the same step.
- **Forge AST export** (`apis/forge-ast.v1.schema.json`): the new attributes and elements, additively, each skipped
  when absent so a document that does not use it keeps its AST; and the predicate as a tree (`atom`, `not`, `and`,
  `or`, `cmp`) in place of the clause with its `or_with` tail. That second change is not additive: the AST of every
  document that holds a gate moves once, in the step that lands it, and its meaning is held by §11.
- **Diagnostics**: eight codes added, one retired (§9).

### 11. No existing document changes meaning

Every element is opt-in: a document that uses none of them has the model, the plan, the AST meaning, the generated
code and the bytes it has today. The predicate grammar is a superset whose old sentences keep their truth tables. The
exhaustiveness rule, read as a partition, is the old rule where a variant has no refusal. The rule moved to the uses
(§5.2) admits documents the old rule refused, and refuses none it admitted. `value=` keeps its meaning (§3). Held by:

- every golden under `tests/forge/expected/`, and every committed generated tree that holds a codec, unchanged by the
  grammar, model and plan steps (steps 1 to 3 of "Landing order"); the backend steps add goldens for new fixtures and
  move none;
- `sce-build/tests/codec_roles.rs` and `sce-build/tests/codec_totality.rs`, whose tables over every codec in the tree
  do not move;
- `sce-build/tests/an_existing_predicate_denotes_what_it_did.rs` (new): for every `sce:present-if` in the tree, the
  formula's truth table over its atoms equals the clause list's under the old reading, and the co-gating judgement of
  every gated list is unchanged;
- `sce-build/tests/forge_ast_export.rs`: the AST of every fixture validates against the schema, and a document that
  uses no new element differs from its previous AST only in the predicate's spelling.

### 12. Tests on every lane

Each test fails when the rule it pins is removed; each landing checks it once against a mutant of that rule.

1. **Fixtures** (`tests/forge/resources/`, each registered in `tests/forge/conformance/fixtures.json`):
   `codec_variant_tag_except.scxml` (the entry of §1.5 with its bodies), `codec_variant_dispatch_at_arm.scxml` (a
   dispatch outside the tag, at an arm and at the default), `codec_present_if_formula.scxml` (a conjunction, a
   parenthesised negation, a comparison of a multi-bit flag, a gate group of three flags),
   `codec_flag_input_wide.scxml` with its child, `codec_variant_refuse.scxml` (a refusal in force and a featured one,
   on an own field, a peek-byte flag and a caller tag), `codec_fixed.scxml` (a fixed flag and a fixed VLE field, under
   a peek-byte parent), `codec_chain_empty_at_end.scxml` (under a parent that scopes it by `sce:length-from`),
   `codec_repeat_until_unmatched.scxml` (a list of network-like messages followed by a transport-like message),
   `codec_value_constraints.scxml` (a range, an open range on a VLE, a non-zero byte string),
   `codec_repeat_literal_count.scxml`.
2. **The independent model** (`binary_model.py`) learns each element from SCE_FORGE.md §4.6, not from the code
   (ADR 0081 §5), and writes for each fixture round-trip cases and reject vectors into `numerical_reference.json`
   (`gen_cases.py`, then `--check`): a frame per arm and per default value; a frame per refusal, with its ordinal, and
   the same frame accepted in the lenient world; a fixed value off by one bit; an empty chain at the end of its scope
   and a chain cut short after a set flag; a list ended by an unmatched byte, by the end of its scope, by a refused
   byte (refused), past `max-count`; each end of each range and one past it; an all-zero and an empty byte string.
3. **`sce-build`**: a test file per element, each holding the parser's acceptance, every refusal of §9 with its
   golden, and the classification of `codec_roles` and `codec_totality`:
   `a_variant_dispatches_on_its_tag_with_the_excepted_flags_cleared.rs`,
   `a_dispatch_on_an_import_holds_wherever_the_import_is_used.rs`,
   `a_gate_is_a_formula_of_flags_inputs_and_comparisons.rs`, `a_flag_input_is_as_wide_as_it_declares.rs`,
   `a_variant_refuses_the_tags_its_rules_cover.rs`, `a_fixed_value_is_written_checked_and_presented_by_no_one.rs`,
   `a_chain_closed_by_an_entry_flag_is_empty_where_its_scope_ends.rs`,
   `a_list_ends_before_a_byte_its_element_does_not_name.rs`, `a_value_constraint_is_refused_by_decode_and_encode.rs`;
   with §11's `an_existing_predicate_denotes_what_it_did.rs`, and a unit test of `codec_constraints.rs` per set it
   states.
4. **Each lane** runs every case of every fixture its backend prints, through the conformance runner it already has:
   Rust `backends/rust/forge-conformance/tests/`, C `backends/c/forge-runtime/tests/conformance/`, C++
   `backends/cpp/forge-runtime/tests/conformance/`, Go `backends/go/forge-runtime/round_trip/`, Kotlin
   `backends/kotlin/forge-runtime/src/jvmTest/`, Python `backends/python/forge-runtime/tests/`, Swift
   `backends/swift/tests/Tests/StaticDatamodelTests/NumericalConformanceTests.swift`. The featured refusal runs in both
   worlds on Rust (with and without the Cargo feature), C and C++ (the macro at 0 and 1). The Interpreter runs no
   codec and has no lane here.
5. **Goldens** under `tests/forge/expected/` for each new fixture on each backend that prints it.
6. **Mutation casefiles** under `sce-build/tests/mutations`: the refusals tested before the arms; the mask dropped
   from a refusal; the default arm's `TagSpace` keeping a refused tag; the fixed check removed from a decode; an empty
   chain read as one cut short; an unmatched list consuming the byte it ends on; a range's end off by one; a
   comparison's operator inverted; the dispatched bits taken from the body and not from the tag.

## Landing order

Each step is one commit series. A construct the parser accepts and a backend does not print is refused by that
backend by name until the backend's step; nothing is generated wrong in between.

1. **The predicate formula and wide flag inputs.** The model's predicate becomes a tree; the parser, the co-gating
   rule, `codec/predicate-operand-width`, the flag-input width and binding; the xsd, the AST (the predicate tree),
   SCE_FORGE.md §4.6.2 and §4.6.3; the appendix and catalog rows. Every printer spells the tree for the forms it
   prints today, byte-identically; a new form is refused by name on every backend. Tests:
   `an_existing_predicate_denotes_what_it_did.rs`, `a_gate_is_a_formula_of_flags_inputs_and_comparisons.rs` (parser
   and refusals), `a_flag_input_is_as_wide_as_it_declares.rs` (parser and binding), `forge_ast_export.rs`, the
   diagnostic goldens; every golden of `tests/forge/expected/` unchanged.
2. **The other elements' grammar and validators.** `tag-except` (sharing the resolver of `entry-id-except`), the
   dispatch at every use (one validator; the retired code), `<sce:refuse>`, `fixed`, `empty-at-end`,
   `until="unmatched"`, `sce:range`, `sce:nonzero`, the literal count; exhaustiveness as a partition and its move to
   the uses for a peek-byte variant without default; the seven new codes; the xsd with its two stale comments
   corrected; the AST; the normative text of SCE_FORGE.md §4.6, which step 4's model reads. Every backend refuses each
   new construct by name. Tests: the parser and refusal halves of the nine test files of §12 item 3; diagnostic
   goldens; `acceptance_doc_covers_every_code`; every golden unchanged.
3. **One statement and the plan.** `codec_constraints.rs`; `codec_roles` and `codec_totality`; the encode plan's
   derivations (fixed values, covered flags, dispatched bits), refinements (R3's set per world, R7's fewest, R2's exact
   count, the default arm's checked pair), the widened R4c and the new refusal kinds; the FIRST and FOLLOW check and
   `codec/repeat-until-unmatched-ambiguous`. Tests: the unit tests of `codec_constraints.rs`; the classification
   halves of §12 item 3; `codec_roles.rs` and `codec_totality.rs` unchanged over the existing tree and extended with
   rows for the new fixtures; `sce-codegen codec-roles` over the new fixtures.
4. **The model and the byte cases.** `binary_model.py` and the cases of §12 item 2, for every new fixture, before any
   backend prints one (ADR 0081 §5: a shape enters a printer with its cases). The harness schedule and each
   generator's refusal read one answer, so a lane runs a case exactly where its backend prints the fixture. Tests:
   `gen_cases.py --check`; the model's own round trip of every case.
5. **Rust.** The runtime's errors and `TagSpace` per world; decode, the printer, the view's walk, the owned mirror and
   the parts for every element; the featured refusal's `cfg`. Tests: the Rust lane over every new case in both feature
   worlds; goldens; the mutation casefiles that name Rust.
6. **C++.** The printer and the template decode; the runtime's errors and `last_refused_rule()`. The header
   `backends/cpp/forge-runtime/include/sce/forge/codec.h` is an embed input, so the step commits it with
   `embed/MANIFEST.json` and `embed/MANIFEST.inputs` emitted again. Tests: the C++ conformance lane in both macro
   worlds; goldens.
7. **C11.** The template; the statuses and the cursor's `refused_rule`. Tests: the C conformance lane in both macro
   worlds; goldens.
8. **Go.** The template; the error values and `TagRefusedError`. Tests: the Go round-trip lane; goldens.
9. **Kotlin.** The template; the `CodecError` cases. Tests: the Kotlin `jvmTest` lane; goldens.
10. **Python.** The template; the error classes. Tests: the Python lane; goldens.
11. **Swift.** `swift_codec.rs` lowers each element in the shapes it lowers today, and each one leaves
    `swift_codec::unlowered`; the runtime declares the errors as it raises them. A featured refusal is in force in
    every build. Tests: `NumericalConformanceTests.swift` over every case of a fixture Swift prints; goldens.
12. **Docs.** `docs/SCE_CODEC_CONSUMER_MIGRATION.md` (ADR 0078 §2): the predicate tree of the AST, the retired code,
    the new runtime errors and the C cursor's member; the prose of docs/SCE_ACCEPTED_SUBSET.md; this record's landed
    commits.

## Consequences

- Every zenoh 1.10.2 element the census marks L1 to L7 is writable as a codec document: an extension's typed body by
  its identity with the unknown ones read by their encoding, the refusals upstream makes, fixed message ids, the
  authentication method list, a `Frame`'s network messages, the non-zero rules and the eight Join QoS pairs. Writing
  them is the consumer's work, not SCE's.
- A default arm can no longer be the place a protocol's violations hide: where a document states a refusal, no value
  holds the tag and no decode yields it, and the strict and the lenient reading come from one document on Rust, C and
  C++.
- A message id leaves every builder: a fixed value has no member, and a top-level decode refuses a frame of another
  message.
- Every runtime's error set grows by five. Rust's `CodecError` grows from one byte to two, which can widen
  `Result<T, CodecError>` by the alignment of `T`; measured on the consumer's images when step 5 lands.
- The AST of every document with a gate changes shape once (pre-release); a consumer that reads the AST follows the
  migration guide.
- On Go, Kotlin, Python and Swift a featured refusal is always in force, and Python's decode does not say which
  refusal it was.
- A backend that has not printed ADR 0065 R3 holds the default arm's tag as a number until it does, and writes a
  refused tag if a caller puts one there.
- A strict and a lenient build of one codec crate under one Cargo build are strict; the two binaries keep separate
  builds.
- A peek-byte variant without default is judged where it is used, so a document that only an `until="unmatched"` list
  uses is admitted where it was refused.

## Alternatives rejected

- **A wide flag input alone, without the masked tag** (the census's alternative to P1 with P2): dispatch on `ext_id`,
  then hand `M` and `enc` to the arm. Upstream's identity is one value, and two extensions of one id differ by
  encoding, so the arms would be a second-level dispatch per id and the refusals of §2 would have no single tag to
  read. Each of the two is the answer to its own question, and the owner took both.
- **A mask on each arm** (`<sce:arm value mask>`, a ternary match). Arms would overlap and dispatch would depend on
  their order. Arms are exact values of one discriminant, a function from tag to body; a refusal's mask classifies
  only what the arms leave, so it cannot change which arm a tag reaches.
- **Leaving refusal to the layer above the decode.** Every consumer would write the rule again, the value type would
  admit what no peer may send, and an encode could produce it.
- **A run-time strict flag on decode** in place of `feature=`: every decode would take it, and the type of the default
  arm's tag could not follow it. A build feature makes the type follow, as ADR 0072 decided for an arm.
- **Refused values** (`sce:refuse-value="0"`, the census's P8): a list of what is refused does not state a type; a
  range states the admitted set, as ASN.1's subtype does, and `sce:nonzero` covers the byte string a range cannot.
- **Speculative decoding for L5** (DFDL's `occursCountKind="parsed"`, upstream's rewind): it buffers, its cost depends
  on how an element fails, and no build can check that it is deterministic. One byte of lookahead decides without a
  rewind, and the build checks FIRST against FOLLOW.
- **Checking `value=` on decode** in place of `fixed`: it would change the meaning of every `value=` in the tree, which
  seeds `Default` and is not checked (ADR 0072 §2).
- **Eight named embeds for L7**: they work today, and state as eight fields what the protocol states as one list.
- **The full mask and value in the error**: it would make every `Result` of a codec larger to carry what the codec's
  table already states; the ordinal names the rule.
- **An expression language for gates** (DFDL's XPath subset, Kaitai's expressions): a gate would no longer have a
  finite set of satisfying assignments over what was read, which the encode's gate groups and checks need.
