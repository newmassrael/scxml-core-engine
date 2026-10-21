# ADR 0051 — A statechart hands a function of another kind a value of an enum or a text

- Status: Accepted (2026-10-21) and implemented, under the owner's standing rule: the most
  long-term correct way, cost ignored
- Date: 2026-10-21
- Amends: ADR 0048 (its refusal of "a slot that is not a number or a truth value")
- Scope: `forge::static_imports::function_callee`; `StaticScope::call_enums` and the crossings a
  statechart's scope registers (`TypeCtx::crossings`, `CallCrossing`, `SlotCrossing`); the
  expression layer's `lower_call_crossings`; the functions a machine declares to cross a value of
  an enum (`StaticTarget::enum_crossing_names`, `enum_crossing_def`) on Rust, Kotlin, C++, Go and
  Python, and C's reading of a lookup whose miss is an error (`c11_lookup_found`); the lookup
  kind's keys and values (`render_lookup`, the Go lookup template); docs/SCE_ACCEPTED_SUBSET.md
  §2.15 ("Calling another kind"), SCE_FORGE.md §4.3
- Related: ADR 0047 (an enum's declared number), ADR 0048 (a statechart calls the forge kinds that
  are functions), the crossing a call of an algorithm already makes for a value of an enum
  (`StaticTarget::enum_crossing`, docs/SCE_ACCEPTED_SUBSET.md "State an algorithm takes and gives
  back")

## Context

ADR 0048 let a `sce-static` statechart call a transform, a condition, a lookup, an interpolation
and a validator, and refused every slot that is not a number or a truth value. An application
found what that costs:

- **A table keyed by an enum respells its numbers.** A lookup from a status to a priority had to
  key its table by the numbers the status enum declares (`key="2"`), and the statechart had to
  hand it `Status.toNumber(status)`: the table and the enum then say the same thing twice, and a
  variant renumbered (ADR 0044 refuses that, but a new enum is free) leaves the table answering for
  the wrong variant. The lookup kind itself keyed only by literals of a number type.
- **A validator of a text is handed its length.** A rule "a title is not empty and is not a
  placeholder" could only be asked of `len(title)`, which says nothing about the text.

What is to be decided: how a value of an enum and a text cross into such a call and back, on six
backends, with the type checking an algorithm's slots already have.

## Decision

**1. The slots a statechart hands over.** A slot of a function of another kind is a number or a
truth value as before; a value of an enum the statechart imports from the same document (the
identity every reader of an enum names it by), closed, as an input or the one output; and a
`string` as an input. Refused where the callee is imported, naming the slot: an enum of a
document the statechart does not import as an enum, or an open one (a value no variant names has
no declared name to cross by); a string output (a call answers a text with no bound a statechart
holds it within); a lookup keyed by a string (no backend's table compares a text alike: C would
compare pointers, C++ cannot hold a table of references); and a byte string anywhere. A slot is
named, as an algorithm's enum slot is, by the alias the statechart imports the enum under, so the
enum judge (`static_enum`) holds the arguments of such a call exactly as it holds an algorithm's:
a value of that enum and nothing else, and the call a value of the enum it answers
(`StaticScope::call_enums`).

**2. A value of an enum crosses by its declared name, through two functions the machine declares.**
A statechart holds a value of an enum in the machine's own type (ADR 0047); the function holds it
in the type the enum document's generation declares. A call of an algorithm crosses through the
adapter it is made by; a function of another kind is called as it is, so the machine declares, for
each enum such a call crosses, two functions of its unit — `<MachineEnum>ToDocument` and
`<MachineEnum>FromDocument` — whose bodies are the crossing a call of an algorithm already makes
(`StaticTarget::enum_crossing`: a `match`, a `when`, a conditional chain, a map, a dict over every
variant), and the expression layer wraps each argument of an enum slot in the first and the call's
value in the second (`lower_call_crossings`). The wrapping comes before the call is read as its
signature says (ADR 0048's `lower_call_readings`), so a lookup whose miss is an error is read
through `found` first and its value crosses once it is found. C holds a value of an enum in the
document's own type (ADR 0047), so nothing crosses there, and its reading of a lookup that may miss
names the document's C type.

Two functions per enum rather than an adapter per call: a function of another kind takes and
answers values, not records or lists, so the one thing that differs between the two sides is the
type of an enum, and one pair of functions serves every call that crosses it.

**3. A text is handed as it is.** Every backend's statechart holds a text in a form the function's
parameter takes — a `String` lent as `&str` (Rust, which the crossing writes as a borrow), a `String`
(Kotlin), a `std::string` (C++), a `string` (Go), a `str` (Python), the buffer's text (C) — so a
text needs no conversion; a validator of a text compares the text itself.

**4. A lookup is keyed and answers by the variants of an enum.** A lookup whose input or output is
`enum:<alias>` names each key and value of its table by the variant's declared name
(`<sce:entry key="open" value="light"/>`, `sce:default` alike), and each backend emits the variant's
reference (`forge::enum_naming::variant_ref`, the one spelling every reference to a variant has) in
the table its numeric strategy already builds over any comparable type. A name the enum does not
declare is refused where the lookup is generated, naming the entry and the variants. Go's numeric
table now imports the enum's package beside the runtime's lookup.

## Alternatives considered

- **Hand the function the number (`Status.toNumber(s)`) and key the table by numbers.** It is
  what the gap was: the table restates the enum.
- **An adapter per call, as an algorithm's call has.** An adapter exists to copy records and lists
  across; for values alone it would be one function per call where one pair per enum serves every
  call, and it would have to read a validator's validity and a lookup's `found` itself, which the
  expression layer already does for every call.
- **Hold the document's own type in the machine.** ADR 0047 decided the machine's type; changing
  it is a change of every machine's saved form and generated API.
- **A string output, held to the variable it is assigned to.** A call's text has no bound the
  build can see; the assignment would hold it, but a guard or an argument that reads it would not.
  Left out until a document needs one, refused by name meanwhile.
- **Tables keyed by text on the backends that compare it.** One rule on six backends is the
  promise a function call keeps; a text key would be four.

## Consequences

- A lookup from a status to a level is written in the status's and the level's own words, and a
  statechart calls it as `Weight(phase)`; a validator is handed `title` itself.
- Every machine that hands a value of an enum to a function of another kind declares two crossing
  functions per enum and imports the enum document's unit; the committed trees of
  `static_forge_calls` gain them, and the regeneration of each backend generates the enum
  documents a called document imports.
- `scenarios/static_forge_calls.json` replays a transform from an enum to an enum, a lookup keyed
  and answering by enums whose miss fails, and a validator of a text on the six generated
  backends; the Interpreter still refuses these calls by name (ADR 0048).
