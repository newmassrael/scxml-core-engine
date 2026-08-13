# AI Loop Example — a statechart supervising a long-running agent.
#
# `ai_loop.scxml` reports needs_script_engine=true (its prompts and standing
# instructions are ECMAScript datamodel values), so this links the scripting
# runtime rather than the pure-static one.

add_executable(ai_loop_example ai_loop_example.cpp)

sce_add_state_machine(
    TARGET ai_loop_example
    SCXML_FILE ai_loop.scxml
)

target_link_libraries(ai_loop_example PRIVATE sce_runtime sce_scripting)

target_compile_features(ai_loop_example PRIVATE cxx_std_17)

# The other examples under examples/ are demonstrations: they print what a
# machine does and always exit 0, so registering them as tests would assert
# nothing. This one runs four scenarios, checks each against the outcome the
# document enumerates, and exits non-zero when one disagrees — it was already
# a test in everything but registration, and until this line nothing ran it.
# Its sibling `backends/rust/tests/tests/ai_loop.rs` asks the Rust engine the
# same questions, so the pair fails loudly when only one engine honours a
# change to the topology.
#
# Guarded on BUILD_TESTS because `enable_testing()` is called there; without
# it CTest is not initialised and add_test would register into nothing.
if(BUILD_TESTS)
    add_test(NAME ai_loop_example COMMAND ai_loop_example)
    set_tests_properties(ai_loop_example PROPERTIES LABELS "example")
endif()
