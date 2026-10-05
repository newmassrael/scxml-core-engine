<!--
  SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
  SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
-->

# SCE Workbench

A place to write a specification in prose, see the model an AI client drew from
it, and accept what you checked. This directory is the application; it knows
nothing about what a specification is about. The text it stores is whatever the
person writes, and what that text means is the authoring tools' business
(`tools/authoring`).

Today it does the first two parts: **works** (a titled piece of specification
text), **immutable revisions** of each, and a **save that refuses to overwrite**
text the caller has not seen; and the **model** of a work, kept beside the text
and shown as the figures **SCE draws of it** and what **SCE says of it** (its check
and the pseudocode page you read against your text): the application draws and
judges nothing, it runs `sce-codegen` and shows what the product writes. On top of
the same folder you **accept a design** against the requirement list your text was
read into, and the application says whether that acceptance still holds.

An authoring client reaches the same folder through the MCP (`works_list`,
`works_read`, `works_save_model`, `works_save_requirements`, in `tools/authoring`):
it reads the text you saved and your answers, writes the model and the requirement
list, and saves them back after the product's own check accepts them and your
answers are kept to. They run `sce-work`, so the application and the MCP cannot
disagree about the folder because only one thing writes it. The text and the
answers are yours: no MCP tool writes either, none removes a work, and none
accepts a design for you.

What does not exist yet, so that nothing below is read as done:

- Nothing starts the AI client. You ask it in its own window ("model the work
  Door lock") and the application shows what it saved when you read again.
- The screens for the examples' results. They are designs, not code. The model
  screen shows what SCE drew, what SCE says of the model, where it stands to the
  text, a field for your answer under each question the model leaves open, and
  what you accepted; it does not edit the model (an AI client writes it) and it does
  not write the requirement list (an AI client reads your text into one).
- Windows has not been run on a window; the lane `windows` in `app.yml` runs the
  same gate there, and the Linux desktop build and the browser have been run.

## How the parts fit

```
app-core/        (root workspace member; no GUI, no network)
  the works folder, revisions, the JSON command layer, the `sce-work` CLI
app/
  src-tauri/     the desktop application: a window around ui/, one command
  web-shell/     the same commands over HTTP, for a machine with no display
  ui/            the screen (TypeScript, no framework)
```

Everything the screen can do is one call, `call(name, args)`. The window routes
it through Tauri's `invoke` and the browser shell through `POST /api/call`;
`sce-work call <name>` is the same call for a process that is not Rust, which the
authoring MCP uses. All of them end in `sce_app_core::call`, so there is one
definition of what a save is.

The works folder is plain files:

```
<root>/<work-id>/work.json            title, created_at
                 source/<sha256>.txt  one file per revision, named by its digest
                 source.head          the current revision, and which save of
                                      source.log made it current (`log <n>`)
                 source.log           one line per save: revision, parent, time
                 model/<sha256>.scxml the same, for the model
                 model.head           the current model
                 model.log            one line per save, and the source revision
                                      the writer said it read (`written_for`)
                 answers/<sha256>.json  your answers to the model's open questions
                 answers.head         the current answers
                 answers.log          one line per save
                 requirements/<sha256>.json  the requirement list the text was read into
                 requirements.head, requirements.log   (and the text it was read from)
                 acceptances/<sha256>.json   what you accepted, one file per acceptance
                 acceptances.head, acceptances.log
                 removed.json         only for a removed work (see below)
```

`<root>` is `SCE_WORKS_DIR` if set, else the per-user data directory
(`~/.local/share/sce-workbench/works` on Linux). A revision is the SHA-256 of
its exact bytes; a save names the revision it was written from and is refused
with `conflict` if that is no longer current. The text, the model, the answers,
the requirement list and the acceptances are five chains kept by one
implementation (one lock, one compare-and-swap).

A folder an older build wrote has pointers that are the digest alone, and nothing in it
says which of several saves of one digest the pointer meant: a save that failed leaves
the same log line a save that worked does. This build reads such a folder as it always
read, with one exception. When two saves of a model disagree about the text it was
written for, the model's text is left unstated (the screen's word for it is
`behind`, never `current`) rather than taken from the last of them, and the first save
that works settles it and writes the pointer in the form above. A save into an older
folder names the place first, so a save that fails leaves nothing in the log. What an
older folder cannot give back is the history of two such disagreeing saves: it may hold
the one that failed. `history` and `model_history` say so on that entry, with
`"unconfirmed": true` (a key present only where it is true). The mark is a reading of
the log, not something written into it, and it stays on the entry after a later save has
made the model current: the save of today does not make the one of the past knowable.
The model's screen does not show a model history; a client that does should show the mark.

### Reading a work as one state

A screen that reads a work with a command for each chain (`read_work`, `read_source`,
`read_model`, `read_answers`, `read_requirements`, `read_acceptance`) can be handed a
text of one moment beside a model of another: a save lands between two of the reads.
`read_work_snapshot` reads the work and its five chains as one state. The pointers of
all five are read, then what they name, then the pointers again, and the read starts
over when any of them moved; what is answered is the folder as the second look found it.

It takes no lock, so a screen that reads on every change never makes a save wait. That
holds because a pointer is replaced by one atomic rename and a revision's file is never
rewritten. A folder written to between every look cannot be read that way, and after five
tries the read takes the lock a save takes, so the answer is one state there too.

Each part of the answer is what the command that reads that chain answers, in the same
words: `source`, `model` with `model_standing`, `answers`, `requirements` with
`requirements_standing`, and `acceptance` as it was saved. It does not say whether the
acceptance still holds. That is the product's to say and asking it is not a read of the
folder, so the screen still asks `read_acceptance` for it.

### Removing a work

`remove_work` (the screen's "Remove this work", after it asks) takes a work out
of the list and refuses every later read and save of it as `not-found`. It does
not delete anything: the work's files stay, with a `removed.json` beside them,
because a specification is a person's writing and the one command that can end
it should not be the one that cannot be taken back. Deleting `removed.json`
brings the work back whole, history included; deleting the folder is the removal
that cannot be undone, done by the person who means it. The marker is written
under the work's lock and a save re-checks for it once it holds the lock, so a
save that was waiting when the removal came writes nothing. No MCP tool removes a
work.

### The model, and where it stands

A model is saved for a text revision (`written_for`), so "is this model about the
text I am looking at" is a comparison of two digests, and the core says the answer
in one word every shell shows the same way (`standing`): `current`, `behind` (the
text moved on), or `unstated`. The same model saved again for a later text is a
new entry of its history, not an `unchanged`: the writer read the new text and
kept the model, and that is a fact about the model.

**A model of several documents.** A statechart written the way the authoring tools
ask for one imports an event schema for each event and closes its interface, so the
model is a set of documents that name each other by file name. The store still keeps
ONE text per revision: a set is a JSON object holding every document under its file
name (`model_set.rs`), a model of one document is the document itself exactly as it
always was, and the two are told apart by their first character (`<` or `{`). The
same documents are the same bytes however they were listed, so saving them again is
no change. `save_model` takes `text` (one document) or `documents` and an `entry`
(several), and a model that is read says its `entry` and lists its `documents`, the
entry first. Every file name is one plain name (letters, digits, `_`, `.`, `-`; no
directory) and is checked again when the set is written out for SCE, because it
becomes a path there. SCE is asked about the whole set: the entry under its own file
name and every other document beside it, where an import finds it. With a closed
interface, a set whose schema is not staged is refused by the product
(`import/file-not-found`), which is how the tests know the schema reached it.

`figures` runs the generator: the first of `SCE_CODEGEN`, an `sce-codegen` beside
the running program, or one on `PATH`. It stages the model under the work's own
name (the product names its figures by its document's file), waits for it at most
30 seconds, writes its output to files and not pipes, and reads back only files
inside the folder it was given. What SCE refuses (a figure that does not fit its
page, a character its font table has no width for, a document it does not
accept) reaches the screen as the product's own code and sentence, with the model
still shown as text. The screen asks for the language it is in (`--lexicon`),
shows each sheet as an `<img>` (an image never runs script), at 150% unless the
person chooses another size, and scrolls a wide sheet sideways instead of
shrinking it.

### What SCE says of a model

`review` runs the same generator twice and passes on what it writes:
`sce-codegen check --lint` (the product's verdict on the document, every record it
wrote with its code and line, what an accepted model still leaves open in the
product's own sentences, and the questions the model marks `sce:unresolved`) and
`sce-codegen pseudo` (the page, in the screen's language, kept byte for byte). A
model SCE refuses is an **answer** (`check.verdict` is `refused`, with its
records and no page); SCE not answering at all is a refusal of the same
`sce-*` kinds `figures` has. When SCE accepts a model and will not write its page,
the verdict is kept and the page's own refusal is shown beside it.

⚠ "Accepted" is the product's verdict on the document. The screen says once, next
to it, that it does not say the model agrees with your text, and nothing on it
claims that: you compare the page with your own words.

### Closing the desktop window

A browser tab is asked "leave this page?" by the browser; a window is not. So the
screen tells the shell whether it holds anything the core has not been given (the
editor's text, or answers typed and not saved), and when the window is asked to
close with something unsaved the shell (`src-tauri/src/close_gate.rs`) holds the
close and runs a fixed script that has the SCREEN ask: save and close, discard and
close, or stay. Only the person's choice lets the window go (`sce_close`); a window
with nothing unsaved closes at once. The commands the window may call are
`sce_call`, `sce_unsaved` and `sce_close`, and nothing else.

⚠ A window that cannot be closed is its own failure. If the screen was asked and says
nothing for three seconds (it crashed, or never loaded), the next close request goes
through: a screen that does not answer protects nothing. That is decided from the
times in `close_gate.rs`, and its tests say so.

Checked on a real window (Linux, under a virtual display, with the close request
sent the way a window manager's close button sends it): held with unsaved text;
"discard and close" closes; "save, then close" saves the text and closes; a window
with nothing unsaved closes at once.

### Your answers to what the model leaves open

A model marks what your text did not say (`sce:unresolved="open-guard"`, with the
question in its own words as `sce:unresolved-reason`). The screen puts a field
under each such question; what you write is saved by `save_answers` as ONE map
(the question's id to your words) on top of the revision it was read as, and the
core refuses a revision that is no longer current, so answers saved from another
window are never overwritten unseen. The screen then keeps what you typed and
loads what turned up.

- An answer is stamped with the time its words last CHANGED, not the time of the
  save that carried it; saving what is already saved is no change at all.
- An answer to a question the model no longer asks is kept apart and kept: the
  next draft may ask it again. Clear a field and save to take an answer back.
- The map is the application's own. The decision record the product's checks
  read is made by the authoring package from it (`works_read` hands the client
  `decisions_text`), so the format of THAT record stays where it is read.
  `works_save_model` holds a draft to the answers with the product's own
  `decisions` check: one that leaves an answered question open, or guesses where
  no answer licenses it, is not saved.
- Nothing here changes the model. Your answers reach the authoring client the next
  time it reads the work, and it applies them.

### Accepting a design

An authoring client reads your text into a **requirement list** (`save_requirements`:
two JSON files, the manifest of coordinates and the sidecar of quoted sentences,
kept as written because an acceptance pins their bytes) and writes the design. The
screen then shows what the product finds, and nothing the application worked out:

- `requirements_report` runs `sce-codegen requirements --manifest` and
  `acceptance-report`: each requirement in SCE's own word (`implemented`,
  `missing`, `needs-scenario`, or one a later SCE adds, shown as spelled), where
  the text anchors it and where the design carries it, and the page you read
  before deciding, byte for byte. It also gives the `basis`: the revisions of the
  text, the model, the list and your answers, which is exactly what you were shown.
- `accept` takes that `basis` back as `expect`. If any of it has moved (the text
  saved from another window, the client's next model, an answer saved since) nothing
  is accepted, the refusal is `moved` and names which, and the screen reads what is
  there now before you can press again. A design or a list written for an earlier
  text is refused as `not-current`. Otherwise the core stages the work in a fixed
  layout (`design/`, `spec/`), asks the product to take the record
  (`sce-codegen accept --channel direct`), and keeps it as a new entry of the
  acceptances chain. The record pins every file by path and hash.
- `read_acceptance` asks the product whether the record still holds
  (`acceptance-check`) for the work as it is now. It is the product that says, by the
  bytes of each pinned file: a design put back to what was accepted is the design that
  was accepted. When it no longer holds the screen shows the product's own sentence of
  what moved.

What this does not claim, stated so nothing on the screen is read as more:

- **A design with a gap is yours to accept.** The screen tells you before you press:
  SCE's count marks the requirements it finds unsettled (`missing`, `unresolved`,
  `dangling`, `contradicted`, `scenario-failed`, `needs-scenario`), each of SCE's
  eleven words is shown with what the product means by it, and a word SCE adds later
  is shown as spelled and not guessed at; the matters SCE lists as left open are
  counted beside the button. The record keeps what was open. Accepting closes none of
  them and is not refused for them.
- **The channel is a statement and not a proof.** `direct` is how this application's
  own button states itself; the product records it and cannot verify it. An
  acceptance a client relayed on your word would say `relayed`, and the screen says
  so. No MCP tool records an acceptance, because the command layer states every one
  it records as `direct`.
- **The product's lapse is one sentence.** It joins its lapses with `; ` and one lapse
  can contain one, so the sentence is passed on whole and not split into a list the
  product does not state.
- **Your answers are pinned, not applied.** The product keeps the answers file as a
  pinned input and does not read it, so the record's list of what was open is the
  product's statement about the design and does not shrink when you answer in the
  application. The screen shows your answers beside it.
- **The sidecar is not pinned.** The product pins the manifest and the files the design
  read; the sentences sidecar only feeds the page, so editing it does not lapse an
  acceptance.
- **What is accepted is what is saved.** The button is withheld while text or answers
  are typed and not saved, while SCE has not measured the design, while the design
  or the list was written for an earlier text, and while the text, the design or the
  answers on screen are not the ones the page measured (a text saved from another
  entrance moves under the screen, and the page is then of a newer text than the one
  being read: the screen is read again before it can be accepted). An answer sheet the
  screen has not read yet is not "no answers": the button waits for it and says so
  (`unread`), because a page measured with answers cannot be shown against none.

## Seeing the screen

### In a browser, on a machine with no display (and from a phone)

```bash
cd app/ui && npm ci && npm run build
cd .. && cargo run -p sce-web-shell -- --ui ui/dist --listen 127.0.0.1:5174
```

It prints an address ending in `#token=…`; open that. The token is generated
per run (or set `SCE_WEB_TOKEN`, 16 characters or more), is sent as a bearer
header, and is removed from the address bar once read.

To open it from a phone: either listen on the machine's Tailscale address
(`--listen 100.x.y.z:5174`) and open the printed address with the Tailscale app
running on the phone, or keep it on `127.0.0.1` and forward the port over SSH
(`ssh -L 5174:127.0.0.1:5174 <machine>`).

The browser shell listens only on loopback and Tailscale addresses
(100.64.0.0/10, fd7a:115c:a1e0::/48); any other address, `0.0.0.0` included, is
refused. It is a separate program from the application, so a release build of the
application cannot contain it. Every wait on a client is bounded (headers, body,
number of connections), so a client that stops partway cannot hold the server.

### While changing the screen

```bash
cargo run -p sce-web-shell            # commands on :5174
cd ui && npm run dev                  # screen on :5173, forwards /api to :5174
```

### The desktop application

Needs Tauri's system libraries (on Debian/Ubuntu: `libwebkit2gtk-4.1-dev
libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`; on Windows the
WebView2 runtime). Then `cargo tauri dev` or `cargo tauri build` from
`app/src-tauri`. The window may call the one registered command and nothing
else: it has no file-system, shell or network permission.

## Tests

| What | Command |
|---|---|
| Works folder and command layer | `cargo test -p sce-app-core --features cli` |
| The model chain and `figures` (a stand-in generator, Unix) | `--test models`, `--test figures` of the same package |
| Removing a work, and a save racing it | `--test removal` of the same package |
| A work read as one state, with a writer saving while it is read | `--test snapshot` (and `--lib`) of the same package |
| A model of several documents (`model_set.rs`, staging, the command's shapes) | `--lib`, `--test model_sets`, `--test figures` of the same package |
| The owner's answers: the chain, stamps, conflicts | `--test answers` (and `--lib`) of the same package |
| What SCE says of a model (a stand-in generator, and the real one with `SCE_CODEGEN`) | `--test figures` of the same package |
| Requirements and acceptance: the commands, and the product behind them (a stand-in, and the real generator with `SCE_CODEGEN`) | `--test acceptance`, `--test acceptance_product` (and `--lib`) of the same package |
| The MCP's works tools, against the real `sce-work` and generator | `python3 -m unittest tests.test_the_works_folder_is_reached_through_the_applications_own_command` (in `tools/authoring`, with `PYTHONPATH=.`) |
| The same with the real generator | `SCE_CODEGEN=<path to sce-codegen> cargo test -p sce-app-core --test figures` (skipped, and says so, without it) |
| Browser shell: handler, sockets | `cargo test -p sce-web-shell` (from `app/`) |
| Screen: guards, editor model, model panel, acceptance, transport, words | `npm test` (in `app/ui`) |
| Screen types and build | `npm run build` (in `app/ui`) |

`app-core/contract/replies.json` is every command's reply, written by running
the commands. `app-core/tests/contract.rs` fails when a reply changes without
that file, and `app/ui/test/contract.test.ts` feeds the same file to the
screen's guards, so a screen and a core that disagree fail a test instead of
rendering `undefined`.

## Licence

The application is under the repository's licence. The only code from outside
that reaches a user at run time is `@tauri-apps/api` (MIT OR Apache-2.0) in the
screen's bundle, and Tauri and its Rust dependencies in the desktop binary;
`ui/test/dependencies.test.ts` pins the set of runtime npm packages and their
licences, so a new one has to be added there on purpose.
