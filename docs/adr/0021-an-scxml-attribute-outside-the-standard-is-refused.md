# ADR 0021 — An SCXML attribute outside the standard is refused

- Status: Accepted and implemented (the owner asked for it on 2026-10-11)
- Date: 2026-10-11
- Scope: `sce-build/src/scxml_attributes.rs` (the table and the refusal), the two parsers
  (`sce-build/src/parser.rs`, `sce-build/src/forge/parser.rs`), `validation/unknown-scxml-attribute`,
  `tests/w3c/impl/TXMLConverter.cpp`

## Context

`schemas/sce-forge.xsd` takes a W3C element laxly and leaves its structure to the parser, and the parsers read the
attributes they know by name. An attribute the Recommendation does not define for an element is never asked for, so
nothing said so.

Measured 2026-10-11, on a document an AI wrote for the platform: `<state id="main" type="parallel">` built with exit 0
from `check` and from `generate`. It meant an ordinary compound state, so only the first of its regions was entered
and six of the eight test cases failed. The element for concurrent regions is `<parallel>`; a `<state>` takes `id` and
`initial` and nothing else.

The same silence was in this repository, in four documents: a `<send expr="_event.data"/>` twice (a `<send>` has no
`expr`, so the data it was meant to carry was never sent), a `<state description=…>`, and three W3C conformance
fixtures (354, 519, 534) whose converter had written a condition as `name` or `expr` on a `<transition>`.

## Rejected: a warning

The document is not the one its author wrote. A warning leaves a build that runs and fails a test case later, which is
the failure measured. The other unread-attribute rule (`validation/sce-attribute-unread`) already refuses rather than
warns, for the same reason.

## Rejected: a list kept beside the parsers, as `sce:` has none

What an element takes of `sce:` is whatever the parsers read, so no list is kept for it. For the unqualified
attributes the Recommendation IS the list, independent of what the parsers happen to read, so the list belongs to the
Recommendation and is checked against it.

## Decision

`SCXML_ATTRIBUTES` holds, for each element, the attributes the Recommendation's attribute table lists, in its order.
Both parsers refuse a document in which an unqualified attribute of an SCXML element is outside its entry, as
`validation/unknown-scxml-attribute` (stage `validation`, spec `W3C SCXML §A.1`, fix `remove_fields`), one record per
attribute at its own row, carrying what the element takes, the nearest spelling, and for the two mistakes whose shape
is known (`<state type="parallel">`, `<send expr>`) the element or child that was meant.

Left alone on purpose: a qualified attribute (its own namespace's; `sce:` has its own check), an element with no entry
(an extension in the SCXML namespace is refused by the parser that reads it), and anything under a `<data>` or
`<content>` (a value carried as written).

`<elseif>` is entered with `cond`. The Recommendation states it in the text of `<if>` and in its example, not in a
table of `<elseif>`'s own, and the test says so where it makes the exception.

## What holds it

`tests/an_scxml_attribute_outside_the_standard_is_refused.rs`:

- the table equals the attribute tables read from the specification the ledger holds, element by element, in order;
- every committed `.scxml` carries none;
- `<state type="parallel">` is refused by each parser at its row, and by the binary's `check` and `generate` with a
  non-zero exit.

## The documents it found

- `server.scxml` of the WebSocket loopback now carries its data in `<content expr="_event.data"/>`, which is where a
  `<send>` carries it.
- The template-parity fixture carries its "sce:use in an attribute value" in a `<log label>`, an attribute SCXML
  defines, and no longer in an invented `description`.
- The TXML converter writes `conf:eventNamedParamHasValue` and `conf:eventdataSomeVal` as `cond`, which a
  `<transition>` and an `<if>` take, and `<conf:someInlineVal/>` as `123`. Test 354 had been tautological: its inline
  value was dropped, so the data was empty, and the transition that was to test for some data carried a `name` nobody
  read and was taken whatever the event carried. 519 and 534 carried their comparison as `expr`, read by nobody, so the
  transition was taken whatever the parameter held. The three pass as regenerated.
