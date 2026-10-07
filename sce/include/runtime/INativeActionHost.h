// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "SCXMLTypes.h"

#include <memory>
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
 *
 * A child session started by an `<invoke>` is given its own host by this one
 * (`hostForChild`), as a generated parent's host interface answers one for
 * each child that declares `<sce:action>`s.
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

    /**
     * @brief The host a child session of an `<invoke>` is built with (§scxml-6.4.1)
     *
     * A child that declares `<sce:action>`s takes its host before it runs: its
     * first `<onentry>` may already perform one. So the machine asks for it each
     * time the invocation starts — when the state is entered, and again when a
     * snapshot restores it — and installs the answer on the child before the
     * child starts, as a generated parent builds its child with the host the
     * operation `actions_for_<invoke>` answers.
     *
     * The default answers none, which leaves the child without a host: an action
     * it performs raises `error.execution`, as it does in a machine nobody
     * installed a host on.
     *
     * @param invokeId The `<invoke>`'s id, the one its `done.invoke` event carries
     * @param document The stem of the document the child was loaded from — the
     *        last path segment of its `src` (or evaluated `srcexpr`) without
     *        extension, which tells the candidates of one `<invoke>` apart — and
     *        empty when the child came inline as `<content>`
     * @return The child's host, or nullptr for none
     */
    virtual std::shared_ptr<INativeActionHost> hostForChild(const std::string &invokeId, const std::string &document) {
        (void)invokeId;
        (void)document;
        return nullptr;
    }
};

}  // namespace SCE
