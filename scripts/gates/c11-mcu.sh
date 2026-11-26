#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: c11-mcu.yml
#
# A C11 machine's core links, is weighed and runs on a Cortex-M4 with no C
# library (docs/adr/0110, D3). For each machine of the ADR's set:
#
#   1. LINK. Generate it, compile what the generator wrote with exactly D3's
#      flags, and link it with `-nostdlib -lgcc` and three things only: the four
#      memory functions, the seams its core declares, and a startup for QEMU's
#      `mps2-an386` (backends/c/mcu/). An undefined symbol fails the gate and is
#      named: that is the proof that the core calls nothing hosted (D1).
#   2. WEIGH. `.text`, `.rodata`, `.data` and `.bss` of the machine's own objects
#      (the runtime is header-only, so what it pulls in is in them), held to
#      backends/c/mcu/footprint.budget in both directions; the linked image's
#      sections and the libgcc members its map file names are reported beside.
#   3. RUN. The machine's replay scenario, turned into a C table when the machine
#      is generated (`sce-codegen scenario-table`), so the program reads no file,
#      run under QEMU with semihosting. QEMU's exit status is the verdict, and a
#      run past RUN_SECONDS fails.
#
# What it measured goes to target/c11-mcu/report.md: the toolchain, the flags,
# the board and each figure, which a consumer cites for its own image.
#
# The compiler is Arm's build of the release scripts/install_arm_gnu_toolchain.sh
# pins (`[platform.linux.tools.arm-none-eabi-gcc]` in bx.toml), because the
# budget's figures are that compiler's; the gate puts its `bin/` first on PATH,
# under the release read from the script's own line, from the root the install
# script installs to (ARM_GNU_TOOLCHAIN_ROOT, which the gate's image names for
# the copy it holds). The gate runs in the image its registry entry names
# (`container` in tools/git-hooks/gate_registry.py), which holds the compiler
# and QEMU; anywhere a tool is missing, or the compiler is not that build, the
# gate ends as "could not run" (exit 3), never as a pass.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

MCU_DIR="backends/c/mcu"
BUDGET="$MCU_DIR/footprint.budget"
OUT="target/c11-mcu"
REPORT="$OUT/report.md"
RUN_SECONDS=60
BOARD="mps2-an386"

# The set of docs/adr/0110 D3, one entry each: `name|kind|document|scenario`. A
# `machine` is generated as a C11 statechart and its scenario replayed; a `codec`
# is generated alone and its program writes and reads one value of it.
# `sce-build/tests/the_c11_mcu_gate_weighs_the_set_its_record_names.rs` holds
# these names to the ADR's and to the budget's.
C11_MCU_SET=(
    "static_counter|machine|sce-build/tests/fixtures/static_datamodel/static_counter.scxml|sce-build/tests/fixtures/static_datamodel/scenarios/static_counter.json"
    "plain_footprint_probe|machine|sce-build/tests/fixtures/no_std/plain_footprint_probe.scxml|backends/c/mcu/scenarios/plain_footprint_probe.json"
    "static_map|machine|sce-build/tests/fixtures/static_datamodel/static_map.scxml|sce-build/tests/fixtures/static_datamodel/scenarios/static_map.json"
    "static_map_take|machine|sce-build/tests/fixtures/static_datamodel/static_map_take.scxml|sce-build/tests/fixtures/static_datamodel/scenarios/static_map_take.json"
    "static_state_algorithm|machine|sce-build/tests/fixtures/static_datamodel/static_state_algorithm.scxml|sce-build/tests/fixtures/static_datamodel/scenarios/static_state_algorithm.json"
    "codec_cbor_map|codec|tests/forge/resources/codec_cbor_map.scxml|"
)

# D3's flags, exactly: every object of an image is compiled with them.
CFLAGS=(-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -std=c11 -ffreestanding -nostdlib -Os
        -ffunction-sections -fdata-sections -Wall -Wextra -Wpedantic -Werror)
INCLUDES=(-I backends/c/runtime/include -I backends/c/forge-runtime/include -I "$MCU_DIR")
# The board's own sources, linked beside each machine and weighed with none.
BOARD_SOURCES=("$MCU_DIR/startup.c" "$MCU_DIR/semihosting.c" "$MCU_DIR/seams.c")

# The Arm GNU Toolchain release, read from the one line that pins it, and its
# `bin/` where the install script puts it: ARM_GNU_TOOLCHAIN_ROOT when it is
# set, as the image sets it, else the default that script and bx's store entry
# share. The version check below refuses any other build either way.
ARM_GNU_TOOLCHAIN_REV="$(sed -n 's/^ARM_GNU_TOOLCHAIN_REV="\(.*\)"$/\1/p' scripts/install_arm_gnu_toolchain.sh)"
[[ -n "$ARM_GNU_TOOLCHAIN_REV" ]] \
    || sce_gate_fail "scripts/install_arm_gnu_toolchain.sh states no ARM_GNU_TOOLCHAIN_REV=\"...\" line"
ARM_GNU_TOOLCHAIN_ROOT="${ARM_GNU_TOOLCHAIN_ROOT:-$HOME/.local/share/arm-gnu-toolchain/$ARM_GNU_TOOLCHAIN_REV}"
PATH="$ARM_GNU_TOOLCHAIN_ROOT/bin:$PATH"

for tool in arm-none-eabi-gcc arm-none-eabi-size qemu-system-arm timeout; do
    command -v "$tool" >/dev/null 2>&1 \
        || sce_gate_cannot_run "$tool is not on PATH. The compiler is Arm GNU Toolchain $ARM_GNU_TOOLCHAIN_REV under $ARM_GNU_TOOLCHAIN_ROOT, installed by scripts/install_arm_gnu_toolchain.sh (the gate's image holds it; a build machine without docker: [platform.linux.tools.arm-none-eabi-gcc] of bx.toml, bx-provision <alias> --apply); QEMU is the package qemu-system-arm, which the gate's image installs"
done

CODEGEN="$(sce_gate_codegen)"
GCC_VERSION="$(arm-none-eabi-gcc --version | sed -n '1p')"
# Another build of the compiler weighs differently, and the budget is this
# one's: a distribution's arm-none-eabi-gcc found first is a property of the
# machine, not of the tree.
[[ "$GCC_VERSION" == *"Arm GNU Toolchain $ARM_GNU_TOOLCHAIN_REV"* ]] \
    || sce_gate_cannot_run "arm-none-eabi-gcc is '$GCC_VERSION', not Arm GNU Toolchain $ARM_GNU_TOOLCHAIN_REV; install it with scripts/install_arm_gnu_toolchain.sh"
QEMU_VERSION="$(qemu-system-arm --version | sed -n '1p')"
sce_gate_step "$GCC_VERSION; $QEMU_VERSION"

rm -rf "$OUT"
mkdir -p "$OUT"

# shellcheck source=/dev/null
source "$BUDGET"

# Compile `$1` into the object `$2` with D3's flags and any further arguments.
compile() {
    local source="$1" object="$2"
    shift 2
    arm-none-eabi-gcc "${CFLAGS[@]}" "${INCLUDES[@]}" "$@" -c "$source" -o "$object"
}

# `.text .rodata .data .bss` of the objects given, in bytes, summed over every
# section whose name begins with one of the four (`-ffunction-sections` names a
# section per function).
sections() {
    arm-none-eabi-size -A -d "$@" |
        awk '$1 ~ /^\.text/ { t += $2 } $1 ~ /^\.rodata/ { r += $2 }
             $1 ~ /^\.data/ { d += $2 } $1 ~ /^\.bss/ { b += $2 }
             END { printf "%d %d %d %d\n", t, r, d, b }'
}

# The board's objects, once for every image.
BOARD_OBJECTS=()
mkdir -p "$OUT/board"
for source in "${BOARD_SOURCES[@]}"; do
    object="$OUT/board/$(basename "${source%.c}").o"
    compile "$source" "$object" || sce_gate_fail "the board's $source does not compile"
    BOARD_OBJECTS+=("$object")
done
# The memory functions with loop-to-call recognition off: each loop would
# otherwise be compiled into a call of the very function it is in (memory.c).
compile "$MCU_DIR/memory.c" "$OUT/board/memory.o" -fno-tree-loop-distribute-patterns \
    || sce_gate_fail "the board's memory functions do not compile"
BOARD_OBJECTS+=("$OUT/board/memory.o")

{
    printf '# The C11 MCU gate (docs/adr/0110, D3)\n\n'
    printf -- '- Board: QEMU `%s` (Cortex-M4 with FPv4-SP), semihosting\n' "$BOARD"
    printf -- '- Compiler: %s\n' "$GCC_VERSION"
    printf -- '- Emulator: %s\n' "$QEMU_VERSION"
    printf -- '- Flags: `%s`\n' "${CFLAGS[*]}"
    printf -- '- Link: `-nostdlib -T %s/mps2_an386.ld -lgcc`, with the board'"'"'s startup, memory functions and seams\n' "$MCU_DIR"
    if [[ -n "${TOLERANCE_BYTES:-}" ]]; then
        printf -- '- Budget: `%s` (measured with: %s)\n\n' "$BUDGET" "${MEASURED_WITH:-unnamed}"
    else
        printf -- '- Budget: `%s` (unmeasured; to be measured with: %s)\n\n' "$BUDGET" "${MEASURED_WITH:-unnamed}"
    fi
    printf '| Machine | .text | .rodata | .data | .bss | image .text | image .rodata | image .data | image .bss | libgcc members | run |\n'
    printf '|---|---|---|---|---|---|---|---|---|---|---|\n'
} >"$REPORT"

MEASURED=()
OUT_OF_BAND=()
UNMEASURED=0
RUN_FAILED=()

for entry in "${C11_MCU_SET[@]}"; do
    IFS='|' read -r name kind document scenario <<<"$entry"
    dir="$OUT/$name"
    gen="$dir/generated"
    obj="$dir/obj"
    mkdir -p "$gen" "$obj"
    sce_gate_step "$name: generate"

    # 1. Generate, with every unit the document imports (`--with-imports`), as a
    # build generates it: a machine's header includes the header of each schema,
    # enum, union and algorithm it names. `--no-format`: the gate compiles what
    # is written, and its spelling is not what is judged.
    "$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate -l c11 --with-imports --no-format \
        --input-root "$(dirname "$document")" --output-dir "$gen" "$document" >"$dir/generate.log" 2>&1 \
        || { cat "$dir/generate.log" >&2; sce_gate_fail "$name: the C11 generation failed"; }

    machine_objects=()
    program_objects=()
    if [[ "$kind" == machine ]]; then
        "$CODEGEN" --workspace-root "$SCE_REPO_ROOT" scenario-table --machine "$document" "$scenario" \
            >"$dir/${name}_mcu_scenario.c" 2>"$dir/scenario-table.log" \
            || { cat "$dir/scenario-table.log" >&2; sce_gate_fail "$name: its scenario could not be tabled"; }
        # Every translation unit the generator wrote, but the saved-state unit: a
        # deployment that does not persist does not link it (D2), and it is the
        # one unit that may use the hosted library.
        for source in "$gen"/*.c; do
            [[ -e "$source" ]] || continue
            case "$source" in *_saved.c) continue ;; esac
            object="$obj/$(basename "${source%.c}").o"
            compile "$source" "$object" -I "$gen" \
                || sce_gate_fail "$name: $(basename "$source") does not compile for the target with D3's flags"
            machine_objects+=("$object")
        done
        (( ${#machine_objects[@]} > 0 )) || sce_gate_fail "$name: the generator wrote no translation unit into $gen"
        compile "$dir/${name}_mcu_scenario.c" "$obj/scenario.o" -I "$gen" \
            || sce_gate_fail "$name: its scenario table does not compile"
        compile "$MCU_DIR/scenario_runner.c" "$obj/scenario_runner.o" \
            || sce_gate_fail "$name: the scenario runner does not compile"
        program_objects=("$obj/scenario.o" "$obj/scenario_runner.o")
    else
        compile "$MCU_DIR/codec_main.c" "$obj/codec_main.o" -I "$gen" \
            || sce_gate_fail "$name: the codec does not compile for the target with D3's flags"
        machine_objects=("$obj/codec_main.o")
    fi

    # 2. Link: an undefined symbol is a call the core makes to something no
    # freestanding image has, and each one is named.
    elf="$dir/$name.elf"
    if ! arm-none-eabi-gcc "${CFLAGS[@]}" -nostdlib -T "$MCU_DIR/mps2_an386.ld" \
        -Wl,-Map="$dir/$name.map" -o "$elf" \
        "${machine_objects[@]}" ${program_objects[@]+"${program_objects[@]}"} "${BOARD_OBJECTS[@]}" -lgcc \
        >"$dir/link.log" 2>&1; then
        undefined="$(grep -o "undefined reference to \`[^']*'" "$dir/link.log" | sed "s/undefined reference to \`//; s/'\$//" | sort -u | tr '\n' ' ' || true)"
        cat "$dir/link.log" >&2
        if [[ -n "$undefined" ]]; then
            sce_gate_fail "$name: the image does not link with no C library; undefined: $undefined"
        fi
        sce_gate_fail "$name: the image does not link"
    fi

    # 3. Weigh.
    read -r text rodata data bss < <(sections "${machine_objects[@]}")
    read -r itext irodata idata ibss < <(sections "$elf")
    libgcc="$(grep -o 'libgcc\.a([^)]*)' "$dir/$name.map" | sed 's/^libgcc\.a(//; s/)$//' | sort -u | tr '\n' ' ' || true)"
    key="$(printf '%s' "$name" | tr '[:lower:]' '[:upper:]')"
    for figure in TEXT RODATA DATA BSS; do
        case "$figure" in
            TEXT) got="$text" ;;
            RODATA) got="$rodata" ;;
            DATA) got="$data" ;;
            BSS) got="$bss" ;;
        esac
        var="${key}_${figure}"
        MEASURED+=("${var}=${got}")
        want="${!var:-}"
        if [[ -z "$want" || -z "${TOLERANCE_BYTES:-}" ]]; then
            UNMEASURED=$(( UNMEASURED + 1 ))
            continue
        fi
        if (( got > want + TOLERANCE_BYTES || got < want - TOLERANCE_BYTES )); then
            OUT_OF_BAND+=("$var is $got, budget $want (band $(( want - TOLERANCE_BYTES ))..$(( want + TOLERANCE_BYTES )))")
        fi
    done
    sce_gate_step "$name: .text $text .rodata $rodata .data $data .bss $bss; image $itext/$irodata/$idata/$ibss; libgcc: ${libgcc:-none}"

    # 4. Run.
    rc=0
    timeout "$RUN_SECONDS" qemu-system-arm -M "$BOARD" -semihosting-config enable=on,target=native -nographic \
        -kernel "$elf" </dev/null >"$dir/run.log" 2>&1 || rc=$?
    case "$rc" in
        0) verdict="passed" ;;
        124) verdict="ran past ${RUN_SECONDS}s"; RUN_FAILED+=("$name: $verdict") ;;
        *) verdict="exit $rc"; RUN_FAILED+=("$name: $verdict") ;;
    esac
    sce_gate_step "$name: run $verdict"
    if (( rc != 0 )); then
        cat "$dir/run.log" >&2
    fi
    printf '| %s | %d | %d | %d | %d | %d | %d | %d | %d | %s | %s |\n' \
        "$name" "$text" "$rodata" "$data" "$bss" "$itext" "$irodata" "$idata" "$ibss" "${libgcc:-none}" "$verdict" >>"$REPORT"
done

{
    printf '\nWhat each run wrote (its last line says how many steps and expectations held, and what it left to the host C11 lane):\n\n'
    for entry in "${C11_MCU_SET[@]}"; do
        IFS='|' read -r name _ <<<"$entry"
        printf -- '- `%s`: %s\n' "$name" "$(tr '\n' ' ' <"$OUT/$name/run.log")"
    done
} >>"$REPORT"
sce_gate_step "report: $REPORT"

if (( ${#RUN_FAILED[@]} > 0 )); then
    sce_gate_fail "on the target: ${RUN_FAILED[*]}"
fi
if (( UNMEASURED > 0 )); then
    printf '  [c11-mcu] the budget holds %d unmeasured figure(s); measured with %s:\n' "$UNMEASURED" "$GCC_VERSION" >&2
    printf '    %s\n' "${MEASURED[@]}" >&2
    sce_gate_fail "$BUDGET is unmeasured: pin the figures above, MEASURED_WITH and TOLERANCE_BYTES, and say in the commit what was measured"
fi
if (( ${#OUT_OF_BAND[@]} > 0 )); then
    printf '  [c11-mcu] %s\n' "${OUT_OF_BAND[@]}" >&2
    printf '  [c11-mcu] running %s; the budget was measured with %s\n' "$GCC_VERSION" "$MEASURED_WITH" >&2
    printf '  [c11-mcu] re-pin the budget in %s if the change is meant, and say in the commit message what the bytes bought.\n' "$BUDGET" >&2
    sce_gate_fail "a C11 machine's cost on the target is outside its budget"
fi
echo "c11-mcu: ${#C11_MCU_SET[@]} unit(s) linked with no C library, weighed within budget and run on $BOARD"
