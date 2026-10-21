# ADR 0050 — A build's bounds are read by name, and a send is held to the bound that reads it

- Status: Accepted (2026-10-21) and implemented, under the owner's standing rule: the most
  long-term correct way, cost ignored
- Date: 2026-10-21
- Scope: `crate::limits` and the readers of `sce:limit` (`forge::parser` keeps the name beside a
  field's `sce:max-size`, `ForgeField::limit`, `RecordField::limit`); the expression layer
  (`forge::expr`'s `lower_build_limits`, `TypeCtx::limits`); the statechart's scope
  (`StaticScope::limits`, `named_limits`) and the algorithm's (`AlgorithmTypes`); the machine's
  declaration of its bounds on the six generated backends (`StaticTarget::limits_def`); the
  event-schema unit's templates; the check of a `<send>` (`forge::static_datamodel`,
  `SentValue::misfit`, `Judge::text_bound`); docs/SCE_ACCEPTED_SUBSET.md ("A bound a deployment
  chooses", "A bound is read by its name", "A text is held to the bound of the field that reads
  it", the EventSchema kind)
- Related: ADR 0046 section 6 (the shared list's bounds, `shared_typed_text` among them),
  ADR 0005 (`sce-static` closes its gaps by bounding and declaring), ADR 0047 (a check hole of the
  same shape, H2), the send rule of 2026-10-21 (a sent value is held to the field that reads it)

## Context

A `sce-static` document bounds every text, byte string and list it holds, and `sce:limit` names a
bound so that a build moves every element that carries the name together
(`sce-codegen --limit name=N`). An application built on the standard shared list found three
gaps around those bounds:

1. **Nothing generated names a bound.** A schema field's `sce:max-size` became the size of a
   buffer and nothing else: no constant in any backend's generated code. A host that must stop a
   title past 32 bytes before it sends it (a screen that counts, an input that refuses) copied
   `32` by hand, and a deployment built with `--limit shared_typed_text=64` left every copy
   behind without a word.
2. **A statechart cannot read a bound.** It holds a text to its bound only by failing an
   assignment past it; a guard that tells the user, or a statechart that cuts a text to fit,
   had no value to compare with.
3. **A send ignores the receiver's bound** (a defect). `check` accepted
   `<param name="text" expr="written.text"/>` where `written.text` is a record's 96-byte field and
   the receiver, the list's `record_list.edit`, reads a 32-byte one. The list refused every
   delivery past 32 bytes when it read it, and the parent was told nothing; the rule that holds a
   sent value to the field that reads it held its kind and not its bound.

## Decision

### 1. One number per name, the build's

The value of a bound is the number the build resolved it to: the one `--limit` gave the name,
or the one its element wrote. The readers already resolve it where they read the element
(`crate::limits::resolve`); the name is now kept beside the resolved number
(`ForgeField::limit` for a schema field, `Variable::limit` for a statechart's variable, and the
algorithm's local and return), so the generated code and an expression can name it. A document
names a set of bounds (`named_limits`): its own variables' and its imported schemas' fields'
for a statechart; its locals', its return's and its records' text fields' for an algorithm.
Where two of its elements name one bound and resolve to two numbers, the name has no one value:
it is declared by no constant and a read of it is refused naming both numbers. (A build of the
repository's documents keeps one number per name, as
`the_bounds_of_the_repository_that_share_a_name_write_one_number` already holds.)

### 2. A statechart and an algorithm read a bound as `_x.limits.<name>`

SCXML reserves the system variable `_x` for what a platform adds to a session's variables, and
no document may declare it, so a read under it cannot collide with a variable, an import's
alias or a builtin. `_x.limits.shared_typed_text` reads the number the build gave the bound.
The expression layer writes the read as the integer literal of that number before anything else
reads the tree (`lower_build_limits`), so it is the same on every backend and in the
Interpreter's lowering, and it is held to the place it lands in as any literal is (a bound of
300 read into a `uint8` is refused when the document is built, as `300` would be). A name the
document does not name is `expression/unknown-member`, naming the ones it does; a name with two
numbers is `expression/unsupported-construct`.

A literal rather than a typed value of its own is the reading because the number is the
build's, fixed before any code is generated: a literal is the one form every backend and the
Interpreter already spell alike, and a typed read would need a declaration in scope at every
place an expression is lowered (a machine's methods, an algorithm's body, an adapter, the
Interpreter's document), where the constant of section 3 lives only in the machine's unit.

### 3. A machine and a schema's unit declare each bound as a typed constant

- **A machine** declares every bound it names (section 1) with one value, named by the machine
  and the bound: Rust `<Machine>Limits::<NAME>` (`u32`, associated constants of a unit struct),
  Kotlin `object <Machine>Limits` (`const val … : UInt`), C++ `struct <Machine>Limits` (`static
  constexpr std::uint32_t`), Go `<Machine>Limit<Name>` (`uint32`, package constants named by the
  machine as everything a package of several machines declares is), Python `class
  <Machine>Limits` (`int`) and C `<MACHINE>_LIMIT_<NAME>` (`UINT32_C`). It is declared beside the
  machine's own types (`StaticTarget::limits_def`).
- **A schema's unit** declares, for each `string` and `bytes` field with a `sce:max-size`,
  `<FIELD>_MAX_SIZE` (Go `<Field>MaxSize`, C `<SCHEMA>_<FIELD>_MAX_SIZE`), its comment naming the
  `sce:limit` when one names the bound, beside the event name and revision the unit already
  names.

A bound is a count of bytes or elements that fits 32 bits (`crate::limits` refuses any other),
so every constant is the language's unsigned 32-bit integer, the type a length is read as in a
`sce-static` document. The machine's constant is what a host that drives the machine reads; the
schema's is what a host that builds the schema's payload reads; both are the number the build
resolved, so neither can disagree with the buffer.

### 4. A send is held to the bound of the field that reads it

Where the rule of 2026-10-21 holds a `<param>` of a `<send>` to the field that reads it (the
schema the document declares the event by, and the schema each child the send is delivered to
declares it by), a string or a byte string is also held to the field's `sce:max-size`: a value
whose declared bound is past it is refused at the `<param>` as `scxml/static-datamodel-rule`,
naming both bounds. The declared bound of a value (`Judge::text_bound`) is a literal's length, a
variable's `sce:capacity`, the `sce:max-size` of a field of a record variable, of the element a
list of records is indexed at, of a `<foreach>`'s record item or of the payload, the larger of a
conditional's two values and the sum of a join's two — each as the build resolved it, so a build
that moves both bounds together keeps a send that fits. A value whose length nothing declares
(a call) is held where it arrives, as before.

The narrowing the language offers is the assignment: the value taken into a variable whose
`sce:capacity` is the field's bound, which fails as `error.execution` when it does not fit (and
the transition's block ends, §4.9 of SCXML, before anything is sent), and that variable sent.
The refusal says so. No new narrowing form is added: an assignment is already the one place a
text is held to a bound, on every backend and in the Interpreter alike.

## Alternatives considered

- **A builtin function, `limit('shared_typed_text')`.** A call of a name an author may also
  declare (an import's alias, a variable), a string argument where only a literal means anything,
  and a new callee the closed scope, the enum judge and seven lowerings would each have to know.
- **Reading a field's bound, `Edit.text.maxSize`.** It answers a schema's field, not a
  deployment's bound: a statechart that holds a list named `merge_values` and imports no schema
  that names it could not read it, and two fields bounded by one name would be two reads of one
  number.
- **A typed read lowered to the machine's constant.** Section 2: a declaration in scope at every
  lowering site, for a value the build already fixed.
- **The constants on each backend's payload type** (a Rust associated constant of
  `<Machine><Event>Payload`). A machine generates a payload type only for the events it reads; a
  bound named by a variable or a list has no payload type to hang on; and the schema's own unit
  already is the type a payload is built from.
- **Refusing every sent text whose bound nothing declares**, as a typed host request does. It
  would refuse a join of a text and a number or a call's text, whose length is a run-time fact,
  and give the author nothing the field's own read does not; what is refused here is a send that
  the build can see may not fit.
- **Holding the bound at the send at run time, before the message leaves.** A send that cannot
  fit is the author's to see when the document is built, and the run-time refusal where the
  message is read already exists and stays.

## Consequences

- A host reads `RecordListParentLimits::SHARED_TYPED_TEXT` (or the schema's `TEXT_MAX_SIZE`)
  and holds a title to it; a deployment that builds with another number moves the host's
  constant with the buffer.
- A statechart compares a text with its bound by name (`len(title) <=
  _x.limits.shared_typed_text`), and so does an algorithm.
- A parent that sends a record's 96-byte text as a 32-byte field is refused when it is built,
  and narrows it by an assignment it writes.
- Every committed tree of a machine that names a bound, and every committed schema unit with a
  bounded field, gains its constants.
