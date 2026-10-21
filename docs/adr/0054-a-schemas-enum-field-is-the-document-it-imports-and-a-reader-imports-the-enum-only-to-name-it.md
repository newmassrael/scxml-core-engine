# ADR 0054 — A schema's enum field is the document it imports, and a reader imports the enum only to name it

- Status: Accepted (2026-10-21) and implemented, under the owner's standing rule: the most
  long-term correct way, cost ignored
- Date: 2026-10-21
- Amends: ADR 0052 decision 1 ("a statechart that reads the payload or keeps a record of it
  imports both", under the aliases the schema writes); docs/SCE_ACCEPTED_SUBSET.md §2.15, "An
  enum field of a payload" and "A record's enum field", which made a reader import a schema's
  enum under the alias the schema writes
- Scope: the parse seam (`SCXMLParser::parse_impl`), which now reads `SCXMLModel::enum_documents`
  before the typed-path validators and names each schema's enum fields there
  (`forge::event_schema_check::name_schema_enums`); `SCXMLModel::schema_enums`,
  `SCXMLModel::held_enum` and `SCXMLModel::held_enums`; the static lowering's type declarations,
  payload reads, adapters and saved shape, and C's enum names (`forge::static_lowering`); the
  judge's refusal of a record variable whose enum field it cannot start
  (`forge::static_datamodel`, `schema_enums_named`) and its reading of a send's receivers; the
  removal of `forge::static_imports::check_record_enum_documents` and of the import an algorithm's
  record field asked for; the Mesh comparison of two schemas (`forge::schema_revision::declared_alike`)
- Related: ADR 0044 (an enum and a schema are named by their document in a saved state and on the
  network), ADR 0047 (the machine's own type for an enum), ADR 0048 and ADR 0051 (a slot of a
  function a statechart calls names the enum the statechart imports from the same document)

## Context

A statechart handled `record_list.refused`, whose schema (`sce:std/shared/record_refused.scxml`)
imports `sce:std/shared/record_command.scxml` as `Command` and types its field `kind` as
`enum:Command`. The statechart read only `_event.data.reason`. Generation for Kotlin stopped
(measured 2026-10-21):

```text
Forge codegen error: feature unsupported in this language: a transition that reads the payload
of `record_list.refused`, whose field `kind` holds the enum `Command`: this document does not
import it under that alias (<sce:import kind="enum" as="Command">) or Kotlin has no enum type for it
```

The statechart had to import an enum it never names, under the alias the schema chose. The
typed channel holds every field of the payload, an enum field in the machine's own type for the
enum (ADR 0047), and that type was looked up by the field's `enum:<alias>` in the statechart's
own imports. Every reader of a schema field did the same: the payload struct, a record variable,
a list of records, a payload taken whole, the saved shape, an algorithm's adapter, the send-side
check and the width narrowing. Three things follow from reading a schema's alias in another
document's scope:

1. **Two documents are coupled by a name neither shares.** An alias is local to the document
   that declares it. A schema that renames its import (`Command` to `Cmd`) breaks every reader,
   though the field holds the same enum.
2. **Some pairs of schemas cannot both be read.** Two schemas that use one alias for two
   different enums ask the reader to import two documents under one alias, which
   `validation/duplicate-alias` refuses.
3. **The import says nothing.** A reader that names no variant imports an enum for the
   generator's lookup alone, and a reader that imports the same document under an alias of its
   own (`Why` for record_refusal) still has to import it a second time under the schema's.

The repository already identified an enum by its document everywhere the name could not be
borrowed: a saved state and the Mesh check name it by document (ADR 0044, `EnumDocuments`), and a
slot of an algorithm or of another kind is renamed to the alias the statechart imports the same
document under (`static_imports::value_signature`, ADR 0051). Only a schema's field was read by
alias.

## Decision

**1. A schema's enum field holds the enum document the schema's own import resolves to.** The
alias in `enum:<alias>` is the schema's name for that document and nothing more. Where a
statechart imports the schema, the parse seam reads which document each enum field holds
(`EnumDocuments::by_field`, read from the schema's own imports, by the document key every reader
of the document computes) and renames the field, in the statechart's copy of the schema, to the
statechart's own name for that document (`name_schema_enums`). Every reader of the field — the
payload struct, a record, a list of records, a payload taken whole, the saved shape, an
algorithm's adapter, the send-side check, the width narrowing — then reads it under that name, so
the rule is held in one place and no backend reads a schema's alias again.

**2. The statechart's name for the document.** It is the alias the statechart imports that
document under, by any path; the schema's own alias first, when the statechart imports the
document under it too, so a machine that already imported the enum the way the old rule asked is
generated as before. Where the statechart does not import the document, the field is held all the
same, in a type the machine declares for the document, under a name made from the enum document's
own name (`record_command`, so `<Machine>RecordCommandEnum`; C's type was always the document's,
`record_command_enum_t`). The made name is kept with its enum in `SCXMLModel::schema_enums`; it is
never the spelling of one of the statechart's own aliases (a suffix `_2`, `_3`… is added where it
would be), so two enums never share one type; and the statechart's text cannot write it, since
the scope its expressions are judged in holds the enums it imports and no other.

**3. A statechart imports an enum only to name it.** A variant (`Why.none`), a variable or a
conversion (`enum:Why`, `Why.fromNumber`), or a slot of a function it calls (ADR 0048, ADR 0051,
unchanged: the slot is named by the statechart's import of the document) names the enum, and needs
the import, under any alias. Reading another field of the payload, comparing or assigning the
field to a value of the same enum, keeping the payload whole, and a list of the records do not. A
record variable starts every field at the value its `<sce:set>` writes, which for an enum field is
a variant, so it names the enum: a statechart that does not import the field's enum document is
refused where the record variable is declared (`scxml/static-datamodel-rule`, "which this document
does not import"), before the variant is read.

**4. Another document under the schema's alias is another enum, and is not refused at the
import.** A statechart may import, under the alias the schema writes, an enum of another document;
the alias then names that enum in the statechart, and the field holds the schema's. A variant of
the statechart's enum is no value of the field, and is refused where it is written, as a value of
any other enum is. The refusal at the schema import (`check_record_enum_documents`) is removed: it
existed only because the field was read by the alias.

**5. Two schemas are compared by the documents their enum fields hold, not by their aliases.** The
Mesh check compares the schema a sender holds for an event with the receiver's (ADR 0044), each
now in its own statechart's names; a field's enum alias is left out of `declared_alike`, and the
enum documents of the two fields are compared as before (`divergence_with_enums`).

## Alternatives considered

- **Keep the requirement and document it as a rule.** It costs nothing to implement and leaves all
  three faults of the context: a schema cannot rename its import, some pairs of schemas cannot be
  read together at all, and the import a reader writes carries no meaning. A documented coupling
  is still a coupling between two documents through a name only one of them declares.
- **Let the reader's text use the schema's alias.** Bringing `Command` into the reader's scope
  because a schema it imports declares it would make the reader's names depend on its imports'
  imports, and two schemas with one alias for two enums would still collide, now in the reader's
  expressions.
- **Name every machine's enum type by its document.** One spelling for every enum, but it renames
  the generated type of every machine that imports an enum (`<Machine><Alias>Enum`), which hosts
  name, to fix a defect that only the enums a statechart does not import have. The alias stays the
  name of what the statechart imports; only an enum it does not import is named by its document.
- **Resolve the document at each use instead of renaming at the seam.** The payload builders, the
  adapters, the saved shape and the judge all look an enum up by name; giving each of them the
  schema's imports and a document lookup is eight readings of one fact. Renaming once where the
  schema is imported is the reading a slot of an algorithm already has (ADR 0051).
- **Import the enum implicitly, under the schema's alias, when the reader does not.** It hides the
  collision of decision 4 instead of resolving it: the reader's own `Command` and the schema's
  would be one name for two enums.

## Consequences

- The statechart of the context generates on every backend with no import of record_command, and
  with record_refusal imported under its own alias (`static_payload_enum_unimported`). A machine
  that imported a schema's enum under the alias the schema writes, from the same document, is
  generated as before: the rename keeps the name.
- A statechart that imported another document under the schema's alias was refused at the import;
  it is now accepted, and refused only where it writes a variant of its own enum as a value of the
  field (a record variable's `<sce:set>`, a comparison, an assignment).
- A field held under a made name appears under that name in the judge's refusals and in a saved
  state's format-1 shape text (`enum:record_command`); format 2 names the enum by its document, as
  before.
- An algorithm's record whose enum field the statechart does not import is handed over and taken
  back (the adapter crosses the field by the variant's declared name, from the one type the machine
  holds the document in); an enum slot of a call still names the statechart's import of the
  document.
- `sce:std/shared/record_refused.scxml` says what a reader imports in these words.
