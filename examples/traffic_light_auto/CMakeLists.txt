# Traffic Light Auto Timer Example

# Create executable first
add_executable(traffic_light_auto_example main.cpp)

# Generate traffic_light state machine
sce_add_state_machine(
    TARGET traffic_light_auto_example
    SCXML_FILE ../traffic_light/traffic_light.scxml
)

# Link SCE library (provides StaticExecutionEngine headers)
target_link_libraries(traffic_light_auto_example PRIVATE sce_runtime)

# C++17 required
target_compile_features(traffic_light_auto_example PRIVATE cxx_std_17)

# Link threading library
find_package(Threads REQUIRED)
target_link_libraries(traffic_light_auto_example PRIVATE Threads::Threads)
