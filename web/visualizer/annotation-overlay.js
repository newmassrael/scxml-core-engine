// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * The annotation family, drawn from the acceptance report's closure.
 * NL-IR closure ledger row G2.
 *
 * ## This file derives nothing, and that is its entire specification
 *
 * A requirement's evidence is NOT the elements carrying its `sce:req`.
 * It is the dependency closure of those elements: the `<send>` that arms
 * the timer, the `<cancel>` naming that send, the target state's entry
 * actions, the source state's exit actions. Measured over the real
 * fixture, 21 of 65 (requirement, dependency) pairs lie on nodes that
 * carry no id at all — every arming send and every cancel among them.
 *
 * So a walk here that picked elements by `sce:req` would draw a picture
 * disagreeing with the table the machine measures, and NOBODY WOULD FIND
 * OUT: a rendered diagram is not bytes a test can diff. That is the
 * reason the Requirement-closure RFC puts this after the report rather
 * than before it.
 *
 * The overlay therefore ARRIVES, computed by `annotation_overlay()` in
 * the codegen WASM, which calls the report's own `fragment()`. Every
 * function below is a lookup over what arrived. There is no traversal of
 * the SCXML in this file, and there must never be one.
 *
 * ## Matching, without parsing a path
 *
 * SCE names a node `states.armed.transitions[0]`. The graph here is keyed
 * by state id and edge index. Parsing that string in JavaScript would put
 * a second copy of the path grammar in this file, free to fall behind the
 * Rust that writes it — so the producer publishes `state_id` and
 * `transition_index` beside `node_path`, and this file matches on those.
 */

const SCE_ANNOTATION_UNCLAIMED = 'sce-unclaimed';
const SCE_ANNOTATION_CLAIMED = 'sce-claimed';

class AnnotationOverlay {
    /**
     * @param {object} overlay - the parsed `annotation_overlay()` JSON.
     *   Passed in rather than fetched, so the caller owns the transport
     *   and this class stays testable outside a browser.
     */
    constructor(overlay) {
        if (!overlay || typeof overlay !== 'object') {
            throw new Error('AnnotationOverlay: no overlay given');
        }
        if (overlay.v !== 1) {
            // A deployed page outlives the build that produced its data,
            // so an unknown shape is reported rather than half-read.
            throw new Error(`AnnotationOverlay: unknown overlay version ${overlay.v}`);
        }
        this.nodes = overlay.nodes || [];
        this.fragments = overlay.fragments || [];

        // node_path -> node, so a fragment's dependency resolves to the
        // element it names without re-deriving anything.
        this.byPath = new Map();
        for (const node of this.nodes) {
            this.byPath.set(node.node_path, node);
        }
    }

    /** Requirement ids a state claims, or [] when it claims none. */
    requirementsForState(stateId) {
        const node = this.nodes.find(
            (n) => n.node_type === 'state' && n.state_id === stateId
        );
        return node ? node.requirements : [];
    }

    /** Requirement ids a transition claims, by the key the graph uses. */
    requirementsForTransition(stateId, index) {
        const node = this.nodes.find(
            (n) => n.node_type === 'transition'
                && n.state_id === stateId
                && n.transition_index === index
        );
        return node ? node.requirements : [];
    }

    /**
     * Every node path the requirement's evidence depends on.
     *
     * Straight off the wire. The set deliberately includes paths whose
     * node claims nothing — see the header.
     */
    fragmentPaths(requirement) {
        const fragment = this.fragments.find((f) => f.requirement === requirement);
        return fragment ? fragment.dependencies.map((d) => d.node_path) : [];
    }

    /**
     * Whether this node is in the requirement's fragment WITHOUT carrying
     * its id — the case a by-id walk gets wrong, exposed as its own
     * question so a caller can style it and a test can assert it.
     */
    isSupportingNode(nodePath, requirement) {
        const node = this.byPath.get(nodePath);
        if (!node) {
            return false;
        }
        return (
            this.fragmentPaths(requirement).includes(nodePath)
            && !node.requirements.includes(requirement)
        );
    }

    /** Nodes no requirement claims — what the reviewer is looking for. */
    unclaimedPaths() {
        return this.nodes.filter((n) => n.requirements.length === 0).map((n) => n.node_path);
    }

    /**
     * Stamp the graph the visualizer built with what arrived.
     *
     * Mutates in place, adding `requirements` and `annotationClass` to
     * each state and transition. The graph's shape is untouched: this
     * says what each element CLAIMS, never what exists.
     */
    applyTo(states, transitions) {
        for (const state of states || []) {
            state.requirements = this.requirementsForState(state.id);
            state.annotationClass = state.requirements.length
                ? SCE_ANNOTATION_CLAIMED
                : SCE_ANNOTATION_UNCLAIMED;
        }

        // Transitions are keyed by (source state, position in that state),
        // which is what the producer published beside the path.
        const seen = new Map();
        for (const transition of transitions || []) {
            const source = transition.source ?? transition.from;
            const index = seen.get(source) ?? 0;
            seen.set(source, index + 1);
            transition.requirements = this.requirementsForTransition(source, index);
            transition.annotationClass = transition.requirements.length
                ? SCE_ANNOTATION_CLAIMED
                : SCE_ANNOTATION_UNCLAIMED;
        }
        return { states, transitions };
    }
}

/**
 * Ask the codegen WASM for the overlay.
 *
 * The transport, and the only place this file touches the module. The
 * closure is built on the Rust side; nothing here recomputes it.
 */
async function loadAnnotationOverlay(wasmModule, scxmlContent, scxmlName) {
    const json = wasmModule.annotation_overlay(scxmlContent, scxmlName || 'untitled');
    return new AnnotationOverlay(JSON.parse(json));
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        AnnotationOverlay,
        loadAnnotationOverlay,
        SCE_ANNOTATION_CLAIMED,
        SCE_ANNOTATION_UNCLAIMED,
    };
}
