# Smart Light Example - C++ Function Integration Demo

add_executable(smart_light_example main.cpp)

# Generate state machine code from SCXML
sce_add_state_machine(
    TARGET smart_light_example
    SCXML_FILE smart_light.scxml
)

# Link with SCE library
target_link_libraries(smart_light_example PRIVATE sce_unified)
