# ADR 0134 — A delay is computed where its send runs with no heap: a count joined to its unit as the count, and any other delay's text in a buffer its bounds size

- Status: Accepted (2026-11-25), under the owner's rule for this work: the most long-term correct, textbook
  structure, cost ignored, no workaround. A downstream bare-metal consumer generates a `no_std`, no-heap Rust
  machine whose states keep deadlines, and its first delay did not compile.
- Date: 2026-11-25
- Scope: `backends/rust/runtime/src/helpers/send.rs`, `tools/codegen/templates/rust/actions/send.rs.jinja2`,
  `sce-build/src/forge/{delay_count,expr,static_lowering,static_datamodel}.rs`, `sce-build/src/{model,parser,
  internal_queue_bound}.rs`, `tests/durations/count_unit.json`, `backends/rust/probes/nostd-delay`, the
  `nostd-mcu` gate.
- Related: ARCHITECTURE.md "Durations (Single Source of Truth)" (the one reader), ADR 0053 (a bare-metal crate asks
  for no feature), ADR 0076 (the proven internal queue bound counts a computed delay), ADR 0077 (a declared bound is
  part of a field's type), ADR 0086 section 8.6 (c) (the bound of an expression's text), docs/SCE_ACCEPTED_SUBSET.md
  (`delayexpr` under `sce-static`).

## Context

Under `datamodel="sce-static"` a `<send>`'s `delayexpr` is a string, the CSS2 time the delay is written in,
computed from the machine's fields when the send runs and read by the one duration reader every engine shares. The
consumer's machine is a session lifecycle: each atomic state arms its deadline with a delayed `<send>` in its
`onentry` and cancels it by the same id in its `onexit`, and a compound state keeps an overall limit. Its delays are
`closing_ms + 'ms'` over integer variables, and the Rust lowering wrote

```rust
let s: String = format!("{}{}", self.closing_ms, "ms").to_string();
sce_rust_runtime::helpers::send::parse_delay_to_ms(&s)
```

which allocates twice, and under `#![no_std]` with no allocator does not compile: neither `format!` nor `String` is
in scope. The lowering wrote every join of texts this way, because a Rust expression's string is owned. C11, whose
string is a bounded buffer, already writes a join into a buffer the generator sizes from what the data model declares
(`SCE_FORGE_CONCAT`), so the subset has the bound; only the Rust lowering did not use it.

## Decision

### D1. An integer joined to the unit it counts is the count

A `delayexpr` of the form `<integer-typed expression> + '<literal>'`, where the literal is a unit the reader takes
(`ms` or `s`, either case) with nothing before it and only ASCII whitespace after it, is computed as the integer
times the unit's milliseconds with checked arithmetic, and no text at all:

```rust
pub fn delay_of_count<T: TryInto<u64>>(count: T, unit: DelayUnit) -> Option<u64>
```

The answer is the reader's answer for the text the join spells, for every value of every integer type. The text is
the integer's decimal digits followed by the unit. A negative value spells a sign, which is no time; it is no `u64`,
so the count is `None`. The reader accumulates the digits in a checked `u64` and multiplies by the unit, and refuses
past `MAX_DELAY_MS`; the count does the same product and the same cap. `None` is the argument error of
§scxml-6.2.4 that the text's `None` is (`SendDelayInvalid`, "`<send>` delayexpr is not a CSS2 time"), and an
operation of the integer expression that fails is the same `ExpressionFailed` the text path raises; in each case
`error.execution` is raised and the message is not sent.

The unit table is the reader's own: `helpers::send::DelayUnit` is what `parse_delay_to_ms` scales a suffix by and
what `delay_of_count` scales a count by. The build reads a literal as a unit by
`forge::delay_count::DelayUnit::after_count`, which also scales the build's reader of a written `delay`.
`tests/durations/count_unit.json` names the literals and the unit each is read as, or none (`' ms'`, `'0ms'`,
`'.5s'`, `'min'`, ...); the build's reading is held to it, and the runtime holds the count's answer equal to the
text's over every literal with a unit, for every value of the 8- and 16-bit types and every edge of the 32- and
64-bit ones (the type's ends, a sign, zero, the count of seconds and of milliseconds that pass the largest delay).

A literal the build reads as no unit is not wrong to read: its join is read as the text it is, by D2, and so answers
as the reader does by construction.

### D2. Any other delay's text is written into a buffer its bounds size

Every other `delayexpr` is lowered as a text lent to the reader, which keeps none of it and reads it in the statement
that computes it (`Position::Lent`, `transpile_into_lent`). On Rust:

- A join is written into `helpers::send::DelayText<N>`, `N` bytes held in the value, on the stack of the send:
  `::sce_rust_runtime::sce_delay_text!(N, part, ...)` writes each part, borrowed, as `core::fmt::Display` writes it
  (a string as itself, an integer as its decimal digits, the text ECMAScript gives each). `N` is the bound of the
  whole text by the one analysis of text bounds (`forge::text_bound`): a string variable by its `sce:capacity`, a
  literal by its bytes, an integer by the digits of its type, a chain `a + b + c` one buffer of the sum. It is
  C11's sizing without the terminator.
- A conditional one of whose branches joins is one buffer: each branch is a `DelayText` of the conditional's bound,
  a branch that joins nothing the one part of its own, so the two are one type and no branch lends a temporary it
  made. That holds under both rules Rust has had for the temporaries of a block's tail. A join or a conditional of
  texts that is a part of a join is a `DelayText` of its own bound.
- A text that joins nothing is borrowed whole, as a copied text is: a literal, `&self.grace[..]`, a conditional of
  them. No buffer.

The reader is handed a `&str`: `parse_delay_to_ms(&sce_delay_text!(12, self.wait, "ms")[..])`. A part that would
not fit, which the sizing rules out, leaves the buffer reading as no text, never as a cut one: the empty text is no
time, the argument error, where a cut text could read as a shorter delay. The emission is the same on the `std`
profile and the bare-metal one, so the one emission compiles on both (ADR 0053), and the `std` machine stops
allocating for a delay too.

### D3. A delay whose text has no bound is refused, on every backend

The judge of the data model refuses a `delayexpr` whose text has no bound the data model declares, at the attribute,
as `scxml/static-datamodel-rule`, naming the subexpression that has none. Every place a `sce-static` document
reads text from is bounded already (a string variable's `sce:capacity` and a record's or a payload's `sce:max-size`
are required), so the text with no bound is a call whose callee declares none on what it returns. The rule is the
data model's, not a backend's, for the reason the subset gives for a string's capacity: a machine holds the same value
wherever it runs, and an engine with no heap holds it at all. C11 refused such a join already, by name, at emission;
it is now refused once, before any backend lowers the document. No new `DiagnosticCode`: the rule is one of the data
model's, which `ScxmlStaticDatamodelRule` carries with the construct, the rule and the value.

### D4. The seam

`StaticTarget::computes_delay_ms` says whether a target's machine computes the milliseconds where the send runs, and
`StaticTarget::delay_ms(DelayMs)` spells them, as the target's optional count of milliseconds, from the lowered count
and unit (D1) or the lent text (D2). A target that computes them is refused by name if it spells none. The lowering
writes `Action::native_delay_ms` (and `native_delay_ms_fails`) in place of `Action::native_delay`; the other
targets keep the text. `Action::delay_is_native` is the one question ADR 0076's queue bound asks of a computed delay,
which counts as it did.

## Which backends allocate a delay's text

| Backend | The text of a `delayexpr` | Heap |
|---------|-----------------------------|------|
| C11 | `SCE_FORGE_CONCAT` into a buffer of the summed bound | none, already |
| Rust | D1 for a count, D2 for any other text | none, on both profiles |
| C++ | `std::string` joins | yes; the C++ machine holds its lists in the standard containers and has no no-heap profile |
| Go, Kotlin, Python, Swift | the language's string | yes; each runtime has a collector and no no-heap profile |
| Interpreter | the document's own expression, evaluated | n/a |

Only C11 and the Rust bare-metal profile promise no heap, and both now keep the promise for a delay.

## Not decided here

A delay's text is what D2 forms. A join a delay reads only as the operand of another operation (an argument of a
call, the operand of `len`) is that operation's operand and is lowered as every Rust operand is. Outside a delay, a
Rust expression's join of texts and a computed event name are still owned strings, which a no-heap machine does not
have: an `<assign>` of a join, a guard that compares one, an `eventexpr`. Each is its own decision, over the place
the text lands in, and none is taken here.

## Consequences

- Every generated Rust machine with a `delayexpr` under `sce-static` changes, and the Rust template's hash with it,
  so the committed Rust trees are regenerated.
- A `sce-static` document whose delay's text has no bound is refused on every backend where C11 alone refused it.
- `ARCHITECTURE.md` names `delay_of_count` and the build's unit reading beside the reader, as the reading of a count
  that is held equal to the reader, not a second parser.

## Verification

- `backends/rust/runtime/src/helpers/send.rs`, `a_count_and_its_text`: the two readings over every literal with a
  unit and every value or edge of every integer type; a count's two outcomes; the buffer's parts in order and a part
  that does not fit.
- `sce-build/src/forge/delay_count.rs`: the build's reading of the shared table, and which expressions are a count.
- `sce-build/src/forge/expr.rs`: the Rust lent text, a join, a chain, a conditional, a part, a text that joins
  nothing, and the other targets unchanged.
- `sce-build/src/forge/static_datamodel.rs`: a delay's text with no bound is refused, naming the call.
- `sce-build/tests/a_static_datamodel_holds_every_variable_to_a_type.rs`,
  `a_rust_delay_is_computed_with_no_heap_on_either_profile`: the generated machine, `std` and `--no-std`.
- The `nostd-mcu` gate builds `backends/rust/probes/nostd-delay`, the machine of
  `sce-build/tests/fixtures/no_std/session_deadline_probe.scxml` (the consumer's shape, every form of D1 and D2),
  for `thumbv7em-none-eabihf` with no allocator.
