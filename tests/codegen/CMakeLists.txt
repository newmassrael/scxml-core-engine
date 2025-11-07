# Static code generator tests

# Test executable
add_executable(test_static_codegen
    test_static_codegen.cpp
)

# Test dependencies
target_link_libraries(test_static_codegen
    PRIVATE
    GTest::GTest
    GTest::Main
    # Add later: static_codegen_lib
)

# Test include paths
target_include_directories(test_static_codegen
    PRIVATE
    ${PROJECT_SOURCE_DIR}
    ${PROJECT_SOURCE_DIR}/sce/include
)

# Register with CTest
add_test(NAME StaticCodegenTests COMMAND test_static_codegen)