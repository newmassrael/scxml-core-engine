#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Build the workbench application as an installer that carries what it needs.
#
#   scripts/package_app.sh [--debug] [--bundles LIST]
#
# The application hosts an AI executor that writes models through the SCE authoring server, which
# checks them with the product's generator and reads and saves a work through `sce-work`. An
# installer cannot count on any of them being installed beside the application, so it carries
# them: this script makes the authoring bundle (scripts/package_sce_author.sh: bin/sce-author-mcp,
# bin/sce-codegen, bin/sce-work, the templates and the Python package), puts it in the folder
# tauri.conf.json names as a resource (app/src-tauri/resources/sce-author), and has Tauri build the
# installer around it. The installed application finds it there (`installed::in_bundle`) with no
# environment naming anything.
#
#   --debug         use the debug builds of the generator and `sce-work`, and build the application
#                   in debug too: for trying the installer on this machine without waiting for a
#                   release build of everything
#   --bundles LIST  what Tauri builds, comma separated (default: deb)
#
# What the installer does not carry is Python: the launcher of the authoring server needs Python
# 3.10 or later with the modules `sce_author/needs.py` lists (PyYAML, jsonschema). The deb says so
# (`python3`, `python3-yaml`, `python3-jsonschema`) and `verify_installed_app.sh` holds the deb it
# built to that list; an installer for a platform with no package manager to ask has to carry or
# install one, which this script does not.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="release"
bundles="deb"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --debug) profile="debug"; shift ;;
        --bundles) [[ $# -ge 2 ]] || { printf 'package_app: --bundles needs a value\n' >&2; exit 2; }
                   bundles="$2"; shift 2 ;;
        *) printf 'usage: %s [--debug] [--bundles LIST]\n' "$0" >&2; exit 2 ;;
    esac
done

cargo_profile=()
[[ "$profile" == "release" ]] && cargo_profile=(--release)

cd "$repo_root"
cargo build "${cargo_profile[@]}" -p sce-build --features cli --bin sce-codegen
cargo build "${cargo_profile[@]}" -p sce-app-core --features cli --bin sce-work
codegen="$repo_root/target/$profile/sce-codegen"
work="$repo_root/target/$profile/sce-work"

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
"$repo_root/scripts/package_sce_author.sh" "$staging" --codegen "$codegen" --work "$work" >/dev/null

resources="$repo_root/app/src-tauri/resources/sce-author"
mkdir -p "$resources"
# Everything but the placeholder that keeps the folder in the tree: that one file, by its place,
# since the templates the bundle carries hold files of that name too.
find "$resources" -mindepth 1 -not -path "$resources/.gitkeep" -delete
cp -R "$staging/sce-author/." "$resources/"

# The screen is built here, from the one place that knows where it lives, and Tauri is told not to
# run its own `beforeBuildCommand`: that is run from a folder this script cannot name from outside
# (it looked for `ui/package.json` under `ui/`), and a build that depends on where it is started
# from is one that works only for whoever wrote it.
npm --prefix "$repo_root/app/ui" ci
npm --prefix "$repo_root/app/ui" run build

tauri_profile=()
[[ "$profile" == "debug" ]] && tauri_profile=(--debug)
cd "$repo_root/app"
cargo tauri build "${tauri_profile[@]}" --bundles "$bundles" --config '{"build":{"beforeBuildCommand":""}}'

find "$repo_root/app/target/$profile/bundle" -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' -o -name '*.msi' -o -name '*.exe' -o -name '*.dmg' \) -print
