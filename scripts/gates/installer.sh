#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: installer.yml
#
# The workbench's installer, judged by installing it.
#
# An installer exists so that nothing has to be installed beside the application. The `app` gate
# judges the application's code, and no part of it can say whether the installer carries what the
# application looks for, or whether the application looks where the installer put it: a bundle
# folder renamed on one side only leaves every test green and an installed application that hosts
# no executor.
#
#   1. `scripts/package_app.sh` builds the application as a `.deb` carrying the generator,
#      `sce-work` and the authoring server.
#   2. `scripts/verify_installed_app.sh` unpacks it into a scratch directory and starts what it
#      installed, with nothing else on the machine to help, and holds the application to saying
#      that it hosts an executor (and, with the generator taken out of the bundle, that it does not
#      and why).
#
# A gate of its own and not a step of `app`, because building the installer compiles the product's
# generator and the desktop shell and bundles them: minutes on a warm tree, far more cold, which is
# not what a change to the screen's tests should wait for. `ci_only` in the registry says so.
#
# Linux and the `.deb` only. Whether an installer for another platform finds what it carries is
# not judged here, because none is built (see `app/README.md`).

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

command -v npm >/dev/null 2>&1 \
    || sce_gate_cannot_run "needs node and npm (the screen is built into the installer)"
cargo tauri --version >/dev/null 2>&1 \
    || sce_gate_cannot_run "needs the Tauri CLI (cargo install tauri-cli --version '^2' --locked)"
for tool in dpkg-deb xvfb-run; do
    command -v "$tool" >/dev/null 2>&1 \
        || sce_gate_cannot_run "needs $tool (the installer is a .deb, started under a virtual display)"
done

scripts/package_app.sh --debug --bundles deb \
    || sce_gate_fail "the installer could not be built (scripts/package_app.sh)"

installer="$(find app/target/debug/bundle/deb -name '*.deb' -print -quit)"
[[ -n "$installer" ]] \
    || sce_gate_fail "scripts/package_app.sh built no .deb under app/target/debug/bundle/deb"

scripts/verify_installed_app.sh "$installer" \
    || sce_gate_fail "the installer is not sound: the application it installs does not find what it carries (scripts/verify_installed_app.sh says how)"
