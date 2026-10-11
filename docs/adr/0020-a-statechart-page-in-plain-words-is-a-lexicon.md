# ADR 0020 — A statechart page in plain words is a lexicon

- Status: Accepted and implemented (the owner asked for it on 2026-10-11)
- Date: 2026-10-11
- Scope: `sce-build/src/forge/page.rs` (`PLAIN`, eight words, `Part::GluedWord`), `sce-build/src/forge/pseudo.rs`
  (the head clause list, the transition type, `raise`), `sce-codegen pseudo --lexicon plain`
  and `diagram --lexicon plain`

## Context

The pseudocode page of a statechart is the document in full, in the words of the standard the document is written
in: `datamodel: ecmascript`, `binding: early`, `initial-children`, `[external]`, `raise`, `parallel`. A specification
owner who does not know that standard meets them on a page meant to be read without it.

Measured on the 742 committed statechart pages that render: 349 carry the `datamodel`/`binding` head, 269 carry
`[external]`, 73 carry `invoke`, 112 carry `send`.

A lexicon is the seam for this: it renames the words the grammar spends, `normalise_page` puts them back, and the
page is the canonical one under other words. It could not be used because the head clause list's keys, the type of a
transition and `raise` were not words: they were text the mapping built, which a lexicon can neither rename nor drop.

## Rejected first: a second view of the page

A first version added a `reading` view beside the shape and the lexicon: the same renderer, with five sites that
wrote differently, dropping a switch left at its default. It was rejected by the owner as a stopgap, rightly:

- It is a third axis for a thing the second axis already is.
- It says LESS than the document (a default is not written), so it cannot be read back. It had to declare itself and
  be refused by the reverse path, which is the cost of every page that is not the document.
- Nothing could check it. A lexicon is swept over every committed document by
  `a_page_in_any_shape_normalises_to_the_canonical_one`; a view that drops something has no law to be swept by.

## Decision

The standard's switches become words, spelled as before in the default lexicon, so the default page is byte for byte
what it was:

| Word | `en` | `plain` |
|---|---|---|
| `ClauseName` | `name:` | `name:` |
| `ClauseDatamodel` | `datamodel:` | `expression language:` |
| `ClauseInitial` | `initial:` | `first state:` |
| `ClauseBinding` | `binding:` | `variables created:` |
| `ClauseQueue` | `queue:` | `event queue size:` |
| `TypeExternal` | `[external]` | `(leaves its source state)` |
| `TypeInternal` | `[internal]` | `(stays in its source state)` |
| `Raise` | `raise` | `tell itself` |
| `Parallel` | `parallel` | `concurrent` |
| `Initial` | `initial` | `starts in` |
| `InitialChildren` | `initial-children` | `starts together in` |

`Part::GluedWord` is a word placed with no separator before it: the first key of the clause list sits right after
the opening parenthesis. The parentheses and commas stay text, as the module note of `page` decides. A transition
type other than `external` and `internal` stays as the author wrote it, in brackets.

`plain` is `en` with those words replaced, so a state, an event, a guard and an action are written exactly as they
are. A value the document wrote (`early`, `ecmascript`) is never translated. Every word keeps a spelling of its own:
`plain` spells `Initial` and `InitialChildren` differently so reading the page back never has to guess which one a
line meant. `ko` names the new words too.

## Consequences

- Nothing is dropped. A `[external]` the author wrote is still written, as `(leaves its source state)`; the page
  says all of the document.
- It is the same law as every lexicon: `plain` is in `LEXICONS`, so the sweep over every committed `.scxml`,
  every shape and every lexicon holds it to `normalise(write(nodes)) == canonical(nodes)` byte for byte.
- `diagram --lexicon plain` draws a figure's boxes and tables in the same words, because the figure asks the page.
  ⚠ It did not at first: the figure's own phrases ("whole document", "inside") were keyed by the lexicon's NAME, so
  `plain` was refused as having none. They belong to a language, not to a lexicon, so a lexicon now says which
  language it is in (`Lexicon::language`: `en` and `plain` are `en`) and the figure keys on that.
- The reverse converter is untouched: it reads the canonical page, and the canonical page is byte for byte what it was
  over all 742 committed statecharts that render.
- A lexicon spells every word, so the next switch that is text the mapping builds is found the same way: it cannot
  be named.

## What this does not decide

- Words still the standard's own in `plain`: `send`, `param`, `to`, `type`, `after`, `script`, `data`, `history`,
  `final`, `invoke`, `on entry`/`on exit`. Each needs plain words that say what it does (for `send`, which processor
  it reaches), and `invoke` (starting another machine from inside this one) needs a decision about what to call it.
- The MCP `pseudo` tool does not offer the lexicon.
