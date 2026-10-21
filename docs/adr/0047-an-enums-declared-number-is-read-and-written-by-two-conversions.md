# ADR 0047 — An enum's declared number is read and written by two conversions

- Status: Accepted (2026-10-21) and implemented
- Date: 2026-10-21
- Scope: the forge expression layer (`forge::types::EnumScope::conversion`,
  `forge::expr`'s conversion lowering, `forge::static_enum`), every imported enum of a
  `sce-static` statechart and of a forge kind; the machine's own enum type on the six
  generated backends (`forge::static_lowering`, `StaticTarget::enum_number_symbols`), the
  enum document's accessors an algorithm reaches (`forge::generator::enum_number_symbols`,
  `c11_enum_from_number`), the Interpreter's library (`static_js/sce_static.js`); the forge
  runtimes of Rust, Kotlin, C++, Go and Python (`found`); docs/SCE_ACCEPTED_SUBSET.md
  §3.4.1 and §2.15 ("Enum variables"), SCE_FORGE.md §4.12
- Related: ADR 0042 (an application keeps an enum in a numeric field of the shared list "by
  the enum's declared wire number"), ADR 0044 decision 6 (an older receiver refuses a variant
  it does not declare), ADR 0005 (`sce-static` closes its gaps by bounding and declaring)

## Context

An enum document declares, for each variant, a name and a number (`<sce:variant name="done"
value="7"/>`). A `sce-static` statechart holds a value of an enum in its own type, compares it,
stores it and hands it over, and the value crosses every wire as the variant's declared name. The
number is the enum document's: a codec reads and writes it, and nothing else could.

ADR 0042 tells an application to keep an enum in a numeric field of the standard shared list "by
the enum's declared wire number". No document could: measured 2026-10-21, a statechart that
assigned `_event.data.status` (a value of an enum) to an `int64` was refused, rightly, as "a value
of the enum `MilestoneStatus` where a number or a bool is expected", and there was nothing it could
write instead. Sending the value to the list instead passed `check` and was refused by the list at
run time (fixed apart: a send is now held to the field that reads it). The same was true of an
algorithm, which held an enum in the enum document's own generated type and had no reading of its
number either; that is where the number would be computed if a rule needed it.

What is to be decided: how a document reads the number a value's variant declares, and how it
turns a number back into a value, given that a number may be one no variant declares.

## Decision

**Every imported enum carries two conversions, written as calls on its alias:**

- `<alias>.toNumber(value)` — the number the variant of `value` declares, a value of the enum's
  carrier (`sce:underlying-type`: `uint8` … `uint64`), which then stands wherever an integer does.
  It takes a value of that same enum (a variant, a variable, a field, a call that returns one);
  anything else is `expression/unsupported-construct`.
- `<alias>.fromNumber(number)` — the variant that declares `number`, a value of the enum, which
  then stands wherever a value of that enum does. It takes an integer; a value of an enum is
  refused.

**A number no variant declares is a failure, `out-of-range`, not a value.** It is the rule a
closed enum keeps everywhere else: a delivery that names no variant is refused, and a codec refuses
a carrier value outside the declared set. A fallback (the first variant, a default) would be a
value the number never named, and a reading that answers "none" would need a type a statechart has
no form for. So `fromNumber` of a closed enum is `may-fail` (SCE_FORGE.md §3.4.1), and the failure
reaches the author by the channel every failing operation already has:

- in a statechart, the statement fails: nothing is written, `error.execution` is raised and the
  block ends (§scxml-4.9's rule for every failed assignment, which an overflow already follows);
- in an algorithm, only a body that declares `may-fail` may call it, and the failure is the
  algorithm's (an algorithm that does not declare it is refused at the call, as for any call of a
  `may-fail` callee);
- in a `<data expr>`, which runs before there is a session to raise in, it is refused at build time
  as any initial value that can fail is;
- a number the carrier cannot hold is the narrowing failure every argument already has (an
  `int64` of `300` handed to a `uint8` enum fails before the enum is asked).

An open enum (`sce:strict-variants="false"`) holds every number of its carrier, so its
`fromNumber` cannot fail and is not `may-fail`. A statechart holds closed enums only.

**The surface is a member call on the enum's alias, not a free function.** The reverse direction
must name the enum it produces, and the alias is the one name a document has for it; the forward
direction is spelled the same way so that the pair reads as one feature. The forge expression
layer already reaches a callable through an import's alias (`smoother.update(x)`), so no new form
is added to the grammar, and a document whose enum happens to declare a variant named `toNumber`
still reads `Status.toNumber` as that variant and `Status.toNumber(v)` as the conversion.
`toNumber`/`fromNumber` follow ECMAScript's naming of conversions (`String.fromCharCode`), which
the forge expression grammar is a subset of (SCE_FORGE.md §3.4).

**The typer, the enum rule and the lowering are each written once.**

- `EnumScope::conversion` is the signature of both (`toNumber`: a value → the carrier;
  `fromNumber`: the carrier → a value, `may-fail` for a closed enum), and
  `TypeCtx::lookup_member_func` answers it beside a stateful import's methods, so inference, the
  call-argument check and the unknown-callee check read one table.
- `forge::static_enum` holds the argument of each to its kind and makes `fromNumber` a value of
  its enum, for a statechart and, through `check_algorithm`, for an algorithm.
- `forge::expr` lowers both to the two functions the context spells for the enum
  (`EnumScope::to_number`, `EnumScope::from_number`) before the variants are lowered. Where the
  backend's reading of a number answers an optional value and a failing call answers something
  else, the call is wrapped in one runtime helper that turns "none" into `out-of-range`:
  `sce_forge_runtime::algorithm::found` (Rust), `SceChecked.found` (Kotlin),
  `SCE::Forge::Checked::found` (C++), `scealgorithm.Found` (Go, an `Outcome`),
  `sce_algorithm.found` (Python). C answers a result struct of its own and the Interpreter
  throws, so neither is wrapped. The helper is named for what it reads, a value that may not
  have been found, and not for the enum: a lookup whose miss is an error answers the same
  optional value and is turned into its failure by the same helper (ADR 0048).

**The functions are the enum document's own accessors, by name and shape.** An algorithm reaches
the enum document's generated type, whose accessors every backend's enum kind already emits
(`to_underlying` / `from_underlying`, `toUnderlying` / `fromUnderlying`, `ToUnderlying` /
`<T>FromUnderlying`). A statechart holds the enum in the machine's own type, which now declares the
same two accessors, so a statechart and an algorithm lower the two calls alike:

| | to the number | to the variant |
|---|---|---|
| Rust | `T::to_underlying(v)` | `T::from_underlying(n)` → `Option` |
| Kotlin | `(T::toUnderlying)(v)` | `T.fromUnderlying(n)` → `T?` |
| C++ | `to_underlying(v)` (an algorithm), `<T>ToUnderlying(v)` (a machine) | `from_underlying(n)`, `<T>FromUnderlying(n)` → `std::optional` |
| Go | `T.ToUnderlying(v)` | `<T>FromUnderlying(n)` → `(T, bool)` |
| Python | `T.to_underlying(v)` | `T.from_underlying(n)` → `Optional` |
| C11 | `<doc>_to_underlying(v)` (an algorithm), `<doc>_number(v)` (a machine) | `<doc>_from_number(n)` and its `_take` |
| Interpreter | `SceStatic.number(table, v)` | `SceStatic.variant(table, n)`, which throws |

A C++ machine's enums share its namespace, and C++ cannot overload on a return type, so a machine
names the two for its type. C has no optional value: `c11_enum_from_number` writes one text — a
result struct, the reading, and the `_take` that records its failure — under a guard of the enum
document's name, which a machine that holds the enum and an algorithm that may fail both declare,
so a program that includes both declares it once. A C machine's constants carry the declared
numbers, so its `<doc>_number` is the value itself.

## Alternatives considered

- **Free functions (`number(v)`, `enumOf(Alias, n)`).** `number(v)` would infer the enum from the
  value, but the reverse needs the enum named, and an alias is no value an argument may be. Two
  spellings for one pair, one of them a new kind of argument, against two member calls the grammar
  already has.
- **An implicit conversion where a number meets an enum.** It would make `n = status` mean the
  number in one place and a refusal in another (an `===` against a number would compare a name to
  a number on the wire), and it would hide from the author that a number may name no variant.
- **A total `fromNumber` that answers a default variant.** A value the number never named; the
  reason a closed enum refuses an unknown wire value (ADR 0044 decision 6) applies unchanged.
- **Carrying the number on the wire instead of the name.** It would change every wire surface and
  every saved state that holds an enum, and the name is what ADR 0044 holds revisions of an enum to.
- **The machine's own conversion functions with names of their own, and the enum document's for an
  algorithm.** Two spellings of one operation per backend. Giving the machine's type the document's
  accessor names makes the lowering one rule.

## Consequences

- An application stores a value of an enum in a numeric field (ADR 0042's shared list) with
  `Status.toNumber(value)` and reads it back with `Status.fromNumber(number)`, where a number no
  variant declares is `error.execution` in the statechart.
- Every generated machine that holds an enum declares the two accessors, read or not; the
  committed Rust, Kotlin, Go and Python trees of such machines change, and so does the C header of
  every algorithm that may fail and imports a closed enum.
- `scenarios/static_enum_number.json` holds the pair on every engine that runs the model, the
  failures included; `sce-build/tests/an_enum_is_read_as_its_number_and_back.rs` holds the rule and
  the lowering of each backend.
