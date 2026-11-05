# The reference harness (docs/adr/0087, section 6) run by a host program, from the
# documents the repository's tests replay, read in place: the harness statechart, the
# answer schema it names, its two text templates and the application's refusal record
# and code enum, all in sce-build/tests/fixtures/static_datamodel/. There is one copy of
# each document; this directory holds the program and its README only.

add_executable(model_harness_example main.cpp)

set(_SCE_MODEL_HARNESS_DOCUMENTS ${CMAKE_SOURCE_DIR}/sce-build/tests/fixtures/static_datamodel)

# The machine reads its imports beside the document it is generated from.
sce_add_state_machine(
    TARGET model_harness_example
    SCXML_FILE ${_SCE_MODEL_HARNESS_DOCUMENTS}/static_model_harness.scxml
)
# The two templates whose units' `render` the machine calls: documents of their own
# kind, generated beside the machine that includes their headers.
foreach(_SCE_MODEL_HARNESS_TEMPLATE harness_ask harness_repair)
    sce_add_algorithm(
        TARGET model_harness_example
        SCXML_FILE ${_SCE_MODEL_HARNESS_DOCUMENTS}/${_SCE_MODEL_HARNESS_TEMPLATE}.scxml
    )
endforeach()

# The reference host of the standard host features (sce/include/host/StandardHost.h)
# arrives with the runtime.
target_link_libraries(model_harness_example PRIVATE sce_runtime)
target_include_directories(model_harness_example PRIVATE ${CMAKE_SOURCE_DIR}/sce/include)
target_compile_features(model_harness_example PRIVATE cxx_std_20)

# Like ai_loop's, this program checks the run against what the documents say and exits
# non-zero when it disagrees, so it is registered as a test. Guarded on BUILD_TESTS
# because `enable_testing()` is called there.
if(BUILD_TESTS)
    add_test(NAME model_harness_example COMMAND model_harness_example)
    set_tests_properties(model_harness_example PROPERTIES LABELS "example")
endif()
