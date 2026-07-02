# SCE Python Bindings - pybind11 wrapper for ReadySCXMLEngine
# Usage: cmake .. -DBUILD_PYTHON_BINDINGS=ON -DCMAKE_BUILD_TYPE=Release

include(FetchContent)

FetchContent_Declare(
    pybind11
    GIT_REPOSITORY https://github.com/pybind/pybind11.git
    GIT_TAG        v2.13.6
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(pybind11)

# Python extension module (_sce.so / _sce.pyd)
pybind11_add_module(_sce src/bindings.cpp)

target_link_libraries(_sce PRIVATE sce_runtime)

# pybind11 macro expansions trigger warnings under -Wall -Wextra -pedantic -Werror
# Suppress -Werror for this target only (same approach as GoogleTest targets)
target_compile_options(_sce PRIVATE
    $<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-Wno-error>
)

# Output the module to backends/python/bindings/ for easy PYTHONPATH setup
set_target_properties(_sce PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}"
)
