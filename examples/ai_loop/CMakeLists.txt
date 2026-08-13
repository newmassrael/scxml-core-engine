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
