# ADR 0110 — A C11 machine's core is freestanding, and a gate weighs it and runs it on an MCU

- Status: Accepted (2026-11-20). Written under the owner's standing rule (the most long-term correct structure,
  textbook in its foundations, cost ignored; decided, not asked), at the request of a downstream consumer that builds
  a firmware image from generated C11 machines and wants this repository's own gate to be the evidence for what the
  image costs and that it runs.
- Date: 2026-11-20
- Amends, once landed (each amended document gains an "Amended by: ADR 0110" line in the landing that moves it):
  - `docs/SCE_ACCEPTED_SUBSET.md`, the C11 lane's section: what a machine's core may include and what each hosted
    unit is (D1, D2);
  - the C templates (`tools/codegen/templates/c/`) and the runtime headers (`backends/c/runtime/include/sce/`,
    `backends/c/forge-runtime/include/sce/forge/`), by the split of D2.
- Scope: the C11 lane: its runtime, its machine templates, a new gate (`scripts/gates/c11-mcu.sh`), its fixtures and
  budget, the toolchain declared in `bx.toml`, and the CI workflow that mirrors the gate.
- Related: ADR 0053 (the Rust runtime's `std` profile, which this is the C11 counterpart of), ADR 0079 (a device core
  does no I/O), ADR 0082 (an algorithm writes into storage its caller holds), ADR 0084 (one inbox, one door), ADR 0108
  (maps, whose C11 runtime is already close to freestanding).

## Context

A consumer that runs generated machines on microcontrollers asked what a C11 machine costs in an image and whether it
runs without a hosted C library. This repository could not say. A survey of `origin/main` (2c6e93f051f) found:

- **The core is hosted.** Every `sce-static` machine's `.c` includes `<stdio.h>`. Advancing time writes a diagnostic
  with `fprintf(stderr, ...)`, the session identity is built with `snprintf`, and `<log>` lowers to
  `fprintf(stderr, ...)`. Headers every machine includes use `snprintf`, `strtod`, `strtol`, `strtoull`, `strcpy`,
  `strrchr` and `errno` (`wire.h`, `number_text.h`, `host_processor.h`, `invoke.h`, `error_data.h` through
  `saved_json.h`, and `event_payload.h` when payloads are typed).
- **Saved state cannot be left out.** It is emitted for every machine whose tree can hold one, and it formats and
  parses text with `snprintf`, `vsnprintf` and `strtod`. No switch leaves it out.
- **Nothing links or runs.** The one C cross-compile in the repository is compile-only and passes because the
  toolchain ships newlib's headers. No gate links a C11 machine for a target, weighs its sections, or runs it.
- **Some parts are already close.** The map runtime (ADR 0108) and the codec headers need only `memcpy`, `memset`,
  `memmove`, `memcmp` and `strlen`.
- **The toolchain is undeclared.** `bx.toml` declares no ARM cross compiler and no emulator, so a gate could not be
  placed on a build machine.

The Rust lane solved the same question with a `std` profile (ADR 0053) and a footprint budget
(`backends/rust/probes/nostd-footprint/footprint.budget`). The C11 lane has neither.

## Decision

### D1. A machine's core uses only what a freestanding implementation provides, and the four memory functions

A C11 machine's core is its state machine, its datamodel, its maps and lists, the algorithms and codecs it calls,
its inbox and its timer queue. The core includes only the headers C11 requires of a freestanding implementation
(`<stdint.h>`, `<stddef.h>`, `<stdbool.h>`, `<limits.h>`, `<float.h>`, `<stdarg.h>`, `<stdalign.h>`,
`<stdnoreturn.h>`, `<iso646.h>`). It calls no library function but `memcpy`, `memset`, `memmove` and `memcmp`.
GCC and Clang emit calls to those four even under `-ffreestanding`, so every C environment must supply them, and the
runtime does not implement them a second time.

Every other string or number routine the core needs (a length, a comparison, a decimal text of an integer) is the
runtime's own, in one header (`sce/text.h`), written once and used by every unit that needs it, the hosted ones
included. The core never parses or formats a real number. Where a value of a real type must become text, it is
handed whole to a seam (D2).

### D2. Everything hosted is a seam or a separate unit, chosen by what a deployment links

Each thing the core used the hosted library for becomes one of two things. It never becomes a preprocessor switch:
a switch that compiles code out is a second program that nothing tests unless it is built both ways, and the unit a
deployment does not link is never built in the first place.

1. **Seams.** Each is a function the core declares and the deployment defines, as `_sce_clock_now_ms` already is.
   - `_sce_log_write(level, label, label_len, text, text_len)`. `<log>` passes the label and the expression's text
     to it; a real value reaches it as text that the value's own seam formats (item 2). The runtime's diagnostics,
     such as time running backwards, go through it at their own level.
   - `_sce_log_real(level, label, label_len, value)`: a `<log>` of a real value, which the seam formats as it likes.
     The core holds no float formatter.
   - The hosted implementation of both is `backends/c/runtime/posix/log.c`, writing to `stderr` as today.
   - `_sce_real_text` and `_sce_real_read` (`sce/real_text.h`): a real number written as text and read back from
     it, for the event data the core still carries as text (item 2). (Amended 2026-11-20.) The hosted
     implementation is `backends/c/runtime/text/real_text.c`, the library `sce_c_text`, over `number_text.h`.
2. **Separate units.** A deployment that wants these compiles them beside the core and may use the hosted library
   there.
   - **Saved state:** `<machine>_saved.c` and the `saved_*.h` headers. The core declares the save and the restore,
     and the unit defines them. A deployment that does not persist does not link them.
   - **Text and payload parsing:** `number_text.h` and its real-number parsing and formatting. (Amended
     2026-11-20.) The core still carries an event's data as JSON text: a `<send>`'s params, done data and a child's
     events are written as text inside the machine and read back at its door. Making them typed is the
     boundary-coupling record's work, which follows this one. Until then the core's JSON reader and writer
     (`wire.h`, `event_payload.h`) stay in the core, written over freestanding headers and `sce/text.h`. Integers
     are read and written digit by digit there, and a real crosses only through the two real-number seams of item
     1, so nothing hosted is reached.
   - **Host processors and invoke** (`host_processor.h`, `invoke.h`): their text building moves onto `sce/text.h`,
     and what still needs the hosted library (`request_field.h`, the HTTP client) stays in the host's unit.

The generated `.c` therefore includes no hosted header. The generated header declares the seams the machine uses,
so a deployment learns what it must define from the header, and the link tells it the same.

### D3. The gate `c11-mcu` links, weighs and runs a fixed set of machines on a Cortex-M4

`scripts/gates/c11-mcu.sh`, mirrored by a CI workflow, does three things for each machine of the set:

1. **Link.** It generates the machine and compiles it with `arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb
   -mfloat-abi=hard -mfpu=fpv4-sp-d16 -std=c11 -ffreestanding -nostdlib -Os -ffunction-sections -fdata-sections
   -Wall -Wextra -Wpedantic -Werror`. It links with `-lgcc` (for 64-bit division and the rest of the compiler's
   support) and with three things only: the gate's four memory functions, its seam definitions, and a startup (the
   vector table and the reset handler) for QEMU's Cortex-M4 board (`mps2-an386`). An undefined symbol fails the gate
   and names the symbol. That is the proof of D1.
2. **Weigh.** It reads `.text`, `.rodata`, `.data` and `.bss` of each machine's own objects and of the runtime they
   pull in (`arm-none-eabi-size` over the objects, and the map file for the linked image), and compares them with
   `backends/c/mcu/footprint.budget`, a file with the shape of the Rust budget. Each figure is measured, stated with
   the toolchain that measured it, and has an allowed drift. A figure over budget fails the gate. A figure moves only
   by a commit that changes the budget and says what the bytes bought.
3. **Run.** It runs the machine's scenario under `qemu-system-arm -M mps2-an386` with semihosting. The steps and
   expectations of the scenario's JSON (`sce-build/tests/fixtures/static_datamodel/scenarios/`) are turned into a C
   table when the gate generates the machine, by the generator that already writes the C11 test tables, so the
   program reads no file. The exit status through semihosting is the verdict. A case past the time the gate allows
   fails.

The set, each of which already replays on the host C11 lane:

- `static_counter`: a plain machine, with `<log>`;
- `no_std/plain_footprint_probe`: the machine the Rust footprint probe weighs, so the two lanes are weighed on one
  document;
- `static_map`: maps of scalars and of records (ADR 0108);
- `static_map_take`: a map lent to an algorithm in place (ADR 0108, step 7);
- `static_state_algorithm`: algorithm calls over records and lists;
- one codec unit (`codec_cbor_map`), weighed and linked alone, because no statechart imports a codec yet.

The gate writes what it measured to `target/c11-mcu/report.md`, which names the toolchain, the flags, the board and
each figure, so a consumer can cite it as the evidence for its own image.

### D4. The toolchain is declared, so a build machine places the gate

(Amended 2026-11-21.) What the gate needs is declared by whether its release moves the verdict (bx RFC 0009,
section 4.8):

- **The compiler is a tool with pinned bytes.** The budget's figures are the compiler's, so `bx.toml` declares
  `arm-none-eabi-gcc` as Arm's GNU Toolchain tarball of one release (13.2.rel1), which
  `scripts/install_arm_gnu_toolchain.sh` downloads and refuses unless its SHA-256 is the one Arm publishes for the
  host. The table reads the release from the script's `ARM_GNU_TOOLCHAIN_REV` line, checks the name Arm's build
  prints (`Arm GNU Toolchain 13.2.rel1`), and is `needed_by = ["c11-mcu"]`. The gate reads the release from the
  same line, puts that toolchain's `bin/` first on PATH, and refuses another build. The hosted lane installs it with
  the same script, and the budget names that build as the one its figures are measured with.
- **QEMU is a package of the gate's unit.** A run passes or fails on the scenario's expectations, on a counter clock
  the scenario alone moves, so the emulator's release does not move the verdict. The gate's registry entry says
  `packages = ["qemu-system-arm"]`, the plan puts it on the unit (bx RFC 0004, section 6), and bx places the unit
  only on a machine that has the package.
- **A link to `/usr/bin` is not a tool.** The first declaration linked the distribution's binaries into the store
  entries. apt can replace the file behind such a link while the key it is checked by stays the same, and the
  distribution's release differs by release of the distribution (Ubuntu 22.04 carries gcc-arm-none-eabi 10.3), so
  the bytes were the machine's and not the declaration's.

A machine without them ends the gate as "could not run" (`sce_gate_cannot_run` in `lib.sh`), never as a pass.

## Consequences

- **The C11 core can be linked into a firmware image with no hosted C library.** The image's cost is measured here,
  by a budget that fails on growth.
- **A deployment defines two more seams** (`_sce_log_write`, `_sce_log_real`) besides the clock, and links the saved
  and parsing units only if it uses them. The hosted test harness links the posix seams and every unit, so no host
  behaviour changes.
- **Generated C11 output changes broadly.** `<log>` calls a seam, the `.c` includes no `<stdio.h>`, and saved state
  moves into its own file. The C11 trees are build outputs (ADR 0097), so nothing committed is regenerated beyond
  the goldens that print a C11 machine.
- **Not decided here.** How a codec at the boundary feeds the door is the next record's (boundary coupling). A
  freestanding real-number formatter is not needed, because the seam formats.

## Alternatives rejected

1. **A preprocessor profile (`SCE_FREESTANDING`) that compiles the hosted parts out.** It doubles every hosted call
   site into two programs, and the one the gate does not build rots. Units that are linked or not linked keep one
   program.
2. **Linking newlib-nano and weighing that.** It would weigh the C library's formatter, the largest single cost, as
   the machine's, and it would hide a hosted call in the core instead of failing the link on it.
3. **A compile-only gate.** That is what exists, and it measured nothing: a compile passes with every hosted header
   present, and a size is meaningless until the image is linked.

## Order of work

1. This record.
2. `sce/text.h`, the two log seams and their posix definitions. The runtime headers the core includes stop including
   hosted headers. The templates: the `.c` without `<stdio.h>`, `<log>` and the time diagnostic through the seam, and
   the session identity through `sce/text.h`.
3. The separate units: saved state into `<machine>_saved.c`, and the text and payload parsing out of the core's
   includes. The host harness links them, and every C11 ctest stays green.
4. The gate: startup, memory functions, seams, the scenario table, the QEMU run, the budget, the report, the
   `bx.toml` tools and the CI workflow. The first budget is measured, not guessed.
5. `docs/SCE_ACCEPTED_SUBSET.md`'s C11 section, and the consumer is told what the report states.
