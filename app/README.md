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

Today it does the first part: **works** (a titled piece of specification text),
**immutable revisions** of each, and a **save that refuses to overwrite** text
the caller has not seen. Showing the model and accepting a design come next, on
top of the same folder.

What does not exist yet, so that nothing below is read as done:

- The authoring MCP does not read or write the works folder. `sce-work` is
  ready for it and the MCP's tool list has no works tool, so an AI client cannot
  yet be pointed at a work saved here.
- The screens that show the model, the open questions, the examples' results and
  the acceptance. They are designs, not code.
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
authoring MCP is meant to use and does not yet. All of them end in
`sce_app_core::call`, so there is one definition of what a save is.

The works folder is plain files:

```
<root>/<work-id>/work.json            title, created_at
                 source/<sha256>.txt  one file per revision, named by its digest
                 source.head          the current revision
                 source.log           one line per save: revision, parent, time
```

`<root>` is `SCE_WORKS_DIR` if set, else the per-user data directory
(`~/.local/share/sce-workbench/works` on Linux). A revision is the SHA-256 of
its exact bytes; a save names the revision it was written from and is refused
with `conflict` if that is no longer current.

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
| Browser shell: handler, sockets | `cargo test -p sce-web-shell` (from `app/`) |
| Screen: guards, editor model, transport, words | `npm test` (in `app/ui`) |
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
