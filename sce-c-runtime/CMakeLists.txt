# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# sce-c-runtime — C11 backend support headers (RFC §5.J.1,
# watching-zenoh consumer).
#
# Mirrors the C++ AOT runtime's header-only-template pattern: every
# data structure and algorithm is instantiated inside the generated
# translation unit (via jinja2 macros under
# `tools/codegen/templates/c/helpers/`), so this crate only ships
# cross-fixture preprocessor defaults. Declared as INTERFACE for that
# reason — the compiler sees no .c sources here.

add_library(sce_c_runtime INTERFACE)

target_include_directories(sce_c_runtime
    INTERFACE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)
