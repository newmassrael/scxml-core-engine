# ADR 0052 — A refused command is named by an enum, and no refusal is a variant

- Status: Accepted (2026-10-21), under the owner's standing rule: the most long-term correct way,
  cost ignored
- Date: 2026-10-21
- Amends: ADR 0046 (section 5, what a refusal tells its parent)
- Scope: `stdlib/shared/record_refused.scxml`, `stdlib/shared/record_refusal.scxml`, a new
  `stdlib/shared/record_command.scxml`, `stdlib/shared/record_list.scxml`;
  `sce-build/tests/fixtures/static_datamodel/record_list_parent.scxml` and
  `scenarios/record_list_parent.json`, and the three drivers that read the refusal by hand (Go, C,
  C++); `sce-build/tests/fixtures/standard_saved_shapes.json`; docs/SCE_ACCEPTED_SUBSET.md, "The
  standard shared record list"
- Related: ADR 0044 (the revisions of a schema and of an enum), ADR 0045 (a standard statechart that
  saves is a promise about its shape), ADR 0047 (an enum's declared number), ADR 0042 (the shared
  record list is a pre-release standard part)

## Context

An application built on the standard shared list found two gaps in what the list tells a parent
when it refuses a change (ADR 0046, section 5):

1. **The command at fault is a raw number.** `record_refused.kind` is a `uint8`: 0 an edit, 1 a
   clear, 2 a retire. The meaning lives in a comment, so a parent compares `kind === 2` and a
   screen spells the three words itself. Everything else the list names by a closed set is an enum
   (`reason` is `record_refusal`), and a statechart holds and compares a value of one by its
   variant's name.
2. **"No refusal" has no value.** A parent keeps the last refusal in a record variable of
   `record_refused`, and a variable of an enum always holds one of its variants: the fixture started
   `reason` at `notWriter`, a refusal that never happened. The language has no optional value a
   variable could hold instead (ADR 0047: "a reading that answers none would need a type a
   statechart has no form for"), so a parent could not say "nothing was refused yet" in the type.

## Decision

**1. The command is the standard enum `record_command`.** `sce:std/shared/record_command.scxml`
declares `edit` = 0, `clear` = 1 and `retire` = 2, the numbers the list already gives a staged
command in `record_part.kind`, so `Command.fromNumber(part.kind)` of a staged command is the
command it is (ADR 0047). `record_refused.kind` is `enum:Command`, imported under the alias the
schema writes, as `reason` is under `Refusal`; a statechart that reads the payload or keeps a
record of it imports both (docs/SCE_ACCEPTED_SUBSET.md, "A record's enum field"). The list holds the
command at fault in its own variable of the enum and sets it from the command it is handling, and
where it names the second of two commands that write one register, which is an edit or a clear,
from that command's kind by a condition rather than by `fromNumber`: the statement cannot fail, so
the refusal it is part of cannot be turned into another.

`record_part.kind` stays a `uint8`. It is the list's own record, which travels in no message, and it
numbers two different sets (a staged command, and an operation: a write or a write-over); an enum
for one of them would be wrong for the other.

**2. "No refusal" is the variant `none`, at 0.** `record_refusal` declares `none` = 0, the number
left free below `notWriter` = 1. The list never tells it: a refusal it sends always has a reason. It
is the value a parent's variable of the enum holds before the first refusal, which the fixture now
starts at, and a parent tests `refused.reason === Refusal.none`. Of the two shapes the question
offered, this is the one the language has: there is no optional value in `sce-static`, and adding
one for a single field would be a language feature (a type, its saved form on six backends, its
crossing on the wire) to say what a zero variant says. The standard library's other enum with a
resting value does the same (`sce:std/mesh/payload_codec.scxml`, `none` = 0).

**3. Both change the first revision, and the list's saved shape moves (ADR 0044, ADR 0046).**

- `record_refused.kind` is re-typed, from a `uint8` to an enum. ADR 0044 decision 1 refuses a
  re-typed field between two revisions of one schema, and there is no revision that expresses it:
  a new field beside the old one would keep the raw number in the schema forever, which is the gap.
  So the schema changes in place.
- `none` could be added to `record_refusal` as ADR 0044 decision 6 grows an enum (`sce:revision="2"`
  on the root, `sce:since="2"` on the variant), and is not. It is the variant a variable of the enum
  starts at, which belongs to the enum's first revision; declared as an addition, every machine that
  holds the enum would name a `revisions` entry in every saved state for a variant that was there
  before any release. And the growth would not keep any shape: the list's own `namedKind` is
  re-typed with `kind`, which no revision expresses either.
- No saved state that holds the list or a refusal exists outside this repository's tests, which is
  the condition ADR 0046 section 7 moved the shape under, and it still holds: ADR 0042's step 1 is
  not verified, so the list is a pre-release part, and ADR 0045's format-2 save of a running child
  (its step 3) is in progress, so no application saves the list as a child yet. A host that saved a
  list it runs at the top level with an earlier build is refused at the restore, as any change of
  shape is, and its peers give the list back by resync. The pin in
  `sce-build/tests/fixtures/standard_saved_shapes.json` moves with this change, and is from here a
  promise again.
- On the network, a parent and the list it invokes are generated by one build, so the two
  `record_refused` schemas a send crosses are always the same revision.

## Alternatives considered

- **Keep the number and document it.** The gap is that the meaning lives in a comment; a parent
  would keep spelling the three words, and a fourth command would change the numbers' meaning with
  nothing to say so.
- **An optional reason (`reason` absent until a refusal).** The language has no absent value for a
  variable: a record variable holds every field. A per-field `sce:optional` would be a new type
  carried on six backends and in the saved state, to say what one variant says.
- **A boolean beside the reason (`refused.any`).** Two fields that must agree, and a reason that
  still holds a refusal that never happened while the boolean is false.
- **`none` as a variant of `record_command` too.** The command of a refusal is always one of the
  three; a parent that has no refusal reads the reason, and the command beside `none` is the
  variable's starting value, `edit`, with no meaning.
- **`namedKind = Command.fromNumber(later.kind)`.** It may fail, so the statement that names the
  command of a refusal would raise `error.execution` inside the refusal's own judgement, which the
  list reads as the failure of a probed statement.

## Consequences

- A parent compares and shows the command at fault by name (`refused.kind === Command.clear`), and
  keeps "nothing refused yet" in the type (`Refusal.none`).
- Every statechart that reads `record_list.refused` imports `sce:std/shared/record_command.scxml`
  as `Command`, as it imports `record_refusal` as `Refusal`.
- The list's saved shape and every committed tree that holds the list or a refusal move; the
  scenario `record_list_parent` names the command by its variant on every driver, starts with the
  reason `none`, and adds a refused clear, by the member's role and as the second command naming a
  register.
