# ADR 0162 — A decimal is a number of declared precision and scale, held exactly, rounded only where an operation names how, and written as one decimal fraction and one text

- Status: Accepted (2026-11-29) under the owner's standing rule, by the main session the owner delegated the decision
  to, after an independent review; amended on acceptance as listed in "Amended on acceptance".
- Built: nothing. No step of "Order of work" is built, and no proof, model or test of it has been run.
- Date: 2026-11-28 (proposed); 2026-11-29 (accepted).
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a facility is built on the backends a consumer needs, Rust by
  default; its contract is general; what is special to one application stays in the application. The need was stated
  by a downstream consumer: a fact that holds a business number cannot be journaled, and a number people write in
  decimal (a rate of 2.5 percent, a target of 95.0) is wrong in binary floating point.
- Amends (each amended document gains an "Amended by: ADR 0162" line in the landing that moves it; ADR 0086 and ADR
  0115, which carry such lines, gain theirs with this acceptance):
  - ADR 0063 §4: the expression vocabulary gains `quantize`, `roundingMul`, `roundingDiv`, `toDecimal`, `toFloat64`
    and `toFloat32`, so a callable spelled as one of them is refused as §4 refuses one spelled as a vocabulary name
    (`validation/duplicate-id`), D4 and D7;
  - ADR 0086 §1.1 (the grammar gains `decimal(P, S)`, D1), §2.1 (its rows of `J` and `C`, D8 and D9), §3.1 (its JSON
    spelling, D9), §7.1 (the Rust rows of the table of types, D12) and §9 (the signed form holds D8's bytes);
  - ADR 0087 §1, the table of `S(T)` and `G(T)`: a row for `decimal(p, s)`, a string of the canonical text by
    digit-prefix alternatives (D9);
  - ADR 0101 §1.1 (the grammar), §2.1 (a row of the table of forms), §2.6 (its CDDL), §3.3 (the row that refuses a
    tagged item: a decimal takes tag 4 at its place and tags 2 and 3 for its mantissa), §4.1 (a row of `C`) and §9
    (the TTCN-3 lane refuses a decimal by name), D8 and D12;
  - ADR 0108 D1: `K` gains `decimal(P, S)`, whose equality is exact and whose every value is bounded (D2);
  - ADR 0133 D1 items 1 and 4, and D3: a decimal operation and a decimal store are classified by the same proof as an
    integer's, over coefficients, a decimal operation computed at its result type and not at its operands' (D3, D5);
  - ADR 0151 D1 (a `sum` of a decimal column is exact, of the type D11 gives it) and D2 (a decimal is compared by the
    number it denotes);
  - ADR 0158, its table of functions: `:number` takes a decimal field, read from its canonical text, its fraction digits
    by default its scale (D9);
  - ADR 0115 D15, its last paragraph: "a real number" that has no translation is a binary one; a decimal has one
    within TLC's integers (D13);
  - `SCE_FORGE.md` §3.3 (the type list and the cross-language mapping), §3.4 (the typing convention, its deliberate
    exception for decimals, literals, string concatenation), §3.4.1 (the decimal operations among those that fail by
    name), §3.4.2 (a decimal is not an operand of a wrapping or saturating operation), §4.6.1 ("Rust and C++ print an
    entry of every type" becomes "Rust prints an entry of every type; C++ of every type but a decimal") and §4.12 (an
    algorithm's slot may be a decimal, on Rust);
  - `ARCHITECTURE.md`, "JSON Number Text": a decimal crosses as its canonical text (D9);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a decimal variable), "Physical-quantity annotation" (a decimal may carry
    `sce:quantity`, and refuses `sce:scale` and `sce:offset`, D1) and the appendix (the one new code, D14);
  - `docs/SCE_HOST_FEATURES.md`, the kinds the record codec describes (D10);
  - `docs/formal/README.md`: a document that holds a decimal is replayed on Rust and against its model alone (D13);
  - the wire surfaces whose text names the types they hold, each `pre-release` in `SCE_WIRE_CONTRACTS.md` and changed
    additively: `apis/forge-ast.v1.schema.json` (a `ValueType` gains the constructor), the payload schema of
    `sce-codegen payload-schema` (D9), and `schemas/sce-saved-state.v4.schema.json` (the description of a value).
- Scope: `sce-build` (the type, its typing, its classification, its lowering for Rust, its sizes, its refusals, its
  grammars and its description for the record codec), the Rust runtimes (a new `no_std` crate `sce-decimal`,
  re-exported by `sce-portable-bytes`, and `sce-forge-runtime` and `sce-rust-runtime`), native,
  `wasm32-unknown-unknown` and `no_std`, the page surface (ADR 0117 D7), the standard library
  (`sce:std/decimal/rounding.scxml`), the cross-backend conformance corpus, and the formal judges (`docs/formal`).
  Every other backend, the Interpreter, the TTCN-3 export and the JANI model refuse the type by name (D12).
- Related: ADR 0044 (revisions of a schema), ADR 0050 and ADR 0086 (a bound by name, part of a type), ADR 0055
  (optional values), ADR 0058 §5 (the round trip of a codec), ADR 0063 (the expression vocabulary), ADR 0087 (the
  grammars of a value), ADR 0099 (a failure has no value), ADR 0101 (one deterministic CBOR form per type), ADR 0108
  (keys), ADR 0114 (the journal), ADR 0115 and ADR 0156 (the levels of formal judgement, the generated model), ADR
  0117 (the page surface), ADR 0118 (a state's bound, proposed), ADR 0119 (the JANI model), ADR 0133 (totality where
  the build proves it), ADR 0151 (views), ADR 0158 (messages formatted by the host).

## Context

Read from the tree at `662e4d060cc`.

- **The numbers of the language are integers and binary reals.** The one type grammar
  (`sce-build/src/forge/value_type.rs`, `TypeOf`; ADR 0086 §1.1) holds `uint8` to `uint64`, `int8` to `int64`,
  `float32` and `float64` (`SceType` in `sce-build/src/forge/model.rs`). A real is a binary32 or a binary64, rounded
  where each operation is made (`SCE_FORGE.md` §3.4.1, "Single-precision reals"), and an integer operation that leaves
  its width fails by name (§3.4.1, "The integer arithmetic contract"). The expression typer's `InferredType`
  (`sce-build/src/forge/types.rs`) has `UntypedInt`, `UntypedFloat`, `Int` and `Float`; a numeric literal keeps its
  source spelling (`ExprKind::Number(String)`, `sce-build/src/ecmascript/mod.rs`), so nothing between the document
  and the emitter has yet turned it into a binary number.
- **A scaled raw number already exists, for physical quantities.** `sce:quantity`, `sce:scale` and `sce:offset` on a
  field or a `<data>` (`sce-build/src/forge/quantity.rs`; `docs/SCE_ACCEPTED_SUBSET.md`, "Physical-quantity
  annotation") declare `physical = raw * scale + offset` in a named unit, with exact rational constants, and refuse
  arithmetic across two units. The raw value keeps its wire type, and the accessor that gives the physical value
  computes it in binary64: it is a codec's conversion of a sensor's raw integer, not a number held exactly at its
  scale.
- **A decimal fraction is held near, not at, its value.** `0.1` has no finite binary expansion, so a `float64` holds
  the binary64 nearest it, a value on a target's boundary may be computed a unit of the last place to either side of
  it, and a sum depends on the order of its additions (Goldberg 1991). ADR 0151 already met the last of these and
  answered it for views alone, with a long accumulator (its D1).
- **A float is refused where a fact is kept.** A journal entry is its record's deterministic CBOR (ADR 0114 D1), and
  the build describes the record for the runtime's record codec, which maps the record's JSON, as a machine's payload
  holds it, to and from that CBOR. The description refuses a float at any depth, because a float has values (a NaN,
  the infinities) that the printed codec writes and no payload spells. The refusal is written once and read at every
  place that keeps or reads a fact; every place it is written is listed in D10. The text a downstream consumer quoted
  is `describe_scalar`'s in `sce-build/src/forge/payload_record.rs`: "`float64` holds values that are not finite,
  which the printed codec writes and no payload spells, ...". The refusal is right, and this record does not change
  it.
- **A float is not a key** (ADR 0108 D1; `NotAKeyWhy::Float` in `value_type.rs`): `NaN` is not equal to itself and
  `-0.0` equals `0.0`.
- **The JSON wire already declines a number where a binary64 cannot hold it.** A 64-bit integer crosses as the text of
  its digits, a JSON string, because a reader that takes a JSON number through a double carries one past 2^53 - 1 with
  its low bits wrong and no error; a typed field of those widths reads the digits, or a JSON number of magnitude at most
  2^53 - 1, and refuses digits that are not canonical (`ARCHITECTURE.md`, "JSON Number Text";
  `sce-build/tests/fixtures/static_datamodel/scenarios/static_wire_wide.json`). A real crosses as ECMAScript's
  shortest round-trip text of its binary value.
- **CBOR takes no tag yet.** ADR 0101 §3.3 refuses a tagged item, a bignum among them, as `CborWrongType`: "the types
  are defined over the basic generic data model".
- **The model generator holds integers and booleans, and TLC's integers are 32-bit.** `sce-codegen model`
  (`sce-build/src/formal/document_model.rs`) translates the fragment ADR 0156's Context lists and refuses every
  `sce-static` document whole (`Construct::Datamodel("sce-static")`); a literal it writes is at most
  `LARGEST_INTEGER`, `i32::MAX`, since TLC's integers are within `-2^31..2^31 - 1`. ADR 0115 D15 refuses "a real
  number" by name.
- **What an application does today** is hold an integer of the smallest unit (cents, or tenths of a percent) and
  remember its scale. Nothing checks that two operands share a scale, a sum of cents and mills is a sum of integers,
  a JSON reader shows `250` for 2.50, a message shows the integer, and every application spells the scale again. A
  downstream consumer asked for a type in place of that, and named what it must hold: a declared precision and scale,
  exactness on every backend, the arithmetic a judgement needs, one deterministic CBOR form (the decimal fraction of
  RFC 8949), one JSON spelling, and a place in a fact.

## The textbook structure

- **A decimal number is an integer coefficient and a power of ten.** Every exact decimal representation, fixed or
  floating, is a pair `(c, e)` whose value is `c · 10^e` (Cowlishaw, "Decimal floating-point: algorism for computers",
  ARITH-16, 2003; the General Decimal Arithmetic Specification, from which the decimal formats of IEEE 754 were drawn).
  A **fixed-point** type fixes `e` for all its values: SQL's exact numeric type `NUMERIC(p, s)` holds the numbers of
  exactly `p` significant digits of which `s` follow the point (ISO/IEC 9075-2, SQL Part 2: Foundation,
  `<exact numeric type>`; its `DECIMAL(p, s)` may hold more than `p`), so `e = -s` and `|c| < 10^p`. This type's
  semantics are `NUMERIC`'s.
- **One number, several representations, unless the exponent is fixed.** IEEE 754-2019 (IEEE Standard for
  Floating-Point Arithmetic, as ADR 0151 cites it; §2.1) calls the set of representations of one number in a decimal
  format its *cohort*: `2.5` and `2.50` are two members of one cohort, equal as numbers and different as data. A
  floating decimal needs a rule for which member an operation returns (its preferred exponent), and a canonical form
  must choose one. A type of fixed scale has one member of each cohort, so its value fixes its representation.
- **An operation is computed as if exact, then rounded once.** IEEE 754 requires each operation to be performed as if
  it first produced the exact result and then rounded it to the destination (§4.3, §5.1), by one of five
  rounding-direction attributes (§4.3): `roundTiesToEven`, `roundTiesToAway`, `roundTowardPositive`,
  `roundTowardNegative` and `roundTowardZero`. `quantize(x, y)` (§5.3.2) gives `x` the exponent of `y` within one
  format, rounding by the attribute; a change of radix is `convertFormat` (§5.4.2), with failures of its own. An
  operation that has no number to give (a NaN converted to a format with none) signals the invalid operation (§7.2).
  A value rounded twice, first to an intermediate place and then to the result, can differ from the value rounded once
  (double rounding; Goldberg 1991). Every mode is monotone: a larger exact value never rounds to a smaller result.
- **Addition, subtraction, multiplication and comparison of decimals are exact; division is not.** Over coefficients
  they are integer operations after an alignment by a power of ten; a quotient such as `1/3` has no finite decimal
  expansion. SQL fixes the scale of a sum and a difference as `max(s1, s2)` and of a product as `s1 + s2`, and leaves
  every precision and the type of a quotient to the implementation (ISO/IEC 9075-2, `<numeric value expression>`,
  Syntax Rules). An exact quotient's digits and its remainder are those of long division over machine words (Knuth,
  *The Art of Computer Programming*, vol. 2, 3rd ed., Addison-Wesley 1997, §4.3.1, Algorithm D), and the remainder
  decides the rounding.
- **A literal's exactness is part of its type.** SQL's `<exact numeric literal>` (digits with an optional point) is of
  an exact numeric type whose scale is its digits after the point, its precision left to the implementation, and a
  literal with an exponent is an `<approximate numeric literal>` (ISO/IEC 9075-2, `<literal>`). The precision this
  record gives a literal is its own (D6).
- **Mixing radices in arithmetic is a rounding.** A binary number converted to decimal, or a decimal to binary, is
  rounded (IEEE 754 §5.4.2, `convertFormat` between formats of different radices, correctly rounded; Clinger, "How to
  read floating point numbers accurately", PLDI 1990). The folklore error is a binary value taken for the decimal it
  was meant to be (Bloch and Gafter, *Java Puzzlers*, Addison-Wesley 2005, Puzzle 2, `2.00 - 1.10`), and Python's
  `decimal` module refuses arithmetic of a `Decimal` and a `float` for that reason, while it compares the two exactly.
- **CBOR has a decimal fraction.** RFC 8949 §3.4.4: tag 4 holds an array of two integers, an exponent `e` and a
  mantissa `m`, whose value is `m · 10^e`; the exponent is of major type 0 or 1, and the mantissa may also be a bignum
  (tags 2 and 3, §3.4.3, whose leading zero bytes a decoder reads, their choice "not intended to have application
  semantics"). Preferred serialization writes an integer in major type 0 or 1 wherever it fits, which is down to
  `-2^64` (§4.2.2 for its deterministic use). RFC 8610's prelude names it `decfrac = #6.4([e10: int, m: integer])`.
- **A JSON number is a binary64 to its readers.** RFC 8259 §6 lets an implementation limit the range and precision of
  the numbers it accepts, and names binary64 as what good interoperability may expect; I-JSON (RFC 7493 §2.2) asks a
  sender not to exceed it. A fraction has no range in which a binary64 holds every value, so a decimal number that
  must survive every reader travels as a string.
- **Text for people is formatted from the number's own digits.** CLDR's plural rules (Unicode Technical Standard #35,
  Part 3) select a category from operands that include `v`, the count of visible fraction digits: `1` and `1.50` select
  differently in English. A decimal of fixed scale has those digits; a binary number does not.

## Decision

**A decimal is a type of the one grammar, `decimal(p, s)`: the numbers of at most `p` significant digits of which `s`
follow the point, held as an integer coefficient at that scale. Its comparison and its addition, subtraction and
multiplication are exact, with a result type the build computes from the operands' extremes to hold every result; a
rounding happens only where an operation names its scale and one of IEEE 754's five modes, and a change of radix only
where a conversion is named; whatever can still fail is classified by the proof of ADR 0133 and fails by name. Its one
CBOR form is RFC 8949's decimal fraction with the exponent `-s`, and its one JSON spelling is its canonical text as a
string. It is a key, it stands in every position of the grammar, a fact holds it, and Rust serves it while every other
lane refuses it by name.**

It adds no kind. `SCE_FORGE.md` §10's admission test is a test of a new `sce:kind`, and a type of the grammar is a
constructor every kind already reads (ADR 0086 §1.1), so the test is not applied and no application is added to §10;
its third axis would pass in any case, since a number of declared scale is needed wherever an amount, a rate, a measure
or a price is kept.

### D1. `decimal(p, s)` is a type of the one grammar, and its parameters are the domain's

```text
T ::= … | decimal(P, S)
K ::= … | decimal(P, S)
P ::= an integer from 1 to 38
S ::= an integer from 0 to P
```

- **Its values.** `decimal(p, s)` holds the numbers `c · 10^-s` for every integer `c` with `|c| <= 10^p - 1`: `p`
  digits in all, `s` of them after the point, as SQL's `NUMERIC(p, s)`. `decimal(9, 2)` holds `-9999999.99` to
  `9999999.99` in steps of `0.01`. A decimal has no NaN, no infinity and no negative zero: every value is a number,
  so nothing about it has a value no payload spells, which is why a float is refused where a fact is kept (Context).
- **Its spelling.** `sce:type="decimal(9, 2)"`, the space after the comma optional as in `map<K, T>`, both parameters
  always written, each a whole number in decimal digits with no sign and no leading zero: `decimal(9)` and
  `decimal(09, 2)` are refused, since one type has one spelling (ADR 0086 §1.3's rule for a bound), and
  `decimal(9, -1)` is refused, a negative scale being outside the grammar. The parentheses are SQL's, and they keep
  the grammar's other two brackets to their meanings: `[ ]` holds a bound a build may move by name, `< >` holds a type,
  and `( )` holds the parameters of a value set. An XML attribute holds them with no escape.
- **Its refusals.** A precision outside 1 to 38, a scale past the precision, a parameter that is not a whole number so
  written (a limit name, a sign, a leading zero included) and one parameter alone are a breach of the grammar, a new
  `TypeRefusal::DecimalParameters` that is reported as the grammar's other breaches of shape are, as
  `validation/attribute-rule-violated` on the attribute, naming the place in the type (`type_rule_refusal`,
  `sce-build/src/forge/parser.rs`).
- **Why 38.** It is the most digits a 128-bit two's-complement integer holds whole (`10^38 - 1 < 2^127`), so a value
  is one 128-bit integer at most, and a product of two is within 256 bits (D4). It is the maximum of Apache Arrow's
  `Decimal128` and of the exact numeric types of SQL Server, Oracle and Snowflake (PostgreSQL's `numeric` holds up to
  1000 digits and MySQL's `DECIMAL` 65; neither is a 128-bit carrier). The wire forms (D8, D9) do not depend on it: a
  later record may admit more digits with the same bytes for every value admitted now.
- **The parameters are not bounds, and no build moves them.** A bound says how many (a list's elements, a text's
  bytes) and a deployment chooses it (ADR 0050). `p` and `s` say what a value means: a fact journaled at scale 2 and
  read by a build at scale 3 would be another number, and a build that moved `p` would move which operations the build
  proves total. So neither is written `name=N`, no `--limit` reaches them, and a revision of a schema that changes them
  changes the field's type, judged as any change of a field's type is (ADR 0044). The type has no bound of its own to
  name: every size it has is a term of its two parameters (`J` in D9, `C` in D8, its storage in D12), which ADR 0118
  D3 reads where it is accepted.
- **A physical quantity may be a decimal.** A decimal field or `<data>` may carry `sce:quantity`, and the unit checks
  of the physical-quantity annotation hold of it as of any number: two decimals of different units are not combined.
  `sce:scale` and `sce:offset` on a decimal are refused (`validation/attribute-rule-violated`): its scale is in its
  type, and the annotation's physical accessor, computed in binary64, would round what the type holds exactly. A
  decimal accessor of a raw integer at a power-of-ten scale is not in scope.
- **Where it stands.** Every position of the grammar: an event schema's field, a `sce-static` variable, an algorithm's
  parameter, return and local, a list's element, an optional's value, a record's or a union case's field, a map's key
  and value, a set's key, a CBOR codec's entry. It composes as every scalar does.

### D2. One representation per value, and the order of the numbers

- **A value is its coefficient.** A value of `decimal(p, s)` is held as `c`, the integer `value · 10^s`. The fixed
  scale chooses one member of each cohort (IEEE 754 §2.1), so within a type two values are equal exactly when their
  coefficients are, and each canonical form (D8, D9) is a function of the value alone.
- **Comparison is the order of the numbers denoted, whatever the types.** `a < b`, `<=`, `>`, `>=`, `===` and `!==`
  of `decimal(p1, s1)` and `decimal(p2, s2)` compare `c1 · 10^(s - s1)` with `c2 · 10^(s - s2)`, `s = max(s1, s2)`:
  `2.5 === 2.50` across two types is true. It is total and its equality is exact. An integer compared with a decimal
  is the decimal of scale 0 its value is; a binary real compared with a decimal is refused (D7).
- **A decimal is a key.** ADR 0108 D1's condition, an exact equality and a bounded value set, holds, so `K` gains it:
  a map's keys are of one type and compare by coefficient, and a record whose fields are decimals is a key.
- **A view compares it by its number** (ADR 0151 D2, amended): "a decimal by the number it denotes" joins the list of
  comparisons, and is consistent with that record's rule that values of equal deterministic forms compare equal.
  Where a join, a group or a key relates two decimal columns of different types, the build widens both, exactly, to
  the least type that holds both (D3's alignment), so the index an operator keeps holds one representation per number.

### D3. The exact operations, and the type of each result

**The rule.** A result's precision is the number of digits of the largest magnitude the operation can return over the
values of its operands' types, rounded away from zero where the operation rounds (every mode is monotone, and the mode
may come from data, so the largest is reached under one of them); the build computes it from the operands' extremes.
The closed forms below are its usual values, and the build's computation, not the closed form, decides: `decimal(1, 0)
+ decimal(1, 1)` is `decimal(2, 1)` (its largest is `9.9`), where the closed form says `decimal(3, 1)`.

| Operation | Scale | Precision, usually | Fails |
|---|---|---|---|
| `-a` | `s` | `p` | never |
| `a + b`, `a - b` | `s = max(s1, s2)` | `max(p1 - s1, p2 - s2) + s + 1` | only past 38 digits |
| `a * b` | `s1 + s2` | `p1 + p2` | only past 38 digits |
| `c ? a : b` | `max(s1, s2)` | `max(p1 - s1, p2 - s2) + max(s1, s2)`, the least type that holds both | never |
| `a < b`, `<=`, `>`, `>=`, `===`, `!==` | | `bool` | never |

- **Exact by its type.** A sum of two values of `decimal(9, 2)` is at most `2 · (10^9 - 1)` in hundredths, which
  `decimal(10, 2)` holds and `decimal(9, 2)` does not; a product's coefficient is below `10^(p1 + p2)`. SQL fixes the
  same scales and leaves the precisions open (ISO/IEC 9075-2, `<numeric value expression>`); this record fixes them, so
  that a result is exact by its type and the build, not the run, knows its range.
- **Computed at its result type, not at its operands'.** `SCE_FORGE.md` §3.4 ("Typing convention") and ADR 0133 D1
  item 1 compute an integer operation at its operands' width, so a sum of two `uint8` values can fail though a
  `uint16` place would hold it. A decimal operation is computed at the result type of the rule, wider than either
  operand. The divergence is deliberate: a precision is not a machine width (every lane computes a decimal in 256 bits,
  D4, whatever carrier holds it), and a result type wide enough for every result is SQL's rule for exact numerics. The
  amendment of §3.4 says so where the convention is stated.
- **An integer operand**, and an integer arm of a conditional, is the `decimal(d, 0)` its type's values fit, `d` the
  digits of the type's largest magnitude: 3 for `int8` and `uint8`, 5 for the 16-bit types, 10 for the 32-bit ones, 19
  for `int64` and 20 for `uint64`.
- **Past 38 digits** a result has no type of its precision: its type is `decimal(38, s)`, and the operation is
  checked: it fails `overflow` where its value has more than 38 digits, unless the build proves it cannot (D5), as an
  integer operation past its width does (`SCE_FORGE.md` §3.4.1). Where that capped type has fewer integer digits than
  each of its operands (`decimal(38, 20) * decimal(38, 18)` is `decimal(38, 38)`, with none, so `5.0 * 3.0` would fail
  at run time), the operation is refused where the document is read (`expression/decimal-rounding-undeclared`): a
  result that cannot hold the operands' own magnitudes is the sign of fraction digits the document does not need, and
  the refusal names `roundingMul`, which keeps the integer digits by rounding the fraction (D4).
- **Past 38 fraction digits** a result has no type at all: `a * b` whose `s1 + s2 > 38` is refused
  (`expression/decimal-rounding-undeclared`), naming `roundingMul`.
- **`/` and `%` are refused on a decimal operand** with the same code: a quotient of two decimals in general has no
  finite decimal expansion, so a division is a rounding, and the language rounds only where a document says how
  (D4). `%` is not offered ("Not in scope").
- **A decimal is no operand of the integer operators.** A bitwise operator, a shift, a wrapping or saturating operation
  (`SCE_FORGE.md` §3.4.2, whose list of what is not an operand gains it), an index and `len` refuse a decimal operand
  (`expression/type-mismatch`).

### D4. A rounding is named where it happens

- **The modes are a closed enum of the standard library**, `sce:std/decimal/rounding.scxml`, `enum:Rounding` where a
  document imports it so, carrier `uint8`: `ties_to_even` (0), `ties_to_away` (1), `toward_positive` (2),
  `toward_negative` (3), `toward_zero` (4), IEEE 754's five rounding-direction attributes (§4.3) by their own names.
  For the exact value `r` and a scale `t`, with `x = r · 10^t`: `toward_negative` is `floor(x)`, `toward_positive`
  `ceil(x)`, `toward_zero` `floor(x)` for `x >= 0` and `ceil(x)` below; `ties_to_even` and `ties_to_away` are the
  nearest integer, a tie going to the even integer and to the one of larger magnitude. A mode is a value, so a document
  may read one from its configuration or a fact, as Java's `RoundingMode` is passed.
- **Three operations round a decimal, each once.** Each computes the exact result and rounds it to the scale `t` by
  the mode, as IEEE 754 requires of every operation, and its type is D3's rule (the closed forms are the usual values):
  - `quantize(x, t, mode)`, IEEE 754's `quantize` (§5.3.2) within the decimal numbers, its exponent written as a
    scale: `x` at scale `t`. Of `decimal(p, s)`, usually `decimal(p + t - s, t)` where `t >= s` (exact) and
    `decimal(p - s + t + 1, t)` where `t < s` (a rounding may carry into one more digit: `9.99` to `10.0`).
  - `roundingMul(a, b, t, mode)`: the exact product, even past 38 digits, rounded once to `t`. A rate times an amount
    rounded to cents is `roundingMul(rate, amount, 2, Rounding.ties_to_even)`.
  - `roundingDiv(a, b, t, mode)`: the exact quotient rounded once to `t`; its magnitude is below `10^(p1 - s1 + s2)`,
    so usually `decimal(p1 - s1 + s2 + t + 1, t)`.

  A result past 38 digits is `decimal(38, t)` and the operation is checked, as in D3. The names are those of the
  wrapping and saturating operations' family (ADR 0063), so `roundingMul` is read beside `wrappingMul` and
  `saturatingMul`, and no plain word (`multiply`, `divide`) that a document may already use as a callable is taken.
  `quantize`, `roundingMul`, `roundingDiv`, and D7's `toDecimal`, `toFloat64` and `toFloat32` join the expression
  vocabulary, and ADR 0063 §4 refuses a callable spelled as one of them (`validation/duplicate-id`); a value so spelled
  is not refused.
- **`t` is a whole-number literal from 0 to 38**, since it is part of the result's type and the build must know it; a
  `t` that is not is refused (`expression/decimal-rounding-undeclared`, the scale of a rounding being part of its
  declaration).
- **One rounding, never two.** `roundingMul` is not `quantize` applied to rounded operands: rounding the operands
  first, or rounding to an intermediate scale, can give another result (double rounding), and neither operation does
  it. `roundingMul(1.005, 1.1, 2, Rounding.ties_to_even)` is `1.11` (the exact product is `1.1055`), where the operand
  rounded first to `1.00` gives `1.10`. Where `a * b` has a type, `quantize(a * b, t, mode)` and
  `roundingMul(a, b, t, mode)` are one value.
- **No mode is ambient.** There is no context, no default and no attribute that sets a rounding for a region: the
  reader of an expression sees every rounding it makes, and its value does not depend on where it is evaluated.
- **Nothing rounds without being asked.** A value stored where fewer fraction digits are declared, a decimal of
  positive scale stored where an integer is declared, and `/` on a decimal are each refused
  (`expression/decimal-rounding-undeclared`), the refusal naming the operation that would say how; a change of radix
  in either direction is refused unless it is named (D7). SQL leaves the store of more fraction digits to round or to
  cut as the implementation chooses; that choice made silently is the error this type exists to remove.
- **No binary floating-point arithmetic is used, and no intermediate overflows.** Alignment, sum, difference and
  product are computed in 256 bits (an aligned value or a product of two coefficients is below `10^76 < 2^253`); a
  comparison is computed so, or without alignment, the integer part first. A 128-bit alignment would overflow on
  legal operands: `decimal(38, 0)` aligned to scale 1 reaches `10^39 - 10 > 2^127`, and `10^37 + (-10^37 + 0.5)` is
  `0.5` though its aligned operands do not fit 128 bits. `roundingDiv` divides by long division over 64-bit limbs
  (Knuth 1997, §4.3.1, Algorithm D), taking the aligned numerator's digits as it goes and never building it whole (it
  reaches `10^114`), and its remainder decides the rounding.

### D5. What is total, what is checked, and how each fails (ADR 0133)

The rule of ADR 0133 D1 holds of decimals as it holds of integers, over coefficients. A statement is total exactly
where the build proves that nothing it computes or stores can fail.

- **Total by its types:** unary `-`; every comparison; a conditional; `+`, `-` and `*` whose result type is within
  38 digits; `quantize` and `roundingMul` whose result type is within 38 digits; `toFloat64` and `toFloat32` (D7); and a
  store of `decimal(p, s)` into a place of `decimal(pt, st)` that holds every value of it, `st >= s` and
  `pt - st >= p - s` (a widening, ADR 0133 D1 item 4). An integer stored where a decimal is declared is total when
  `pt - st` is at least its type's digits (D3).
- **Checked, unless the interval analysis proves it:** an operation whose result type was capped at 38 digits
  (`overflow`); `roundingDiv` (`divide-by-zero` where the divisor's range holds 0, `overflow` where its type was
  capped); a store into a place of fewer integer digits, `pt - st < p - s` (`overflow`, as an integer store into a
  narrower type); a decimal of scale 0 stored where an integer is declared whose range the integer type does not hold
  (`overflow`); `toDecimal` (`precondition` for a value that is not finite, `overflow` past its type, D7). The
  analysis (`int_ranges::proven_operations`) holds a decimal's range as an interval of coefficients at its scale and
  narrows it by the conditions of its expression, so `b !== 0 ? roundingDiv(a, b, 2, Rounding.toward_zero) : a` is
  proven; where its own arithmetic cannot hold a bound, the operation is not proven and stays checked, which is sound.
- **Refused where the document is read:** the undeclared roundings of D3 and D4, a change of radix not named (D7), a
  decimal operand of an integer operator (D3), and a literal that is no value of the type it takes (D6). In an
  algorithm that does not declare `may-fail`, a decimal operation the analysis does not prove is refused as an integer
  one is (`algorithm/undeclared-integer-failure`, `SCE_FORGE.md` §3.4.1).
- **One classification.** The static lowering writes the result on the statement (`Action::native_fails`), and every
  reader of a failure reads it there (ADR 0133 D2). The failures are the names `SCE_FORGE.md` §3.4.1 already gives,
  `overflow`, `divide-by-zero` and `precondition`: no new name enters any runtime's vocabulary.
- **Printed as an integer operation is** (ADR 0133 D3): on Rust a proven decimal operation through
  `sce_forge_runtime::algorithm::proven::decimal`, a checked one through `sce_forge_runtime::algorithm::decimal`,
  returning `AlgorithmError` in an algorithm that declares `may-fail` and raising `error.execution` in a statechart.

### D6. A literal is read from its text

- **A literal in positional notation is a decimal where a decimal meets it.** Digits with an optional point and
  digits, and no exponent, is a decimal where it meets a decimal operand, lands in a decimal place or is passed to a
  decimal parameter. It is read from its source spelling, never through a binary number, and is of the type its text
  writes: `decimal(max(1, i + f), f)`, `i` the digits before the point less the leading zeros and `f` the digits after
  it. `95.0` is `decimal(3, 1)` and `0.025` is `decimal(3, 3)`. The scale is SQL's rule for an exact numeric literal,
  and the precision this record's (SQL leaves it to the implementation). A leading minus is part of the literal, as
  for an integer.
- **An operation of literals alone is typed by where it lands**, as an integer operation of literals alone is
  (`SCE_FORGE.md` §3.4): `0.1 + 0.2` stored where a decimal is declared is a decimal throughout, and is exactly `0.3`.
- **A hexadecimal, octal or binary literal** that meets a decimal is the `decimal(d, 0)` of its value, `d` its digits
  in decimal.
- **A literal is the author's own value, and is not rounded.** Where it lands in a place, it must be a value of the
  place's type: `2.505` in a `decimal(9, 2)` place is refused (`expression/literal-out-of-range`), as `300` in a
  `uint8` place is.
- **An initial value is held to the same rule.** A `<data expr>` of a decimal variable is a literal judged so, and an
  event schema field's `sce:default` of a decimal type is its canonical text (D9), refused where it is no value of the
  type.
- **A constant folds by the proved arithmetic.** A `<sce:const>` whose value is a decimal expression is folded by the
  build through the same module the runtime runs (D12), so there is no second arithmetic.
- **What stays a binary real.** A literal with an exponent (`1.5e3`) where a decimal is expected is refused
  (`expression/type-mismatch`): SQL calls such a literal approximate, and in SCE's expressions it is a binary real. A
  literal of more than 38 digits is refused (`expression/literal-out-of-range`). A literal with a point that meets no
  decimal is a real, as today, so no document written before this record changes its meaning.

### D7. Conversions

- **An integer becomes a decimal** where a decimal is declared, exactly (D5 says when the store is total).
- **A decimal becomes an integer** only at scale 0: a `decimal(q, 0)` stands where an integer is declared, checked
  unless the integer type holds `10^q - 1` and its negative. A decimal of positive scale where an integer is declared
  is an undeclared rounding (D4), and the refusal names `quantize(x, 0, mode)`.
- **A binary real becomes a decimal** only by `toDecimal(f, t, mode)`: the exact value of the binary number (every
  finite binary number is a finite decimal fraction) rounded once to the scale `t`, IEEE 754's `convertFormat` between
  radices (§5.4.2). It is a function of its own, not `quantize`: IEEE 754's `quantize` is within one format and
  cannot fail for a value that is not a number, where this conversion can. A NaN or an infinity fails `precondition`,
  the conversion's domain being the finite reals, which is IEEE 754's invalid operation (§7.2) under the house's name
  for an input outside a function's domain; a finite value past the type fails `overflow`; `toDecimal(-0.0, t, mode)`
  is `0`, a decimal having no negative zero. A binary real where a decimal is declared is refused
  (`expression/decimal-rounding-undeclared`), naming `toDecimal`.
- **A decimal becomes a binary real** only by `toFloat64(x)` or `toFloat32(x)`: the nearest binary64 or binary32, a
  tie to even, the correctly rounded conversion (IEEE 754 §5.4.2; Clinger 1990), total and the same on every lane. A
  decimal where a real is declared is refused (`expression/type-mismatch`), its fix naming the conversion: the store
  would be a rounding the document did not write, which D4 refuses in every other direction.
- **A decimal and a binary real in one arithmetic operation are refused** (`expression/type-mismatch`): the operation
  would have to choose a radix, and either choice is a rounding the document did not write.
- **A decimal and a binary real in one comparison are refused** (`expression/type-mismatch`) for a reason of their
  own, since an exact comparison of the two exists (Python's `decimal` compares them so): the binary operand stands
  for a decimal it is not (the binary64 written `0.1` is above one tenth), so `x >= 0.1` with a binary `0.1` judges
  a decimal `x = 0.1` on the wrong side, the error the request describes. The document converts one side by name.
- **A decimal joined to a string is its canonical text** (D9), the same on every lane, as an integer joined to a string
  is its digits (`SCE_FORGE.md` §3.4, "String concatenation", which gains the decimal among the operands that join).

### D8. Its CBOR form is one decimal fraction, whose exponent the scale fixes

- **The encode writes `4([-s, c])`**: RFC 8949's decimal fraction (§3.4.4), tag 4 over an array of the exponent `-s`
  and the mantissa `c`, in ADR 0101's core deterministic encoding: every head shortest, definite lengths, and the
  mantissa in preferred serialization (§3.4.3, §4.1, §4.2.2): an integer of major type 0 or 1 where
  `-2^64 <= c <= 2^64 - 1`, and a bignum (tag 2, or tag 3 below zero) of no leading zero byte otherwise. The exponent
  is the type's, so the bytes are a function of the value: one form per value, as ADR 0101 §2.1 asks of every type.
  The exponent is written though the schema fixes it, so a reader that does not hold the schema (a follower of another
  system, a diagnostic tool, RFC 8949's own diagnostic notation) reads the number, and the form is the one a downstream
  consumer named.
- **Vectors** (hexadecimal; each a case of the corpus, "Order of work" step 6):

  | Value | Type | Encoding |
  |---|---|---|
  | 2.50 | `decimal(9, 2)` | `c4 82 21 18 fa` |
  | -2.50 | `decimal(9, 2)` | `c4 82 21 38 f9` |
  | 0.00 | `decimal(9, 2)` | `c4 82 21 00` |
  | 95.0 | `decimal(4, 1)` | `c4 82 20 19 03 b6` |
  | 0.025 | `decimal(5, 3)` | `c4 82 22 18 19` |
  | 7 | `decimal(5, 0)` | `c4 82 00 07` |
  | 2^64 - 1 | `decimal(20, 0)` | `c4 82 00 1b ff ff ff ff ff ff ff ff` |
  | 2^64 | `decimal(20, 0)` | `c4 82 00 c2 49 01 00 00 00 00 00 00 00 00` |
  | -2^64 | `decimal(20, 0)` | `c4 82 00 3b ff ff ff ff ff ff ff ff` |
  | -(2^64 + 1) | `decimal(20, 0)` | `c4 82 00 c3 49 01 00 00 00 00 00 00 00 00` |
  | 10^38 - 1 | `decimal(38, 0)` | `c4 82 00 c2 50 4b 3b 4c a8 5a 86 c4 7a 09 8a 22 3f ff ff ff ff` |
  | -(10^38 - 1) | `decimal(38, 0)` | `c4 82 00 c3 50 4b 3b 4c a8 5a 86 c4 7a 09 8a 22 3f ff ff ff fe` |

- **In CDDL** (ADR 0101 §2.6): `decimal(9, 2)` is written `#6.4([-2, -999999999..999999999])`; what a decode reads is
  RFC 8610's `decfrac` whose value is a value of the type.
- **The decode is variation-tolerant within ADR 0101's profile** (its §3.1, §3.2 and §3.4): it reads every tag 4 item
  whose exponent `e` is an integer and whose mantissa `m` is an integer or a bignum, and whose value `m · 10^e` is a
  value of the type, since a sender that wrote it otherwise said the same number. Each of these is a case of the
  corpus:
  - another exponent: at `decimal(9, 2)`, `c4 82 20 18 19` (`4([-1, 25])`) and `c4 82 22 19 09 c4` (`4([-3, 2500])`)
    are each 2.50;
  - a head in a longer form, the tag's own included: `d8 04 82 21 18 fa` (tag 4 in a two-byte head), and a bignum tag
    so written (`d8 02`), are read;
  - a bignum's leading zero bytes, at any length: `c4 82 21 c2 42 00 fa` is 2.50, its value deciding and not its
    length, as RFC 8949 §3.4.3 requires of a decoder.

  It refuses, by the kinds ADR 0101 §3.3 already names in every runtime:
  - an item that is not tag 4 (a bare integer, a float, a bigfloat of tag 5, any other tag), a tag 4 whose content is
    not an array of two items, an exponent that is not of major type 0 or 1 (RFC 8949 §3.4.4 allows no bignum there,
    so `c4 82 c3 40 18 fa` is refused), and a mantissa that is neither an integer nor a bignum: `CborWrongType`;
  - a value with more fraction digits than `s` (`4([-3, 2505])` at `decimal(9, 2)`), or of magnitude past
    `10^p - 1` at scale `s`: `CborOutOfRange`, as an integer outside its width is;
  - a mantissa whose value is past the 128-bit two's-complement range, whatever its length in bytes:
    `CborOutOfRange`. This is the profile's one narrowing, stated as ADR 0101 states its definite lengths: every value
    of every decimal type has an encoding inside it (`10^38 - 1 < 2^127`), and a decode then holds no more than the
    value's own width whatever the sender wrote, as ADR 0101 §4.3 asks ("Storage is bounded by the type alone"). The
    exponent then needs no bound of its own: past 38 either way, only a zero mantissa is a value, and it is zero.

  No new refusal kind enters a runtime's vocabulary. ADR 0101 §3.3's row that refuses a tagged item is amended to
  read: a tagged item, except tag 4 where a decimal stands and tags 2 and 3 as its mantissa. It still refuses a bignum
  where an integer type stands: `c2 41 05` at `int64` is `CborWrongType`, a case of the corpus.
- **Its size** (ADR 0101 §4.1, ADR 0086 §2.1): `C(decimal(p, s)) = 2 + E(s) + M(p)`, where `E(s)` is 1 for `s <= 24`
  and 2 above, and `M(p)` is `h(10^p - 1)` for `p <= 19` and `2 + L(p)` above, `L(p)` the bytes of `10^p - 1` (9 at
  `p = 20`, 16 at `p = 38`). `decimal(9, 2)` takes 8 bytes, `decimal(20, 0)` 14 and `decimal(38, 0)` 21. It is tight:
  a value of largest magnitude is that long.
- **The signed form is the same bytes** (ADR 0086 §9): a record a trust layer signs holds a decimal as the encode
  writes it.
- **In the model** (ADR 0115 D7) a codec is a partial function of values: a decimal's encode refuses no value of its
  type, so `Refuses_C` gains no disjunct for it.

### D9. Its JSON spelling is its canonical text, as a string

- **The canonical text** of a value of `decimal(p, s)` is a minus sign for a negative value, the integer part in ASCII
  digits with no leading zero (a lone `0` for a value below one), and, where `s > 0`, a point and exactly `s` digits.
  Zero has no sign. It is the coefficient written at its scale: `"2.50"`, `"-2.50"`, `"0.00"`, `"95.0"`, `"0.025"`,
  `"7"`. It is the one member of the cohort the type holds, written, so the JSON and the CBOR forms name the same
  representation.
- **It is a string, not a JSON number.** A reader that takes a JSON number through a binary64 (RFC 8259 §6; I-JSON,
  RFC 7493 §2.2), as an ECMAScript `JSON.parse` does, turns `0.1` into the binary number this type exists to keep out,
  and `10^20` into one that has lost its digits. The house already writes a 64-bit integer as its digits for the same
  reason (Context).
- **The reader takes only the canonical text.** The 64-bit integer's reader also takes a JSON number up to
  `2^53 - 1`, because every integer in that range is exact in a binary64; no such range exists for a fraction (`0.1` is
  inexact), so a decimal's reader takes no JSON number at all. Within a string it takes the one spelling, as the
  integer's reader refuses `"007"`: `"2.5"` and `"2.500"` at scale 2, `"+2.50"`, `"02.50"`, `"-0.00"`, `".50"`, `"2."`,
  an exponent, a space and a value past the type are each refused where the field is read, naming its place. A typed
  value read from JSON and written again is the same bytes. ADR 0087 §1 lets the lift accept more than the grammar
  writes for other types; for a decimal it accepts exactly what the grammar writes.
- **Where it is written:** a typed payload's field in `_event.data`, a `<param>` and a `<donedata>` pair, the data a
  typed raise writes, a host's completion record (`sce:result`), the record codec's JSON (D10), the saved state of a
  `sce-static` machine and its snapshot's text; `schemas/sce-saved-state.v4.schema.json`'s description of a value gains
  "a decimal is a text of its canonical spelling".
- **Its grammars** (ADR 0087 §1, a row of its table of `S(T)` and `G(T)`): `S` is `{"type":"string","pattern":P}` and
  `G` the quoted text, both written as ADR 0087 writes an integer's range, by digit-prefix alternatives and with no
  lookaround (a GBNF grammar has none): zero at its one spelling, then one alternative per position of the first
  non-zero digit, each with an optional minus. For `decimal(9, 2)`:
  `^(0\.00|-?0\.(0[1-9]|[1-9][0-9])|-?[1-9][0-9]{0,6}\.[0-9]{2})$`.
- **Its size** (ADR 0086 §2.1): `J(decimal(p, s)) = 3 + max(1, p - s) + (s + 1 where s > 0)`, the quotes, the sign,
  the integer digits and the fraction: 13 for `decimal(9, 2)` (`"-9999999.99"`), tight.
- **Shown to a person** through ADR 0158's host formatting, `:number` takes a decimal field, its operand the canonical
  text, and its default minimum and maximum fraction digits are the field's scale, which a message's own options may
  override: MessageFormat 2's `:number` with ICU's defaults would print `1.50` as `1.5` (`v` = 1). So CLDR's plural
  operands come from the declared scale: `1.50` is shown `1.50` and selects `other` in English, where the integer `1`
  selects `one`. The table of ADR 0158 that lists `:number` for "an integer or a real field" gains "or a decimal
  field, its fraction digits by default its scale".

### D10. Payloads, codecs, records and the journal

- **An event schema's field** of a decimal type is read and written in the JSON spelling of D9; a `<send>` of the
  event to another machine holds the sent value to the field's type as D5 holds a store (ADR 0050's send rule).
- **A CBOR codec's entry** of a decimal type is written and read as D8 says, on every lane that prints CBOR codecs and
  serves the type (D12).
- **The record codec describes it.** `payload_record::describe` gains the row `{"decimal":{"precision":p,"scale":s}}`,
  and `sce_rust_runtime::record_cbor` a `Shape::Decimal` that maps the canonical text (D9) to the decimal fraction (D8)
  and back, refusing a text or an item that is no value of the type. The description serves every reader of a record
  by its description: a journal entry, an entry a pool follows, a page of a journal read, and a signed or sealed record
  (`sce:payload`, docs/SCE_HOST_FEATURES.md "Signing and sealing").
- **So a fact holds a decimal.** An entry is its record's deterministic CBOR (ADR 0114 D1), and a record of decimal
  fields has one, so `<send type="sce:journal">` takes it, the follower reads it through the description, and a fact
  applied, followed, read in a page or restored holds the same number. Nothing in ADR 0114 changes.
- **Every place the float refusal for a fact is written**, found by reading the tree, and what becomes of each:
  - `sce-build/src/forge/payload_record.rs`: `describe_scalar`'s arm for `Float32 | Float64`, the message a
    downstream consumer quoted; the doc of `describe`, which names the float and the open enum as the two kinds refused;
    and the test `a_float_is_refused_since_no_payload_spells_one_that_is_not_finite`;
  - `sce-build/src/forge/journal.rs`: `entry_record`, which hands `describe`'s reason on; `resolve`'s refusal of a
    `<send type="sce:journal">` whose record "has no deterministic CBOR an entry could hold"
    (`scxml/static-datamodel-rule`); and the check of each case of a journal read's `sce:entries` union ("holds a record
    with no deterministic CBOR a fact is kept as");
  - `sce-build/src/mesh/topology.rs`: the check of the entries a machine follows ("whose record has no deterministic
    CBOR an entry is kept as") and `resolve_keyed_pool`, which keeps each followed record's description
    (`FollowedRecord`);
  - `sce-build/src/forge/payload_record.rs` `resolve`, through `describe`, for `sce:payload`
    (`validation/payload-record-schema`);
  - the runtimes' second line: `backends/rust/runtime/src/record_cbor.rs` (its module doc, and the `Shape::Float` arm,
    "is a float that is not finite, which no payload spells") and `sce/include/common/RecordCbor.h` (the same refusal on
    C++);
  - the documents that state it: `docs/SCE_HOST_FEATURES.md` (the record codec's refusals at parse and "One set of
    bytes with the printed codec"), `docs/SCE_ACCEPTED_SUBSET.md` (the refusals of `validation/payload-record-schema`)
    and `docs/SCE_CODEC_CONSUMER_MIGRATION.md`.

  Each refusal stays: a binary float still has values no payload spells, and a decimal is the answer to the need, not
  a weakening of the reason. `describe_scalar`'s message gains one sentence naming the type a number written in decimal
  is declared with, `decimal(p, s)`, so the refusal says what to write; the documents gain the decimal among the kinds
  the record codec describes.

### D11. A view orders and sums decimals exactly (ADR 0151)

- A `min`, a `max`, an order and a key compare decimals by D2.
- A `sum` of a `decimal(p, s)` column is the integer sum of its coefficients, of the type `decimal(p + d, s)`, `d` the
  digits of the group's largest count (D8 of ADR 0151), proven by the build as an integer sum is; past 38 digits it is
  refused (`view/aggregate-may-overflow`). It is linear and exact, so a removed row is subtracted and the sum has the
  same bytes maintained and recomputed, with no accumulator beside it.
- The reference evaluator (`sce-view-reference`, ADR 0151 D11) and its laws take decimals as they take integers.

### D12. Rust serves it; the lane-neutral surfaces serve it; every other lane refuses it by name

- **Rust**, native, `wasm32-unknown-unknown` and `no_std`, with no heap:
  - the value and its arithmetic are a leaf `no_std` crate, `sce-decimal` (`backends/rust/decimal`), with no
    dependency, re-exported by `sce-portable-bytes` as `sce_portable_bytes::decimal`, so the proved module is alone in
    the unit that compiles it (its `compiled_by` in `docs/formal/proofs.json`). The value is `Decimal32<P, S>`,
    `Decimal64<P, S>` or `Decimal128<P, S>`, the coefficient stored in the narrowest of `i32` (`p <= 9`), `i64`
    (`p <= 18`) and `i128`, its parameters `const` generic parameters, so the type carries its precision and scale and
    a value outside them cannot be built (ADR 0086 §7.4's reasons for a bound held in a type). It is `Copy`, its
    `Eq`, `Ord` and `Hash` are the coefficient's, its `Display` is the canonical text and its `FromStr` the strict
    reader of D9. Its storage is 4, 8 or 16 bytes;
  - the arithmetic is written once in that crate, and the three types call it: the alignment, the exact sum,
    difference and product and the comparison in 256 bits, the long division over 64-bit limbs, the five roundings,
    and the conversions of D7 (D4). Storage is 128 bits at most; no intermediate is computed in 128 bits where it could
    pass them. It is the code D13's theorems are of, and the build's constant folding (D6) calls it;
  - `sce_forge_runtime::algorithm::decimal` and `::proven::decimal` print a checked and a proven operation (D5);
    `sce_forge_runtime::cbor` writes and reads D8; `sce_rust_runtime`'s JSON, saved state and `record_cbor` read and
    write D9 and D10;
  - the generated machine (the static data model), the schema unit's payload struct, an algorithm and a CBOR codec
    print it.
- **The lane-neutral surfaces serve it now**, since they are what every lane is held to: the payload schema and its
  GBNF form (D9), the conformance corpus (D8, D9 and the kit of D13), and the page surface (ADR 0117 D7,
  `sce-build/src/page_surface.rs`), which types a decimal as a string through the payload schema and is held by
  `backends/rust/browser-node/surface/pages/right.mts` (a page that reads a decimal field as a string) and
  `wrong.mts` (one that reads it as a number, which the check refuses).
- **Kotlin, Go, Python, Swift, C++ (its machine and its record codec), C11, and the Interpreter refuse it by name**:
  `generate/unsupported-feature` naming the place, the type and the lanes that serve it, read from one table,
  `DECIMAL_BACKENDS` in `sce-build/src/forge/codegen_matrix.rs` (`&[Language::Rust]`), as
  `ALGORITHM_LISTED_RECORD_BACKENDS` is read; the conformance kit's `lang_supports_fixture`
  (`sce-build/src/conformance.rs`) reads the same table, so a decimal fixture runs exactly where it generates. A C++
  machine whose `sce:payload` record holds a decimal is refused by the same code, since the C++ record codec does not
  describe one. The TTCN-3 export (ADR 0101 §9; `sce-build/src/ttcn3_export/`) refuses a decimal by name through its
  `refuse` (`generate/unsupported-feature`), and so does the test profile that it exports. The JANI model (ADR 0119,
  `sce-build/src/formal/document_jani.rs`) refuses it by name (`formal/no-translation`). Each lane is a step of its
  own when a consumer needs it; the corpus is language-neutral, so a lane that comes is held to the bytes and the
  refusals Rust is held to.
- **Every match over the type gains its arm, by a closed list.** `SceType` (or `TypeOf`) gains a constructor, so every
  exhaustive match over it is a compile error until its arm is written, and a match with a wildcard arm is read and
  given a named one in step 2. The places that match over the type, found by searching `sce-build/src` for the
  `float64` scalar and the text constructor, and the arm each gains:
  - **serves it:** `forge/value_type.rs` (grammar, keys, `J`, `C`, `S`, `G`), `forge/model.rs`, `forge/parser.rs`,
    `forge/types.rs`, `forge/type_ctx.rs`, `forge/expr.rs`, `forge/const_fold.rs`, `forge/static_lowering.rs`,
    `forge/encode_plan.rs`, `forge/cbor_codec.rs`, `forge/payload_record.rs`, `forge/payload_schema.rs`,
    `forge/journal.rs`, `forge/lift.rs`, `forge/schema_revision.rs`, `forge/schema_unit_type.rs`,
    `forge/event_schema_check.rs`, `forge/text_bound.rs`, `forge/value_path.rs`, `forge/sensitivity.rs`,
    `forge/import_use.rs`, `forge/static_imports.rs`, `forge/native_action.rs`, `forge/host_invoker_interface.rs`,
    `forge/audit_entry.rs`, `forge/model_answer.rs`, `forge/text_template.rs`, `forge/composed_lowering.rs` with
    `composed_lowering/rust.rs`, `forge/rust_algorithm_map.rs`, the Rust arms of `forge/generator.rs`,
    `forge/generator/rust_test_tables.rs`, `forge/generator/rust_test_template.rs`,
    `forge/generator/test_template_walk.rs`, `mesh/topology.rs`, `mesh/deploy_config.rs`, `scenario_replay.rs`,
    `lib.rs`, `conformance.rs` and `page_surface.rs`;
  - **refuses `sce:scale` and `sce:offset` on it:** `forge/quantity_codegen.rs` (D1);
  - **refuses it by name:** `forge/composed_lowering/{c11,cpp,go,kotlin,python,swift}.rs`, the other lanes' arms of
    `forge/generator.rs`, `forge/generator/{c11,cpp,go,kotlin,python,swift}_test_tables.rs` and `_test_template.rs`,
    `forge/generator/c11_scenario_table.rs`, `forge/generator/cpp_source/cbor.rs`,
    `forge/generator/kotlin_source/cbor.rs`, `forge/static_js.rs` and `forge/static_js_algorithm.rs` (the
    Interpreter), `forge/test_profile.rs`, `forge/test_template_check.rs`, `test_component/rules.rs`,
    `ttcn3_export/{types,templates,behaviour,trace_check}.rs`, `formal/document_model.rs` (D13) and
    `formal/document_jani.rs`.

### D13. How each claim is judged

The levels are ADR 0115's: a function proved (level 6, D10 and D13 there), a model per document (level 4, D7), and the
kit that holds the lanes. A claim of arithmetic is of every value of an infinite family of types, so it is a function's
proof; a claim of what a document does with its decimals is a property of that document's model; and agreement of the
lanes is the kit's.

| Claim | Level | Why that level |
|---|---|---|
| `+`, `-`, `*` and the comparison give the exact result, aligned, for every value of their types, and no intermediate overflows | Verus, T1 (ADR 0115 level 6) | a sample of an arithmetic claim misses its edges; the proof is of the Rust that runs |
| each result type holds every result, and is the least that does (D3, D4) | Verus lemmas beside T1, and an exhaustive test of the build's rule | the build's rule is a function of two finite parameters, 779 types and every pair of them, so a test of every one is complete; the lemmas say why it holds |
| each of the five roundings gives what its definition says, in `quantize`, `roundingMul`, `roundingDiv` and `toDecimal` | Verus, T2 (level 6) | as T1 |
| the comparison is a total order, the order of the numbers | Verus, T3 (level 6) | as T1 |
| `toFloat64` and `toFloat32` give the nearest binary number, a tie to even | generated cases against Python's `float(Fraction(...))`, which rounds a rational correctly | a correctly rounded binary conversion is a property of the platform's parser the crate calls, which Verus does not hold |
| one text and one CBOR form per value; a decode reads back the value an encode wrote; a decode accepts exactly the values of the type, and refuses the rest by name | the conformance corpus (vectors, the variants of D8, every refusal) and generated cases: a property, ADR 0058 §5's law 1 | the codecs of the house are held at this level on every lane (ADR 0101, "Conformance"); a proof of the Rust codec alone would not hold the next lane |
| every lane agrees with exact arithmetic | the kit: `tests/forge/conformance/gen_cases.py`, a model over Python's integers and `fractions.Fraction`, held also to Python's `decimal` module | the method `gen_cases.py` states: a model whose numbers have no width cannot share a lane's overflow, and a second implementation of the General Decimal Arithmetic Specification is a witness the model did not write |
| what a document does with its decimals (a value on its target is judged on the target's side) | the document's generated model, judged by TLC (ADR 0115 D7, D15), and the runs replayed against it (D8) | a property of one document is a property of its model; a decimal translates exactly where TLC's integers hold it |

- **The proofs** are of the crate of D12, entered in `docs/formal/proofs.json` as `client_mailbox.rs` and
  `fresh_step.rs` are, each with the changes Verus must refuse: a tie under `ties_to_even` rounded away from zero;
  `toward_zero` computed as a floor for a negative quotient; an alignment to the smaller scale; a comparison of
  coefficients not aligned; an alignment computed in 128 bits; and the `+ 1` taken out of `quantize`'s precision lemma.
- **How they are proved.** The specifications are over `int`: a value is `c / 10^s`, and `pow10` a recursive spec
  function with `vstd::arithmetic`'s power, `div_mod` and `mul` lemmas. A nonlinear goal is discharged
  `by(nonlinear_arith)` with its bounds given, an identity `by(integer_ring)`, and a carry of the 256-bit limbs
  `by(bit_vector)`. T2's conversion of a binary real is stated over its sign, significand and exponent; the
  decomposition of a `f64` or `f32` into them by `to_bits` is listed as trusted in `docs/formal/README.md`, and is
  backed by an exhaustive test over every one of the 2^32 binary32 values and by generated binary64 cases.
- **The model** (ADR 0115 D7, ADR 0156 D3 and D4). A `decimal(p, s)` is its coefficient, an integer; its `TypeOK` is
  `-(10^p - 1)..(10^p - 1)`; its operations are integer operators of `docs/formal/Decimal.tla`, written once beside
  `AppendixD.tla` and citing IEEE 754 §4.3 (an alignment by `10^k`, and `Round(n, d, mode)` by `\div` for each of the
  five modes); and an exploration names a decimal input's finite values as decimal texts, read exactly. TLC's integers
  are exact within `-2^31..2^31 - 1`, the range the generator already holds a literal to (`LARGEST_INTEGER` in
  `document_model.rs`). So the generator translates a decimal only where every coefficient the document's decimal
  expressions can take, after alignment, multiplication and rounding, lies in that range, as D5's interval analysis
  bounds them; otherwise it refuses the document by name, `Construct::Decimal` (`formal/no-translation`), saying that
  the coefficients pass TLC's integers. `decimal(9, 2)` fits (its coefficients are below `10^9 < 2^31`);
  `decimal(10, 2)` does not. ADR 0115 D15 refuses "a real number" because a binary real has no exact model; a decimal
  has one within those integers. Today `sce-codegen model` refuses every `sce-static` document whole
  (`Construct::Datamodel("sce-static")`), so a document that holds a decimal has no model and says why. When the typed
  translation of ADR 0156 D3 lands, it translates a decimal as above in the same step or refuses it by name with
  `Construct::Decimal` until it does; never a model of part of the document.
- **Replay** (ADR 0115 D8). A document that holds a decimal is generated on Rust alone, so its runs are replayed on
  Rust and against its model, and on no other lane; `docs/formal/README.md` says so.
- **No hand TLA+ model** is written of the type itself: it has no state, no concurrency and no protocol, and TLC's
  finite exploration would only sample what T1 to T3 state of every value.

### D14. Refusals, and each construct's account

One new `DiagnosticCode`, with its row in the appendix of `docs/SCE_ACCEPTED_SUBSET.md` added in the step that makes
it: **`expression/decimal-rounding-undeclared`**, "the value of `<expr>` at `<place>` would be rounded, or would lose
its integer digits, and no rounding is declared", its fix naming the operation that declares one (`quantize`,
`roundingMul`, `roundingDiv` or `toDecimal`). Every other refusal is an existing code at a new site:

| Construct | The build checks | Refused as | Bounds | Lanes | Model |
|---|---|---|---|---|---|
| the type `decimal(p, s)` | the grammar (D1), the positions, the key, `J` and `C` | `validation/attribute-rule-violated` (its parameters; `sce:scale` or `sce:offset` on it); `generate/unsupported-feature` (a lane that does not serve it) | none by name: `p` and `s` are the domain's (D1); `J`, `C` and the storage are terms of them | Rust and the lane-neutral surfaces; every other lane, the Interpreter, the TTCN-3 export and JANI refuse by name | its coefficient, within TLC's integers; `Construct::Datamodel` today, `Construct::Decimal` until translated or past them |
| `+`, `-`, `*`, `? :`, comparisons | result types (D3), classification (D5) | `expression/decimal-rounding-undeclared` (a product past 38 fraction digits or losing every integer digit, `/`, `%`); `expression/type-mismatch` (with a binary real, or as an operand of an integer operator); `algorithm/undeclared-integer-failure` (unproven, without `may-fail`) | none | as the type | integer operators of `Decimal.tla` |
| `quantize`, `roundingMul`, `roundingDiv` | arity, a literal `t`, a `Rounding` mode, result types, classification | `expression/argument-count-mismatch`; `expression/decimal-rounding-undeclared` (`t` not a literal from 0 to 38); `expression/type-mismatch` (a mode of another type); `validation/duplicate-id` (a callable so spelled) | none | as the type | `Round(n, d, mode)` |
| `toDecimal`, `toFloat64`, `toFloat32` (D7) | arity, operand types, `t` and mode as above | as the row above; `expression/type-mismatch` (a store of either radix where the other is declared) | none | as the type | `toDecimal` and the binary conversions have no translation: `Construct::Decimal` |
| `sce:std/decimal/rounding.scxml` | an enum of the standard library | `expression/unknown-enum-variant` | none | every lane reads an enum; it is read only where a decimal is | an enum value |
| a literal, an initial value, a constant (D6) | its text against its type | `expression/literal-out-of-range`; `expression/type-mismatch` (an exponent) | none | as the type | an integer |
| stores and conversions (D5, D7) | widening, narrowing, scale, radix | `expression/decimal-rounding-undeclared`; `expression/type-mismatch` | none | as the type | integer operators |
| the CBOR form (D8) | `C` | at decode, `CborWrongType` and `CborOutOfRange` | `C` | Rust | `Refuses_C` gains nothing |
| the JSON spelling (D9) | `J`, `S`, `G` | at read, the place of the field | `J` | Rust; the payload schema and the page surface | none: the model holds values |
| the record's description (D10) | `describe` | on C++, `generate/unsupported-feature` | `C` of the record | Rust | none |
| a view's decimal (D11) | ADR 0151's plan | `view/aggregate-may-overflow` | the sum's type | Rust (ADR 0151 D11) | ADR 0151 D7 |

## Not in scope

A decimal floating-point type (IEEE 754's `decimal64` and `decimal128`, with their NaNs, infinities, signed zeros and
cohorts); a precision past 38 digits; a negative scale; `%` and a remainder of decimals; powers, roots, logarithms and
every other function the expression language does not offer for integers either (an absolute value, a minimum and a
maximum among them, each added where the language adds it, with the same exactness); a rounding mode outside IEEE
754's five (half down, round-05-up); a rounding set for a region of a document; a decimal accessor of a raw integer
at a power-of-ten scale for the physical-quantity annotation (D1); a model of a document whose decimal coefficients
pass TLC's 32-bit integers, whose route is Apalache's unbounded integers (approved for ADR 0115 D3), a step of its own;
a number with a unit or a currency (an amount and its ISO 4217 code are an application's record of a decimal and a
text, and a currency's minor unit is its scale); formatting for a locale, which is the host's (ADR 0158); lifting the
refusal of a binary float where a fact is kept; and every lane but Rust, each its own step when a consumer needs it.

## Consequences

- A fact, a payload, a variable, a key and a codec's entry can hold a number written in decimal, and hold it exactly:
  `0.1` is `0.1`, a value on its target compares equal to it, and a sum's value does not depend on the order of its
  terms (whether an intermediate passes 38 digits, and so fails `overflow`, can).
- The scale is in the type, so the build knows the range of every result, refuses an operation that would round
  without saying how, and proves where nothing can fail; an application stops spelling a scaled integer of its own.
- A rounding is visible where it is made, by its scale and its mode, and happens once; a change of radix is visible
  where it is made, by its name.
- A journal entry that holds a decimal has one form of bytes, readable as a number without its schema, and a JSON
  payload carries it as text no binary64 reader can damage.
- A document's model holds a decimal as the integer it is, so a property of a judgement on a decimal is checked by
  TLC with no approximation, where its coefficients are within TLC's 32-bit integers; a document past them is refused
  a model by name.
- One new diagnostic code and six names of the expression vocabulary; no new refusal kind or failure name in any
  runtime; the float refusal keeps its reason and gains a pointer to the type.
- A lane other than Rust that meets a decimal refuses the document by name until its own step.

## Alternatives rejected

- **A scaled integer each application spells for itself.** The consumer refused it, and the textbook agrees: the scale
  is not in the type, so nothing checks that two operands share one, and every wire and every message shows the
  integer.
- **`sce:scale` with an `f64` accessor**, the physical-quantity annotation that exists. Its scale is a codec's
  conversion of a raw value, and its physical value is computed in binary64, which rounds what a decimal holds exactly;
  it stays what it is for, and a decimal refuses it (D1).
- **IEEE 754 decimal floating point.** It is exact for what it represents, and it is a floating format: it rounds
  silently at its precision, holds a NaN, the infinities and a negative zero (so a fact could hold a value no payload
  spells, and it is no key), and holds a number in several members of a cohort, so a canonical form must normalise
  what a fixed scale never lets vary. A business number is declared with its scale; the fixed type is that declaration.
- **A decimal of unbounded precision.** A type is a set of values with a largest member (ADR 0086 §1.4), and nothing
  could bound its sizes, its storage or the work of its operations.
- **Exact rationals.** Division would be exact, and no number a person writes has a denominator that is not a power
  of ten; the denominators of a computation grow without bound.
- **A JSON number for the JSON spelling.** Every reader that parses JSON numbers as binary64 would hold the binary
  number this type exists to keep out (D9).
- **The coefficient alone in CBOR, the scale left to the schema.** Two bytes shorter, and not a number to any reader
  without the schema; RFC 8949 registers the decimal fraction for this, and the consumer named it.
- **A canonical CBOR read strictly** (refusing `4([-1, 25])` at scale 2). It would refuse a value, which ADR 0101's
  reader never does within its profile (its §3.4); the encode is what must be canonical, and it is.
- **Implicit rounding on a store to fewer fraction digits**, SQL's implementation-defined choice. A silent rounding is
  the error described in the request.
- **A decimal stored where a real is declared**, as an integer is. It is a rounding the document did not write; the
  conversion is named instead (D7).
- **A context or a default rounding mode.** The value of an expression would depend on where it is evaluated.
- **A binary real accepted where a decimal is declared.** It is the conversion the request names as the defect; the
  conversion is offered by name, with its rounding (D7).
- **The closed forms of D3 as the result types**, SQL Server's rule. They are sufficient and not the least, so a
  result type would be wider than any value it holds and a store judged by it would be checked where it need not be;
  the build computes the least from the operands' extremes (D3).
- **`multiply` and `divide` as the names**, and **`quantize` for the conversion of a binary real.** Plain words a
  document may already use as callables would become refused (ADR 0063 §4); and IEEE 754's `quantize` is within one
  format, where a change of radix is `convertFormat` with failures of its own (D4, D7).
- **`decimal[p, s]` or `decimal<p, s>`.** The square brackets hold a bound a build moves by name and the angle
  brackets a type; neither is what `p` and `s` are.
- **`p` and `s` as named bounds.** A build that moved them would change what a fact means and which operations are
  total (D1).
- **Every backend at once.** The owner's scope rule builds a facility on the backends a consumer needs; the corpus
  holds the next lane to the same bytes when it comes.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted, with its review's amendments.
2. The type: the grammar (`decimal(p, s)` in `T` and `K`, `TypeRefusal::DecimalParameters`), `J` and `C`, the forge AST
   export, the arm of every match of D12's closed list, `DECIMAL_BACKENDS` with every lane's refusal by name, and the
   refusal of `sce:scale` and `sce:offset` on a decimal:
   `sce-build/tests/a_decimal_type_is_read_with_its_precision_and_scale.rs`.
   Its cases: `decimal(9, 2)` and `decimal(9,2)` read as one type; `decimal(0, 0)`, `decimal(39, 0)`, `decimal(2, 3)`,
   `decimal(9)`, `decimal(9, -1)`, `decimal(09, 2)` and `decimal(digits=9, 2)` each refused as
   `validation/attribute-rule-violated`; a `map<decimal(9, 2), uint8>[4]` accepted; `J` 13 and `C` 8 for
   `decimal(9, 2)` and `C` 21 for `decimal(38, 0)`; a generation for every `Language` but Rust, the Interpreter's and
   the TTCN-3 export each refused by name. Red without it: `decimal(9, 2)` names no type.
3. The value and its arithmetic, proved: the crate `sce-decimal` with T1, T2 and T3 in Verus, entered in
   `docs/formal/proofs.json` with the six changes of D13 that Verus must refuse; and
   `sce-build/tests/a_decimal_result_type_holds_every_exact_result.rs`, over every one of the 779 types and every pair,
   for each operation of D3 and D4: the result type holds the extremes, and the type one digit narrower does not. Red
   without it: the crate is absent; with it, each listed change fails the proof; a build that leaves out a rounding's
   carry fails the test at `quantize(9.99, 1, Rounding.ties_to_even)` of `decimal(3, 2)`, and a build that uses D3's
   closed forms fails its second half at `decimal(1, 0) + decimal(1, 1)`. The binary32 exhaustive test and the
   generated binary64 cases of D13 land here.
4. The expressions: typing, literals, the comparisons, `+`, `-`, `*`, `? :`, `quantize`, `roundingMul`, `roundingDiv`,
   `toDecimal`, `toFloat64`, `toFloat32`, `sce:std/decimal/rounding.scxml`, the classification of ADR 0133, the Rust
   lowering of a statechart and of an algorithm, `expression/decimal-rounding-undeclared` with its appendix row, and the
   six names in the vocabulary: `sce-build/tests/a_decimal_operation_is_exact_or_names_its_rounding.rs`, each case
   named and each red without its part:
   - `a_product_is_rounded_once`: `roundingMul(1.005, 1.1, 2, Rounding.ties_to_even)` is `1.11` (rounding the operand
     first gives `1.10`);
   - `a_nan_made_a_decimal_fails_precondition`: `toDecimal` of a NaN;
   - `a_decimal_and_a_real_in_one_operation_are_refused`, and `an_exponent_literal_meeting_a_decimal_is_refused`
     (`1.5e3`);
   - `a_decimal_stored_where_a_real_is_declared_is_refused`, and `to_float64_of_a_tenth_is_the_binary64_tenth` (bit for
     bit);
   - `a_decimal_joined_to_a_string_is_its_canonical_text`;
   - `a_conditional_of_two_decimal_types_is_the_least_type_holding_both`;
   - `an_unproven_decimal_operation_without_may_fail_is_refused` (`algorithm/undeclared-integer-failure`);
   - `a_product_that_keeps_no_integer_digit_is_refused` (`decimal(38, 20) * decimal(38, 18)`);
   - `a_callable_spelled_as_a_rounding_name_is_refused` (`validation/duplicate-id`);
   - the classification: a sum of two `decimal(9, 2)` values stored into `decimal(10, 2)` total and into
     `decimal(9, 2)` checked, and a guarded `roundingDiv` total;

   and `backends/rust/scenarios/tests/decimal.rs`, run native and on `wasm32-unknown-unknown`:
   `two_and_a_half_percent_of_an_amount_is_rounded_once`, `a_value_on_its_target_is_judged_on_the_target` (950 tenths
   summed against a target of 95.0, which the same document over `float64` judges below it: its binary64 sum is
   94.99999999999888), `ten_thousand_tenths_sum_to_one_thousand` (binary64 gives 1000.0000000001588) and
   `a_division_by_zero_raises_error_execution`. The kit's fixture `decimal_arith` (`gen_cases.py` and
   `numerical_reference.json`, every operation, every mode, every edge, and the binary conversions against
   `float(Fraction(...))`) is replayed by `backends/rust/forge-conformance/tests/numerical_conformance.rs`. Red without
   it: an operation on a decimal is refused as one on a type the typer does not hold.
5. The text: the canonical writer and the strict reader in the payload, the `<param>` and `<donedata>` texts, the saved
   state and the snapshot, the payload schema and its GBNF form, and the page surface:
   `backends/rust/runtime/tests/a_decimal_payload_field_is_read_from_its_canonical_text.rs` (every spelling D9 refuses,
   each by its place, and a JSON number), the decimal case of
   `sce-build/tests/a_schema_unit_declares_the_grammar_of_its_payload.rs` (the pattern of D9, and a GBNF grammar that
   accepts exactly the canonical texts), and the page surface's `right.mts` and `wrong.mts`. Red without it: a payload
   field of a decimal type has no reader.
6. The CBOR form: `codec_cbor_decimal` in `tests/forge/conformance/fixtures.json` and `numerical_reference.json`, with
   every vector of D8 (the four at the edges of 64 bits among them), the variants it reads (another exponent, a tag and
   a bignum tag in a longer head, a bignum's leading zeros), each refusal by its kind (an exponent written as a bignum,
   a bigfloat, a fraction past the scale, a mantissa past 128 bits) and the tightness of `C`, replayed by
   `backends/rust/forge-conformance/tests/forge_cbor_decimal.rs`; and the case that keeps a bignum refused at an integer
   type (`c2 41 05` at `int64`). Red without it: the codec's decimal entry has no printer and the case is refused.
7. The record codec and the journal: `describe`'s row and its test
   `a_decimal_is_described_by_its_precision_and_scale` in `payload_record.rs`; `Shape::Decimal` in `record_cbor`; the
   float refusal's added sentence; the C++ refusal by name; and
   `backends/rust/mesh/tests/a_fact_holding_a_decimal_is_kept_and_followed_exactly.rs`: a session says a fact of an
   amount 2.50 (`decimal(9, 2)`) and a rate 0.025 (`decimal(5, 3)`), the entry's bytes hold `c4 82 21 18 fa`, a follower
   is delivered `"2.50"` and `"0.025"`, a page of a journal read gives the same, and a restore keeps them. Red without
   it: the send is refused, the record having no description.
8. The view: with ADR 0151's step 3, the laws of `sce-build/tests/a_view_is_answered_by_its_plan.rs` gain a decimal
   `sum` that is exact, an order of decimals across two scales, and `view/aggregate-may-overflow` for a sum past 38
   digits. Red without it: a view over a decimal column has no comparison.
9. The model: with ADR 0156 D3's typed translation, `docs/formal/Decimal.tla` and the translation of D13, tested by
   `a_decimal_is_modelled_as_its_coefficient` and `a_decimal_past_tlcs_integers_is_refused_by_name` (a `decimal(10, 2)`
   document, refused as `Construct::Decimal`) in `sce-build/src/formal/document_model.rs`; an example document in
   `docs/formal/models.json` whose property says a value on its target is judged on the target's side, with a
   configuration that replaces `ties_to_even` by `toward_zero` in one `quantize`, which TLC must find violates it; and
   the line of `docs/formal/README.md` on replay. Red without it: the document's model is refused as
   `Construct::Decimal`.
10. The message: with ADR 0158's step for `:number`, a catalog that formats a decimal `1.50` as the text `1.50` and
    selects `other` in English. Red without it: `:number` refuses a decimal field.
11. The documents named in "Amends".

Steps 2 to 7 need nothing of another record; step 8 waits for ADR 0151's step 3, step 9 for ADR 0156 D3, and step 10
for ADR 0158's formatting step.

## Open questions

Each decided, with its reason, and kept as accepted.

1. **The spelling is `decimal(p, s)`.** SQL's, and it keeps `[ ]` for a bound and `< >` for a type (D1). `decimal`
   rather than `numeric`: it is the name of IEEE 754, of the General Decimal Arithmetic Specification and of the
   request; its semantics are SQL's `NUMERIC` (exactly `p` digits).
2. **The precision is at most 38.** A value is one 128-bit integer at most, a product of two fits 256 bits, and it is
   Arrow's `Decimal128` maximum and SQL Server's, Oracle's and Snowflake's (D1). The wire forms do not depend on it.
3. **`p` and `s` are not named bounds.** They are what a value means, and a build that moved them would change the
   meaning of a fact already kept (D1).
4. **A store into fewer fraction digits is refused, never rounded.** Every rounding is written (D4).
5. **The CBOR form writes its exponent, and the decode reads any exponent within the profile.** The encode is
   canonical because the type fixes the exponent; the decode reads a value however a sender wrote it, as ADR 0101's
   reader does and as RFC 8949 §3.4.3 asks of a bignum's leading zeros, and refuses only what is no value of the type or
   past 128 bits (D8). The JSON reader is strict for the decimal's own reason: a JSON reader takes a number through a
   binary64, and a fraction has no range in which that is exact, so no JSON number is taken, and within a string only
   the canonical text is, as `"007"` is refused for an integer (D9; RFC 7493 §2.2).
6. **Each change of radix is named, in both directions.** `toDecimal(f, t, mode)` rounds a binary real to a decimal and
   `toFloat64` or `toFloat32` rounds a decimal to a binary real; a store of either where the other is declared is
   refused, since it would be a rounding the document did not write (D7). A NaN or an infinity given to `toDecimal`
   fails `precondition`, the conversion's domain being the finite reals, which is IEEE 754's invalid operation (§7.2)
   under the house's existing name, so no new failure name enters the runtimes.
7. **The modes are IEEE 754's five, as values of an enum of the standard library**, so a mode may come from data, and
   none is ambient (D4).
8. **A literal with a point is a decimal only where it meets one.** Every document written before this record keeps
   its meaning, and a decimal literal is still read exactly, from its text (D6).
9. **Rust alone serves it among the lanes**, by the owner's scope rule (ADR 0151 is the precedent, and
   `wasm32-unknown-unknown` is the Rust lane); the lane-neutral surfaces (the payload schema and its GBNF form, the page
   surface, the corpus) serve it now, since every lane is held to them (D12).
10. **Arithmetic is judged by proofs of functions, the build's rule exhaustively, the codec by the corpus and a
    document by its model** (D13): each claim at the level that says it for every case it covers.
11. **The proved code is its own crate**, `sce-decimal`, re-exported by `sce-portable-bytes` (D12): the review left the
    choice open and recommended it, and a proof whose unit holds nothing else is judged alone and changed by nothing
    else.
12. **A product that keeps no integer digit of its operands is refused, not warned.** The review asked that it be
    reported at build time; a refusal is the house's one form of a report, and the operation would fail on `5.0 * 3.0`,
    so its code is the undeclared rounding's, naming `roundingMul` (D3).

## Amended on acceptance

Each amendment is the independent review's, by its number there, decided by the main session as the review
recommends.

1. D13, step 9, Consequences and Not in scope: TLC's integers are exact within `-2^31..2^31 - 1`
   (`LARGEST_INTEGER`); a decimal is translated only where its coefficients stay there, else refused as
   `Construct::Decimal`; a `decimal(10, 2)` case; Apalache as the later route; today's whole refusal of `sce-static`.
2. D3, D4 and step 3: a result's precision is the digits of the largest magnitude the operation can return, rounded
   away from zero where it rounds, computed from the operands' extremes; the closed forms are its usual values.
3. D7, D13, D14, step 4 and open question 6: a decimal where a real is declared is refused; `toFloat64` and
   `toFloat32` added, judged against `float(Fraction)`.
4. D4, D12 and D13: alignment, sum, difference, product and comparison in 256 bits, `roundingDiv` by Algorithm D over
   64-bit limbs without the whole numerator; T1 states no intermediate overflows, with the mutant of a 128-bit
   alignment.
5. D8: major type 1 down to `-2^64`; the two vectors at that edge; RFC 8949 §4.2.2 cited.
6. D8 and step 6: the tolerant decode kept, with corpus cases for a tag in a longer head, a bignum's leading zeros at
   any length, and a bignum still refused at an integer type.
7. D9 and open question 5: the strict reader kept with the decimal's own reason; the misstatement of the 64-bit reader
   corrected (also in Context).
8. D9 and Amends: the pattern and the GBNF form by digit-prefix alternatives, no lookaround; ADR 0087 §1's rows.
9. Context, D1, Not in scope and Alternatives rejected: `sce:quantity` named; a decimal may carry it; `sce:scale` and
   `sce:offset` on a decimal refused; the `f64` accessor rejected.
10. D12, D14, Amends and step 2: the TTCN-3 export, JANI and the page surface; the closed list of matches over the type;
    `SCE_FORGE.md` §3.4.2, §4.6.1 and §4.12, ADR 0087 §1, ADR 0101 §9 and ADR 0063 §4 amended.
11. D4, D7, D14 and Amends: `roundingMul`, `roundingDiv` and `toDecimal` in place of `multiply`, `divide` and
    `quantize` of a real; the reservation under ADR 0063 §4.
12. D3, D5 and D6: the conditional, operations of literals alone, a hexadecimal literal, the integer operators,
    initial values, constants, an unproven operation without `may-fail`, and `toDecimal(-0.0)`.
13. D3 and Amends: a decimal operation is computed at its result type, a deliberate divergence from the integers'
    operand width, with its reason.
14. D13: the proofs are ADR 0115 level 6, functions, as `client_mailbox.rs` and `fresh_step.rs` are.
15. D13: the Verus plan (specifications over `int`, `pow10`, the `vstd` lemmas, the three tactics, the binary
    decomposition trusted and tested exhaustively for binary32).
16. D9 and step 10: `:number`'s fraction digits default to the scale, and the text `1.50` is asserted.
17. Order of work: the named red tests of steps 2, 4 and 6.
18. The textbook structure, D1, D7, Consequences and Sources: the SQL engines named, the literal's precision as this
    record's, `NUMERIC`'s semantics, comparison with a real given its own reason, IEEE 754-2019 cited, "(E11)"
    removed, the fixture's path, and a sum's overflow that order can move.
19. D12 and open question 11: the leaf crate `sce-decimal`, re-exported by `sce-portable-bytes`.
20. D3 and open question 12: a capped product with no integer digit of its operands is refused at build time.
21. D12 and D13: the replay of a decimal document is on Rust and the model alone, said in `docs/formal/README.md`;
    `lang_supports_fixture` reads `DECIMAL_BACKENDS`.

## Sources

ISO/IEC 9075-2, SQL Part 2: Foundation (`<exact numeric type>`, `<numeric value expression>` and its Syntax Rules on
the scale of a result, `<exact numeric literal>` and `<approximate numeric literal>`); IEEE 754-2019 (IEEE Standard for
Floating-Point Arithmetic, as ADR 0151 cites it, the revision of IEEE 754-2008 whose decimal arithmetic it keeps: §2.1
the definition of a cohort, §4.3 the rounding-direction attributes, §5.1 the rule that an operation is rounded once
from its exact result, §5.3.2 `quantize`, §5.4.2 conversions between formats, §7.2 the invalid operation); M. F.
Cowlishaw, "Decimal floating-point: algorism for computers", Proceedings of the 16th IEEE Symposium on Computer
Arithmetic (ARITH-16), 2003, and his General Decimal Arithmetic Specification; D. Goldberg, "What every computer
scientist should know about floating-point arithmetic", ACM Computing Surveys 23(1), 1991; W. D. Clinger, "How to read
floating point numbers accurately", PLDI 1990; D. E. Knuth, *The Art of Computer Programming*, vol. 2, *Seminumerical
Algorithms*, 3rd ed., Addison-Wesley 1997, §4.3.1 (Algorithm D); J. Bloch and N. Gafter, *Java Puzzlers*,
Addison-Wesley 2005, Puzzle 2; RFC 8949, Concise Binary Object Representation (§3.4.3 bignums, §3.4.4 decimal
fractions, §4.1 preferred serialization, §4.2.1 core deterministic encoding, §4.2.2 its further considerations); RFC
8610, CDDL (its prelude, `decfrac`); RFC 8259, JSON (§6, numbers); RFC 7493, I-JSON (§2.2); Unicode Technical Standard
#35, Part 3 (plural operands); Unicode MessageFormat 2 (`:number`); the Apache Arrow columnar format (`Decimal128`);
the Python standard library's `decimal` and `fractions` modules; Lattuada et al., Verus (SOSP 2024), as ADR 0115 cites
it, and its `vstd` library.

What was checked, and by whom. Every statement about this repository was read from the tree at `662e4d060cc` when the
record was drafted (the type grammar and `TypeRefusal`, `SceType`, the parser's mapping of a refusal to its code, the
record codec's description and its float refusal, every site of D10, the journal's lane refusal, `InferredType` and the
literal's spelling, the model generator's closed list, `codegen_matrix.rs`'s tables, `docs/formal/proofs.json`'s form,
`SCE_FORGE.md` §3.3, §3.4, §3.4.1 and §10, `ARCHITECTURE.md` "JSON Number Text", and ADRs 0086, 0101, 0108, 0114, 0115,
0133, 0151, 0152, 0154, 0156 and 0158 in the parts named), and again on acceptance for the review's findings
(`LARGEST_INTEGER` in `document_model.rs`, `quantity.rs` and the physical-quantity annotation, ADR 0063 §4, ADR 0087
§1's table, the TTCN-3 export's `refuse`, `page_surface.rs` with `right.mts` and `wrong.mts`, `lang_supports_fixture`,
`algorithm/undeclared-integer-failure`, and the fixture's path). The independent review checked, against the texts,
RFC 8949 §3.4.3 and §3.4.4, RFC 8610's `decfrac`, IEEE 754-2008 §2.1, §4.3, §5.1, §5.3.2 and §5.4.2, RFC 8259 §6, RFC
7493 §2.2, UTS #35's operand `v`, Arrow's `Decimal128`, the CBOR vectors of D8, the sizes `C` and `J`, the count of
779 types, the bounds `10^38 - 1 < 2^127` and `(10^38 - 1)^2 < 2^253`, and the binary64 sums of step 4; and it reported
MessageFormat 2's default display of `1.50` and the SQL engines' maxima. The passages of ISO/IEC 9075-2, Knuth's
Algorithm D, IEEE 754 §7.2 and the 2019 edition's numbering of the sections above were not opened for this record, and
are to be checked against the texts in the step that builds on each.
