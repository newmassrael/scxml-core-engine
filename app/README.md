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
and shown as the figures **SCE draws of it** (the application draws nothing: it
runs `sce-codegen diagram` and shows the SVG it writes). Accepting a design comes
next, on top of the same folder.

An authoring client reaches the same folder through the MCP (`works_list`,
`works_read`, `works_save_model`, in `tools/authoring`): it reads the text you
saved, writes the model, and saves it back after the product's own check accepts
it. They run `sce-work`, so the application and the MCP cannot disagree about the
folder because only one thing writes it. The text is yours: no MCP tool writes
it, and none removes a work.

What does not exist yet, so that nothing below is read as done:

- Nothing starts the AI client. You ask it in its own window ("model the work
  Door lock") and the application shows what it saved when you read again.
- The screens for the open questions, the examples' results and the acceptance.
  They are designs, not code. The model screen shows what SCE drew and where the
  model stands to the text; it does not edit the model (an AI client writes it).
- A model of several documents. `figures` draws ONE document: a link that
  imports a codec is refused by SCE for the import it cannot find, in SCE's words.
- A desktop window asks nothing when it is closed with text not yet saved. A
  browser tab does (the browser's own prompt); the window has no such event wired.
- Windows has not been run; the Linux desktop build and the browser have.

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
                 source.head          the current revision
                 source.log           one line per save: revision, parent, time
                 model/<sha256>.scxml the same, for the model
                 model.head           the current model
                 model.log            one line per save, and the source revision
                                      the writer said it read (`written_for`)
                 removed.json         only for a removed work (see below)
```

`<root>` is `SCE_WORKS_DIR` if set, else the per-user data directory
(`~/.local/share/sce-workbench/works` on Linux). A revision is the SHA-256 of
its exact bytes; a save names the revision it was written from and is refused
with `conflict` if that is no longer current. The text and the model are two
chains kept by one implementation (one lock, one compare-and-swap).

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
| The MCP's works tools, against the real `sce-work` and generator | `python3 -m unittest tests.test_the_works_folder_is_reached_through_the_applications_own_command` (in `tools/authoring`, with `PYTHONPATH=.`) |
| The same with the real generator | `SCE_CODEGEN=<path to sce-codegen> cargo test -p sce-app-core --test figures` (skipped, and says so, without it) |
| Browser shell: handler, sockets | `cargo test -p sce-web-shell` (from `app/`) |
| Screen: guards, editor model, model panel, transport, words | `npm test` (in `app/ui`) |
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
