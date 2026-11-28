# ADR 0161 — A key an algorithm looks up again with nothing between that can move it is found once

- Status: Proposed (2026-11-28), for the owner's acceptance. Written under the owner's standing rule (the most long-term correct structure, textbook in
  its foundations, cost ignored; decided, not asked) at the request of a downstream consumer, whose measurement named
  the cost: a step of a generated algorithm that asks `has(m, k)`, reads `m[k]` and puts at `k` searches the map for
  `k` three times.
- Date: 2026-11-28
- Amends: nothing a document says. ADR 0108 D3 and D6 keep every read and statement as they are; this record decides
  how a lane lowers them.
- Scope: the generator's analysis of an algorithm's body (lane-independent), the Rust runtime's map
  (`sce_portable_bytes::map`) and the Rust lowering of an algorithm's map reads and puts. C11 is step 4; the
  Interpreter and the heap lanes are not changed (D5).
- Related: ADR 0108 (a map; D3 its reads and statements, D6 item 2 the proof a key is held, D8 no operation moves an
  entry), ADR 0154 D2 (an entry stays in its slot from the put that inserts it to the remove that takes it out).

## Context

`has(m, k)`, `m[k]`, `m[k].f` and `<sce:put target="m" key="k">` each find `k` in `m` from nothing: a hash of
the key and a probe of the index, or in `Linear` storage a scan of up to the bound's keys, each probe comparing the
key with one the map holds. A consumer's table rules are written as the document language reads best, a test, a
read and a write of one key:

```xml
<sce:if cond="has(peers, zid)">
  <sce:var name="last" type="uint64" init="peers[zid]"/>
  <sce:if cond="gen > last"><sce:put target="peers" key="zid" expr="gen" outcome="o"/></sce:if>
<sce:else/>
  <sce:put target="peers" key="zid" expr="gen" outcome="o"/>
</sce:if>
```

so the cost it measured is three searches where a hand-written table makes one. The document should not change: a
language that made the author hold a slot would put a handle in a document's value, which ADR 0108 D2 refuses, and
would make every author do by hand what a compiler does for all of them.

## The textbook structure

- **Common subexpression elimination over available expressions** (Aho, Lam, Sethi, Ullman, *Compilers*, 9.2.6): an
  expression is available at a point when every path to it computed the expression and nothing on the path since wrote
  an operand. A computation that is available is replaced by the value computed before. Here the expression is the
  search of `m` for `k`, its operands are the map and the key, and what it computes is where `k` is in `m` or
  that it is absent.
- **What a write kills is what it can change**, not every write: a search's result is killed by a write that can move
  the entry or change whether the key is held. ADR 0108 D8 and ADR 0154 D2 say what can: no put moves an entry, a put
  of a key that is present writes the value where it stands, and only a remove or a clear frees a slot. A put of
  another key can make an absent key present (the two keys may be equal at run time), so it ends what is known of
  every key of that map.
- **A structured body is its own dominator tree.** In a body of blocks, an `<sce:if>` and loops, a statement earlier
  in a block, or in a block that holds it, runs before it on every path. A search made before a statement of the same
  or an enclosing block is therefore available there unless something between kills it, and a loop is entered with
  what its body as a whole does not kill (the fixed point of a loop whose body only removes facts).
- **A position is a handle the compiler holds, checked where it is cheap.** The search's result is a position of the
  entry (`MapCursor`, ADR 0108's walk) or the absence with what an insert of the key needs (its stamp), `Copy`, holding
  no borrow, so the code between may read the map. The proof that nothing between moved it is the analysis'; a debug
  build asserts it where the handle is used, as a proven index of a list is read.

## Decision

### D1. A search is available where an earlier one of the same map and key reaches with nothing between that kills it

- **A search** is the lookup each of `has(m, k)`, `m[k]` and `m[k].field` makes, and the one a
  `<sce:put target="m" key="k">` makes, where `m` names a map or a set the body holds (a parameter or a local) and
  `k` is a name or a field of one (`zid`, `peer.zid`): the keys the proof that a key is held already reads (ADR 0108
  D6 item 2). A key that is a call, an arithmetic expression or a read at an index is not one: hoisting it could move a
  failure, and the analysis does not decide what such a key equals.
- **It is killed** by:
  - a `<sce:remove>` or `<sce:clear>` of the map, a put of the map at another key, a put into a place inside the map
    (`<sce:put target="m[k]">`), an assignment of the map whole, and any other write of it the narrowing reads a
    statement as making (`narrowing::Writes`): every search of the map;
  - a write of the key's name or of the field it reads, or of the name whole: every search with that key.
  A put of the map at the same key does not kill it: the entry stays in its slot (ADR 0154 D2) or is put at the
  position the search found absent, and the search then names the entry.
- **It is available** at a statement where a search of the same map and key was made by an earlier statement of the
  same block, by the statement that holds the block, or before that, with no kill between. A path that returns
  constrains nothing after it (an unreachable point meets as no constraint). A loop's body and condition are entered
  with what the body does not kill; a search the body makes is not available after the loop or on its next
  turn; a search a branch of an `<sce:if>` makes is not available after the `<sce:if>`.
- **One search is made** where a search of a map and key is first made and is available at one more, at least, before
  it is killed: it is made before the statement that first makes it (its key cannot fail, so the moment it is made in
  is not observable), and every search of the same map and key it reaches reads its result. A search reached by no
  other is lowered as before.

### D2. The Rust runtime finds a key once and acts through what it found

- **`MapFound<G, H>`**, `Copy`: `At(MapCursor<G>)`, the position of the key's entry, or `Vacant(H)`, the key absent
  with the stamp an insert of it keeps (its hash in an indexed storage, `()` in `Linear` and `Lent`).
- **`BoundedMap::find(&key) -> MapFound`**, and through what it found: `found_holds`, `value_found` (`&V`),
  `put_found(&mut found, key, value) -> PutOutcome`, which writes the value at the position (`Replaced`) or inserts
  the key with its stamp without searching again (`Inserted`, or `Full` at the limit, the map as it was) and leaves
  `found` naming the entry. A set has `find` and `put_found` too, with no value. The reads module gains `found_holds`,
  `found_value`, `found_value_cloned`, `found_held` and `found_held_cloned`, beside the reads by key they stand for.
- **A debug build checks the proof.** `put_found` asserts that `Vacant` names a key the map does not hold and that the
  entry at `At` is the key's (`debug_assert!`); a read at `At` reads the position as a walk does, refused where its
  slot holds no entry or, in a storage a host lends, an entry put since (ADR 0154 D2).

### D3. The Rust lowering

- Before the statement where a search is first made the body declares `let sce_found_<n> = m.find(&k);`, the key
  spelled as the probe a remove looks a key up by (`let mut` where a put goes through it). `has(m, k)` is `found_holds(&sce_found_<n>)`, `m[k]` and `m[k].f` read
  through `found_value`, `found_value_cloned`, `found_held` or `found_held_cloned` as their reads by key would, and the
  put is `m.put_found(&mut sce_found_<n>, key, value)`, its outcome written as before.
- What the generated code does is what it did: the same answers, the same outcome, the same failure where a read of an
  absent key fails (the read through `Vacant` answers `None`, which `found` fails as before).

### D4. The analysis is the generator's, once, for every lane

The analysis is one module of `sce-build` (`forge::map_searches`) over `AlgorithmStmt`, using what a statement
writes as the narrowing reads it (`narrowing::Writes`), so every lane that lowers the search through what it found
reads one plan. A lane that does not lowers every search as before.

### D5. Lanes

Rust lands with this record. C11 follows (Order of work, step 4): `sce_map_find` and the reads and put through what it
found, in `sce/forge/map.h`, lowered from the same plan. The Interpreter's `Map` and the heap lanes' maps search with
their language's own map and are not changed.

## Consequences

- The consumer's rule above searches the table once on every path.
- A generated Rust algorithm reads a `sce_found_<n>` local where it searched its map more than once; nothing a document,
  a scenario or a saved state shows changes.
- A wrong plan would be a wrong program, not a slow one: a debug build asserts the plan where it is used, and the
  analysis is held by tests of what it keeps available and what it kills.

## Alternatives rejected

1. **An entry or a handle in the document language** (`<sce:entry>`, a slot value). It puts a handle into a document's
   value (ADR 0108 D2 refuses it) and asks each author for what a compiler gives every one.
2. **Rust's `Entry` API in the generated code.** `entry` borrows the map `&mut` until the entry is used, and the code
   between a test and a put reads the map; a position that holds no borrow is the form that code can keep.
3. **A cache inside the map** (the last key searched and its result). It costs every map a word and a comparison on
   every search, and a key compared again is the cost this record removes.
4. **Killing on every write of the map.** Correct, but a put of the key itself is the write the consumer's rule ends
   with, and ADR 0154 D2 already promises the entry does not move.

## Order of work

1. This record.
2. The analysis (`forge::map_searches`) and its tests: what is available, what each statement kills, the loop and
   branch rules, and the plan's openings.
3. Rust: `MapFound`, `find`, the reads and the put through it, a set's; the lowering of D3; a test that a body which
   tests, reads and puts one key searches once, and that the conformance and the numerical reference run unchanged.
4. C11: `sce_map_find` and the reads and put through it in `sce/forge/map.h`, lowered from the same plan, measured on
   the consumer's C image before it lands.
