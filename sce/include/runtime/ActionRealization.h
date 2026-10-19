// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

namespace SCE {

/**
 * @brief What a host does with the duties of a machine (docs/adr/0038)
 *
 * A `<sce:action effect="duty">` is an operation the machine's correctness
 * depends on the host performing. A host of a machine that states one declares,
 * in its own type, whether it performs the duties or only simulates them (a
 * recorder, a test double), and the machine refuses a host that simulates unless
 * the caller accepted a simulated host by name.
 *
 * `Performs` is a statement the host's author makes about the bodies of its
 * methods, in a line a reviewer can find. SCE cannot read the bodies, and an
 * empty body under `Performs` is that author's false statement and not an
 * unwritten fact.
 *
 * One definition serves the Interpreter's `INativeActionHost` and every
 * generated C++ host interface.
 */
enum class ActionRealization {
    /// The host performs the duties: its methods do what the names say.
    Performs,
    /// The host records, drops or fakes them.
    Simulates,
};

}  // namespace SCE
