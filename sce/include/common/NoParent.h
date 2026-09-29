// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/ForwardedEvent.h"

namespace SCE::Common {

/**
 * @brief The parent of a session its HOST started — there is none.
 *
 * A generated machine that sends to `#_parent` is a class template over its
 * parent's type, because an invoking parent hands itself in. A host starting
 * the same machine has no parent to hand in, and W3C SCXML C.1 still lets it
 * run: the send then raises error.communication on the sender's own queue.
 * This type is the default for that template parameter, so the host writes
 * `machine<> sm;` and the policy's parent pointer stays null.
 *
 * It declares only what the generated code names on a parent — delivery by
 * event NAME, the one identity two machines share (`ForwardedEvent`). The
 * member is never called: every delivery first asks whether the pointer is
 * null, and for this type it always is.
 */
struct NoParent {
    void raiseExternal(const ForwardedEvent &) {}
};

}  // namespace SCE::Common
