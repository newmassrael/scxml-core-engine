// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "actions/NativeActionContract.h"

#include "actions/ForeachAction.h"
#include "actions/IfAction.h"
#include "actions/NativeAction.h"
#include "model/IStateNode.h"
#include "model/ITransitionNode.h"
#include "model/SCXMLModel.h"
#include "parsing/SemanticError.h"

#include <algorithm>
#include <map>
#include <memory>
#include <string>

namespace SCE::NativeActionContract {

namespace {

using ActionList = std::vector<std::shared_ptr<IActionNode>>;

void collect(const ActionList &actions, const IStateNode &state, Site site, std::vector<Occurrence> &out) {
    for (const auto &node : actions) {
        if (const auto *native = dynamic_cast<const NativeAction *>(node.get())) {
            out.push_back(Occurrence{native, &state, site});
        } else if (const auto *conditional = dynamic_cast<const IfAction *>(node.get())) {
            for (const auto &branch : conditional->getBranches()) {
                collect(branch.actions, state, site, out);
            }
        } else if (const auto *loop = dynamic_cast<const ForeachAction *>(node.get())) {
            collect(loop->getIterationActions(), state, site, out);
        }
    }
}

const char *word(const NativeAction &action) {
    return NativeAction::effectWord(action.getEffect());
}

[[noreturn]] void refuseRelease(const NativeAction &action, const std::string &detail) {
    throw parsing::SemanticNativeActionRelease(action.getOperation(), action.getReleasedBy(), detail);
}

}  // namespace

std::vector<Occurrence> occurrences(const SCXMLModel &model) {
    std::vector<const IStateNode *> states;
    for (const auto &state : model.getAllStates()) {
        states.push_back(state.get());
    }
    // The build tool holds its states in a map keyed by id, so it walks them in
    // the byte order of the id; the same order makes the first refusal of a
    // document with several the same one.
    std::stable_sort(states.begin(), states.end(),
                     [](const IStateNode *a, const IStateNode *b) { return a->getId() < b->getId(); });

    std::vector<Occurrence> out;
    for (const IStateNode *state : states) {
        for (const auto &block : state->getEntryActionBlocks()) {
            collect(block, *state, Site::Entry, out);
        }
        for (const auto &block : state->getExitActionBlocks()) {
            collect(block, *state, Site::Exit, out);
        }
        if (const auto initial = state->getInitialTransition()) {
            collect(initial->getActionNodes(), *state, Site::Initial, out);
        }
        // A `<history>` carries one transition, the default configuration it
        // stands for; any other state's transitions are the ones an event takes.
        const Site transitionSite = state->getType() == Type::HISTORY ? Site::HistoryDefault : Site::Transition;
        for (const auto &transition : state->getTransitions()) {
            collect(transition->getActionNodes(), *state, transitionSite, out);
        }
    }
    return out;
}

bool statesDuty(const SCXMLModel &model) {
    const auto all = occurrences(model);
    return std::any_of(all.begin(), all.end(),
                       [](const Occurrence &o) { return o.action->getEffect() == NativeAction::Effect::Duty; });
}

void require(const SCXMLModel &model) {
    const auto all = occurrences(model);

    // One name, one effect: the first call site fixes it, a later one that says
    // otherwise is the refusal.
    std::map<std::string, const NativeAction *> firstOfName;
    for (const auto &occurrence : all) {
        const NativeAction &action = *occurrence.action;
        if (action.getEffect() == NativeAction::Effect::Unstated) {
            continue;
        }
        const auto [at, inserted] = firstOfName.emplace(action.getOperation(), &action);
        if (!inserted && at->second->getEffect() != action.getEffect()) {
            throw parsing::SemanticNativeActionSignatureConflict(
                action.getOperation(), std::string("effect '") + word(action) + "' here disagrees with '" +
                                           word(*at->second) +
                                           "' at another call site: one host operation is either a duty or a notice");
        }
    }

    // What each name states, for judging a release by what it is.
    std::map<std::string, NativeAction::Effect> effectOf;
    for (const auto &occurrence : all) {
        effectOf[occurrence.action->getOperation()] = occurrence.action->getEffect();
    }

    // A state's transitions are judged before the rest of it, as the build tool
    // judges them, so a document with several bad releases is refused at the same
    // one by both. The sort is stable: states stay in id order, and within a state
    // the rest keeps `<onentry>`, `<onexit>`, `<initial>`, history default.
    std::vector<const Occurrence *> judged;
    for (const auto &occurrence : all) {
        judged.push_back(&occurrence);
    }
    std::stable_sort(judged.begin(), judged.end(), [](const Occurrence *a, const Occurrence *b) {
        if (a->state->getId() != b->state->getId()) {
            return a->state->getId() < b->state->getId();
        }
        return (a->site == Site::Transition) && (b->site != Site::Transition);
    });

    for (const Occurrence *occurrence : judged) {
        const NativeAction &action = *occurrence->action;
        if (action.getReleasedBy().empty()) {
            continue;
        }
        if (occurrence->site != Site::Entry) {
            refuseRelease(action, "released-by is stated only on an action of an <onentry>: it names the action of "
                                  "the same state's <onexit> that gives back what the entry took");
        }
        if (action.getEffect() != NativeAction::Effect::Duty) {
            refuseRelease(action, std::string("a ") + word(action) +
                                      " takes nothing to release; released-by belongs to an action whose effect "
                                      "is duty");
        }
        const std::string &release = action.getReleasedBy();
        const auto releaseEffect = effectOf.find(release);
        if (releaseEffect == effectOf.end()) {
            refuseRelease(action, "names no <sce:action> of this document");
        }
        const bool onExit = std::any_of(all.begin(), all.end(), [&](const Occurrence &other) {
            return other.state == occurrence->state && other.site == Site::Exit &&
                   other.action->getOperation() == release;
        });
        if (!onExit) {
            refuseRelease(action, "'" + release + "' is not performed on the <onexit> of state '" +
                                      occurrence->state->getId() +
                                      "', whose <onentry> takes this duty: a way out of the state would not "
                                      "release it");
        }
        if (releaseEffect->second != NativeAction::Effect::Duty) {
            refuseRelease(action, "'" + release + "' is a " + NativeAction::effectWord(releaseEffect->second) +
                                      ", which the host may drop, so the duty it is to release would stay held; "
                                      "make it a duty");
        }
    }
}

}  // namespace SCE::NativeActionContract
