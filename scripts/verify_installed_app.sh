#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Judge an installer by installing it into a scratch directory and starting what it installed.
#
#   scripts/verify_installed_app.sh <installer.deb>
#
# An installer exists so that nothing has to be installed beside the application. What is judged is
# that: the application the installer installs finds the programs it carries (the generator,
# `sce-work` and the authoring server) with nothing on the machine to help, and says what it found
# where the owner looks, in the record a shell keeps of its executor (`.sce-hosts/desktop.json` in
# the works folder).
#
#   0. What it asks the package manager for: the deb declares a package for every Python module
#      the server says it needs (`sce_author/needs.py`), because it does not carry Python.
#   1. Whole, as the installer made it: the record says an executor is hosted.
#   2. With the bundled generator removed: the record says none is, and names the generator. This is
#      the control for the first. If the application still hosted an executor, something other than
#      the installer would be supplying what the installer lacks, and the first answer would prove
#      nothing about the installer.
#
# Nothing the environment could name stands in for the bundle: the variables that point at a
# program are unset, the search path is the system's alone, and the client is a stand-in that
# answers `--version`, the one thing a shell asks of it before it takes a request. Whether a real
# client writes a model is a different question, asked by `tests/claude_code_live.rs`.
#
# Exit status: 0 the installer is sound, 1 it is not, 2 the arguments are wrong, 3 a tool this
# check needs (dpkg-deb, xvfb-run, timeout, python3) is not installed, which says nothing of the
# installer.

set -euo pipefail

fail() {
    printf 'verify_installed_app: %s\n' "$1" >&2
    exit 1
}

installer="${1:-}"
if [[ -z "$installer" ]]; then
    printf 'usage: %s <installer.deb>\n' "$0" >&2
    exit 2
fi
if [[ ! -f "$installer" ]]; then
    printf 'verify_installed_app: %s is not a file\n' "$installer" >&2
    exit 2
fi
for tool in dpkg-deb xvfb-run timeout python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'verify_installed_app: needs %s\n' "$tool" >&2
        exit 3
    fi
done

unset SCE_CODEGEN SCE_WORK SCE_AUTHOR_MCP SCE_EXECUTOR SCE_CLAUDE_MODEL SCE_CLAUDE_BUDGET_USD
unset SCE_TEMPLATE_DIR SCE_WORKS_DIR PYTHONPATH

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
root="$scratch/root"
mkdir -p "$root" "$scratch/client"
dpkg-deb -x "$installer" "$root"

app="$root/usr/bin/sce-workbench"
[[ -x "$app" ]] || fail "the installer does not install usr/bin/sce-workbench"
bundle="$(find "$root/usr/lib" -maxdepth 2 -type d -name sce-author -print -quit)"
[[ -n "$bundle" ]] || fail "the installer carries no sce-author bundle under usr/lib"
for program in sce-author-mcp sce-work sce-codegen; do
    [[ -x "$bundle/bin/$program" ]] || fail "the bundle carries no bin/$program"
done

# What the installer asks the package manager for, since it does not carry Python. The server says
# which modules it needs (`sce_author/needs.py`, read from the bundle's own copy), and the deb that
# was built has to declare a package for each. Starting the application below cannot judge this:
# the machine it runs on may have a module the deb never asks for, and an installer that leaves out
# `python3-jsonschema` would start, and pass, here.
declares() {
    grep -Eq "(^|, )$2([ ,(]|$)" <<<"$1"
}
declared="$(dpkg-deb -f "$installer" Depends)"
needed="$(PYTHONPATH="$bundle/python" python3 -c \
    'from sce_author import needs; print(*(need.debian for need in needs.NEEDS))')"
[[ -n "$needed" ]] || fail "the bundle's server says it needs no Python module, which it does"
if declares "python3" "python3-yaml"; then
    fail "the dependency check says a list that holds only python3 declares python3-yaml"
fi
for package in python3 $needed; do
    declares "$declared" "$package" \
        || fail "the installer declares '$declared' and the server needs $package"
done
printf 'verify_installed_app: the installer declares what the server needs (%s)\n' "$needed"

client="$scratch/client/claude"
printf '#!/bin/sh\necho "0.0.0 (Claude Code)"\n' > "$client"
chmod +x "$client"

# Start the installed application under a virtual display and print what it reported about its
# executor. The report is written as the application starts, in its setup, so this waits for the
# report and not for a fixed time, then stops the application: what is judged is what it found, and
# a window that fails later (a runner with no graphics) is not the installer's fault. An
# application that stops before it reports, or never reports, is.
host_record() {
    local works="$scratch/works-$1"
    local record="$works/.sce-hosts/desktop.json"
    local log="$scratch/app-$1.log"
    local pid ticks=0
    mkdir -p "$works"
    SCE_WORKS_DIR="$works" SCE_CLAUDE="$client" PATH=/usr/bin:/bin \
        timeout --kill-after=5 60 xvfb-run -a "$app" >"$log" 2>&1 &
    pid=$!
    until [[ -f "$record" ]]; do
        if ! kill -0 "$pid" 2>/dev/null; then
            cat "$log" >&2
            fail "the application ($1) stopped before it reported anything about its executor"
        fi
        if (( ticks >= 120 )); then
            kill "$pid" 2>/dev/null || true
            cat "$log" >&2
            fail "the application ($1) reported nothing about its executor in 60 seconds"
        fi
        ticks=$(( ticks + 1 ))
        sleep 0.5
    done
    cat "$record"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
}

field() {
    python3 -c 'import json, sys; print(json.dumps(json.load(sys.stdin)[sys.argv[1]]))' "$1"
}

whole="$(host_record whole)"
if [[ "$(printf '%s' "$whole" | field hosting)" != "true" ]]; then
    fail "installed whole, the application hosts no executor: $(printf '%s' "$whole" | field reason)"
fi
printf 'verify_installed_app: whole: an executor is hosted\n'

rm "$bundle/bin/sce-codegen"
broken="$(host_record broken)"
if [[ "$(printf '%s' "$broken" | field hosting)" != "false" ]]; then
    fail "without the bundled generator the application still hosts an executor: something other than the installer supplies it"
fi
reason="$(printf '%s' "$broken" | field reason)"
if [[ "$reason" != *"sce-codegen"* ]]; then
    fail "without the bundled generator the application does not say the generator is missing: $reason"
fi
printf 'verify_installed_app: without the generator: no executor, and it says why\n'
