// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The directory of one tree of sessions: where an input that names a session is
// handed to that session (docs/adr/0084, D2 as amended at step 10;
// ARCHITECTURE.md "Engine Threading").
//
// Every session of a tree has its own inbox. An input another thread has for a
// named session, a host-run invocation's ending, is resolved when it is posted:
// the directory finds the session by its id and hands the input to that
// session's own door, so it enters that session's queue in the order the post
// linearised against the session's own enqueues. Nothing is held in the root's
// inbox for a session the root finds only when it drains.
//
// A header of the shared layer because both C++ engines keep one: the generated
// engine's door posts to the session's inbox (`StaticExecutionEngine.h`), and the
// Interpreter's takes the ending through the session's own door, as a direct
// call would (`runtime/StateMachine.h`).

#pragma once

#include "core/EngineInbox.h"
#include "core/HostProcessor.h"

#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <shared_mutex>
#include <string>
#include <utility>

namespace SCE::Core {

/**
 * @brief The sessions of one tree, by session id: the door each takes an input
 *        that names it through
 *
 * Read from any thread: a lookup takes a shared lock for the length of a map
 * search, and the door is called outside it, so a door that drives a session
 * (the Interpreter's) may start and end sessions of the tree on the calling
 * thread. A session enters when it starts, before its first macrostep, and
 * leaves when it ends, after it closed its inbox; a door found just before the
 * session left reaches an inbox that refuses, and answers so.
 *
 * Made by the engine that is the root of the tree and shared by every session
 * of it, as the tree's invokers are. Held by a `std::shared_ptr`, from which each
 * membership takes its own hold.
 */
class TreeSessions : public std::enable_shared_from_this<TreeSessions> {
public:
    /// How a session takes a host-run invocation's ending posted for it; whether
    /// it took it.
    using EndingDoor = std::function<bool(const ::SCE::HostInvokeEnding &)>;

    /**
     * @brief A session's place in the directory, held by the session
     *
     * It leaves when `leave` is called or the membership goes, whichever comes
     * first, and only the entry it made: a session entered again under the same id
     * since keeps its own. Movable with its session; not copyable.
     */
    class Membership {
    public:
        Membership() = default;

        ~Membership() {
            leave();
        }

        Membership(Membership &&other) noexcept
            : tree_(std::move(other.tree_)), sessionId_(std::move(other.sessionId_)), ticket_(other.ticket_) {
            other.tree_.reset();
        }

        Membership &operator=(Membership &&other) noexcept {
            if (this != &other) {
                leave();
                tree_ = std::move(other.tree_);
                sessionId_ = std::move(other.sessionId_);
                ticket_ = other.ticket_;
                other.tree_.reset();
            }
            return *this;
        }

        Membership(const Membership &) = delete;
        Membership &operator=(const Membership &) = delete;

        /// Leave the directory; nothing for a membership that holds no entry.
        void leave() {
            if (tree_) {
                tree_->remove(sessionId_, ticket_);
                tree_.reset();
            }
        }

    private:
        friend class TreeSessions;

        Membership(std::shared_ptr<TreeSessions> tree, std::string sessionId, std::uint64_t ticket)
            : tree_(std::move(tree)), sessionId_(std::move(sessionId)), ticket_(ticket) {}

        std::shared_ptr<TreeSessions> tree_;
        std::string sessionId_;
        std::uint64_t ticket_ = 0;
    };

    /**
     * @brief Enter the session @p sessionId, which takes what is posted for it
     *        through @p door; the membership that leaves again
     *
     * An entry already under that id is replaced: the id names the session
     * starting now.
     */
    Membership enter(std::string sessionId, EndingDoor door) {
        auto shared = std::make_shared<const EndingDoor>(std::move(door));
        std::uint64_t ticket = 0;
        {
            std::unique_lock<std::shared_mutex> lock(mutex_);
            ticket = ++lastTicket_;
            sessions_[sessionId] = Entry{ticket, std::move(shared)};
        }
        return Membership(shared_from_this(), std::move(sessionId), ticket);
    }

    /**
     * @brief Hand @p ending to the session @p sessionId names, through its own door
     *
     * @return `NoSuchSession` when the tree holds no session of that id: it never
     *         started, or it has ended. `Delivered` when the session's door took the
     *         ending, and `NotRunning` when it refused it.
     */
    ::SCE::HostInvokeEndOutcome endHostInvokeIn(const std::string &sessionId,
                                                const ::SCE::HostInvokeEnding &ending) const {
        std::shared_ptr<const EndingDoor> door;
        {
            std::shared_lock<std::shared_mutex> lock(mutex_);
            const auto found = sessions_.find(sessionId);
            if (found != sessions_.end()) {
                door = found->second.door;
            }
        }
        if (!door) {
            return ::SCE::HostInvokeEndOutcome::NoSuchSession;
        }
        return (*door)(ending) ? ::SCE::HostInvokeEndOutcome::Delivered : ::SCE::HostInvokeEndOutcome::NotRunning;
    }

private:
    struct Entry {
        /// Which entering made this entry, so a membership removes only its own.
        std::uint64_t ticket = 0;
        std::shared_ptr<const EndingDoor> door;
    };

    void remove(const std::string &sessionId, std::uint64_t ticket) {
        std::unique_lock<std::shared_mutex> lock(mutex_);
        const auto found = sessions_.find(sessionId);
        if (found != sessions_.end() && found->second.ticket == ticket) {
            sessions_.erase(found);
        }
    }

    mutable std::shared_mutex mutex_;
    std::map<std::string, Entry> sessions_;
    std::uint64_t lastTicket_ = 0;
};

/**
 * @brief What every session of one generated tree shares, handed by a parent to
 *        each child it starts or resumes, before the child runs
 *
 * The invokers the host registered (docs/adr/0045), the directory of the tree's
 * sessions, and the waker its posts call (docs/adr/0084, D2 as amended at step 10).
 */
struct SessionTree {
    std::shared_ptr<::SCE::TreeInvokers> invokers;
    std::shared_ptr<TreeSessions> sessions;
    std::shared_ptr<InboxWaker> waker;
};

}  // namespace SCE::Core
