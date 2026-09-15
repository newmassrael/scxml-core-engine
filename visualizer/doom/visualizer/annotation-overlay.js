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

    /**
     * What a graph element carries, given what it claims.
     *
     * ⭐ The ONE place the two marks are decided. The node builder, the
     * link builder and `applyTo` all come through here, so an element
     * cannot be marked one way when drawn as a state and another when
     * drawn as an edge.
     */
    annotationFrom(requirements) {
        return {
            requirements,
            annotationClass: requirements.length
                ? SCE_ANNOTATION_CLAIMED
                : SCE_ANNOTATION_UNCLAIMED,
        };
    }

    /** What a state carries, for the node builder. */
    annotationForState(stateId) {
        return this.annotationFrom(this.requirementsForState(stateId));
    }

    /** What a transition carries, for the link builder. */
    annotationForTransition(stateId, index) {
        return this.annotationFrom(this.requirementsForTransition(stateId, index));
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
            Object.assign(state, this.annotationForState(state.id));
        }

        // Transitions are keyed by (source state, position in that state),
        // which is what the producer published beside the path.
        const seen = new Map();
        for (const transition of transitions || []) {
            const source = transition.source ?? transition.from;
            const index = seen.get(source) ?? 0;
            seen.set(source, index + 1);
            Object.assign(transition, this.annotationForTransition(source, index));
        }
        return { states, transitions };
    }
}

/**
 * The class suffix the renderer appends to a drawn element.
 *
 * ⭐ A function rather than an inline `d.annotationClass` in each of the
 * renderer's class builders, because there are several of them and the
 * first version of this feature was closed by a gate that grepped one of
 * them for the field name — while nothing anywhere WROTE it. One
 * function is what a probe can call, and what a probe can call is what
 * stops a dead read from reading as a live feature.
 *
 * Empty string, not `undefined`: the caller concatenates it.
 */
function annotationClassFor(d) {
    return d && d.annotationClass ? ` ${d.annotationClass}` : '';
}

/**
 * The `data-sce-req` attribute value, or `null` to omit the attribute.
 *
 * Null rather than an empty string, so an element claiming nothing
 * carries no attribute at all — an empty attribute on everything would
 * make the mark say nothing.
 */
function requirementIdsFor(d) {
    return d && d.requirements && d.requirements.length ? d.requirements.join(' ') : null;
}

/** The class the page sets while the annotation family is on display. */
const SCE_ANNOTATIONS_ON = 'sce-annotations-on';

/**
 * Put the overlay where the visualizer will look, and turn the styling on
 * only if there is something to style.
 *
 * ⭐ A function rather than a few lines inside the page's load path, and
 * the reason is the defect this whole row already produced once: logic
 * that lives where no probe can reach it gets certified by a grep. The
 * conditional below is exactly the kind that inverts silently — mark the
 * diagram when there is NO data and it asserts "nothing is claimed",
 * which is a different statement from "claims are unknown" and a false
 * one. So it is put somewhere it can be run.
 *
 * `container` may be null (the page may not have built it yet); a missing
 * container must not prevent the overlay from reaching the structure.
 */
function applyOverlayToPage(structure, overlay, container) {
    structure.annotationOverlay = overlay || null;
    if (overlay && container && container.classList) {
        container.classList.add(SCE_ANNOTATIONS_ON);
    }
    return structure;
}

/**
 * The page's whole wiring, as one call: fetch the overlay and apply it.
 *
 * Split from [`applyOverlayToPage`] so the part carrying a decision can
 * be exercised without a browser and without the WASM.
 */
async function attachAnnotationOverlay(structure, scxmlContent, container, wasmBase) {
    const overlay = await annotationOverlayFromWasm(
        scxmlContent,
        structure.name || 'diagram',
        wasmBase
    );
    return applyOverlayToPage(structure, overlay, container);
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

/**
 * Initialise the codegen WASM and build the overlay from it.
 *
 * ⚠ Returns `null` rather than throwing when the module cannot be
 * reached — a visualizer deployed without the codegen WASM must still
 * draw the diagram. The builders treat a null overlay as "no annotation
 * data", which leaves every element unmarked, and that is the honest
 * rendering: an UNMARKED diagram says nothing about claims, where a
 * diagram marking everything unclaimed would assert that nothing is
 * claimed. Those are different statements and only one of them is true.
 */
async function annotationOverlayFromWasm(scxmlContent, scxmlName, wasmBase) {
    const base = wasmBase || 'wasm/';
    try {
        const wasm = await import(`./${base}sce_build.js`);
        await wasm.default(`./${base}sce_build_bg.wasm`);
        return await loadAnnotationOverlay(wasm, scxmlContent, scxmlName);
    } catch (error) {
        if (typeof logger !== 'undefined' && logger.warn) {
            logger.warn(`annotation overlay unavailable, diagram drawn unmarked: ${error}`);
        }
        return null;
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        AnnotationOverlay,
        loadAnnotationOverlay,
        annotationOverlayFromWasm,
        applyOverlayToPage,
        attachAnnotationOverlay,
        annotationClassFor,
        requirementIdsFor,
        SCE_ANNOTATION_CLAIMED,
        SCE_ANNOTATION_UNCLAIMED,
        SCE_ANNOTATIONS_ON,
    };
}
