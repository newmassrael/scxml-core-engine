#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: app.yml
#
# The specification workbench application (`app/`): the screen, the browser
# shell and the desktop shell.
#
# `app/` is a workspace of its own, excluded from the root one, so the root
# lanes never build it: the desktop shell links a GUI toolkit, and every Rust
# lane would otherwise need that installed. What has no screen lives in
# `app-core/`, which the root lanes DO build and test (`cargo test --workspace`
# runs its tests, including the contract test that writes down every reply of the
# command layer). This gate is the rest:
#
#   1. The screen: types, unit tests, and the production build. The unit tests
#      read `app-core/contract/replies.json`, so a core that changed a reply and
#      a screen that still expects the old one disagree here.
#   2. The two Rust shells: formatting, clippy with warnings denied, and the
#      browser shell's tests (handler, and real sockets with clients that stop
#      partway). Clippy and the build compile the Tauri shell too, which needs
#      the platform's WebKit libraries (see app.yml).
#
# Needs node and npm, and the toolchain `cargo tauri` would use. A machine
# without them cannot judge this, which is exit 3 and not a verdict on the tree.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

command -v npm >/dev/null 2>&1 \
    || sce_gate_cannot_run "needs node and npm (the screen is TypeScript)"

# `npm ci` installs exactly what package-lock.json records and refuses a lock
# that disagrees with package.json; `npm install` would quietly repair it.
( cd app/ui && npm ci --no-audit --no-fund ) \
    || sce_gate_fail "app/ui: npm ci (package.json and package-lock.json disagree, or the registry is unreachable)"

( cd app/ui && npm run typecheck ) \
    || sce_gate_fail "app/ui: the screen does not typecheck"

( cd app/ui && npm test ) \
    || sce_gate_fail "app/ui: the screen's tests (guards against the core's replies, editor model, transport, words, dependencies)"

# The Tauri shell embeds this build at compile time, so it has to exist before
# any cargo command below reads the shell.
( cd app/ui && npm run build ) \
    || sce_gate_fail "app/ui: the production build"

# The workspace is named with `--manifest-path`, not entered with `cd`. Both run
# the same cargo; only the first is visible to a reader of the command line, and
# `cli_feature_gating` is one: it reads these lines to judge whether a command
# sweeps `sce-build` (whose gated test targets need `--features cli`), and
# `--workspace` here means `app/`'s members, never the root's. A manifest under
# a directory the root `Cargo.toml` excludes says so on the line itself.
# Formatting names the two packages and does not use `--all`. `--all` adds every
# local path dependency (`app-core`, which pulls `sce-build` and the backends in
# through its dev-dependencies) and hands rustfmt every file of them on ONE command
# line: ~97 KB, over the 32767 characters Windows lets a process be started with
# ("The filename or extension is too long", measured on the `windows` lane).
# `app-core` is a member of the root workspace, so `fmt-check.yml` already judges it.
cargo fmt --manifest-path app/Cargo.toml -p sce-workbench -p sce-web-shell --check \
    || sce_gate_fail "app: cargo fmt --check"

cargo clippy --manifest-path app/Cargo.toml --workspace --all-targets --locked -- -D warnings \
    || sce_gate_fail "app: cargo clippy (the Tauri shell and the browser shell)"

cargo test --manifest-path app/Cargo.toml -p sce-web-shell --locked \
    || sce_gate_fail "app: the browser shell's tests"

# The desktop shell's own logic, which has no window in it: when a close request is
# held and when it is let through (`close_gate.rs`).
cargo test --manifest-path app/Cargo.toml -p sce-workbench --lib --locked \
    || sce_gate_fail "app: the desktop shell's close gate"
