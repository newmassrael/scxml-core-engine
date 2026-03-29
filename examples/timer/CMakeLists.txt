# Timer Example - TimerManager API Demo

add_executable(timer_example timer_example.cpp)

# Generate state machine code from SCXML
sce_add_state_machine(
    TARGET timer_example
    SCXML_FILE timer_example.scxml
)

# Link with SCE library
target_link_libraries(timer_example PRIVATE sce_runtime)
