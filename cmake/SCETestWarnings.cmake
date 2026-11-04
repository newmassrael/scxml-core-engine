# SCETestWarnings.cmake
# The warning classes every test program this repository builds turns off, and
# why. One definition, read by every project that builds tests: the C++ test
# tree (`apply_sce_test_policies` in tests/CMakeLists.txt) and the C++ forge
# conformance harness (backends/cpp/forge-runtime/tests/conformance), which is a
# standalone CMake project and so cannot call a function the test tree defines.
#
#   include(${REPO_ROOT}/cmake/SCETestWarnings.cmake)
#   target_compile_options(<target> PRIVATE ${SCE_TEST_WARNING_OPTIONS})
#   # or, for every target a directory defines after it:
#   add_compile_options(${SCE_TEST_WARNING_OPTIONS})
#
# Provides:
#   SCE_TEST_WARNING_OPTIONS
#     A generator expression, so it is decided by the compiler a target is built
#     with and never expands to an empty argument list where none applies.
#
# GCC 13+ false positive (GCC bug 80635 and its family): -Wmaybe-uninitialized
# inside libstdc++'s std::variant and std::optional, reached whenever one of them
# is copied or moved through an inlined path at -O2. Whether it fires is an
# inlining decision, not a property of the target, so it is set for every test
# target rather than per target. It was per target (js_engine_tests alone) until
# mesh_dds_pool_test tripped it the first time a CycloneDDS host built it: its
# generated <onentry> moves a <param> ScriptValue into the send list. The forge
# conformance harness tripped it next, on a generated algorithm reading a
# std::optional<std::uint8_t> parameter after has_value() once it was inlined
# into the test that calls it. Only this class is turned off; -Werror and every
# other class stay as each target sets them.
include_guard(GLOBAL)

set(SCE_TEST_WARNING_OPTIONS
    "$<$<AND:$<CXX_COMPILER_ID:GNU>,$<VERSION_GREATER_EQUAL:$<CXX_COMPILER_VERSION>,13>>:-Wno-maybe-uninitialized>"
)
