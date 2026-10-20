// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "events/IEventTarget.h"
#include "runtime/IEventRaiser.h"

#include <functional>
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief The Mesh router a child session is given by its parent (SCE_MESH.md,
 *        Child Sessions in Mesh; docs/adr/0043)
 *
 * The Interpreter's twin of the generated engines' child router. A machine's
 * host reaches its Mesh peers through the target creator it registers under
 * `SendHelper::MESH_TARGET_SCHEME`; a child session started by `<invoke>` is
 * given none of that. Its parent asks the factory registered with
 * `StateMachine::registerChildMeshRouters` instead, and builds the child with
 * a dispatcher of its own whose Mesh peers are reached only through the router
 * that factory answered, so the child's sends leave under its own name and not
 * its parent's. With none answered the child has no route to a peer, and its
 * peer send raises `error.execution` in the child.
 */
class IChildMeshRouter {
public:
    virtual ~IChildMeshRouter() = default;

    /**
     * @brief The target a `<send target="#<peer>">` of the child is delivered to
     *
     * Asked as the creator a host registers under the Mesh scheme is asked for
     * a machine it starts, at the send and for a delayed one when it is made.
     * Null refuses the send, which raises `error.execution` in the child.
     */
    virtual std::shared_ptr<IEventTarget> createTarget(const std::string &targetUri) = 0;

    /**
     * @brief The child session this router serves has been built
     *
     * Called once, before the child starts. What arrives for the child from
     * then on is raised on `inbound`, the child's own raiser, on its external
     * queue and with the `_event` the endpoint built for it (for a peer's event,
     * the one SCE_MESH.md gives a Mesh-delivered event): the child takes it in its own event loop,
     * as a machine its host started takes what its host raises.
     */
    virtual void attach(std::shared_ptr<IEventRaiser> inbound) = 0;

    /**
     * @brief The session this router was attached to has ended
     *
     * It reached a top-level `<final>`, or its parent left the invoking state
     * and cancelled it. Called once; nothing is raised on the child after it.
     */
    virtual void detach() = 0;

    /**
     * @brief The router of a child this child starts, asked as the parent's
     *        factory is asked (`ChildMeshRouters`)
     *
     * None by default: a child whose router answers none starts children with
     * no router, whose peer sends raise `error.execution` in them.
     */
    virtual std::shared_ptr<IChildMeshRouter> childRouter(const std::string &invokeId, const std::string &document) {
        (void)invokeId;
        (void)document;
        return nullptr;
    }
};

/// The factory of the routers of a machine's children, registered with
/// `StateMachine::registerChildMeshRouters`: asked, each time an invocation
/// starts a child, with the invoke's id and the stem of the child's document
/// (empty for a child written inline). None leaves the child with no router.
using ChildMeshRouters =
    std::function<std::shared_ptr<IChildMeshRouter>(const std::string &invokeId, const std::string &document)>;

}  // namespace SCE
