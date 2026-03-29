# Async Dispatcher Example
# Demonstrates asynchronous event processing with StdThreadDispatcher

# Create executable first
add_executable(async_dispatcher_example main.cpp)

# Generate traffic_light state machine
sce_add_state_machine(
    TARGET async_dispatcher_example
    SCXML_FILE ../traffic_light/traffic_light.scxml
)

# Link SCE library
target_link_libraries(async_dispatcher_example PRIVATE sce_runtime)

# C++20 required for concepts and other features
target_compile_features(async_dispatcher_example PRIVATE cxx_std_20)

# Link threading library
find_package(Threads REQUIRED)
target_link_libraries(async_dispatcher_example PRIVATE Threads::Threads)
