#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: sce-rust-runtime-no-std.yml
#
# no_std MCU-target mirror (sce-rust-runtime-no-std.yml).
#
# `clippy` only lints the host target with default features, so it never
# sees the `heapless`-sized types the runtime selects without its `std`
# feature — a regression
# that compiles on host but breaks the bare-metal build (a reintroduced
# `std::*` import, an alloc-coupled emission, or a no_std-only clippy lint
# like result_large_err) slips every host-target gate and surfaces only on
# the cloud no_std workflow. This mirrors that workflow: build + clippy the
# runtime for thumbv7em, then generate and compile the allocator-free probe
# machines. `rustup target add` is idempotent — a no-op when the target is
# present, an auto-install otherwise — so the gate always runs rather than
# silently skipping on a fresh checkout.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)"

rustup target add thumbv7em-none-eabihf >/dev/null 2>&1 || true

# The bare-metal profile is the runtime without its `std` feature
# (docs/adr/0053): no features at all.
sce_gate_step "no_std runtime build + clippy"
cargo build -p sce-rust-runtime --no-default-features \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "no_std runtime build"
cargo clippy -p sce-rust-runtime --no-default-features \
    --target=thumbv7em-none-eabihf -- -D warnings \
    || sce_gate_fail "no_std runtime clippy"

# The `std` profile with none of the features that add to it (docs/adr/0053):
# no `http-send`, so no BasicHTTP processor, and no `macrostep-diagnostics`.
# Every other build of the runtime in this repository has both, through
# `default` or through the workspace's union, so a `cfg` that only this build
# reads — the refusal of a BasicHTTP send, the bound without its report on a
# host — is compiled, linted and run nowhere else. The tests that read the
# refusal (`a_basic_http_send_needs_the_http_send_feature`,
# `helpers::send`'s `supported_send_types`) run their other half here.
sce_gate_step "std runtime without http-send or the macrostep report: clippy + tests"
cargo clippy -p sce-rust-runtime --no-default-features --features std \
    --all-targets -- -D warnings \
    || sce_gate_fail "std runtime without http-send, clippy"
cargo test -p sce-rust-runtime --no-default-features --features std \
    || sce_gate_fail "std runtime without http-send, tests"

# The forge runtime on a target whose atomics are loads and stores alone (ARMv6-M). The queue algorithms
# need compare-and-swap, swap or fetch-add, which that target does not have, so a queue module that needs
# one must be absent there and not fail inside: a consumer that builds the runtime with no default features
# for thumbv6m (a firmware codec crate) found out from its own CI, after a pin moved, that `queue::hazard`
# was compiled on every target. thumbv7em, the target above, has the compare-and-swap and cannot show it.
sce_gate_step "forge runtime, no read-modify-write atomics (thumbv6m)"
rustup target add thumbv6m-none-eabi >/dev/null 2>&1 || true
cargo build -p sce-forge-runtime --no-default-features --target=thumbv6m-none-eabi \
    || sce_gate_fail "forge runtime build for thumbv6m"
cargo clippy -p sce-forge-runtime --no-default-features --target=thumbv6m-none-eabi -- -D warnings \
    || sce_gate_fail "forge runtime clippy for thumbv6m"

sce_gate_step "allocator-free probe machine"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/nostd-build/src \
    sce-build/tests/fixtures/no_std/parallel_history_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "no_std probe generation"
cargo build --manifest-path backends/rust/probes/nostd-build/Cargo.toml \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "no_std probe compile"

# The native-action probe: a machine whose `<sce:action>` lowers to a
# host call has to compile under no_std on the MCU target too. CI ran
# this and the gate did not, so the push-time check was the weaker of
# the two — the asymmetry `hook_ci_parity` exists to catch, and the one
# it could not see: both sides spell the step `cargo build
# --manifest-path ...`, and which source was generated into that crate
# is not a token the extractor compares.
sce_gate_step "native-action probe (no_std thumb)"
NATIVE_ACTION_OUT="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$NATIVE_ACTION_OUT'"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir "$NATIVE_ACTION_OUT" \
    sce-build/tests/fixtures/event_schema/statechart_native_action.scxml >/dev/null 2>&1 \
    || sce_gate_fail "native-action probe generation"
cp "$NATIVE_ACTION_OUT/statechart_native_action_sm.rs" \
    backends/rust/probes/nostd-build/src/parallel_history_probe_sm.rs \
    || sce_gate_fail "native-action probe staging"
cargo build --manifest-path backends/rust/probes/nostd-build/Cargo.toml \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "native-action probe compile"

# One workspace holding a bare-metal crate and an application crate, as a
# downstream workspace does (docs/adr/0053). Cargo builds one runtime for both
# with the union of their features; when `no_std` refused `http-send`, this
# host check stopped at the runtime although each crate built alone, and the
# application crate reads the macrostep report, which a removing feature
# (`no_macrostep_diagnostics`, now the additive `macrostep-diagnostics`) took
# away from the whole build. The
# bare-metal crate holds the `--no-std` emission generated here (the
# portable-emit machine), so the host check is also the single-emit
# portability gate on the std side (the bytes-field `E0433: cannot find
# heapless in sce_rust_runtime` regression), and the thumb build of that crate
# alone is the same gate on the bare-metal side and proves no feature of the
# application crate leaks into it.
sce_gate_step "one workspace, an MCU crate and an AP crate (std host + MCU thumb)"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/portable-emit/src \
    sce-build/tests/fixtures/no_std/portable_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "portability probe generation"
cargo check --manifest-path backends/rust/probes/feature-unification/Cargo.toml --workspace \
    || sce_gate_fail "MCU and AP crates in one host build"
cargo build --manifest-path backends/rust/probes/feature-unification/Cargo.toml \
    -p sce-feature-unification-mcu --target=thumbv7em-none-eabihf \
    || sce_gate_fail "MCU crate alone, bare-metal runtime, thumb"

# Per-machine event-queue sizing: a `<scxml sce:capacity="2">` machine must
# size its no_std queues to depth 2, not the crate-global depth-64 default.
# The const size assertion catches a silent revert of the per-machine
# `StatePolicy::EventQueue` to the bare (depth-64) form — which a plain
# compile gate misses, since the bare form still compiles. This is the
# downstream MCU SRAM blocker class.
sce_gate_step "per-machine event-queue sizing"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/nostd-queue-size/machine/src \
    sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "queue-size probe generation"
cargo build --manifest-path backends/rust/probes/nostd-queue-size/Cargo.toml \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "queue-size gate — const size assertion"

# What the engine COSTS an MCU image, which every step above this line is
# blind to. They ask whether generated code compiles without an allocator; a
# generic function that nothing instantiates compiles fine and weighs nothing,
# so `Engine<P>` had never been monomorphised for an MCU target in this
# repository at all — `nm` on the probe rlib returns zero engine symbols.
#
# The consequence was measured from outside on 2026-08-21: a bare-metal
# consumer reported `Engine<..>::run_main_event_loop` growing 60-198 bytes per
# instantiation across two SCE pins, multiplied by the five machines its image
# holds, and no gate here could confirm or deny a byte of it. Their firmware
# was the only witness in existence.
#
# `nostd-footprint` is a staticlib that actually drives an engine, so the code
# exists and can be weighed. Appendix D runs once per program over a document's
# tables (docs/adr/0064), so the algorithm's code carries no machine's name, and
# weighing the symbols that do would read the algorithm moving into the shared
# core as a shrink. The instrument links one machine and then two different
# ones, and holds three figures, each in BOTH directions (a drop is a failure
# too: the probe's first shape emitted LLVM bitcode and `nm` saw nothing, which
# a one-sided gate would have called green forever):
#
#   1. what one more machine costs — the image of two minus the image of one,
#      `.text` plus `.rodata`: the figure a consumer multiplies by its machines;
#   2. what the program pays once — the shared core's `.text`, by module path;
#   3. that the core is not instantiated per machine — no core symbol carries a
#      probe machine's type;
#   4. that a program pays for the Appendix D features its documents use
#      (docs/adr/0066): the procedures only a `<history>` or a `<parallel>`
#      reaches are linked when the first machine uses both, and absent from a
#      third configuration, `plain-documents`, whose two machines use neither.
sce_gate_step "MCU footprint budget (what a machine costs, what the core costs)"
# Regenerated here rather than trusted: the native-action step above stages a
# DIFFERENT machine into this same path, so whichever ran last decides what the
# probe crate is. The instrument names `ParallelHistoryProbePolicy`, and the
# second machine is the queue-size probe's, generated by the step above.
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/nostd-build/src \
    sce-build/tests/fixtures/no_std/parallel_history_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "footprint probe generation"
# The first machine of the `plain-documents` configuration: neither a `<history>` nor a
# `<parallel>` (docs/adr/0066, decision 4).
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/nostd-footprint/plain-machine/src \
    sce-build/tests/fixtures/no_std/plain_footprint_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "footprint probe generation (plain documents)"

FOOTPRINT_LIB="backends/rust/probes/nostd-footprint/target/thumbv7em-none-eabihf/release/libsce_nostd_footprint_probe.a"
FOOTPRINT_BUDGET="backends/rust/probes/nostd-footprint/footprint.budget"
# shellcheck source=/dev/null
source "$FOOTPRINT_BUDGET"

# A section-size reader for ARM objects: binutils for ARM when the machine has
# it, else LLVM's, a rustup component as the disassembler below is.
sce_footprint_size_tool() {
    if command -v arm-none-eabi-size >/dev/null 2>&1; then
        command -v arm-none-eabi-size
        return
    fi
    rustup component add llvm-tools >/dev/null 2>&1 || true
    local candidate
    candidate="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-size"
    [[ -x "$candidate" ]] && printf '%s\n' "$candidate"
}

# Build the probe with the features given and print its `.text` plus `.rodata`.
sce_footprint_image() {
    local label="$1"; shift
    cargo build --release --manifest-path backends/rust/probes/nostd-footprint/Cargo.toml \
        --target=thumbv7em-none-eabihf "$@" >&2 \
        || sce_gate_fail "footprint probe compile ($label)"
    [[ -f "$FOOTPRINT_LIB" ]] \
        || sce_gate_fail "footprint probe produced no staticlib at $FOOTPRINT_LIB ($label)"
    local tool
    tool="$(sce_footprint_size_tool)"
    [[ -n "$tool" ]] || sce_gate_fail "no ARM size reader: arm-none-eabi-size, or the rustup component llvm-tools"
    "$tool" -A -d "$FOOTPRINT_LIB" 2>/dev/null |
        awk '$1 ~ /^\.(text|rodata)/ { s += $2 } END { printf "%d\n", s }'
}

# The shared core's code symbols, `size name` in decimal, one per line — the
# one selection the sum, the floor and the breakdown read. A core symbol is one
# whose own path starts in a core module; a machine's implementation of the
# core's trait names the trait inside its `<.. as ..>` and is not one.
#
# `-t d` rather than converting hex in awk: `strtonum` is a GNU extension, and
# this machine's /usr/bin/awk is mawk. Measured the hard way — the gate was
# written and verified on a build host where awk is gawk, and the first push it
# guarded died with `function strtonum never defined` after every earlier gate
# had passed. Letting nm print decimal needs no awk dialect.
sce_footprint_core_symbols() {
    nm --print-size --size-sort -t d -C "$FOOTPRINT_LIB" 2>/dev/null |
        awk -v core="$CORE_SYMBOLS" \
            '$3 ~ /^[tTwW]$/ { name = substr($0, index($0, $4)); if (name ~ core) print $2 + 0, name }'
}

# The procedures only a `<history>` or a `<parallel>` reaches (FEATURE_PROCEDURES) that the image
# links, one name per line.
sce_footprint_feature_procedures() {
    nm -C "$FOOTPRINT_LIB" 2>/dev/null |
        awk -v pattern="^${FEATURE_PROCEDURE_PATTERN}\$" \
            '$2 ~ /^[tTwW]$/ { name = substr($0, index($0, $3)); if (name ~ pattern) print name }' |
        sort -u
}

# Whether the image links the feature procedures as its documents decide (docs/adr/0066): every
# one of them when a document uses both features (`linked`), none when no document uses either
# (`absent`). Held both ways for the reason every figure here is: a check that only ever finds
# nothing would pass on an instrument that stopped seeing.
sce_footprint_features() {
    local label="$1" expected="$2"
    local linked name missing=()
    linked="$(sce_footprint_feature_procedures)"
    case "$expected" in
        linked)
            for name in $FEATURE_PROCEDURES; do
                grep -qx "sce_rust_runtime::helpers::microstep::$name" <<<"$linked" \
                    || missing+=("$name")
            done
            if (( ${#missing[@]} > 0 )); then
                printf '  [nostd-mcu] %s: not linked, though a document uses its feature: %s\n' \
                    "$label" "${missing[*]}" >&2
                printf '  [nostd-mcu] a renamed procedure, or a module FEATURE_PROCEDURES no longer names.\n' >&2
                sce_gate_fail "MCU feature procedures not seen ($label)"
            fi
            ;;
        absent)
            if [[ -n "$linked" ]]; then
                printf '  [nostd-mcu] %s: linked, though no document uses its feature:\n' "$label" >&2
                printf '%s\n' "$linked" >&2
                printf '  [nostd-mcu] the core reaches a feature only through the tables of a document that\n' >&2
                printf '  [nostd-mcu] uses it (docs/adr/0066); a direct call is the whole feature in every program.\n' >&2
                sce_gate_fail "MCU feature code in a program that uses no feature ($label)"
            fi
            ;;
    esac
    printf '  [nostd-mcu] %-16s feature procedures %s\n' "$label:" "$expected"
}

# One configuration: the image with one machine, then with two, and the core
# read off the second, where it is a function of its own rather than inlined
# into the only caller. Each figure held to its budget in both directions.
sce_footprint_weigh() {
    local label="$1" machine_budget="$2" core_budget="$3" features="$4"
    local stack_roots="$5" stack_budget="$6"; shift 6
    local one two machine core_syms core_total typed lo hi
    one="$(sce_footprint_image "$label, one machine" "$@")"
    two="$(sce_footprint_image "$label, two machines" "$@" --features second-machine)"
    sce_footprint_features "$label" "$features"
    machine=$(( two - one ))
    read -r core_syms core_total < <(
        sce_footprint_core_symbols | awk '{ n++; s += $1 } END { printf "%d %d\n", n, s }'
    )
    typed="$(sce_footprint_core_symbols | grep -cE "$MACHINE_TYPES" || true)"

    if (( core_syms < MIN_CORE_SYMBOLS )); then
        printf '  [nostd-mcu] %s: found %d core symbol(s) matching %s, floor is %d\n' \
            "$label" "$core_syms" "$CORE_SYMBOLS" "$MIN_CORE_SYMBOLS" >&2
        printf '  [nostd-mcu] the instrument is not measuring the core — a moved module,\n' >&2
        printf '  [nostd-mcu] a bitcode staticlib, or a driver that stopped driving.\n' >&2
        printf '  [nostd-mcu] running %s; the budget was measured with %s\n' \
            "$(rustc --version)" "$MEASURED_WITH" >&2
        sce_gate_fail "MCU footprint instrument is blind ($label)"
    fi
    if (( typed > 0 )); then
        printf '  [nostd-mcu] %s: core symbol(s) instantiated for a machine:\n' "$label" >&2
        sce_footprint_core_symbols | grep -E "$MACHINE_TYPES" >&2
        printf '  [nostd-mcu] the core is compiled once for every program (docs/adr/0064);\n' >&2
        printf '  [nostd-mcu] a generic parameter added back to it is a copy per machine.\n' >&2
        sce_gate_fail "MCU core instantiated per machine ($label)"
    fi

    printf '  [nostd-mcu] %-16s images %d / %d byte(s)\n' "$label:" "$one" "$two"
    lo=$(( machine_budget - TOLERANCE_BYTES ))
    hi=$(( machine_budget + TOLERANCE_BYTES ))
    printf '  [nostd-mcu] %-16s one more machine %5d byte(s) (budget %d, band %d..%d)\n' \
        "$label:" "$machine" "$machine_budget" "$lo" "$hi"
    if (( machine > hi || machine < lo )); then
        printf '  [nostd-mcu] running %s; the budget was measured with %s\n' \
            "$(rustc --version)" "$MEASURED_WITH" >&2
        printf '  [nostd-mcu] re-pin the budget in %s if the change is meant, and say\n' \
            "$FOOTPRINT_BUDGET" >&2
        printf '  [nostd-mcu] in the commit message what the bytes bought.\n' >&2
        sce_gate_fail "MCU cost of a machine outside its budget ($label)"
    fi

    lo=$(( core_budget - TOLERANCE_BYTES ))
    hi=$(( core_budget + TOLERANCE_BYTES ))
    printf '  [nostd-mcu] %-16s shared core      %5d byte(s) over %2d symbol(s) (budget %d, band %d..%d)\n' \
        "$label:" "$core_total" "$core_syms" "$core_budget" "$lo" "$hi"
    if (( core_total > hi || core_total < lo )); then
        printf '  [nostd-mcu] per-symbol breakdown (bytes, symbol):\n' >&2
        sce_footprint_core_symbols >&2
        printf '  [nostd-mcu] running %s; the budget was measured with %s\n' \
            "$(rustc --version)" "$MEASURED_WITH" >&2
        printf '  [nostd-mcu] re-pin the budget in %s if the change is meant, and say\n' \
            "$FOOTPRINT_BUDGET" >&2
        printf '  [nostd-mcu] in the commit message what the bytes bought.\n' >&2
        sce_gate_fail "MCU shared core outside its budget ($label)"
    fi

    sce_footprint_stack "$label" "$stack_roots" "$stack_budget"
}

# A disassembler for ARM Thumb: binutils for ARM when the machine has it, else LLVM's, which is
# a rustup component (installed here the way the target is, and a no-op when present).
sce_stack_objdump() {
    if command -v arm-none-eabi-objdump >/dev/null 2>&1; then
        command -v arm-none-eabi-objdump
        return
    fi
    rustup component add llvm-tools >/dev/null 2>&1 || true
    local sysroot host candidate
    sysroot="$(rustc --print sysroot)"
    host="$(rustc -vV | sed -n 's/^host: //p')"
    candidate="$sysroot/lib/rustlib/$host/bin/llvm-objdump"
    [[ -x "$candidate" ]] && printf '%s\n' "$candidate"
}

# The stack of the deepest call chain the engine drives, which `.text` cannot say: a change can
# make a frame larger and leave the code where it was. The number is static (tools/mcu_stack):
# each function's frame from its prologue, and the largest sum along the calls of the object.
# What is not static is stated in the budget and printed, not assumed silently. Held in BOTH
# directions, for the reason the footprint is: a number that falls without a reason means the
# instrument stopped seeing.
sce_footprint_stack() {
    local label="$1" stack_roots="$2" stack_budget="$3"
    local objdump disassembly lo hi bytes report
    objdump="$(sce_stack_objdump)"
    [[ -n "$objdump" ]] \
        || sce_gate_fail "no ARM disassembler: arm-none-eabi-objdump, or the rustup component llvm-tools"
    disassembly="$(mktemp)"
    sce_gate_on_exit "rm -f '$disassembly'"
    "$objdump" -d -r -C --no-show-raw-insn "$FOOTPRINT_LIB" >"$disassembly" 2>/dev/null \
        || sce_gate_fail "stack disassembly ($label)"

    local roots=() recursion=() assume=() dispatch=()
    local item
    for item in $stack_roots; do roots+=(--root "$item"); done
    for item in $STACK_RECURSION; do recursion+=(--recursion "$item"); done
    for item in $STACK_ASSUME; do assume+=(--assume "$item"); done
    # Split with `read`, not by an unquoted expansion: the patterns hold `?` and `[`, which the
    # shell would otherwise take as a glob against the files of the working directory.
    local dyn_items=()
    read -r -a dyn_items <<<"$STACK_DYN"
    for item in "${dyn_items[@]}"; do dispatch+=(--dyn "$item"); done

    # `${recursion[@]+...}`: the list is empty while no walk recurses, and an empty array is an
    # unbound variable to `set -u` on a bash older than 4.4.
    report="$(python3 tools/mcu_stack/worst_case_stack.py "$disassembly" \
        "${roots[@]}" ${recursion[@]+"${recursion[@]}"} "${assume[@]}" "${dispatch[@]}" \
        --dyn-complete "$STACK_DYN_COMPLETE" \
        --indirect "$STACK_INDIRECT_BYTES" \
        --min-functions "$STACK_MIN_FUNCTIONS" --report)" \
        || sce_gate_fail "worst-case stack could not be computed ($label)"
    # The last line is the worst root's number; the lines above it are each root's chain.
    bytes="$(printf '%s\n' "$report" | awk '/^worst-case-stack / { print $2 }')"
    [[ "$bytes" =~ ^[0-9]+$ ]] \
        || sce_gate_fail "worst-case stack gave no number ($label)"

    lo=$(( stack_budget - STACK_TOLERANCE_BYTES ))
    hi=$(( stack_budget + STACK_TOLERANCE_BYTES ))
    printf '  [nostd-mcu] %-16s worst-case stack %5d byte(s) (budget %d, band %d..%d; static, %s assumed)\n' \
        "$label:" "$bytes" "$stack_budget" "$lo" "$hi" \
        "fmt $STACK_ASSUME, host callbacks $STACK_INDIRECT_BYTES B"

    if (( bytes > hi || bytes < lo )); then
        printf '  [nostd-mcu] each root and the chain that sets it (frame bytes in brackets):\n' >&2
        printf '%s\n' "$report" >&2
        printf '  [nostd-mcu] running %s; the budget was measured with %s\n' \
            "$(rustc --version)" "$MEASURED_WITH" >&2
        printf '  [nostd-mcu] re-pin its stack budget in %s if the change is meant, and say in the\n' \
            "$FOOTPRINT_BUDGET" >&2
        printf '  [nostd-mcu] commit message what the stack bought.\n' >&2
        sce_gate_fail "MCU worst-case stack outside its budget ($label)"
    fi
}

# With the macrostep report, which a bare-metal crate asks for by naming the
# runtime's `macrostep-diagnostics`, then as a bare-metal crate that asks for
# no feature builds it (docs/adr/0053). Both builds are weighed: a feature
# nothing builds is one that stops compiling and nobody finds out until a
# consumer sets it.
sce_footprint_weigh "diagnostics on" "$MACHINE_BYTES" "$CORE_BYTES" linked \
    "$STACK_ROOTS" "$STACK_BYTES" --features macrostep-diagnostics
sce_footprint_weigh "diagnostics off" "$MACHINE_BYTES_NO_DIAGNOSTICS" "$CORE_BYTES_NO_DIAGNOSTICS" \
    linked "$STACK_ROOTS" "$STACK_BYTES"
# A program none of whose documents has a `<history>` or a `<parallel>`, built as a bare-metal
# crate that asks for no feature builds it: the shape of a consumer whose machines are flat or
# nested compound states, which pays for neither feature (docs/adr/0066, decision 4).
sce_footprint_weigh "plain documents" "$MACHINE_BYTES_PLAIN" "$CORE_BYTES_PLAIN" absent \
    "$STACK_ROOTS_PLAIN" "$STACK_BYTES_PLAIN" --features plain-documents

# The stack tool's own cases: a trait object's call is followed into every
# implementation it may reach, which is what the stack above rests on.
python3 -m unittest discover -s tools/mcu_stack/tests -q \
    || sce_gate_fail "worst-case stack tool cases"
