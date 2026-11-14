# ADR 0099 — A failure is not a value, and no type has a zero

- Status: Accepted (2026-11-12). Written and accepted under the owner's standing rule (the most long-term correct
  structure, textbook in its foundations, cost ignored; decided, not asked); the owner asked for it to be built the
  same day. "Landing order" lists the steps. (Numbered 0098 when first written, as the next free one after 0097 on
  `origin/main` as fetched on 2026-11-12, 0094 being taken on a branch; renumbered 0099 on 2026-11-14 because ADR
  0098, the value an invoke hands its child, had landed under 0098 earlier the same day.)
- Date: 2026-11-12
- Amends:
  - SCE_FORGE.md section 3.4.1, "The integer arithmetic contract": a failure has no value and nothing stands in for
    it (section 1); the failure-channel table gains its Swift row (section 3) and states what the three
    state-passing backends may write in place of a value (section 2);
  - docs/SCE_ACCEPTED_SUBSET.md section 2.15, the Swift machine's paragraph: a statement that can fail catches a
    thrown failure, and records no `Failure` (section 3.3);
  - ADR 0092 section 7 (the Swift row) and its amendment of 2026-11-07: "a union has no default value" becomes the
    case of a rule over every type (section 1 item 4), and a Swift record may hold a union (section 4);
  - ADR 0092 section 6.1 item 5: a whole union taken from a list's element at an index lowers on every lane
    (section 5).
- Scope: the Swift forge runtime (`backends/swift/forge-runtime`, `Checked.swift`), the Swift lowering of
  expressions, statements, conditions and adapters (`sce-build/src/forge/expr/swift.rs`, `forge/expr.rs`,
  `forge/static_lowering.rs`, `forge/generator.rs`, the Swift templates under `tools/codegen/templates/swift` and
  `tools/codegen/templates/forge/swift`), the Swift record and payload types (`static_lowering.rs`'s record
  definition, `forge/composed_lowering/swift.rs`'s, `event_schema.swift.jinja2`), the
  `ComposedSpelling::record_starts_at_zero` hook (`forge/composed_lowering.rs`) and its refusal, the whole-union
  copy of `static_lowering.rs::whole_union_copy` on every lane, and the committed Swift trees those generate.

## Context

**Two refusals on one lane.** The Swift machine lowering of ADR 0092 (step 4) landed with two constructs refused
by name, where every other lane lowers them:

1. **A record that holds a union field.** A Swift record is generated as `struct R: Equatable, Sendable,
   SceDefault` with `public init() {}` and an initializer on every field (`static_lowering.rs` and
   `composed_lowering/swift.rs`, the record definitions: "Every field has a value at its zero, so the record is
   `SceDefault`"). A union has no value to write there (ADR 0092, amendment of 2026-11-07), so a record that
   holds one is refused through `ComposedSpelling::record_starts_at_zero`, whose own comment names the cause: "a
   Swift record is `SceDefault`, which a checked read of an element not there answers ... A backend that builds a
   record whole only (Rust) has no such record to refuse."
2. **An index read of a list of unions** (`us[i].kind`, or a whole `us[i]`). A Swift machine reads an element
   through `Checked.at(sceFailure, xs, i, fallback: zero)` or `Checked.atDefault(sceFailure, xs, i)`
   (`expr/swift.rs`, the `CheckedOp::Index` arm), which answers the element type's zero for an index outside the
   list. A union has none, so the read is refused (`expr.rs::reject_zeroless_element_reads`).

**Both come from one cause, and it is not the union.** The Swift lane lowers a failure the way C++ and Go do: each
checked operation records the failure in a `Failure` object and answers a value of its type, the statement runs on
to its end, and the generated code checks the record after it (`Checked.swift`'s header; the generated statement
`{ () -> Bool in let sceFailure = Failure(); let sceValue: UInt8 = Checked.at(sceFailure, self.vPicked, 0,
fallback: 0); if sceFailure.failed { ... return true }; self.vFirst = sceValue; return false }()`). That protocol
needs, at every point that can fail, a value of the point's type that nobody wrote, and the runtime grew one
helper per way of naming such a value:

| Where a failure stands | What the Swift lane writes in its place |
|---|---|
| a number, a bool, a string, bytes | `fallback: 0` / `false` / `""` / `[]` (`swift::zero_of`) |
| an optional | `fallback: nil` |
| an enum | `Checked.takeCase`: the first case (`allCases.first`) |
| a record, a list | `Checked.takeDefault`, `Checked.atDefault`: `T()`, which needs `SceDefault` and `init()` on every record |
| a union | nothing: refused |

The record's `init()`, its field initializers (an enum field starts at `allCases[0]`), the `SceDefault` protocol,
`takeCase`, `takeDefault`, `atDefault` and the `fallback:` parameters exist only to feed this protocol. The union is
the first type for which no such value can be named; it is not the first type for which naming one was wrong.

**The language already says there is no zero.** docs/SCE_ACCEPTED_SUBSET.md section 2.15: "Every variable
declares its initial value; no zero, empty string or first variant stands in." SCE_FORGE.md section 3.4.1: an
overflow, a division by zero and an index outside a collection "are not values ... so no backend may produce
one". ADR 0092's amendment applies the first sentence to a union. The Swift lane's fillers contradict the spirit
of both: they are values of the language's types, made by the backend, that stand where the language says there is
no value. They are never observed only because the generated code commits after the check, an ordering
convention of the emitter that no type enforces.

**Swift is the one lane whose language does not support the protocol.** C and Go define a value for every type
(C's zero bytes, Go's zero value; the Go Programming Language Specification, "The zero value"), and C++
value-initializes every type the generator writes (`T{}`; a `std::variant` holds its first alternative). In those
languages a filler is the language's own, written by no one and needing no name, so the state-passing protocol
costs nothing per type. Swift has no zero value: it requires every stored property to be initialized before use
(definite initialization) and gives a type a no-argument initializer only when one is written. In Swift the
protocol has to invent a value per type, and fails at the first type that has none. Swift has, instead, what C,
C++17 without exceptions and Go do not: a failure channel in the expression itself, `throws` and `try`.

## The textbook structure

**A failure is an abrupt completion.** ECMA-262 defines every evaluation's result as a Completion Record, normal
with a value or abrupt (`throw`) with a reason, and every compound step propagates an abrupt completion of its
operand before using it (ECMA-262, section 6.2.4 "The Completion Record Specification Type"; section 5.2.3.4,
the `?` shorthand for ReturnIfAbrupt). The forge expression language is a typed subset of ECMAScript, and W3C SCXML
says what an abrupt completion does to executable content: the element raises `error.execution` and the rest of
its block is not processed (§scxml-4.9), and a condition that cannot be evaluated is false (§scxml-5.9). An abrupt
completion has no value; nothing downstream of it runs.

**Exceptions are an effect, not a value of the result type.** In denotational terms a computation that may fail
denotes in `T + E` (the exception monad, Moggi, "Notions of computation and monads", Information and Computation
93(1), 1991), not in `T`: the failure is the other summand, and `bind` does not apply the continuation to it. A
lowering that answers a `T` for a failure embeds `E` into `T`, which is only defined when `T` is inhabited by a
value the program can name and only sound when no continuation observes it.

**Each language's native channel.** Rust spells `T + E` as `Result<T, E>` and its bind as `?`; Swift as a function
`throws(E) -> T` and its bind as `try` (The Swift Programming Language, "Error Handling"; typed throws,
SE-0413, Swift 6.0); Kotlin and Python as an exception; ECMAScript as `throw`. A Swift `throw` is not stack
unwinding: "the performance characteristics of a throw statement are comparable to those of a return statement"
(The Swift Programming Language, "Error Handling"), and a typed throw carries its error without an existential box
(SE-0413, "Motivation"), which is what Embedded Swift needs. C11, C++17 built without exceptions and Go have no
expression-level non-local exit; their standard encoding of `T + E` is state passing (an error flag beside a value
of `T`, as C's `errno` convention and Go's `(T, error)` with its zero value), sound in those languages because
every type has the language's own value to fill the `T` with.

**One semantics, lowered per language.** ADR 0092 section 7 gives one sum type and lowers it to each language's
own (an `enum`, a sealed interface, a `std::variant`, a tagged struct). A failure is the same kind of choice: one
rule of the language, and on each backend the construct that language has for it.

**Narrowing is over stable paths.** Occurrence typing narrows a name whose value cannot change between the test
and the read (Tobin-Hochstadt and Felleisen, ICFP 2010; Kotlin's smart casts apply to stable values only, The
Kotlin Language Specification, "Smart casts"). `us[i]` is not one: `us` and `i` are separately assignable. The
pattern-matching languages narrow by binding the value to a new name (`case .absolute(let r)`, `if let`), and the
statechart's binder is a `<foreach>` item or a variable a whole union is copied into (ADR 0092 section 6.1 item 5).

## Decision

**A failure is an abrupt completion: it has no value, nothing stands in for it in the language, and no type has a
zero. Each backend lowers abrupt completion to its language's own channel: Rust, Swift, Kotlin, Python and the
Interpreter to the language's effect (`?`, `try`, `throw`, `raise`), with no value at the failure; C11, C++ and Go
to state passing, where the value beside the failure flag is the language's own default of the type, never one the
generator names, and is never committed. Swift moves from state passing to typed throws (`throws(AlgorithmError)`),
its records and payloads lose their zero (`SceDefault`, `init()`, field initializers) and are built only whole, and
the two refusals disappear as consequences, not as special cases. Whole-union copy from a list element lowers on
every lane.**

### 1. The language rule (every backend)

1. **A failure has no value.** An operation of SCE_FORGE.md section 3.4.1 that fails (an overflow, a division by
   zero, a narrowing that does not fit, a capacity exceeded, a precondition, an index outside its collection, a
   may-fail call that fails) completes abruptly. Evaluation is left to right; the first abrupt completion ends the
   evaluation of the expression around it, and no operand after it is evaluated. `a && b` and `a || b` evaluate `b`
   only where `a` decides nothing, and `c ? t : e` only the branch `c` chooses.
2. **What an abrupt completion does is where it stands:** in an algorithm, the algorithm fails with that failure
   (section 3.4.1, unrenamed through calls); in executable content, the element raises `error.execution` and its
   block ends (§scxml-4.9), except a host call's arguments, which raise in the call's place without ending the
   block (as today, docs/SCE_ACCEPTED_SUBSET.md section 2.15); in a condition, the condition is false and
   `error.execution` is raised (§scxml-5.9); in a `<donedata>` `<param>`, the pair is left out (§scxml-5.7.1).
3. **A statement that completes abruptly has no effect.** An `<assign>`, an `<sce:append>`, a record built from
   `<sce:set>`, a `<send>` whose content or `<param>` fails: nothing is written, appended, built or sent.
4. **No type has a zero.** No construct of the language yields a value nobody wrote: not a variable (docs/
   SCE_ACCEPTED_SUBSET.md section 2.15, unchanged), not a record field, not an element read outside its list, not a
   failed call. ADR 0092's amendment ("a union variable has no default value") is this rule applied to a union.
   A schema-declared default of a field added by a revision (ADR 0086 section 10.3) is a value the document writes
   and is not a zero.

This rule is what SCE_FORGE.md section 3.4.1 and the accepted subset already say case by case; this ADR states it
once, and section 3.4.1's paragraph on the contract cites it.

### 2. The two lowering families

| Family | Backends | Abrupt completion is | The value at a failure |
|---|---|---|---|
| Effect | Rust, Swift, Kotlin, Python, Interpreter | the language's own: `Result` + `?`; `throws(AlgorithmError)` + `try`; `AlgorithmFailure` thrown; `AlgorithmFailure` raised; a JavaScript `throw` | none exists |
| State passing | C11, C++, Go | a recorded failure (`sce_failure_`, `AlgorithmFailure&`, `*algorithm.Failure`) checked after each operation that can record one | the language's own default of the type (`{0}`, `T{}`, Go's zero value), never one the generator names |

1. **A backend whose language has an expression-level failure channel uses it.** That is the rule that puts Swift
   in the first family. A state-passing backend is one whose language has none that the backend can use (C11; C++,
   whose forge code uses no exceptions, SCE_FORGE.md section 3.4.1's table; Go, whose `panic` is not a failure
   channel).
2. **The obligation of a state-passing backend.** The value beside a recorded failure is a dead store: the
   generated code computes every value of a statement into locals and commits only when no failure was recorded
   (section 1 item 3), which C++ and Go already do. The value is spelled by the language's value-initialization of
   the type and never by a literal or a case the generator chooses, so no SCE value is invented: a C++ union's
   filler is the `std::variant`'s first alternative because C++ makes it so, and no SCE rule reads it. A C++
   record or union whose type would not be value-initializable is no type the generator writes (each case is a
   struct of its own, ADR 0092 section 7 item 2).
3. **The family is the backend's, not the construct's.** No backend lowers some failures one way and others the
   other way.

### 3. Swift lowers abrupt completion to typed throws

1. **The runtime.** Each checked operation of `Checked` takes no `Failure` and `throws(AlgorithmError)`:
   `add`, `sub`, `mul`, `div`, `rem`, `neg`, `narrow`, `bounded`, `within`, and the element read
   `at<T, I: BinaryInteger>(_ xs: [T], _ i: I) throws(AlgorithmError) -> T`, generic in an unconstrained `T`.
   `found` throws `AlgorithmError` too. `Failure`, `SceDefault` and its conformances, `atDefault`, `take`,
   `takeCase`, `takeDefault` and every `fallback:` parameter are removed.
2. **One spelling of a call that throws.** Every call that can throw is written `(try f(...))`, marked where it
   stands: the parentheses let it stand as any operand, Swift refusing a bare `try` to the right of a binary
   operator. A construct that runs its operand in a closure (an autoclosure of `&&`, `||` and `??`, a closure the
   generator writes) does not take a throwing operand into the standard library's `rethrows`, which may erase the
   error's type: where the operand throws, it is written through the runtime's own short-circuit,
   `Checked.and`, `Checked.or` and `Checked.coalesce`, each `throws(AlgorithmError)` over a
   `() throws(AlgorithmError) -> T` closure, and a closure the generator writes is itself called with `try`. Where
   the operand does not throw, Swift's own operator is written as before, so a `try` never covers nothing.
3. **Algorithms.** A `may-fail` algorithm is `func f(...) throws(AlgorithmError) -> T`; its body records nothing and
   checks nothing after a statement, because no statement runs after a failure. A `<sce:require>` is
   `guard cond else { throw AlgorithmError.precondition }`. An algorithm that does not declare `may-fail` does not
   throw (its operations are proven, section 3.4.1, "Enforced").
4. **Statechart statements.** The machine's dispatcher keeps its contract with every lane (a statement that can
   fail is an expression that answers whether it failed, so the block can end; §scxml-4.9), and Swift spells it
   with no `Failure`:

   ```swift
   if ({ () -> Bool in do { let sceValue: UInt8 = (try Checked.at(self.vPicked, 0)); self.vFirst = sceValue }
         catch { engine.raise(...); return true }; return false }()) {
       break sceBlock  // §scxml-4.9: the error ends the block
   }
   ```

   The value is computed before the write and the write is in the `do` after it, so a failure leaves the variable
   as it was by Swift's own rule, not by the emitter's order. A condition answers its value from the `do` and
   `false` after raising in the `catch`; a host call's arguments are computed into locals in the `do` and the host
   called there, the `catch` raising in the call's place without ending the block; a `<send>`'s and a
   `<donedata>`'s value is computed in a `do` whose `catch` is what the template did where `sceFailure.failed` held.
5. **Adapters.** A machine's adapter of a may-fail algorithm throws what the algorithm throws and is called as any
   throwing call is; it builds the machine's record or union from the algorithm's whole, so it needs no zero either.
6. **The error message** a statement raises is unchanged by this ADR.

### 4. Swift records and payloads are built only whole

1. A Swift record (`static_lowering.rs`'s and `composed_lowering/swift.rs`'s record definitions) and an event
   payload struct (`event_schema.swift.jinja2`) declare their stored properties with no initializer, have no
   `init()` and do not conform to `SceDefault` (which no longer exists). Each has its memberwise initializer, one
   parameter per field in the schema's order, and nothing else constructs one.
2. Every construction is whole: a record variable's `<sce:set>`s become one memberwise call whose arguments are
   computed first, so a failed field builds nothing (section 1 item 3); a payload is read whole by the runtime's
   reader and built by the same call; a saved state's `fromSaved` already builds by it. Swift's definite
   initialization then checks at compile time what ADR 0092 section 6.1 item 5 and section 2.15 require: every
   field written, none left at a value nobody wrote.
3. A record that holds a union field (or a list of unions, or an optional union) is then a struct with a field of
   the union's enum, built by the same call. Nothing about it is refused. The refusal that named it is removed.
4. A field of an existing record is written as today (`r.f = v`, or through `SceOptional.proven` for an optional
   one), after its value is computed.

### 5. Reading a list of unions at an index

1. **On Swift**, `us[i]` is `(try Checked.at(self.vUs, i))`, so `us[i].kind` reads the tag of an element that
   exists and fails `out-of-range` for one that does not, as on every other lane. The refusal of
   `reject_zeroless_element_reads` is removed.
2. **An element at an index is not a narrowing subject**, unchanged (ADR 0092 section 6.1 item 2; ADR 0055
   section 3): `us[i].count` stays `expression/union-read-unproven` on every lane, since `us` or `i` may be
   assigned between the test and the read. The repair the diagnostic names is to bind the element: a `<foreach>`
   item, or a union variable the element is copied into whole and then tested.
3. **The whole copy lowers on every lane.** ADR 0092 section 6.1 item 5 admits `<assign location="u"
   expr="us[i]"/>`; `static_lowering.rs::whole_union_copy` refuses it on every target ("a union read through an
   index has no ... lowering of a whole copy yet"). It is lowered on all eight: the element is read by the lane's
   checked read (section 3.4.1, `out-of-range`), copied into `u`'s type by the copier the other places already use,
   and committed only if the read completed normally.

## Landing order

1. **This ADR and the documents**: SCE_FORGE.md section 3.4.1 (the rule of section 1, the table's Swift row and
   the two families), docs/SCE_ACCEPTED_SUBSET.md section 2.15 (the Swift paragraph), ADR 0092 section 7 and its
   amendment (a pointer to section 1 item 4 here).
2. **Swift, sections 3 and 4 in one change**: the runtime, the expression, algorithm, statement and template
   lowering, the records and payloads, the removal of the old API and of the two refusals, and every committed
   Swift tree regenerated. The runtime's old API has no caller once the generator moves, and the generated trees
   are committed beside the generator, so a change that kept both would hold an API nothing calls; one change
   keeps every commit building. A Swift host that built a payload or a record with `init()` and field writes
   migrates to the memberwise initializer.
3. **Every lane**: the whole-union copy from an index (section 5.3).
4. **Fixtures**: `static_union` gains a record variable whose record holds a union field (built, narrowed, saved and
   restored), an index read of a list of unions in range and out of range (the variable unchanged, the block ended,
   `error.execution` raised), and a whole copy from an index followed by narrowing; its scenario is replayed on all
   eight lanes. A conformance algorithm over `list<union:...>` reads at an index with a case that `"fails":
   "out-of-range"`.

**Where the verdict is read.** `sce-build`'s `swiftc` type checks are skipped where no `swiftc` is installed. The
compile verdict is the Swift packages (`scripts/test_swift_backend.sh`), run where the toolchain `bx.toml` pins
(`[platform.linux.tools.swift]`, Swift 6.4) is installed, and on the CI lane that installs it.

## Decided while landing steps 3 and 4 (2026-11-13)

1. **The element is read by the read every lane already has.** `StaticTarget::element_at` is the checked read of
   an index (`record_at`, which names it for a list of records) on every lane but Rust, whose element owns its parts
   and is read by reference (`at_ref`) and cloned out of the list. The copy is the union copier's, and the write is
   the lane's own receiving write of a value held in a local of the variable's type, so a read outside the list
   writes nothing, raises `error.execution` and ends the block on every lane.
2. **A list itself reached through an index stays refused by name**, as the whole copy and the loop over such a list
   are (ADR 0086 section 7.3): `place_text` spells no element, and the refusal names the construct.
3. **Step 4 lands the index read, its failure and the narrowing of the copy** in `static_union` (the event
   `shapes.picked`), replayed on all eight lanes. Two of its items rest on lowerings this ADR did not land:
   - A record that holds a union is held by a statechart only as `optional<record:R>` written whole by a lift (ADR
     0087 "Landing order" item 8, `ComposedLowering::records`), which Rust and Kotlin lower and the other lanes
     refuse by name. Its fixture follows that item on each lane.
   - An algorithm's slot holds no `list<union:U>` on any lane (ADR 0092, a union inside another type), and the
     conformance manifest has no argument of that type, so the conformance algorithm over one follows that
     extension.

## Consequences

- Swift needs no value nobody wrote, for any type. The two refusals go, and so does the reason any later type
  (a quantity, a nested union, a record that holds one) would have hit the same wall.
- The generated Swift holds no `Failure` per statement, no `fallback:` and no dead filler. A failed operation
  evaluates nothing after it, where state passing evaluated the rest of the expression on fillers.
- Section 1 item 3 is held by the type checker on Swift (a `try` that throws leaves the assignment after it
  unexecuted) instead of by the emitter's order of statements.
- The three `preconditionFailure` paths for a foreign error in `Checked.swift` go: typed throws makes the closed set
  of failures a compile-time fact, as `Result<T, AlgorithmError>` does on Rust.
- A Swift host's code that built a payload or record with `init()` and field writes migrates to the memberwise
  initializer. No wire, saved-state or schema surface changes.
- C11, C++ and Go are unchanged in kind; their filler becomes a stated obligation (section 2.2) instead of a habit.
- Every lane gains the whole-union copy from an index that ADR 0092 admitted and none lowered.

## Alternatives rejected

- **Give a union a zero** (its first case, or a case the document marks `sce:default`). It is the value ADR 0092's
  amendment refused: a shape no author wrote, moving with the enum's order; and it fixes the union while leaving the
  protocol that needs a zero for every type.
- **Answer `T?` from the checked read and unwrap after the check** (`Checked.at(...)!`, or an implicitly
  unwrapped optional in the record). The force unwrap is a trap the emitter's ordering keeps unreached, which is
  the state-passing protocol with a crash where its filler was; an implicitly unwrapped field is a companion state
  the type admits and the language does not (ADR 0055, "never a companion flag that admits impossible states").
- **A `Never`-returning filler** (`fallback: fatalError()`). Generated code that traps, which SCE_FORGE.md
  section 3.4.1 forbids ("never a trap"), reached only if the emitter's order is wrong.
- **A record built field by field from an `init()` whose union field is optional storage.** The same companion
  state as above, inside the record.
- **Untyped `throws`.** The catch then needs a branch for an error that cannot occur, the error is boxed in an
  existential, and the closed set of failures is a convention again. Typed throws (Swift 6.0, which every Swift
  package here declares as its tools version) is the spelling of `Result<T, AlgorithmError>`.
- **One `try` at the head of each statement** instead of one at each call. A `try` does not reach into a closure
  body, so the closures the generator writes would need one of their own whether or not they throw, and a `try`
  over nothing is a warning on every statement that cannot fail.
- **One family for every backend.** Moving Rust, Kotlin and Python to state passing would give them fillers their
  languages do not need; moving C11, C++ and Go to effects would need exceptions in C++ (which its forge code does
  not use), `panic` as control flow in Go, and `setjmp` in C. The language has one rule and each backend its own
  channel, as ADR 0092 section 7 has one sum and each backend its own type.
- **Make `us[i]` a narrowing subject**, invalidated by an assignment to `us` or `i`. The path is unstable by
  construction, and the binding repair already exists in the language; widening the subject set would make the
  narrowing analysis track index expressions for no construct it cannot already express.
- **Keep the two refusals and document them.** A lane that refuses what every other lane builds is a limit dodged,
  and its cause stays to refuse the next type.

## Open questions

None is put to the owner: each fork above was decided by the standing rule, and the rejected branch is recorded
with its reason.
