// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "SCXMLTypes.h"

#include <string>
#include <vector>

namespace SCE {

/**
 * @brief The host a machine's `<sce:action>`s are performed by (§scxml-G-7)
 *
 * A generated machine names its host operations as an interface the host
 * implements, and cannot be constructed without one. The Interpreter loads a
 * document at run time, so it has no interface to generate: the host is this
 * one, and it is told the operation by name with the argument values the
 * document computed.
 *
 * It has to be installed before the machine starts: an `<onentry>` of the
 * initial state performs its actions during `start()`, so a host installed
 * afterwards arrives one action too late. An action nobody performs — no host
 * installed, or `performNativeAction` answering false — is not silently
 * dropped: the machine raises `error.execution`, the rule §scxml-6.4.1 gives
 * an `<invoke>` of a type it does not implement.
 *
 * Arguments are `bool`, integers, reals and strings — what the host reads off
 * an event's typed payload. An argument that evaluates to anything else (an
 * array, an object, `undefined`) is no value a host operation takes, and
 * raises `error.execution` before the host is called.
 */
class INativeActionHost {
public:
    virtual ~INativeActionHost() = default;

    /**
     * @brief Perform the host operation `name`
     * @param name The operation, as the document's `<sce:action name>` spells it
     * @param args Each `<sce:arg>`'s value, in document order
     * @return true when the host performed it; false when it provides no such
     *         operation, which raises `error.execution`
     */
    virtual bool performNativeAction(const std::string &name, const std::vector<ScriptValue> &args) = 0;
};

}  // namespace SCE
