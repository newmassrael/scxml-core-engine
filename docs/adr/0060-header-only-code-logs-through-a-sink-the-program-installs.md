# ADR 0060 — Header-only code logs through a sink the program installs

- Status: Accepted (2026-10-27), on the owner's instruction of that day: the most long-term correct structure,
  textbook in its foundations, cost ignored. It closes the one exception ADR 0059 §3 named
- Date: 2026-10-27
- Scope: `sce/include/core/LogMacros.h`; a new `sce/include/common/LogSink.h`; `sce/include/common/Logger.h` and
  `sce/src/common/Logger.cpp`; every `SCE_ENABLE_RUNTIME_LOGGING` definition (`sce/CMakeLists.txt`,
  `scripts/embed_CMakeLists.txt`, `scripts/package_embed.sh`, `scripts/sce_repo.bzl`); the allowlist of
  `tools/interface-standard/interface_standard.py`; ADR 0059 §3, which points here
- Related: ADR 0059 (the interface means one thing in every translation unit)

## Context

Seventeen public headers log, through `SCE_LOG_*` from `core/LogMacros.h`. The macros expand by
`SCE_ENABLE_RUNTIME_LOGGING`, which SCE defines `PRIVATE` on `sce_base`, `sce_scripting` and `sce_runtime`:

- in SCE's own translation units they format the message (`std::format` under C++20, otherwise spdlog's bundled
  `fmt`, otherwise the format string alone) and call `SCE::Logger`, which `sce_base` defines;
- in a consumer's translation units they discard their arguments.

The design had a reason: a consumer of the header-only tier (`sce_core`) need not link `Logger.cpp`. Its cost
is the one ADR 0059 §3 describes. An inline function or template from those headers is compiled in both kinds of
translation unit of one program with different bodies, which the One Definition Rule forbids. The linker keeps one
of the copies, so whether a header-only helper logs depends on which object file the linker met first. The format
branch also chose by language standard (`__cpp_lib_format`), the one standard-selecting `#if` ADR 0059 had to
allow.

Measured over the tree: 1,615 `SCE_LOG_*` calls, every one of them formatting with `{}` alone. None uses a format
specification.

## The textbook structure

A library whose header-only code reports diagnostics, and which must not force a dependency on its consumers,
separates three things:

1. **the call site**, which must be the same text in every translation unit;
2. **the formatting**, which, to be the same text, can use only what every translation unit has (the standard
   library at the interface standard);
3. **the sink**, which the program, not the translation unit, decides.

This is the structure of protobuf's `SetLogHandler`, of gRPC's `gpr_set_log_function`, and of the C library's
`atexit`/`signal` style of process-wide hooks: one process-wide entry point, installed at run time, and a
call-site check of whether anyone is listening. C++17's inline variables give such an entry point a single
definition across every translation unit of a program without a source file to hold it ([basic.def.odr], an
inline variable with external linkage is the same object in every translation unit).

## Decision

### 1. The call site is the same in every translation unit

`SCE_LOG_TRACE` … `SCE_LOG_ERROR` expand to one definition, with no `#if` on any macro. Each one asks the sink
whether its level is wanted, and only then formats and emits:

```cpp
if (::SCE::detail::logWanted(level)) ::SCE::detail::logEmit(level, ::SCE::detail::formatLog(...), loc);
```

`SCE_ENABLE_RUNTIME_LOGGING` is removed from every build (CMake, the embed package, Bazel).

### 2. Formatting is SCE's own, C++17, and replaces `{}` only

`SCE::detail::formatLog(const char *fmt, const Args &...args)` replaces each `{}` with the next argument written
through `std::ostream <<`, and `{{`/`}}` with a literal brace. That is the whole grammar the 1,615 calls use. The
`std::format` and `fmt` branches go. An argument with no `operator<<` is a compile error at the call site, as it
is with `std::format` today. Formatting runs only when the level is wanted, so a program that installs no sink
pays one relaxed atomic load and a comparison per call site.

### 3. The sink is installed by the program

`common/LogSink.h` (header-only, standard library only) declares:

```cpp
struct LogSink {
    bool (*wanted)(LogLevel) noexcept;
    void (*emit)(LogLevel, std::string message, const source_location &where);
};
inline std::atomic<const LogSink *> g_logSink{nullptr};
void installLogSink(const LogSink *sink) noexcept;   // inline, defined in the header
```

When no sink is installed, nothing is wanted and nothing is formatted. `SCE::Logger` (`sce_base`) installs its
own sink, which forwards to the configured `ILoggerBackend`. It does so in `Logger::initialize`,
`Logger::setBackend` and `Logger::setLevel`, and on the first use of any `sce_base` entry point that logs today. A
program that links `sce_base` therefore logs as it did. A program that uses only the header-only tier logs
nothing until it installs a sink, as it did. What changes is that one program now has one behaviour: header-only
code logs, or not, by what the program installed, never by which translation unit was compiled where.

### 4. Limits

An inline variable is one object per program image. A Windows DLL that does not export it has its own copy, so a
program made of several such DLLs would need to install the sink in each. SCE builds static libraries
(`BUILD_SHARED_LIBS` off). The limit is stated here so that a shared-library build meets it as a known
condition, not a surprise.

## Consequences

- ADR 0059 §3 loses its exception. The gate's allowlist keeps only `__cpp_lib_to_chars`, and the
  `__cpp_lib_format` entry is removed. The gate refuses an allowance that matches nothing, which holds the
  allowlist to the tree.
- A consumer's own use of header-only SCE code now logs when the program links `sce_base` and has initialized
  the logger. Before, it was silent in the consumer's translation units and logged in SCE's.
- Log call sites in hot paths gain an atomic load and a branch where a consumer's copy had none.
- Messages are formatted by `{}` substitution only. A future need for format specifications is a change to
  `formatLog`, made once for every translation unit.
