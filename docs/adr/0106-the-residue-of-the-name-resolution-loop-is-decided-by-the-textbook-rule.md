# ADR 0106 — The residue of the name-resolution loop is decided by the textbook rule

- Status: Accepted (2026-11-16). Written and accepted under the owner's standing rule (the most long-term correct
  structure, textbook in its foundations; decided, not asked), which the owner extended on 2026-11-16 to the items
  the enum and forge name-resolution loop had left as "owner decisions" when it stopped on 2026-09-25. Written as
  0105 and renumbered 0106 the same day, 0105 being promised to the tlv-chain record that keeps ids.
- Date: 2026-11-16
- Scope: the decisions only, item by item, each with the evidence of the tree as it stood on 2026-11-16, what was
  implemented with this ADR, and what is left and whose it is. The items keep the numbers of the loop's defect
  ledger (G3 is the NL to IR closure ledger's row).

## Context

The loop stopped with six items marked for the owner and seven residual diagnostic items, plus the G3 row's
follow-up. Many had moved since: other rounds fixed some, and some decisions had been taken in passing without a
record. Each item below was checked against the tree first. "Fixed" cites the commit that fixed it.

## Decisions

### #155b — a misplaced SCE element is a move, not a rename

Implemented. `validation/misplaced-element`: when the read ledger shows another element of the document was asked
for the unread element's name, the refusal says where it is read, with `fix` `move_element` and a `destination`
related site; otherwise `validation/unexpected-child-element` as before. The destination is derived from the
parse's own reads, never from a list of where elements belong.

### `sce:*` attributes nothing reads (the forge parser's lax `xs:anyAttribute`)

Fixed before this ADR: `8f837f9933` (2026-09-28) records every `sce:` read through `sce_attr` on the parse's ledger
and refuses the rest as `validation/sce-attribute-unread`. No new code was needed.

### G3 — the root of every forge document is a claim site

Decided, not yet implemented. The `NoAnnotationSite` wording already says what SCE reads, not what the grammar
refuses (`ReviewTableUnavailable`'s message, "SCE reads sce:req on no node of this kind"); the stale comments
that still said a root `sce:req` was dropped were corrected with this ADR.

The decision: every forge document is one artifact, and its root is the one node every kind has, so `sce:req` on the
root is read for every forge kind and `forge::requirement_nodes` answers every kind with at least the root's row
(lookup keeps its per-entry rows besides). `ReviewScope::NoAnnotationSite`, `CliError::ReviewTableUnavailable` and
`cli/review-table-unavailable` then become unreachable and are removed in the same change (the schema is
`pre-release`, SCE_ERROR_CONTRACT.md section 8.1). The change is wider than the parser: the root claims are a
fact of `ParsedForge` (as `kind_basis` is), so `requirement_nodes` and its four callers (`review_table`,
`requirement_manifest`, `requirements_report`, `acceptance_report`) take it; the pseudocode page writes the root's
`req` lines and `unpseudo` reads them back (the page is total by contract); the forge AST export and its schema gain
the field. Left for a round of its own, because none of it can be checked without building.

### #142 — a link carries the events it declares

Decided: implement the RFC's codegen contract, `driver.poll() -> bytes -> framer.decode() -> event inject` and
`event extract -> framer.encode() -> pool slot -> driver.send()`, in every link backend. The RFC's own invariant
(section 2.4, "kinds are additive": added when wired, never a built-but-unconsumed parser surface) rules out the
alternative of rewriting the comment and the RFC to say a link is a byte pipe while `<sce:events>` stays parsed and
unread. Left to the owners of the link templates (`tools/codegen/templates/forge/{rust,c}/link*.jinja2`), with the
framer codec resolved across imports; the template header comment is to be made true by that change, not edited
ahead of it.

### #127 — a deployment's refusal names the deployment's path

Implemented. `DeployConfig` keeps the path it was read from (`source`, set by `parse_deploy` and by the CLI's
loader), and every set-level refusal and artifact label of `compile_scxml_with_imports` uses `DeployConfig::label`,
the literal `deploy.yaml` only for a deployment given as text.

### #76 — a deployment's refusal carries the row of the key it names

Decided: yes. A diagnostic locates the author's input (SCE_ERROR_CONTRACT.md section 2.2); a key path is a name, not
a coordinate a consumer can open. `serde_yaml_ng` keeps no marks, so the deploy reader gains a marked parse of the
same text (a YAML event reader that reports line and column), consulted only on the failure path to map the dotted
key path a refusal already names (`machines.x.bindings.y`) to the row of its last key. Left: it adds a dependency,
which needs a lockfile change and a build this ADR could not make.

### #112 — an arriving event name is matched by its token prefix

Fixed before this ADR: `a6806700fb` (2026-10-04) resolves an arriving name to the longest token prefix the document
writes, on all six backends, held by one shared scenario.

### #102 — an inline kind is an artifact, not a value of the datamodel

Decided: keep the explicit host binding SCE_FORGE.md already states (section 4.4 and "Kind Reference Resolution",
corrected in `c8d25c2f92`); do not bind a guard's name to an inline kind by matching its inputs to the statechart's
data by name, which would make the meaning of a guard depend on a coincidence of names. Left: the refusal of
`cond="canEnterProgramming"` should say that the name is an inline kind's artifact and how the host binds it,
rather than that the name is not declared.

### #184 — the `code:` vocabulary is removed from the C++ parser

Decided: remove `<code:action>`, `<code:external-action>`, `code:guard`, `code:scope` and
`GuardParser::parseExternalImplementation` from the C++ interpreter's parser. They are recognised by prefix string,
not by namespace; nothing in the tree uses them; no AOT backend has a counterpart, so keeping them is a partial
feature and an engine parity breach; `<sce:action>` replaces them. Left to the engine owners: it changes public
headers (`sce/include/parsing/ActionParser.h`, `GuardParser.h`), whose embed manifest is regenerated with them.

### #178 — whitespace between comments in a body

Fixed before this ADR: `16f527c9e4` (2026-11-05) parses with `parse_ws_pcdata`, and `getTextContent` concatenates
every PCDATA and CDATA child, so `x<!--a--> <!--b-->y` reads `x y` in both engines.

### #74 — an undeclared enum value is refused with the values to pick from

Implemented. `validation/enum-value-not-declared`, `fix` `replace_one_of` over the declared values in declaration
order. A value past the enum's carrier stays `validation/cross-kind-type-mismatch`: no variant can be meant by it.
This supersedes the earlier design lock that reused the mismatch code: SCE_ERROR_CONTRACT.md section 3.2 puts a
closed set in `fix`, and a set carried as prose in a message is not one a consumer can read.

### #79 — an import cycle is named by its documents

Implemented. The compiled document is on the walk's stack when it is a file, and the cycle's start is found by
canonical path, not by the `src` spelling. (The record's `actual` was already dropped by `94e56bc16c`.)

### #84 — a manifest cycle is one cycle at its imports

Implemented. The documents Kahn's sort leaves are walked to one cycle, begun at its least path; the record stands on
the cycle's first import with the others as `also-refused` sites.

### #83 — a value placed at its attribute is reported as written

Implemented in `located()`, where every node-placed refusal passes: when the record names an attribute's decoded
value and the document escapes it, `actual` is the value as written; the id keeps the decoded value.

### #81 — what `col` means

Decided: `col` is the column at which `actual` begins on `line` when the record carries `actual`, and the column at
which the element begins otherwise; the guard of SCE_ERROR_CONTRACT.md section 3.1.1 is then `line[col-1..]` starts
with `actual`. Left: `located()` places a refusal at the attribute's start, not its value's, and other producers have
not been measured; the producers move first, measured by the corpus that `diagnostic_fix_is_applicable` reads, and
the guard is tightened in the same change.

### #53g — `actual` is an input token

Partly fixed before this ADR (`94e56bc16c`: counts and cycle paths are no longer `actual`). Left: the remaining codes
whose golden `actual` is prose, a relation or a computed value move that text to `message` or `expected`, found by
the same golden scan the loop ran (303 goldens then).

### #111 — `_event.data.reason` is structured

Decided: the reason an `error.*` event carries is a record (`code`, `message`), not a sentence with a code inside
it, on every engine. Left to the engine owners.

## Consequences

The implemented items land with the commits that cite this ADR's items. The items left above are open debt with a
decision, not open questions: none of them is to be put to the owner again.
