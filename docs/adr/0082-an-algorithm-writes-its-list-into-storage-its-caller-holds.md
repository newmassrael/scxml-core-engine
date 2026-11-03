# ADR 0082 — An algorithm writes its list into storage its caller holds

- Status: Proposed (2026-11-03). The owner ordered the work started on the recommendation of that day, under the
  rule of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost
  ignored. This draft is for the owner's review; it becomes Accepted when the owner accepts it.
- Date: 2026-11-03
- Amends:
  - SCE_FORGE.md §4.12, whose byte-buffer-build and bounded-list tables give C11 a "by-value
    `<symbol>_result_t`" and Rust a `Result<SceOwnedList<T, N>, _>` return.
  - docs/SCE_ACCEPTED_SUBSET.md §2.15 ("A bound a deployment chooses"), which tells the author of a large bound to
    build "for a backend whose lists live on the heap".
  - The C11 refusal of 2026-10-17 (`C11_LIST_STACK_LIMIT_BYTES`, `c11_list_too_large`,
    `StaticTarget::list_stack_limit_bytes`), which this ADR removes.
- Scope:
  - `sce-build/src/forge/generator.rs`: `render_algorithm`, `ListSpelling` (`return_type`, `declare`,
    `c11_result_typedef`), `ReturnChannel`, `c11_result_struct`, `c11_take_whole`, and the C11 stack refusal;
  - `sce-build/src/forge/static_lowering.rs`: the C11 and Rust `state_adapter`, `state_list_within`,
    `list_within`, `assign_list`, `list_result_type`, and the two stack refusals in the scope's lowering;
  - `tools/codegen/templates/forge/{c,rust}/conformance/kinds/algorithm.*.jinja2`, the harness that calls an
    algorithm;
  - `backends/c/forge-runtime/include/sce/forge/algorithm.h` and `backends/rust/portable-bytes`, which gain what a
    destination needs;
  - every host in the tree that calls a C11 or Rust algorithm returning a list or bytes.
- Related:
  - ADR 0050 (a build's bounds are read by name);
  - ADR 0053 (a bare-metal Rust crate asks for no feature);
  - ADR 0056, ADR 0071, ADR 0076 (a buffer is the document's size, and its storage is not a stack frame the runtime
    names);
  - ADR 0078 (a consumer follows SCE one landing at a time, and each API change says how);
  - ADR 0081 (a decision is made once, and each backend prints it).

## Context

An algorithm that returns a `list<T>` or `bytes` hands the list back by value on the two backends that hold it in
bounded storage. A survey of the tree on 2026-11-03 found the following.

- **C11 returns the whole bound in a struct.** A list-returning algorithm is
  `<symbol>_result_t <symbol>(…)`, where the struct is `{ T items[N]; size_t len; bool ok; [why;] }`
  (`ListSpelling::c11_result_typedef`). A `bytes` return is the same struct over `uint8_t bytes[N]`. The struct
  holds `N` elements whatever `len` is.
- **A statechart's call copies that struct several times.** For `<assign location="entries"
  expr="AddEntry(entries, …)"/>` over a list of records, the C11 machine's adapter (`CTarget::state_adapter`)
  copies the variable into `a0[bound]` of the algorithm's records, the algorithm builds its result in a local struct
  and returns it into the adapter's `r`, the adapter converts `r` into its own `out` and returns it, the machine
  passes that through `<adapter>_take` and `<adapter>_within` (each takes and returns the struct by value) into a
  local, and `memcpy`s the local into the variable. A list of numbers skips the first copy and the conversion. The
  C standard guarantees no elision of a struct copy, so each of these is a frame of `N` elements on the stack. The
  generator's own measurement, written beside the refusal: at a team calendar's bound, 65,536 values of 88 bytes, a
  call needs about thirty megabytes of an eight-megabyte stack.
- **The bound is therefore refused, not held.** Since 2026-10-17, a list that crosses a call with more than 256 KiB
  (`C11_LIST_STACK_LIMIT_BYTES`) is refused where the bound is built in, in the algorithm (`render_algorithm`) and
  in the machine (`static_lowering.rs`, twice). The refusal is correct as a guard. But it makes the largest bound a
  property of the C11 calling convention, not of the document: the same document builds at a bound on every other
  backend that C11 refuses.
- **Rust's bare-metal tier has the same shape, and no guard.** A Rust algorithm returns
  `Result<SceOwnedList<T, N>, _>`. Under `alloc` that is a `Vec`, and the return moves a pointer. Without `alloc` it
  is `InlineList<T, N>`, a `heapless::Vec<T, N>` of the whole bound, moved into a `Result` and returned. Rust does
  not guarantee that a return is built in its caller's slot (placement by return is not part of the language), and
  no refusal guards the bound. A bare-metal host that calls such an algorithm at a large bound overflows its stack
  silently.
- **The heap backends are not affected.** C++ (`std::vector`), Go, Kotlin, Python and Swift return a handle to a
  heap container; the return moves the handle.
- **A list-returning call is always a whole statement.** A statechart takes a returned list only as the whole
  expression of an `<assign>` to a list variable (SCE_ACCEPTED_SUBSET §2.15: "The call is the whole of the
  expression"), and an algorithm hands another algorithm only scalars and records. So two calls that return lists
  are never live at the same time in one machine, and every call site is known when the machine is generated.
- **A failed call leaves the variable as it was.** The same section requires that a call that fails is a failed
  statement: nothing is written, `error.execution` is raised and the block ends. An algorithm can fail after it has
  appended (an overflow in a later element).

## The textbook structure

A function whose result is an aggregate of bounded but large size returns it through a destination that its caller
supplies. This is destination-passing style. Minamide introduced the destination as a value in a functional
language ("A Functional Representation of Data Structures with a Hole", POPL 1998). Shaikhha, Fitzgibbon, Peyton
Jones and Vytiniotis showed that a compiler can lower array-returning functions to it so that no intermediate array
is allocated ("Destination-Passing Style for Efficient Memory Management", FHPC 2017). C's own library is written
this way wherever a result is a buffer: `snprintf`, `memcpy`, `strtol`'s end pointer, and the POSIX `_r` functions
(`localtime_r`, `strerror_r`), which replaced a static buffer inside the function with one the caller passes.

Where the caller's storage lives follows from two facts of this language: there is no recursion through a
list-returning call, and the call sites are known statically. Storage for such a program is allocated statically,
at the size of the largest frame that can be live at once (Aho, Lam, Sethi and Ullman, *Compilers*, 2nd ed., §7.1,
static against stack allocation). Embedded C codifies the same choice (MISRA C:2012 Rule 17.2, no recursion, and
Directive 4.12, no dynamic memory). The memory the machine needs is then a constant of its type, which a linker map
shows, and not a depth of its stack.

A statement that must leave its target unchanged on failure gives the strong exception guarantee (Abrahams,
"Exception-Safety in Generic Components", LNCS 1766, 2000). The textbook way to give it is to build the result
apart from the target and to commit it only on success.

## Decision

### 1. An algorithm that returns a list or bytes writes it into a destination

On the bounded backends the algorithm's signature takes the destination last and returns only whether it
succeeded:

| Backend | Destination | Return | Failure |
|---|---|---|---|
| C11 | `<symbol>_list_t *out`, a struct `{ T items[N]; size_t len; }` (bytes: `uint8_t bytes[N]`) | `bool` | a `may-fail` algorithm also takes `sce_forge_algorithm_failure_t *failure`, which it records into, as a body's checked operations already do |
| Rust (both profiles) | `out: &mut SceOwnedList<T, N>` (bytes: `&mut SceBytes<N>`) | `Result<(), E>` with the `E` of today | as today |

The body's one buffer (SCE_FORGE.md §4.12, v1 scope) is the destination itself. It starts by emptying it, appends
to it and reads it back (`len(out)`, `out[i]`) through the pointer or reference. Nothing of `N` elements is ever a
local, a parameter by value or a return. On failure the destination's contents are not specified, and no caller
reads them.

Rust takes the destination on both profiles, so a Rust signature does not depend on a feature (ADR 0053). Under
`alloc` the destination is a `Vec` the caller may reuse across calls, which also removes one allocation per call.

The heap backends keep returning their container. A destination there would save nothing, and their idiom is the
return.

### 2. The C11 machine holds the storage of its calls

A C11 machine that calls a list-returning algorithm gains one member, `scratch`: a union with one member per call
site. A call site's member holds the destination, in the algorithm's element type, and, for a list of records, the
buffer the machine's records are copied into before the call (the crossing of §2.15, which turns the machine's
record type and enum values into the algorithm's). Since no two such calls are live at once, the union is as large
as the largest call site, and a machine with no such call has no `scratch`. The storage is part of the machine's
`sizeof`, at the bounds it was built with, and is never on the stack.

The storage is the machine's and not the algorithm's. A `static` buffer inside the generated function would be
shared by every machine instance and every thread of a program, which is the defect the POSIX `_r` functions were
made to remove.

### 3. The call is a statement that commits on success

A list assignment from a call lowers to one statement:

1. copy the machine's records into the call site's input buffer, if the list is of records;
2. call the algorithm with views of its inputs and the call site's destination;
3. on failure, record it into the machine's `sce_failure_` and end the block, as every failed statement does today;
4. check the destination's `len` against the variable's `sce:capacity` (the check `_within` makes today);
5. copy the destination's `len` elements into the variable, converting each record back to the machine's type.

The variable is written only in step 5, after every way the statement can fail, so the strong guarantee holds by
construction. The commit costs `len` elements, not `N`. The `<adapter>_take` and `<adapter>_within` functions and
`list_result_type` are removed, because the statement no longer passes a list through an expression.

### 4. The stack refusal is removed

`C11_LIST_STACK_LIMIT_BYTES`, `c11_list_too_large`, `StaticTarget::list_stack_limit_bytes` and their three call
sites are removed in the landing that makes the last C11 call a statement. A C11 build then holds every bound
another backend holds, limited only by the memory its target has. SCE_ACCEPTED_SUBSET §2.15 loses the advice to
build "for a backend whose lists live on the heap".

### 5. What holds this

- **A large bound runs on C11.** The test that asserts the refusal
  (`a_bound_the_c_stack_cannot_hold_is_refused_where_it_is_built_in`) becomes a test that builds the calendar
  document at the bound of a team (`--limit merge_values=65536`, with its peers) and runs its scenario on C11.
- **No frame grows with a list's bound.** The C11 machine and the algorithms it calls are compiled with
  `-fstack-usage` at two values of the list limits (`merge_entries`, `merge_values`, `merge_digest`) that differ by
  a factor of 64. Every function's frame must be equal in both builds; a frame that differs is a defect of this
  ADR's kind, wherever it is. The same is
  measured for Rust's bare-metal tier through `backends/rust/probes/nostd-footprint`, with a list-returning
  algorithm added to the probe.
- **Every backend still agrees.** The conformance cases of §4.12 (COBS and the bounded list) and every scenario
  that takes a list (`static_list_assign`, `static_state_algorithm`, the todo, sheet and calendar worlds) replay on
  every lane, with the C11 gate still driving an input past `returns-max-size` and asserting that the call fails.
- **The headers still compile as C++.** The new signatures use nothing a C++ compiler refuses (no `restrict` and
  no compound literal of an array), and the `cxx_consumption` units hold that.

### 6. The change a host sees

A host that calls a C11 or Rust algorithm returning a list or bytes changes its call, once. The landing that
changes each backend says how (ADR 0078):

- C11: `T_result_t r = F(a, b); if (!r.ok) …` becomes `F_list_t out; if (!F(a, b, &out[, &failure])) …`, with
  `out` placed in storage the host owns.
- Rust: `let list = f(a, b)?;` becomes `f(a, b, &mut list)?;`, with `list` a `SceOwnedList<T, N>` the host keeps.

## Order of work

1. This ADR.
2. Rust. The algorithm takes `&mut` its destination, on both profiles. The Rust machine's adapter, the conformance
   harness and every Rust host in the tree move with it in the same landing, and the regeneration changes the
   generated files. The bare-metal probe gains its list-returning algorithm, and the frame measurement of §5.
3. The C11 algorithm. The destination struct, the `bool` return and the failure pointer; the conformance harness
   and the COBS gate move with it. The machine keeps calling through an adapter that holds the destination in a
   local until step 4, so this landing changes no machine's behaviour.
4. The C11 machine. `scratch`, the statement of §3, and the removal of `_take`, `_within` and
   `list_result_type` for lists. The stack refusal goes (§4). The refusal test becomes the large-bound test, and the
   `-fstack-usage` gate is added.
5. SCE_FORGE.md §4.12 and SCE_ACCEPTED_SUBSET §2.15 are rewritten to the new signatures and the removed refusal, in
   the landings that change them.

## Consequences

- The largest bound a document can be built at is the same on every backend. On C11 and bare-metal Rust it is
  limited by the target's memory, as a static object, and a linker map shows it.
- A C11 machine that calls a list-returning algorithm grows by its largest call site, permanently. Before, the same
  memory was taken from the stack several times over, for the length of the call.
- A list assignment from a call copies `len` elements once into the variable (and, for records, once into the
  input buffer), instead of the several copies of `N` that the Context counts.
- Hosts of C11 and Rust algorithms that return lists or bytes change their calls once (§6).
- Rust's bare-metal tier loses a stack overflow that no refusal guarded.

## Considered alternatives

- **Raise the limit, or ask for a larger stack.** Refused: a microcontroller's stack is a few kilobytes, a thread's
  stack is set by whoever starts the thread and not by the machine, and the bound would stay a property of the
  calling convention.
- **Allocate the result on the heap in C11.** Refused: SCE_FORGE.md §4.12 makes an algorithm allocation-free on the
  no-alloc profile, and C11 is that profile's backend.
- **A `static` buffer inside each generated algorithm.** Refused: it is shared by every machine instance and thread
  of a program, and a second call overwrites the first one's result. This is the defect the POSIX `_r` functions
  were made to remove (§2).
- **Write straight into the variable.** Refused: an algorithm can fail after it has appended, and the statement
  must leave the variable as it was (§3). It would be correct only for an algorithm proven to fail before its first
  append. The algorithm language has no such effect analysis, and the commit costs only `len` elements.
- **Two buffers per list variable, committed by a swap.** Refused: it doubles every such variable's storage for the
  life of the machine to save a copy of `len` elements, and a host's view of a published list would change address
  at every commit.
- **One C type per schema, so that no record crosses.** Not this ADR's subject, and refused here: a machine declares
  its own record types so that its header stands alone. Two machines of one program may also be built at different
  `--limit` values, which give the "same" schema different string bounds and so different layouts.

## Questions left open

None that block the work. This draft waits for the owner's acceptance. The first step after it is step 2 above.
