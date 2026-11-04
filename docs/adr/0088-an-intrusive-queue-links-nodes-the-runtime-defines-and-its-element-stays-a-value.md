# ADR 0088 — An intrusive queue links nodes the runtime defines, and its element stays a value

- Status: Accepted (2026-11-05). The owner ordered this structure ("시작") after reviewing its alternatives, under the
  rule of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored.
  The question came up in ADR 0081 step 4: a printed C++ codec value is made only by its constructor, and the
  intrusive queue wrote a field of its element in place.
- Date: 2026-11-05
- Amends:
  - SCE Protocol-Synthesis RFC §synth-5-P, the `intrusive` storage mode: "each element carries its own link field"
    stops being true. The queue links nodes of a type its runtime defines, and the element is the value a node holds.
  - The authoring grammar: `<sce:intrusive>` takes no attribute, and `link-field` is removed.
- Scope:
  - `sce-build`: the queue's model and reader (`QueueStorage::Intrusive`), its cross-reference check, its
    diagnostics, the queue generator for Rust, C++ and C11, and its tests (`queue_kind.rs`, `queue_deploy_limits.rs`);
  - the Rust, C++ and C11 forge runtimes' intrusive lists (`queue/intrusive.rs`, `sce/forge/queue.h` in both, and
    C's `queue_irq.h`), their tests and history recorders;
  - the conformance fixtures `queue_conformance_node`, `queue_conformance_intrusive`,
    `queue_conformance_intrusive_many` and `queue_intrusive_mpsc`;
  - `schemas/sce-forge-ext.xsd`, `apis/forge-ast.v1.schema.json`, the diagnostic schema, SCE_ERROR_CONTRACT.md and
    docs/SCE_ACCEPTED_SUBSET.md.
- Out of scope: Go, Kotlin, Python and Swift, whose runtimes have no intrusive mode and refuse it
  (`QueueStorageRuntimeMissing`). Nothing changes there.

## Context

The `intrusive` mode is Vyukov's intrusive MPSC list. The caller owns an array of nodes, a push cannot fail, and the
caller gets back the very node it pushed. A link is the index of the next node, so the array means the same in every
process that maps it.

Until now the link was a field of the element. The queue document named it (`<sce:intrusive link-field="next"/>`),
and the element codec declared it as one of its fields: `queue_conformance_node` holds `sensor_id`, `value` and a
`uint32` `next` at byte 3. Each runtime reached that field and wrote it in place, as an atomic:

- Rust, by byte offset: a marker type per queue implements `unsafe trait Link<T> { const OFFSET }`, and the list
  casts `nodes + i * size_of::<T>() + OFFSET` to `*const AtomicU32`;
- C++, by a pointer to a data member: `IntrusiveMpsc<T, std::uint32_t T::*Link, …>` reinterprets
  `&(nodes_[i].*Link)` as a `std::atomic<std::uint32_t>*`;
- C11, by offset: the runtime takes the element's `stride` and `link_offset`, and the generated header checks the
  field's size and alignment with `SCE_STATIC_ASSERT`.

ADR 0081 step 4 makes a C++ codec value immutable: made only by its constructor, read through accessors. The node
codec is among the first codecs the C++ printer prints, and its `next` is then no data member that a pointer to a
member could name, nor one anything may write. The conflict is not one of C++ alone: it shows that the link was
never an element of the value.

## The textbook structure

A list's link belongs to the list, not to the value a node holds.

- In Boost.Intrusive the link is a *hook*. A hook lives in the object, but it is not part of the object's value:
  copying a hook does not copy the link, and comparing values never reads it. The container owns the hook while the
  object is in the container.
- Stepanov and McJones, *Elements of Programming*, chapter 1, separate an object from the value it represents. An
  object's identity (here, the node's index) and its bookkeeping are not part of its value.
- Vyukov's MPSC node is `struct node { node* next; T value; }`, with `next` an atomic object.
- An object read and written atomically must be an atomic object. Casting a plain integer to `std::atomic<uint32_t>`
  is not a conversion the C++ standard provides; that is why C++20 added `std::atomic_ref` (P0019). Rust's
  `AtomicU32::from_ptr` likewise requires the place to be accessed only atomically.

The link held as a codec field contradicted each of these. The element's encode wrote the link into the wire form,
although an index means nothing outside the array it indexes. Copying an element copied the link. Comparing two
elements compared their links. A value written by its constructor had to be written in place. And every runtime cast
a plain integer to an atomic.

## Decision

### 1. The node is the runtime's, and depends only on the element type

Each runtime defines one node type, generic over the element:

- Rust (`sce_forge_runtime::queue::intrusive`): `#[repr(C)] pub struct Node<T> { pub value: T, next: AtomicU32 }`,
  made by `Node::new(value)`. The list is `Mpsc<T, const MANY_CONSUMERS: bool>` over the caller's array of
  `Node<T>`, taken as now by a raw pointer and a length (`unsafe fn new(nodes: *mut Node<T>, len, stub)`), and it
  reaches a link as `&(*nodes.add(i)).next`, an `AtomicU32` field. The `Link` trait and its `OFFSET` are retired.
- C++ (`sce/forge/queue.h`): `template <class T> struct Node { T value; std::atomic<std::uint32_t> next; }`, made
  from a value (`explicit Node(T value)`) or by default. The list is `IntrusiveMpsc<T, bool ManyConsumers>` over
  `Node<T>*`. There is no pointer to a member and no `reinterpret_cast`. A node is not copyable, as its atomic is
  not: a node is an object, named by its index.
- C11 (`sce/forge/queue.h`, `queue_irq.h`): C has no generics, so the queue header generates the node:
  `typedef struct { <element>_t value; uint32_t next; } <element>_node_t;`. A guard macro named after the element
  defines it once, so two queues over one element share one node type. The runtime keeps its type-erased core over
  `stride` and `link_offset`, now `sizeof(<element>_node_t)` and `offsetof(<element>_node_t, next)`. The link is
  reached only through the runtime's atomic operations (`sce_atomic_*_u32`), as every ring of that runtime is, and
  under masked interrupts in `intrusive_irq`.

In every runtime the link is placed after the value. `#[repr(C)]` in Rust, and a standard-layout `T` in C++ and C,
fix its offset, so an array of nodes is laid out the same in every process that maps it. Whether a node array may be
shared between processes also depends on its element: a value that holds a pointer may not. That is a property of
the element type, and the shared-memory channel that needs it will check it (RFC §synth-5-P, Migration). The node
adds nothing that is not position-independent.

Because the node type depends only on the element type, a node may be pushed to one queue, popped, and pushed to
another queue over the same element; a free list and a work queue over one array is the usual use. A node is in at
most one queue at a time. Membership in several queues at once would need a node with several hooks; that is an
additive extension (a node type over a list of hook names), and nothing asks for it today.

### 2. The element is a value

An intrusive queue's element is any codec, as any queue's is. It declares no link, and its encode, decode,
constructor and accessors know nothing of queues. `queue_conformance_node` is retired. The intrusive conformance
queues hold `queue_conformance_event`, the element the other conformance queues hold.

### 3. The document

`<sce:intrusive/>` takes no attribute. A document that still writes `link-field` is refused with
`queue/intrusive-link-field-removed`, stage validation. Its fix is `remove_fields` on `sce:intrusive` with the field
`link-field`, and its message says that the element no longer carries the link and that the field the attribute
named can leave the element codec. This follows the precedent of `worker/inbox-inline-removed`.

`queue/intrusive-link-field-missing` and `queue/intrusive-link-field-not-u32` are retired: the link is no longer the
author's to name, so neither can happen. The authoring grammar, the forge AST and the diagnostic schema are
`pre-release` (SCE_WIRE_CONTRACTS.md), so these non-additive changes are admissible.

### 4. Wire surfaces

- `schemas/sce-forge-ext.xsd`: `intrusive` becomes an empty element.
- `apis/forge-ast.v1.schema.json`: the `intrusive` storage variant carries no `link_field`.
- `schemas/sce-diagnostic.v1.schema.json`, SCE_ERROR_CONTRACT.md and the appendix of docs/SCE_ACCEPTED_SUBSET.md:
  the two retired codes leave, and the new one joins.

### 5. Generation

The queue generator stops reading a link field. Each intrusive queue names its node type:

- Rust: `pub type {Queue}Node = Node<{Element}>;` and `pub type {Queue} = Mpsc<{Element}, MANY_CONSUMERS>;`;
- C++: `using {Queue}Node = ::SCE::Forge::Queue::Node<{Element}Type>;` and
  `using {Queue} = ::SCE::Forge::Queue::IntrusiveMpsc<{Element}Type, MANY_CONSUMERS>;`;
- C11: `<element>_node_t` as in section 1, and the queue's `init` takes an array of it.

The checks on the target's atomics (a 32-bit exchange, or masked interrupts for `intrusive_irq`) are unchanged.

### 6. Tests

- The contract scenarios of `tests/forge/conformance/queue_contract.json` name nodes by index and do not change.
- Each runtime's generated test holds an array of `Node<Event>` (`queue_conformance_event_node_t` in C) and checks
  that the queue leaves each node's value as it was: what the old test checked as "every field but the link".
- Each runtime's hand-written runtime test builds its nodes from the runtime's `Node`, not from a struct of its own
  with a link field.
- `queue_kind.rs` replaces its missing-field and not-u32 cases with a case for `link-field` refused, and checks that
  an intrusive queue over a codec with no `uint32` field is generated on Rust, C++ and C11.
- The history recorders link the runtime's nodes; the histories they write do not change.

## Landing order

1. This ADR.
2. One change for all of it: the reader, check and diagnostics; the wire surfaces; the three runtimes and their
   templates; the fixtures; the tests. The document grammar and the runtimes change together, so no state in between
   builds.
3. ADR 0081 step 4, landing 1 (the C++ printer's first codecs), rebased onto it: no element of a queue is then
   written in place.

## Consequences

- A codec value is a value in every runtime, whatever holds it. ADR 0081's immutable C++ value needs no exception.
- No runtime casts a plain integer to an atomic.
- An intrusive element's encode no longer writes an index onto the wire.
- A consumer's intrusive queue document that writes `link-field` is refused with a fix, and the field the attribute
  named leaves its element codec. A consumer's code that built its own node struct builds the runtime's `Node`.
  `docs/SCE_CODEC_CONSUMER_MIGRATION.md` says so.
- A node is in one queue at a time (section 1).

## Alternatives rejected

- **Keep the link in the element, marked as a link in the codec document, and leave it a public data member of the
  printed C++ class.** This kept the documents and runtimes as they were. It keeps the index in the wire form, the
  link in the value's copy and comparison, and the integer-to-atomic cast, and it makes one element of an immutable
  value writable. It is the cost-led answer to the conflict, not the structural one.
- **Publish a pointer to each element's storage from every printed C++ class, for intrusive containers.** It gives
  every element a way to be written past the constructor, which voids ADR 0081 step 4's decision for every codec to
  serve one queue.
- **Keep the node codec out of the C++ printer.** The printer decides from a codec's own document whether to print
  it, and a codec does not know which queue holds it. The only way to know would be a marker on the codec, which is
  the first alternative.
- **Let each queue define its own node type.** Two queues over one element would then have two node types, and a
  node could not move from one to the other.

## Owner's answers

- 2026-11-05: the link is the list's hook, not an element of the value; the runtime defines the node ("시작", in
  answer to the recommendation of this structure over the three alternatives above).
