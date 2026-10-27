# ADR 0059 — A C++ consumer compiles SCE's interface at C++17, and SCE compiles its implementation at C++20

- Status: Accepted (2026-10-27), on the owner's instruction of that day: the most long-term correct structure,
  textbook in its foundations, cost ignored. Prompted by a downstream build (tc8-harness) that vendors SCE, builds
  at C++17 and broke under GCC 15
- Date: 2026-10-27
- Scope: the `target_compile_features` of every C++ target in `sce/CMakeLists.txt` and
  `backends/cpp/forge-runtime/CMakeLists.txt`, and the vendoring channels that compile SCE without its CMake
  (`embed/`, Bazel); every header under `sce/include/` and `backends/cpp/forge-runtime/include/`; every C++ file
  the generators write (the statechart templates under `tools/codegen/templates/`, `tools/codegen/templates/forge/cpp/`,
  and the C++ halves of `sce-build/src/forge/generator.rs`, `forge/expr.rs`, `forge/static_lowering.rs` and
  `forge/host_invoker_interface.rs`); SCE_FORGE.md and the synthesis RFC where they name `std::span`; a CI gate
- Related: ADR 0003 (one portable document compiled per target: the generated code is part of what a consumer
  builds), ADR 0053 (the runtime's features only add)

## Context

A downstream project vendors SCE and builds it at `-std=c++17`. Under GCC 15 it stopped building: a public header
named `uint64_t` without `<cstdint>`, and GCC 15's libstdc++ no longer provides `<cstdint>` through `<string>` at
C++17 (it still does at C++20). Fixing that header was the start. Measuring the class with GCC 15.3 (every public
header compiled alone, every library translation unit and every generated C++ header compiled with its target's own
flags, at C++17 and at C++20, 1,108 compiles) showed that the defect was a symptom of a contract nobody checked:

- `sce_core`, `sce_base`, `sce_mesh_common` and `sce_forge_runtime` declare `cxx_std_17`, but the top-level
  `CMAKE_CXX_STANDARD 20` compiles every SCE target at C++20. The C++17 declaration had never been compiled.
- `sce_runtime` and `sce_scripting` declare `PUBLIC cxx_std_20`, so a consumer that links them is raised to C++20,
  although every public header under `sce/include/` compiles alone at C++17. What needs C++20 is four
  implementation files (`std::string::starts_with`, a designated initializer).
- The generated C++ needs C++20 in 84 of 672 measured headers: 1,317 designated initializers, `std::span`
  parameters for list and byte arguments, and defaulted `operator==`. The Forge C++ conformance harness was raised
  from C++17 to C++20 to compile it (the synthesis RFC records the bump), while the Forge C++ runtime it tests still
  declares `cxx_std_17`.
- About thirty sites in public headers choose their definition by language standard (`__cpp_concepts`,
  `__cplusplus >= 202002L`): template parameter lists constrained by a concept under C++20 and unconstrained under
  C++17, `SCE::source_location` that is `std::source_location` under C++20 and an SCE struct under C++17, attribute
  macros. Within one standard that is harmless. Across standards, in one program, it is not.

## The textbook structure

A library has two language requirements, and build systems name them separately because they are different things.
CMake calls them usage requirements: what a target needs to be built (`PRIVATE`), and what anyone who uses it needs
(`INTERFACE`), with `PUBLIC` meaning both. The first is the library's own choice. The second is a contract with
every consumer, and it should be as low as the consumers need.

What a consumer compiles is the **interface**: the public headers and, for a code generator, the code it generates,
which is source compiled in the consumer's translation units at the consumer's standard. Protocol Buffers states its
generated code's language requirement as part of its support policy for this reason. What only SCE compiles is the
**implementation**: its `.cpp` files, built through SCE's own targets.

One program then holds translation units of two standards that include the same headers. The One Definition Rule
(\[basic.def.odr\]) requires every inline function, class and template definition to be the same sequence of tokens
in every translation unit. A header that picks a definition by `__cplusplus` breaks that rule exactly when the
standards differ, which is the situation this structure creates on purpose. So the interface must be
**standard-invariant**: its meaning cannot depend on which standard compiles it.

Finally, a declared requirement is only as good as the build that holds the code to it. The C++17 declaration above
was a claim; it becomes a fact when CI compiles the interface at C++17.

## Decision

### 1. The interface is C++17

Every header under `sce/include/` and `backends/cpp/forge-runtime/include/`, and every C++ file a generator writes,
compiles at `-std=c++17` and at `-std=c++20`. Every library target declares the interface standard as `PUBLIC`
(`INTERFACE` for a header-only target) `cxx_std_17`. A consumer that links SCE is never raised above C++17 by SCE.

### 2. The implementation is C++20

A target whose own sources use C++20 adds `PRIVATE cxx_std_20`. CMake compiles that target's sources at the higher
of the two and passes only the `PUBLIC` requirement on. A consumer that vendors SCE through `add_subdirectory`
therefore builds SCE's `.cpp` files at C++20 and its own at C++17.

A channel that ships SCE's sources to be compiled by the consumer's toolchain (the `embed/` package,
`scripts/embed_CMakeLists.txt` and the Bazel rules in `scripts/sce_repo.bzl`) compiles them at the consumer's
standard, so a tier it ships has a C++17 implementation as well. Today those channels ship `sce_core` and
`sce_base`, both C++17 in full. A tier with a C++20 implementation may be added to such a channel only together
with the per-target standard the channel would then have to carry.

The implementation is not lowered to C++17. Its standard is invisible to consumers, and lowering it would give up
language features for no consumer's benefit.

### 3. The interface is standard-invariant

No public header and no generated file selects a definition by language standard: no `#if` on `__cplusplus` or on
a language or library feature-test macro (`__cpp_*`, `__cpp_lib_*`) may change a declaration, a class, an inline
function, a template's parameter list or a macro the interface expands. A constraint that is a concept under C++20
is written once, as a C++17 trait checked by `static_assert`, so it reads the same in both. `SCE::source_location`
is one SCE type in both standards, filled from the compiler builtins that `std::source_location` itself is built on.
A guard on something other than the standard (a platform, an optional dependency, `SCE_ENABLE_RUNTIME_LOGGING`) is
not affected, as long as it is the same for every translation unit of one program.

### 4. A list is viewed through an SCE span

A list or byte parameter in generated C++ is `SCE::Forge::Span<const T>` (`sce/forge/span.h`): a non-owning view of
contiguous elements, `data()` and `size()`, indexing and iteration, constructible from any contiguous container
(anything with `data()` and `size()`, which includes `std::vector`, `std::array`, `std::string_view` and, in a C++20
caller, `std::span`). It is always this one type; it is never an alias that becomes `std::span` under C++20, which
would make the interface depend on the standard (§3). This is the structure of `absl::Span` and of the Core
Guidelines' `gsl::span`, which exist for the same reason. SCE_FORGE.md and the synthesis RFC say `Span` where they
said `std::span`.

### 5. Generated aggregates are initialized positionally

A generated aggregate is initialized by position, in declaration order, with each member's name beside its value
as a comment (`T{/*value=*/v, /*fault=*/f}`). The generator that writes the initializer also writes, or reads, the
declaration, so the order is the generator's to keep; C++20 already required designated initializers to appear in
declaration order, so every initializer the generators emit today is in that order. A member a designated
initializer left out is written out with the value its default member initializer gives it, because position
cannot skip.

### 6. Generated comparison is written out

A generated `operator==` compares member by member, and an `operator!=` is its negation. C++17 has no defaulted
comparison.

### 7. The gate

A CI gate compiles, at C++17 and at C++20 and with `-Werror`: every public header alone; the Forge C++ goldens; and
the generated C++ of the corpora the C++ suites already generate. It refuses a standard-selecting `#if` in a public
header or a template (§3). The Forge C++ conformance harness returns to C++17. The C++17 declaration is then a
measurement, and the transitive-include defect that started this is caught by compiling each header alone, which
the same gate does.

## Consequences

- A C++17 consumer can use the whole of SCE, runtime and generated code included, without raising its standard.
- SCE keeps C++20 in its own sources.
- The generated C++ is longer where it initializes aggregates and compares records, and the names in those
  initializers are comments the compiler does not check. The order they follow is the one C++20 compilers checked
  until now, and the conformance suites hold the values.
- Code that passed a `std::span` to a generated function still compiles, through `Span`'s container constructor.
  Code that named the parameter type `std::span` must name `SCE::Forge::Span`.
- The interface loses C++20-only conveniences (concepts in signatures, `[[likely]]`) and gains one meaning in every
  translation unit.

## Order of work

1. This ADR.
2. The target split (§1, §2) in CMake and in the vendoring channels.
3. The standard-invariant interface (§3).
4. `Span` (§4), then the generators (§4, §5, §6), with the committed generated trees regenerated in the same
   commits.
5. The gate (§7), with the Forge harness at C++17.
