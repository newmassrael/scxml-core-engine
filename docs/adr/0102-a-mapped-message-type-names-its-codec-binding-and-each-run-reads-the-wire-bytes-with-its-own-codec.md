# ADR 0102 — A mapped message type names its codec binding, and each run reads the wire bytes with its own codec

- Status: Accepted (decided by the owner 2026-11-14), under the standing rule of 2026-10-26 and 2026-10-27: the most
  long-term correct structure, textbook in its foundations, cost ignored. The owner chose the six decisions below; this
  record states their reasoning, checks the tree they change, and decides what they leave open: what a codec message
  type is (§1.1), where a binding is declared (§1.3), whether the comparison of encoded bytes is part of C3 (§2.5), the
  log record a decoded input writes (§2.4), the TTCN-3 types of a codec's value (§4) and the order of the landings.
  Numbered 0102 because 0099 to 0104 are written in parallel.
- Date: 2026-11-14
- Amends:
  - ADR 0091 answer 1 ("C2's codecs are independent libraries"), C2 and §7: the reference set's codecs are not all
    libraries (§5), and a message type may name a binding that is neither SCE's nor the downstream's (§1.2);
  - `docs/SCE_TEST_PROFILE.md` (TP): §6.3 and §11.3 to §11.5 (a codec document is a message type), §11.8 (the
    suite's `<sce:codec-binding>`), §9.2 (rule E1, and a rule E16), §9.3 obligation 6, §9.4 (the place `import`),
    §9.5 (the types of a codec's value, the `encode` attribute, which types get codec functions, consumer modules, the
    name contract, the manifest), §12.2 and §12.4 (a codec type crosses the system adapter as octets), §12.5 (the
    record `message-detected`);
  - `docs/SCE_TTCN3_LANE.md`: §1, §2.1, §3, §8, §12.4 and §14 in the commit after this one; §4 (the runner's
    `--record-dir`) and §10 (the trace's `octets` and `outputs`) with their landings;
  - the wire surfaces `sce-test-trace.v1`, `sce-test-log.v1`, `sce-ttcn3-types-manifest.v1` and
    `sce-ttcn3-witness.v1`, all `pre-release`, and their rows of `SCE_WIRE_CONTRACTS.md`.
- Scope:
  - the parser, model and checks of the profile (`sce-build/src/forge/test_profile.rs`, `test_profile_assembly.rs`,
    `test_template_check.rs`, `sce-build/src/test_component/`), `schemas/sce-forge-ext.xsd`, two new
    `DiagnosticCode` variants with their rows (`SCE_ERROR_CONTRACT.md`, `docs/SCE_ACCEPTED_SUBSET.md`);
  - the export (`sce-build/src/ttcn3_export/`, `tools/codegen/templates/ttcn3/`) and the validator
    (`sce-ttcn3-validator/`);
  - the trace and the log in the seven test runtimes and their generators: the configuration unit's codec door, the
    trace player, the recording adapter and the runner (Rust, Kotlin, C++, Go, Swift, Python, C11), and
    `sce-codegen check-trace`;
  - the corpus under `sce-build/tests/fixtures/test_profile/`.
- Related: ADR 0010, 0014 and 0015 (the content-line codec, its line records and its enum entries); ADR 0055 (a value
  that may be absent is of an optional type); ADR 0058 (presented and derived elements); ADR 0065 (declared totality);
  ADR 0077 (a declared bound is part of a field's type); ADR 0086 and 0092 (bounded lists and unions in the type
  grammar); ADR 0091 (the test profile and its TTCN-3 export, C1 to C3); ADR 0097 (generated code is a build output);
  ADR 0100 (a positional codec dispatches on a masked tag, refuses by rule, and states fixed values, value ranges and
  where a list ends); ADR 0101 (a CBOR entry takes the whole type grammar, a union as a map of one entry, the shape of
  TP §12.5's canonical value form); ADR 0103 (a frame is walked as events printed from the plan its decode is printed
  from); ADR 0104 (the content-line codec conforms to RFC 5545 and RFC 6868, as codec-format 3).

## Context

### What a protocol conformance suite asks of the lane

A consumer writes a conformance suite for a protocol in the test profile. Its implementations under test generate
their codecs with SCE, so the suite's lower tester must not read the wire with those codecs (ADR 0091, Context: a
codec shared by the tester and the implementation hides its own faults). The consumer therefore keeps two codecs of
its protocol that share nothing with the implementations: an SCE codec document written from the protocol's encoding
requirements, which the native run uses, and a TTCN-3 types module written apart from it by another author, encoded
by Titan's RAW engine and by external functions for what RAW cannot express (variable-length integers, extension
chains ended by a flag). The suite needs three things the lane does not have:

1. **A message type whose TTCN-3 codec is the consumer's.** The lane binds a codec for a message type only through
   the downstream's reference bindings, and those cover the formats SCE reads, not a consumer's.
2. **Wire bytes in the trace, read by both sides.** The comparison the suite exists for is that two independent codecs
   read the same bytes as the same value. A trace record holds the decoded value, and the native run uses no codec,
   so nothing in the lane ever puts one set of bytes in front of both codecs.
3. **A recorded live run.** The two executors cannot be fed the same inputs live. One runs against the
   implementation; a recorder writes what crossed its system interface, at the level of bytes, and the other replays
   it. Every mapped port is recorded, the port of the test management protocol among them.

### What the tree holds, checked at `eaec7ae62e`

- **A message type is an event schema.** `<sce:message type>` names an imported `event-schema` (TP §11.3), and every
  profile document's record is one. No document of the profile names a codec document, and nothing in the grammar
  relates a message type to one (`schemas/sce-forge-ext.xsd`, `<sce:message>`).
- **The export declares codec functions for every type any port carries.** `codec` in
  `sce-build/src/ttcn3_export/types.rs` walks every `test-port-type` the suite reaches, the port types of connected
  ports included, and declares `enc_R` and `dec_R` for each message, with no `with { encode … }` attribute anywhere.
  A connected port never encodes (ADR 0091 answer 1: a `connect` does not pass through the binding), so some of those
  functions are declared for a call that never happens.
- **A trace input holds a value.** `schemas/sce-test-trace.v1.schema.json` requires `value` on every input, and the
  native trace players deliver it as a value: the native side runs no codec (TP §12.2, "matching is on values, not on
  encoded bytes"). The player records what the components send to the system ports, as values, and writes no file of
  it.
- **The lane leaves both gaps open.** `docs/SCE_TTCN3_LANE.md` §14 lists "SCE's own codecs in the lane" (only the
  TTCN-3 side encodes, so SCE's generated codecs and the reference bindings are never compared on the same bytes) and
  "Positional binary codecs" (no reference binding) as open.
- **The content-line binding no longer runs on libical.** libical 4.0.7, the library ADR 0091 answer 1 pinned, does
  not write SCE's content-line format: it strips trailing whitespace from a value, writes a backslash of an
  `X-` property's value twice, and writes RFC 6868's caret escapes in parameter values, which codec-format 2 does not
  read (ADR 0104 takes RFC 6868 into format 3). It refused 39 of the 52 encode cases of SCE's oracle
  (`codec_content_line_*` of `tests/forge/conformance/numerical_reference.json`). The downstream replaced it with a
  binding written from RFC 5545 §3.1 and `SCE_FORGE.md` §4.6.4 alone, without reading SCE's codec sources, which
  passes the oracle whole at `SCE_PIN` `eaec7ae62e`: 98 of 98 decode cases, 52 of 52 encode cases, 77 of 77
  rejects. The lane document's §1, §2.1 and §3 still name libical and its licence.
- **The binding refuses three content-line shapes by name.** An enum entry, a `sce:separator` list of values and the
  line records of a repeated property with parameters (ADR 0014, 0015) have no TTCN-3 type stated anywhere, so the
  binding has no value to read them into.

## The textbook structure

- **TTCN-3 separates a type from its encoding, and the encoding from its implementation.** A type is declared once; an
  `encode` attribute names the encoding rules its values take on the wire, and a `variant` attribute their parameters
  (ES 201 873-1 clause 27). The implementation of the rules is the test system's: TCI's codec interface (ES 201
  873-6, TCI-CD) encodes and decodes for the TTCN-3 executable. One abstract test suite runs with any codec that
  implements its encodings.
- **The system interface carries octets.** TRI's message operations (ES 201 873-5), `triSend` outward and
  `triEnqueueMsg` inward, carry the encoded message; the codec sits between the executable and the system adapter, on
  the executable's side. An adapter that handed values across would have no place for the codec, which is the one
  component the lane exists to check. (The texts of -5 and -6 were not at hand, as `docs/SCE_TTCN3_LANE.md` records;
  slice D1 checks these operations against them, as it checks the others.)
- **Two independent implementations given one input are compared on their outputs.** This is differential testing
  (McKeeman, *Differential Testing for Software*, Digital Technical Journal 10(1), 1998). Its power is bounded by how
  independent the two are (Knight and Leveson, 1986, cited by ADR 0091 C2), so the independence of each pair is
  stated, not assumed (§5).
- **An encoding is canonical or it is not.** ASN.1 has BER, which admits several encodings of one value, beside DER,
  which admits one (ITU-T X.690). RFC 8949 §4.2 defines deterministically encoded CBOR on the same principle. Two
  encoders can be compared byte for byte only where both write the canonical encoding; where the rules admit several,
  a difference is no defect of either (§2.5).
- **A data description defines its infoset.** A DFDL schema (OGF GFD.207) describes the format and, by the same
  declarations, the infoset a parse yields; ADR 0100 takes the codec document as such a description. The value of a
  codec document is therefore defined by the document, and a second statement of it elsewhere would be a second
  definition of one thing (§1.1).

## Decision

### 1. A message type at the system interface, and its binding

#### 1.1 A codec document is a message type: its value is a record, its record view

A message type of the profile is an event schema, as today, or a codec document. A codec document is named where an
event schema is: by `<sce:message type>` of a `test-port-type` (TP §11.3), by `record:<alias>` of a template's
`sce:type` (§11.4), and by the imports of a component that reads its fields (§11.5), each through
`<sce:import kind="codec">`.

The value of a codec message is the codec's **record view**: a record of the type grammar whose fields are the
codec's presented entries (ADR 0058 §1), in declaration order, each of the type §4 gives it. The build derives the
record view from the document, once, before any generator reads it, as an event-schema model and one more for each
record it holds that no document declares (a content-line line record, §4.1); every rule of the profile over a
record, its templates, its matching, its canonical value form (TP §12.5), its native readers and writers and E1,
reads the derived schema as it reads a written one. The record view's name, in the types module, the log and the
trace, is the codec's unit name, and so is the event it travels under (TP §11.3's rule that two types of one port
differ by their event names holds it). Its native unit is named `<codec unit>_message`, since the codec's own unit
holds the codec's name on every backend.

A codec whose value cannot be read from octets alone is no message type: one that takes a flag input or a caller's
tag (SCE_FORGE.md §4.6, §4.6.3), and one an entry of which has no type of the profile (§4). Each is refused by name
(`generate/unsupported-feature`), naming the entry or the input.

Why the codec's own value and not an event schema beside it: the codec document defines its value (the DFDL view
above). An event schema written beside it would state every field a second time, and a check that the two agree would
be a third reading. The record view is not a second definition: it is derived, and nothing an author writes can make
it disagree with the codec. Why a derived record and not the codec's own generated type: each backend holds a codec's
value in a type of its own shape (on Rust, a borrowed form, an owned mirror generic over a storage profile, gate
groups; C11's fixed buffers), and the profile's walk would need a reading of each shape on each backend. The record
view gives the profile one shape, the one it already reads, and a backend converts between it and the codec's value
in one place (§2.2).

#### 1.2 Each codec message type a system port carries has one binding

A binding is the implementation of a codec message type's encoding on the TTCN-3 side. There are two kinds:

- **The reference binding of the codec's encoding**, the default: the downstream's codec for the encoding the
  document is written in (DD §8.4), which reads the codec document for the format's parameters (keys, properties,
  parameters, separators, the enum documents' texts, the layout). CBOR runs on libcbor; content-line is the
  downstream's own, written from RFC 5545 §3.1 and SCE_FORGE.md §4.6.4; positional is written the same way from
  SCE_FORGE.md §4.6 once ADR 0100 lands (step 8 below).
- **A consumer binding**, named by the TTCN-3 module that implements it: a module the consumer supplies from its own
  repository, under its own licence, that defines the message type, every type it reaches but the integer types of
  the type grammar, and its `enc_R` and `dec_R` of the signatures TP §9.5 states, by whatever means the consumer
  chooses (Titan's RAW engine with its `variant` attributes, external functions in C++, or both).

The native run always uses SCE's codec, generated from the codec document, whichever binding the TTCN-3 side uses: the
two runs read the wire with two codecs, which is what the comparison needs (§2).

An event schema carried by a system port has no SCE codec and takes no binding: its messages cross the native
system adapter as values, as today, and its encoding is the test system's, which TTCN-3 permits an abstract test suite
to leave. The export declares its codec functions without an `encode` attribute (§1.4). The lane's reference set has
no codec for it, so a suite that runs in the lane carries codec types on its system ports (§12.4 of the lane, rewritten
by this record's second commit).

#### 1.3 The binding is declared in the test suite

```xml
<sce:import kind="codec" src="codecs/frame.scxml" as="Frame"/>
<sce:codec-binding type="Frame" ttcn3-module="proto_types" canonical="true"/>
```

`<sce:codec-binding>` is a child of the `test-suite` root. `type` names an imported codec that a system port of a
configuration of the suite carries; `ttcn3-module` names the consumer's module, a TTCN-3 identifier under the name
contract (§4.5); `canonical`, `false` where it is not written, states that the module's encode writes the one encoding
the codec document writes, which the encode check holds (§2.5). A codec type with no `<sce:codec-binding>` has its
reference binding. Refused, as `test-profile/codec-binding`: a `type` that is not a codec, or that no system port of
the suite carries; a type bound twice; and a type the closures of two modules reach (below). A consumer binding of an
event schema is refused by the same code: the native run would have no codec to read the same octets with, so the
binding would buy nothing the comparison can use.

The closure of a consumer module is every type its bound message types reach, but the integer types. A type in the
closures of two consumer modules, or of a consumer module and a type the types module defines (the record view of a
reference-bound type, or a type a template, a component variable or another port's message reaches outside the
closure), is refused: TTCN-3 has one definition of a name, and a consumer's codec needs its `variant` attributes on
the definitions its encoding reads.

Why the suite, and not a lane manifest:

- **The export is a function of the documents.** The binding changes the export's text: which module defines a type
  and its codec functions, and what the behaviour module imports. `docs/SCE_CODEGEN_DETERMINISM.md` and TP §9.5
  ("Text") make the export a function of the documents and the generator, and the witness relates every place of the
  modules to a construct of the documents (TP §9.4). A manifest outside the documents would make the export a
  function of a file the validator does not read.
- **SCE reads no file of the downstream.** `docs/SCE_TTCN3_LANE.md` §2.4 forbids it, and the native run and
  `check-trace` need to know which types are codec types. A manifest in the downstream is out of SCE's reach; one in
  SCE beside the suite is a second document kind, outside the authoring grammar and `check`, saying what the suite can
  say.
- **ADR 0091 put the same kind of choice in the suite.** A test case's feature selection is written on its
  `<sce:execute>` in the suite, as TTCN-3's control part writes it, and not in a configuration of the lane. The
  binding is a property of the suite's executable test suite, which is the suite's export.

Why the suite and not the port type or the system port: a `test-port-type` is shared by connected and mapped ports
and by several suites, and a binding concerns the system interface of one suite. A system port is finer than a type:
two system ports carrying one type under two bindings would need two definitions of one name in one module, which
TTCN-3 does not have. One suite, one types module, one definition and one codec per type.

A suite that is run both with the reference bindings and with a consumer's is two suite documents. A suite document
is its imports, its `<sce:execute>`s, its module parameters and its bindings, so the second one is a few lines that
import the same test cases.

#### 1.4 What the export writes

- **E1** gains the codec document: a reference-bound codec message type is its record view's `type record`, with the
  attribute `with { encode "<encoding>" }`, where `<encoding>` names the encoding rules and their version:
  `sce-cbor/2`, `sce-content-line/2` (`/3` once ADR 0104 lands), `sce-positional` (no format is written on a
  positional document). The records the record view holds that no document declares (§4.1) are written beside it,
  with no attribute. A consumer-bound type and its closure are not written into the types module.
- **The codec functions** are declared for each type a system port of the suite carries, and for no other: a connected
  port does not encode. For a reference-bound codec type and for an event schema they are declared in the types
  module, as today; for a consumer-bound type, the consumer's module defines them.
- **E16 (new)**: the consumer bindings of the suite. The behaviour module imports each consumer module once, after the
  types module, in the order of the modules' names: `import from proto_types all;`. The witness names each import as a
  place of the kind `import` of one E16 entry, whose construct is the suite's root, as the integer types are E1's
  places under the suite's root. The validator's form check holds each import to a module a `<sce:codec-binding>`
  names, and each module so named to one import.
- **The manifest** states every type, a consumer module's types among them with the member `module`, and each codec
  function pair with its binding (§4.6). The validator reads SCE's two modules; a consumer's module is held to the
  manifest by the lane before it runs (DD §11), which is how C2 asks a replaced types module to be held. TP §9.3
  obligation 6 ("the types module and the templates denote the same sets of values") then holds for a consumer module
  through the manifest and the lane's check, not through the validator, and TP §9.3 says so.

### 2. Wire bytes in the trace

#### 2.1 An input holds a value or its octets

Each input record of `sce-test-trace.v1` holds exactly one of:

- `value`, the message in the canonical value form of TP §12.5, as today;
- `octets`, the message as it was on the wire, in the byte form of TP §12.5: the octets in upper-case hex, two digits
  each, the empty text for no octets. One input is one message, so a decode must take every octet of it.

An input holds `octets` only for a codec message type; `octets` for an event schema is refused (§2.6). The trace also
gains `outputs`, written by a recorder (§3): the messages the run sent to its system ports, each with `at`, `port`,
`type` and one of `value` or `octets`, in the order they were sent. A player replays `inputs` and does not deliver
`outputs`; a reader holds them to their form and order. Both are additive to a `pre-release` surface.

#### 2.2 A codec message crosses the system adapter as octets, on both sides

As TRI places it, a message of a codec type crosses the system adapter encoded, in both directions and on both sides,
whatever form the trace gives it in:

- **An `octets` input** is decoded at its instant (TP §12.4 step 1), in the order of the trace: the native run with
  SCE's codec, generated from the document, the TTCN-3 run with its binding's `dec_R`.
- **A `value` input** of a codec type is encoded and then decoded, at its instant: the native run with SCE's codec,
  the TTCN-3 run with its binding's `enc_R` and then `dec_R`. A value input of an event schema is delivered as a
  value, as today.
- **A send** of a codec type to a system port is encoded: the native run with SCE's codec, the TTCN-3 run with its
  binding's `enc_R`.

The decoded value is the message the run delivers; it is the value its templates match and its log writes. On the
native side the codec's door is the unit of the test configuration, the home of the system ports' readers already
(`docs/SCE_TTCN3_LANE.md` §10.5): for each codec type a system port takes, it declares `decode_<type>` (octets to the
record view, through the codec's generated decode and the conversion of its value to the record view) and
`encode_<type>` (the reverse), each generated from one walk of the record view that every backend's generator shares,
as `canonical_form` is shared (TP §12.5). The system adapter interface of each runtime gains the octets of a codec
type in both directions beside the value of an event schema (TP §12.2).

#### 2.3 A codec failure is `error`

A decode that refuses the octets, that needs more octets than the input holds (`dec_R`'s 2), or that leaves octets
unread, and an encode that refuses a value, are a codec failure: P13's "a codec or adapter failure", `error`, on both
sides alike. The failure is recorded as a dynamic error is (TP §12.5): a `verdict-set` with `error` and no reason, of
the component whose port the system port is mapped to for an input, and of the sending component for a send, after
that send's `message-sent`; nothing more is delivered, and the test case ends with `error`. A test that wants to judge
a malformed frame rather than stop on it maps a port whose message type holds the frame as a byte string, which every
octet sequence decodes into.

The TTCN-3 side must write the same records in the same order, which the downstream's binding and logger plugin keep
(DD §9.3): a send is logged before its encode, and a refusal of `enc_R` or `dec_R`, or octets `dec_R` leaves unread,
is raised as a dynamic test case error, which the plugin writes as the `verdict-set` above.

#### 2.4 The log records each decoded input: `message-detected`

`sce-test-log.v1` gains the record `message-detected`, with `component`, `port`, `type` and `value`: written when the
run appends a message from the system adapter to its port's queue (TP §12.4 step 1), after it is decoded, before any
snapshot of the round; `component` and `port` are the component and its port the system port is mapped to, and
`value` is the decoded value in the canonical form. It is TCI's event of a message detected at a port (`tliMDetected_m`
of ES 201 873-6's test logging interface, a name slice D1 checks against the text with the others). It is written for
every input, an event schema's included, so the native writer has one rule.

Without it the two codecs' readings are compared only where a branch takes the message (`message-received`), which is
after matching: a message that decodes to two values, one of which no template matches, shows as a difference of
behaviour some rounds later, or not at all where the message stays in its queue until the test case ends. With it,
C3 compares every decoded input at the instant it was decoded. The record changes the expected log of every run whose
system adapter delivers an input, `answers_the_job.json` among them, so it lands on all seven writers at once (step 4).

The log records values only. The octets of an input are the trace's, the same bytes on both sides by construction, and
writing them into the log would add nothing to C3.

#### 2.5 The encode check: a check of the codecs beside C3, not part of it

Both runs encode each send of a codec type. Where both encoders write one canonical encoding, the lane compares the
two runs' octets of each such send. This is a separate check of the lane, the **encode check**, and not part of C3:

- **C3 is the executed half of the export's theorem** (ADR 0091 §1, TP §9.3), whose observables are values: the
  messages sent and received on each port, the timeouts and the verdict. An encoding is not an observable of the
  abstract test suite. Folding bytes into C3 would make a disagreement of two codecs read as a failure of the
  translation, which is what C3's `error` asserts.
- **Its subject is a pair of codecs, not a test case.** A divergence of two encoders shows on every test case that
  sends that value; the check reports it once per type and value, naming the test cases.
- **It is conditional, and C3 is not.** Two encoders may disagree on bytes with neither wrong, where the encoding
  admits several spellings of a value (BER beside DER). SCE's encodings each write one: CBOR's deterministic encoding
  (SCE_FORGE.md §4.6.1, RFC 8949 §4.2), content-line's one folding (§4.6.4), a positional layout's bytes with every
  derived element derived (ADR 0058). A reference binding implements those rules and is canonical by construction; a
  consumer binding is canonical where its `<sce:codec-binding canonical="true">` says so, and is not checked
  otherwise.

The check compares the `outputs` of the two runs' records (§3) of a test case whose logs C3 found equal: the `n`-th
output of each, on the same port and of the same type, where both carry `octets` and the binding is canonical. A
difference names the test case, the instant, the port, the type and both octet strings; the lane's run fails, and no
verdict and no log changes. A test case C3 failed is not judged by it.

#### 2.6 `check-trace`

`sce-codegen check-trace` reads `octets` and `outputs` with the Rust reader and refuses, beside the refusals of
`docs/SCE_TTCN3_LANE.md` §10.2: an input or an output that gives `octets` for a type that is no codec type
(`test-profile/trace-octets`, new); an output on a port that is no system port, or of a type its port's `out` list
does not name (`trace-port`, `trace-type`, at `/outputs/<n>`). It does not decode: a codec failure is an outcome of the
run that both sides must reach alike (§2.3), not a malformed trace, and a recorded trace of an implementation that
sent a malformed frame is a correct trace.

#### 2.7 What this closes

The two items of `docs/SCE_TTCN3_LANE.md` §14: SCE's generated codecs now read the same bytes as the binding in every
octets input, and write bytes the encode check compares; a positional message type is bound by a consumer's module
now and by the reference positional binding once ADR 0100 lands.

### 3. The recorder

A recorder is a system adapter that wraps another and records, at the boundary of TRI, what crosses it: every input,
as the octets received for a codec type and as the value for an event schema, at the instant the run delivered it (the
`NOW` of TP §12.4 step 1, not the time the bytes reached a socket), and every output, as the octets the encode wrote
for a codec type and as the value for an event schema, at the instant it was sent. It records every mapped port,
those of the test management protocol and of any other test control among them: the replay must reproduce the run's
inputs whole, and an indication of an upper tester steers the run as much as a frame of the implementation does. It
writes one `sce-test-trace.v1` per test case, `inputs` and `outputs`, which either executor replays.

Each executor is recorded by a recorder at its own adapter boundary, since neither can tap the other's:

- **The TTCN-3 executor**: the downstream's recorder (DD §8.1), in its test ports, between the transport and `dec_R`
  inward and between `enc_R` and the transport outward.
- **The native executor**: a recording adapter in each of the seven test runtimes, which wraps any system adapter, the
  trace player or a consumer's live one. The runner writes the record of each test case when it is given
  `--record-dir <dir>`, `<test case>.json` each (`docs/SCE_TTCN3_LANE.md` §4); a consumer's live executor wraps its
  own adapter in it. The record of a replay holds the inputs as they crossed: a value input of a codec type is recorded
  as the octets its encode wrote.

### 4. The TTCN-3 types of a codec's value

The record view of a codec of each encoding is given by the tables below. Each row is a type of the type grammar,
which TP §6.3 already exports; the rows add only the records no document declares and the rule of absence.

#### 4.1 Content-line

| Entry | Type of the record view | TTCN-3 |
|---|---|---|
| a single-valued `string` with `sce:max-size` N | `string[N]` | `universal charstring`, its bound stated as TP §9.5 states one |
| an integer, a `bool` | its type | the integer type of its width, `boolean` |
| an enum entry, `enum:E` | `enum:E` | the `enumerated` of E's document (§4.2) |
| a single-valued property with `sce:separator` | `list<T>[m..V]`, `V` its `sce:max-values` | `record length (m .. V) of T` |
| a repeated property, no parameter, no separator | `list<T>[m..C]`, `C` its `sce:max-count` | `record length (m .. C) of T` |
| a repeated property with a parameter or a separator | `list<record:L>[m..C]`, `L` its line record | `record length (m .. C) of <codec>_<entry>_line` |
| a parameter of a single-valued property | its type | a field of the codec's record |
| `optional<T>` on a single-valued entry or parameter | an optional field | an `optional` field, `omit` where absent |

`T` is the type of one value (`string[N]`, an integer, a `bool` or an enum). `m` is 1 where the entry is required,
since a required entry of any list shape needs a line (SCE_FORGE.md §4.6.4), and 0 where it is `optional<…>`. A list
entry that is optional is a list that may be empty, not an optional field: the codec has one wire form for "no line",
the empty list, and a record view with both `omit` and the empty list would give one wire form two values, so a value
decoded from its own encoding would not be the value encoded.

**The line record** `L` of a repeated property is `type record <codec>_<entry>_line { … }`, the record the codec
itself gives each line (`<Codec><Entry>Line`, SCE_FORGE.md §4.6.4): each parameter of the property, by its entry's
id, in declaration order, optional or required per line as its type says (ADR 0014), then the line's value, by the
property entry's own id, of type `T`, or `record length (1 .. V) of T` where the property has a separator; the value
is never `optional`, since a line holds one. For `tests/forge/resources/codec_content_line_records.scxml`:

```ttcn3
type record codec_content_line_records_attendee_line {
  universal charstring attendeeCn optional, // at most 24 bytes of UTF-8
  universal charstring attendeeRole, // at most 16 bytes of UTF-8
  universal charstring attendee // at most 40 bytes of UTF-8
}

type record codec_content_line_records_exdate_line {
  universal charstring exdateTzid optional, // at most 24 bytes of UTF-8
  record length(1 .. 3) of universal charstring exdate // at most 20 bytes of UTF-8
}

type record codec_content_line_records {
  universal charstring uid, // at most 24 bytes of UTF-8
  record length(0 .. 3) of codec_content_line_records_attendee_line attendee,
  record length(0 .. 3) of codec_content_line_records_exdate_line exdate,
  record length(0 .. 4) of universal charstring categories, // at most 12 bytes of UTF-8
  record length(0 .. 2) of universal charstring geo // at most 16 bytes of UTF-8
} with { encode "sce-content-line/2" }
```

The line's value is named by the entry's id, and not `value` as the codec's generated record and the oracle name it:
`value` and `values` are words the export reserves (`ttcn3_export::names::RESERVED`; `value` is a keyword of -1), and
the entry's id is a name of the document, which the name contract prefers to any name the export makes. The
conversion between the codec's value and the record view (§2.2) renames the one field.

Refused by name (`generate/unsupported-feature`), as having no type of the profile, whose types are bounded (TP §6.3):
a `string` entry with no `sce:max-size`, and a repeated property with no `sce:max-count`. Refused as TP §9.5 refuses
them in any record: an enum of an open set.

#### 4.2 An enum entry: no name is mangled

An enum entry's TTCN-3 type is the `enumerated` E1 writes for its enum document (TP §9.5), with the variants' declared
names and numbers. The text a variant is written by on the wire (its `sce:text`, `NEEDS-ACTION`, else its name) is
the encoding's: the binding reads it from the enum document, as SCE's codec reads it from the same table, and it never
names anything in TTCN-3. There is no name-mangling contract because there is no mangling: the contract is TP §9.5's
identity, every name the export writes the document's own, and a name TTCN-3 cannot take is refused
(`test-profile/export-name`), never renamed. A mangling (`NEEDS-ACTION` to `NEEDS_ACTION`) would give one variant two
names, the document's and the module's, which the log, the witness and a consumer's module would have to map, and it
would collide where two texts differ only by the characters it replaced; the texts are unique under ASCII case folding
(ADR 0015) and nothing makes them unique after a replacement.

#### 4.3 CBOR

ADR 0101 gives a CBOR entry the whole type grammar, so a CBOR codec's record view is its entries as the fields of
`type record <codec>`, by their ids, in declaration order (the keys are the encoding's, as a `variant` would be), each
of its own type, which TP §6.3 already exports:

| Entry type | TTCN-3 |
|---|---|
| an integer, `bool`, `enum:E` | as TP §6.3 |
| `string` with `sce:max-size` N, `bytes` with `sce:max-size` N | `universal charstring`, `octetstring`, bound stated |
| `bytes` with `sce:length` N | `octetstring`, its exact length stated in the manifest (`min_octets` = `max_octets` = N) and in the comment |
| `record:R`, at any depth | the `record` of R's event schema |
| `union:U` | the `union` of U, one alternative per case (ADR 0092) |
| `list<T>[m..N]` | `record length (m .. N) of T` |
| `optional<T>` as an entry, or as a field of a record at any depth | an `optional` field, `omit` where absent |

A list of `optional<T>`, and `optional` anywhere a TTCN-3 field is not, are refused as TP §9.5 refuses them in an
event schema. A `string` or `bytes` with no bound is refused by name (`generate/unsupported-feature`): it has no type of
the profile, as on content-line.

#### 4.4 Positional, once ADR 0100 lands

A positional codec is a message type from step 8 of the landing order, after ADR 0100 lands; until then, a positional
codec named as a message type is refused by name (`generate/unsupported-feature`, naming this record's step 8). Its
record view is its presented value (ADR 0058 §1), and every derived element is the encoding's, not a field: a length,
a count, a flag a gate or an arm derives, a fixed value (ADR 0100 §3) and a tag are what a consumer's RAW module
states with its own attributes (`LENGTHTO`, `PRESENCE`, `TAG`, `CROSSTAG`) and the reference binding reads from the
document. A presented field is a field of its type; an embedded body a field of the body's record view; a
`<sce:repeat>` and a chain a `record length (m .. n) of` the entry's record view; a variant a `union` of its arms, each
named by the document's name of its arm, an arm with no body `record {}`; a gated field an `optional` field. Step 8
writes the rows into TP §9.5, with the made names of a gate group and of a default arm's alternative under the rule of
§4.5, and holds them to ADR 0103's descriptors of the same elements.

#### 4.5 The name contract

TP §9.5's contract is unchanged: identity, and a refusal where TTCN-3 cannot take a name. It gains:

- among the names the export makes, `<codec>_<entry>_line` for each line record, `<codec>` the codec's unit name and
  `<entry>` the entry's id as written; where it collides with a name of the documents, the written one is refused, as
  for every made name;
- among the names of the documents, each consumer module named by `ttcn3-module`, which is in the scope of the module
  definitions: it must be a TTCN-3 identifier, not a reserved word, and distinct from the names of the two modules and
  of every definition;
- the record view's fields, which are the codec's entry ids, and the line record's, which are entry ids too.

#### 4.6 The manifest

`sce-ttcn3-types-manifest.v1` (`pre-release`) gains, each member additive:

- on a `record`: `event` becomes optional (a line record travels as no event); `line`, on a line record, names the
  codec type and the entry it is the line of; `module`, on a type a consumer module defines, names that module;
- on an `octetstring`: `min_octets`, where the codec states an exact length;
- on each entry of `codec`: `binding`, absent for an event schema (its encoding is the test system's), else one of the
  two kinds; `codec` lists only types a system port carries (§1.4), which removes entries a suite with a connected
  port of a type no system port carries had before, a change the `pre-release` status admits.

```json
{"kind":"record","name":"codec_content_line_records_attendee_line",
 "line":{"codec":"codec_content_line_records","entry":"attendee"},
 "fields":[{"name":"attendeeCn","type":{"kind":"universal charstring","max_utf8_bytes":24},"optional":true},
           {"name":"attendeeRole","type":{"kind":"universal charstring","max_utf8_bytes":16},"optional":false},
           {"name":"attendee","type":{"kind":"universal charstring","max_utf8_bytes":40},"optional":false}]}

{"type":"codec_content_line_records","encode":"enc_codec_content_line_records",
 "decode":"dec_codec_content_line_records",
 "binding":{"kind":"reference","encoding":"sce-content-line/2","document":"codecs/records.scxml"}}

{"type":"frame","encode":"enc_frame","decode":"dec_frame",
 "binding":{"kind":"consumer","module":"proto_types","canonical":true}}
```

`document` names the codec document as the witness names a document (relative to the suite's directory, or by its
`sce:std/` name), so the binding finds the document it reads the format's parameters from. `encoding` is the text of
the type's `encode` attribute.

### 5. The lane document

`docs/SCE_TTCN3_LANE.md` is edited in the commit after this one:

- **§1, the third condition.** The TTCN-3 side's codecs run on a library neither SCE nor Titan wrote, or on an
  implementation written in the downstream from the specifications without reading SCE's codec sources, or, for a
  type a suite binds so, on a consumer's module. The three are not equally independent, and the condition says how:
  a library written from the RFC by others (libcbor) reads the format apart from SCE's generator and apart from SCE's
  page; a binding written clean-room from SCE's page reads it apart from the generator only, so it removes the
  generator's defects from the comparison (C2's first kind, an independent encoding engine) and shares any error of
  the page, on both sides of the wire where the comparison cannot see it; a consumer's module written from the
  protocol's requirements by another author is C2's second kind, an independent protocol reading. Agreement is
  evidence to the extent of that independence and no further (ADR 0091 C2).
- **§2.1**: the reference bindings, without libical; a consumer's binding is not the downstream's.
- **§3**: the licence of libical and MPL-2.0 is removed; libcbor stays, and a consumer binding is the consumer's
  code under the consumer's licence. `LICENSE-THIRD-PARTY.md`'s section on the lane says the same.
- **§8**: the summary of the reference set, without libical.
- **§12.4**: the corpus's codec documents and its octets traces, per §1 and §2.
- **§14**: the two open items closed by this record, and the move off libical among what is closed.

## Conformance

Each property lands with the step that makes it hold, and is held on every backend from the landing that gives the
backend the step.

- **C-1. A codec is a message type through its record view.** `sce-build/tests/a_codec_is_a_message_type.rs`: the
  record view of every content-line and CBOR codec of `tests/forge/resources` that has one is the table of §4 (each
  row by a fixture: an enum entry, a separator list, a repeated property with a separator, line records, optional and
  required lists, a CBOR record at depth, a union, an exact-length byte string); each refusal of §1.1 and §4 by name.
- **C-2. The record view is the oracle's value.** For every decoded case of the `codec_content_line_*` and CBOR
  entries of `numerical_reference.json`, the decode of its `encoded` bytes by the Rust codec, converted to the record
  view, writes the canonical form of the case's `decoded` value, an absent entry left out and the line's `value` named
  by its entry: the conversion is checked against the independent models, not against itself.
- **C-3. The binding is the suite's.** `a_codec_binding_is_declared_by_its_suite`: each refusal of
  `test-profile/codec-binding` (§1.3), and its row in the acceptance document's appendix
  (`acceptance_doc_covers_every_code`).
- **C-4. The export.** `a_test_suite_is_exported_to_ttcn3.rs` gains a suite whose system ports carry a content-line
  codec, a CBOR codec and a consumer-bound codec: the types module holds the record views with their `encode`
  attributes and their line records, no consumer-bound type and no codec function of a type only a connected port
  carries; the behaviour module imports the consumer module (E16); the manifest is held to its schema
  (`a_types_manifest_is_held_to_its_wire_schema`), the witness to its own; and every export of the fixtures is held to
  the validator (`a_ttcn3_export_is_validated_apart_from_the_exporter`), whose rule table and reader take E16 and the
  attribute.
- **C-5. The trace.** `a_test_trace_is_held_to_its_wire_schema.rs` gains `octets` and `outputs`, and refuses an input
  with both `value` and `octets` or with neither; `a_trace_is_checked_against_its_suite.rs` gains
  `test-profile/trace-octets` and the outputs' refusals of §2.6. Each backend's reader of a trace is held to the same
  refusals, in the words of the Rust one.
- **C-6. Each side's codec reads the octets.** Per backend: an octets input of each codec kind replays to the log the
  same value input writes; a value input of a codec type is encoded and decoded (a codec whose decode of its own
  encoding differs is caught by C-2 first); an octets input the codec refuses, one cut short, and one with an octet
  left over each end the test case with `error` at the input's instant, the `verdict-set` of §2.3 written; a send its
  codec refuses ends it after its `message-sent`.
- **C-7. `message-detected`.** All seven log writers write it at the point of §2.4, and the seven runs of
  `answers_the_job` are held to the one expected log, which gains the record
  (`a_test_log_is_held_to_its_wire_schema.rs` and each backend's twin of it).
- **C-8. The record.** Per backend, the runner with `--record-dir` writes the record of each trace of the corpus, and
  the seven records of each are held to one expected record, byte for byte: the seven generated codecs of each codec
  kind encode the corpus's sends alike, which is the encode check run inside SCE across its own backends.
- **C-9. The corpus.** Every trace of the corpus replays to its expected log on every backend that generates its
  suite (`the_corpus_replays_to_its_expected_logs`), the octets traces of each codec kind among them, and a test of
  the corpus holds that every message type a system port of a runnable suite carries is a codec type and that each
  codec kind has an octets trace and a refused frame.

## Landing order

Each step is one or more landings; each keeps every lane green, and each landing that changes a generated tree says
which and regenerates it, where ADR 0097 has not yet moved the tree out of the repository.

### SCE

1. **This record**, and the lane document's edits of §5.
2. **The codec message type and its binding, in the build.** The xsd and the parser: `<sce:import kind="codec">` in a
   `test-port-type`, a `test-template`, a `test-component` and a `test-suite`, and `<sce:codec-binding>`; the derived
   record view of content-line and CBOR codecs, and its refusals (§1.1, §4); `test-profile/codec-binding` with its
   rows. TP §6.3 and §11.3 to §11.8 are amended. Tests: C-1, C-2 (Rust codec), C-3. A codec named as a message type is
   refused by name on every backend until step 5 or 6 lifts the refusal for it (TP §11.9's pattern).
3. **The export and the validator.** E1 for codec types with the `encode` attribute and the line records; codec
   functions for system-port types only; E16; the manifest's members and the witness's place `import`, with both
   schemas and their `SCE_WIRE_CONTRACTS.md` rows; the validator's reader, form check and rule table. TP §9.2 to §9.5
   are amended. Tests: C-4.
4. **`message-detected` on all seven writers at once**, with the expected log, the log's schema and its row, and TP
   §12.5 amended. One landing, because the seven are held to one file. Tests: C-7.
5. **The trace's octets and outputs, the codec door and the recorder, on Rust.** The trace schema, its reader and
   `check-trace` (`test-profile/trace-octets`, its rows); the codec message type generated on Rust (its record view's
   unit and the configuration unit's `decode_<type>` and `encode_<type>`); the octets path of the system adapter and
   the trace player; the recording adapter and the runner's `--record-dir`. The trace's row of `SCE_WIRE_CONTRACTS.md`,
   the lane's §4 and §10 and TP §12.2 and §12.4 are amended. Tests: C-5, C-6 and C-8 on Rust.
6. **The other six backends, one at a time**, in the order S1 took (Kotlin, C++, Go, Swift, Python, C11), each lifting
   the refusal of step 2 for itself as it lands. A backend whose test profile matches no union yet
   (`codegen_matrix::TEST_PROFILE_UNION_BACKENDS`) refuses by name a codec message type whose record view reaches one,
   as it refuses an event schema that does. Tests: C-5, C-6 and C-8 on each.
7. **The corpus.** Codec documents for the message types of the system ports of the runnable suites, CBOR and
   content-line; octets traces of each codec kind with a refused frame among them; value traces of a codec type; the
   expected logs and records. Tests: C-9.
8. **Positional message types**, after ADR 0100 lands: the record view of a positional codec (§4.4) with its rows
   written into TP §9.5, the native conversion on each backend in the order of step 6, and octets traces of a
   positional codec in the corpus.

### Downstream (`sce-ttcn3-titan`)

Named here as slices of the downstream; DD §13.2 numbers them and may split them further.

- **D6. Bindings by type.** The reference bindings read each codec function pair's `binding` from the manifest and the
  codec document it names, and refuse an `encoding` they do not implement by its text (the content-line binding
  refuses `sce-content-line/3` until it implements ADR 0104's format); the lane holds a consumer module to the
  manifest before it runs and compiles the consumer module the suite's E16 imports; the positional reference binding,
  written from SCE_FORGE.md §4.6 without reading SCE's codec sources, after ADR 0100 lands, held to SCE's positional
  oracle whole as the content-line binding was.
- **D7. Octets at the test ports, and the recorder.** The test ports decode each input of a codec type with `dec_R`
  and encode each send with `enc_R` (§2.2), and the trace player hands an octets input to `dec_R` and a value input to
  `enc_R` then `dec_R`; a codec failure is the dynamic error of §2.3; the logger plugin writes `message-detected`; the
  recorder of DD §8.1 writes `inputs` and `outputs` of every mapped port (§3).
- **D8. The encode check** of §2.5, in the lane runner beside C3, reported apart from it.

D6 and D7 need SCE's steps 3 and 5 at the `SCE_PIN` they move to; D8 needs step 5 and D7.

## Consequences

- A protocol's conformance suite can run in the lane with a TTCN-3 codec of its own: the native run reads the wire
  with SCE's codec of the protocol, the TTCN-3 run with the consumer's, and C3 compares what each read from the same
  bytes. That is C2's second kind of independence, which no reference binding gives.
- SCE's generated codecs are in the lane: every octets input is a differential test of SCE's codec against a binding,
  and C-8 holds SCE's seven backends to one encoding of every send of the corpus.
- The native system adapter interface carries octets for a codec type, which is TRI's shape and not the value-carrying
  shape TP §12.2 had. A consumer's live adapter for a codec type moves bytes and no longer encodes, since the run
  does.
- Every log of a run with system inputs gains `message-detected` records, and every expected log of the corpus
  changes once (step 4).
- A message type's value is read by one more walk in every backend's generator, the conversion between a codec's value
  and its record view; it is the price of keeping the profile on one shape of record.
- A suite run under two bindings is two suite documents.
- The lane's independence is stated per binding (§5); a reader of a lane report learns how far the agreement it shows
  goes.

## Alternatives rejected

- **A lane manifest that names each type's binding.** The export would stop being a function of the documents, the
  validator could not check it, and SCE would read the downstream's file or keep a second kind of document outside
  its grammar (§1.3).
- **A command-line option of `export-ttcn3` that names bindings.** Two exports of one suite would differ by an input
  no document records, and the witness would name places no construct accounts for.
- **The binding on the port type or on the system port.** Too coarse and too fine respectively (§1.3).
- **An event schema for each message, and a codec document checked to agree with it.** Each field stated twice, and a
  third reading to hold the two together.
- **A codec document that encodes a named event schema**, the ASN.1 split of abstract syntax and encoding rules. It
  would re-express every codec as an encoding of a schema, leave a positional codec's derived elements, flags and
  arms without a field in that schema, and turn the consumer's codec documents into two documents each. The codec
  document is a data description that defines its infoset, as ADR 0100 takes it, and the record view is that
  infoset.
- **The codec's own generated type as the message.** Each backend's codec value has its own shape, a storage profile,
  a borrowed form or fixed buffers, and the profile's walk would read each (§1.1).
- **Octets in the log.** The two sides log the same input bytes by construction, so C3 would gain nothing; and the
  octets of a send would make C3 compare encodings, which §2.5 keeps apart.
- **The encode check inside C3** (§2.5).
- **Decoding a trace in `check-trace`.** The build does not run generated codecs, and a decode failure is a run's
  outcome, not a malformed trace (§2.6).
- **A decode failure as a message that does not match.** The message would stay at the head of its port's queue and
  block every receive branch of that port (TP §5, a receive examines the head), so the run would end on its guard
  for a reason it does not name; and ADR 0091 §4 makes a codec failure `error`.
- **The native run decodes inputs and does not encode sends.** A send the TTCN-3 side's encode refuses would end that
  run with `error` and the native run with a verdict, and C3 would report the difference without its cause.
- **Keeping libical, with its output corrected around it.** A library that does not implement the format would be
  wrapped in a second implementation of the format, and the wrapper, not the library, would be the codec.
- **Mangled enum names** (§4.2).
- **`value` and `values` as the line record's field names**, as the codec's record and the oracle name them: both are
  reserved, and the entry's id is a name of the document (§4.1).

## Questions left open

None. Decided 2026-11-14; the first step is 2 above. The name `tliMDetected_m` and the shape of TRI's message
operations are checked against ES 201 873-5 and -6 in the downstream's slice D1, as the lane's other TRI and TCI names
are, and the downstream design is corrected there where the texts differ.
