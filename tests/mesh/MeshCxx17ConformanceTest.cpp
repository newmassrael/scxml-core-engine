// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-10.4
//
// Generated mesh code must compile at C++17, because that is what it
// promises: `sce_mesh_common` declares
// `target_compile_features(... PUBLIC cxx_std_17)`, so every consumer
// linking it is entitled to build at 17.
//
// Nothing enforced that promise before this file. Several mesh targets
// declare `cxx_std_17`, but `target_compile_features` states a *minimum*
// — with the project's global `CMAKE_CXX_STANDARD 20` those targets all
// compile at `-std=c++20`, so a C++20-only construct entering the mesh
// template would build green here and fail only in a downstream tree
// honouring the declared contract. This target is the ceiling the others
// are not: `CXX_STANDARD 17` + `CXX_STANDARD_REQUIRED ON` +
// `CXX_EXTENSIONS OFF` pins the exact dialect, so a designated
// initializer or any other post-17 construct in generated mesh output is
// a build error at the moment it lands.
//
// The custom_tcp multi-pattern fixture is the payload because it is the
// densest generated router in the tree — it exercises every pattern over
// a transport whose emitted code carries device-level constants, per-
// target clients, a server, and the socket-option factory. Compiling it
// is the check; the `main` below exists only because a linked executable
// is a stronger statement than a syntax-only pass.

#include "brake_tcp_multi_transport.h"
#include "motor_tcp_multi_transport.h"

int main() {
    return 0;
}
