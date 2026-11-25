# ADR 0133 — A statement is total exactly where the build proves it, and every reader of its failure reads one classification

- Status: Accepted (2026-11-25), under the owner's rule for this work: the most long-term correct, textbook
  structure, cost not weighed; no workaround, no placeholder.
- Date: 2026-11-25
- Scope: `sce-build/src/forge/int_ranges.rs` (`proven_operations`), `sce-build/src/forge/expr.rs`
  (`discharge_proven_arithmetic`, `lower_at`), `sce-build/src/forge/types.rs` and `type_ctx.rs`
  (`TypeCtx::proves_operations`), `sce-build/src/forge/static_lowering.rs` (`Rewrites::held_to_bound`,
  `Rewrites::held_text`, `fitting_text_bound`, `StaticTarget::fitting_string`, `payload_record_value`),
  `sce-build/src/model.rs` (`Action::statement_can_fail`), `sce-build/src/internal_queue_bound.rs`,
  `backends/rust/portable-bytes` (`SceString::within`), `backends/cpp/forge-runtime` (`SCE::Forge::Proven::held`),
  `docs/SCE_ACCEPTED_SUBSET.md`, `SCE_FORGE.md` §3.4.1.
- Related: ADR 0076 (an internal queue is sized by what its document can raise), ADR 0086 (the bounded text
  type, section 7.4, and the bound of an expression's text, section 8.6 (c)), ADR 0063 (an operation that names
  its rule is total), SCE_FORGE.md §3.4.1 (the integer arithmetic contract, E12 D5).

## Context

A downstream bare-metal consumer generates a `no_std`, heap-free Rust machine from a session lifecycle: a delayed
`<send>` in each atomic state's `<onentry>`, a `<cancel>` of the same id in its `<onexit>`, an overall limit on a
compound state, and typed `<assign>`s under the `sce-static` data model. Nine of those `<assign>`s were counted
as statements that can raise `error.execution` (§scxml-5.4, §scxml-5.9), and the internal queue the build proves
(ADR 0076) went from 0 deep to 6 and 18, each slot carrying a 40-byte payload.

Every reader of a statement's failure reads one field, `Action::native_fails`, which the static lowering writes
beside the statement it lowers: the templates print the error path where it is set
(`actions/_native_statement.rs.jinja2` and its siblings on every backend), and `InternalQueueBound` counts one
`error.execution` for each block that holds such a statement. The field was right about which statements hold a
failure in their code. It was wrong about which statements can fail, because the lowering put a failure in the code
of statements that have none:

1. Under `receives_failures` (set for every `sce-static` statechart expression, E12 D5) every integer
   `+ - * / %` and unary `-` became a checked operation, whatever its operands. `count / 2 + 1` on a `uint8`
   cannot overflow and `x % 10` cannot divide by 0, yet both were lowered to the runtime's checked helpers and the
   statement holding them was marked as one that can fail.
2. Every store of a text into a `string[N]` place went through the target's checked construction, whatever the
   text. `<assign location="phase" expr="'waiting'"/>` into a `string[16]` held a capacity check that can never
   fire, and so did a copy of one `string[16]` variable into another, and a payload taken whole into a record of the
   payload's own schema.

The build already had the two analyses that decide both questions: the interval analysis that proves the integer
operations of an algorithm that declares no `may-fail` (`int_ranges`, SCE_FORGE.md §3.4.1 "Enforced"), and the
bound of an expression's text (`text_bound`, ADR 0086 section 8.6 (c)), which the C11 join and the size calculus
read. The statechart lowering asked neither.

## Decision

### D1. The rule

A statement that stores a value (an `<assign>`, the location of an `<sce:call>`, a field of a union written in
place, a payload taken whole into a record) is total, and is classified as one that cannot fail, exactly when the
build proves that neither computing the value nor storing it can fail under the `sce-static` data model. At any point
of a run each variable and each payload field holds a value of its declared type and nothing else, so the proof reads
every name at the whole range its declared type holds. It is total when all of the following are proven:

1. **Arithmetic.** Each integer `+ - * / %` and unary `-` of the value is proven by the interval analysis
   (`int_ranges::proven_operations`): its result lies in the type it is computed at, its divisor's range holds no 0,
   and a signed one does not divide its type's minimum by -1. A condition of the same expression narrows what its
   branch reads, as a guard narrows a name in an algorithm's body: `n > 0 ? n - 1 : 0` on a `uint8` is proven,
   `n + 1` is not, and `b !== 0 ? a / b : 0` is. The width an operation is judged at is the width it is emitted at:
   the analysis reads the very tree the emitter prints. A sum of two `uint8` values stored into a `uint16` is
   computed at `uint8` (SCE_FORGE.md §3.4, "Typing convention") and is not proven.
2. **Calls.** No call of a callee that declares `may-fail`; such a call fails by its own declaration.
3. **Index reads and key lookups.** None: a list's declared bound holds its length from above and never from
   below, so by declared types alone an index can always miss, and a key can always be absent.
4. **Integer stores.** The value's type is one the place's type holds every value of (a widening, a `uint8` into a
   `uint16` or an `int32`). A store into a narrower type is judged by the types alone, as it was, and stays checked.
5. **Text stores.** The most bytes the build proves of the text (`text_bound`) is no more than the place's
   `sce:capacity` or `sce:max-size`: a literal by its length, a place by its declared bound, a join by the sum, a
   conditional by the larger branch, a call by the bound its callee states on what it returns
   (`fitting_text_bound`). A payload's text field taken whole into a record of the payload's schema is of the
   record field's own bound, and fits by its type.
6. **Everything else stays as it is.** A byte-string store, an append to a list, a list or a map an algorithm
   returns, a write of a map at a key: each stays checked. The Interpreter's byte-string store refuses a character
   past U+00FF as well as a length past the bound, and a byte string of the Go machine is copied by its checked
   constructor, so a byte-string store can fail on an engine for a reason no bound rules out; an append can always
   find its list full; an algorithm's list and map bounds are advisory where lists grow (ADR 0082).

Anything the build does not prove stays fallible. The proof is the build's, never the author's: there is no
attribute that declares a statement total, since such a declaration would be a claim nothing checks.

### D2. One classification, read by every consumer

The static lowering classifies each statement once, from the two proofs above, and writes the result on the
statement as `Action::native_fails`, beside the code it describes. `Action::statement_can_fail` is how a reader
asks it. The internal queue bound counts an `error.execution` only for a statement it says can fail, and every
backend's dispatcher prints an error path only where it does, so the bound counts exactly the error paths the
machine holds. A total statement's generated code holds no error path at all: no `?`, no closure that receives a
failure, no failure record, no `error.execution` raised.

The classification is the same document property on every backend: the proof reads the document and its declared
types, never the target. What differs is only how each target prints an operation or a store the proof discharged
(D3, D4).

The same expression rule classifies every expression the statechart lowers, since one lowering lowers them all: a
guard, an `<if>` condition, a `<send>` parameter, a `<log>` value and an `<sce:action>` argument that the
declared types prove raise nothing either.

### D3. A proven operation is printed as a proven operation of an algorithm's body is

An operation the proof discharged is printed by each target as it prints a proven operation of the body of an
algorithm that declares no `may-fail`: Rust through the forge runtime's `algorithm::proven` functions
(`Discharge::AtBuild`), so the crate a machine is compiled in holds no arithmetic operator of its own; every other
backend and the Interpreter by the operator itself, with the conversions each already writes for one
(`Math.trunc` for an integer division in the Interpreter). An operation the proof did not discharge is the checked
helper it was. `TypeCtx::proves_operations` turns the proof on; the scope a statechart's expressions are typed in
sets it with `receives_failures`, and nothing else does. The body of an algorithm that declares `may-fail` keeps
checking every operation, as SCE_FORGE.md §3.4.1 says it does.

### D4. A proven text store builds the place's value with no failure in it

- Rust holds a text in `SceString<N>`. A text proven to be at most `L` bytes is built by
  `SceString::<N>::within::<L>(&value)`, which judges `L <= N` when the machine is compiled (`Fits`) and answers
  no `Result`. A text that breaks the proof stops the program rather than be cut or emptied, as
  `algorithm::proven::at` does for an index.
- C++ holds a text in `SCE::Forge::OwnedText<N>`. A proven text is built by `SCE::Forge::Proven::held<OwnedText<N>>`,
  which records no failure; a value that breaks the proof calls `std::abort`.
- Kotlin, Go, Python, Swift, C and the Interpreter hold a text in a type that holds any text, and a proven text is the
  value itself.

### D5. The internal queue bound

ADR 0076's rule is unchanged: each block counts the most internal events one run of it can enqueue. A total
statement enqueues nothing, so a document whose statements are all total keeps a bound of 0 from them, and a
statement the proof does not discharge is counted as before.

## Consequences

- Every generated tree whose documents hold an operation or a text store the proof discharges changes: the Rust,
  Go, Kotlin, Python and Swift static data model trees, the C and C++ machines, the Interpreter's lowered
  documents, and each golden that holds one. The change is the removal of a check that could not fire and of the error path around it.
- A document's internal queue bound can only fall: no statement that was total is newly counted.
- The runtimes gain one constructor each where the text place holds its bound in its type: `SceString::within` in
  `sce-portable-bytes` and `SCE::Forge::Proven::held` in the C++ forge runtime.

## Alternatives not taken

- **An author's declaration that a statement is total.** The build can decide it, and a declaration it did not check
  would be the silent disagreement the integer contract exists to remove.
- **Flow-sensitive facts from a transition's guard and the statements before.** A guard such as
  `cond="retries < 3"` bounds what the transition's content reads until a statement writes it, and a proof that read
  it would discharge more. This decision proves by declared types and by the conditions of the expression itself; a
  proof over the guard and the block is a decision of its own, with its own account of which statement ends a fact.
- **Leaving a check in the generated code and only not counting it.** The bound would then count fewer error paths
  than the machine holds, and an undercount stops a bare-metal machine (ADR 0076).

## Verification

- `int_ranges`: a sum that can pass its type is not proven; an operation the declared ranges keep in its type is; a
  divisor that can be 0 is not, unless the expression rules it out; a condition narrows its branch; a signed negation
  and a division by -1 are not proven.
- `expr`: a statechart checks an operation its declared types do not prove and discharges one they prove, in Rust
  through `algorithm::proven` and in the Interpreter and C as the operator; a division its own condition guards is
  discharged; a literal, a read of the same type and a widening store cannot fail, a narrowing one can.
- `static_lowering`: a literal and a variable that fit a Rust `string[16]` are built by `within` with no `?`; a text
  store fails only past what the build proves of the text.
- `internal_queue_bound`: a document whose only statements are total assigns keeps the bound at 0 and its Rust
  holds no error path; one assign the declared types do not prove is counted.
- `sce-portable-bytes`: a text proven to fit is stored whole, and one that breaks its proof stops the program.
