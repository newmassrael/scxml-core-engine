// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Link Builder - Handles link/transition creation and filtering
 */

class LinkBuilder {
    constructor(visualizer) {
        this.visualizer = visualizer;
    }

    buildLinks() {
        const links = [];

        // Initial transition
        if (this.visualizer.initialState) {
            links.push({
                id: `__initial___${this.visualizer.initialState}`,
                source: '__initial__',
                target: this.visualizer.initialState,
                linkType: 'initial'
            });
        }

        // Containment links
        this.visualizer.states.forEach(state => {
            if (state.children && state.children.length > 0) {
                state.children.forEach(childId => {
                    links.push({
                        id: `${state.id}_contains_${childId}`,
                        source: state.id,
                        target: childId,
                        linkType: 'containment'
                    });
                });
            }
        });

        // History delegation
        this.visualizer.states.forEach(state => {
            if (state.type === 'history' && state.defaultTarget) {
                links.push({
                    id: `${state.id}_delegates_${state.defaultTarget}`,
                    source: state.id,
                    target: state.defaultTarget,
                    linkType: 'delegation'
                });
            }
        });

        // Transition links
        this.visualizer.transitions.forEach(transition => {
            links.push({
                id: transition.id,
                source: transition.source,
                target: transition.target,
                event: transition.event,
                cond: transition.cond,
                linkType: 'transition',
                actions: transition.actions || [],
                guards: transition.guards || []
            });
        });

        return links;
    }

    getVisibleLinks(allLinks, nodes) {
        return allLinks.filter(link => {
            const sourceNode = nodes.find(n => n.id === link.source);
            const targetNode = nodes.find(n => n.id === link.target);

            if (!sourceNode || !targetNode) return false;

            // Hide containment/delegation
            if (link.linkType === 'containment' || link.linkType === 'delegation') {
                return false;
            }

            // Check hidden states
            const sourceHidden = this.visualizer.findCollapsedAncestor(link.source, nodes);
            const targetHidden = this.visualizer.findCollapsedAncestor(link.target, nodes);

            // Both hidden in same compound
            if (sourceHidden && targetHidden && sourceHidden.id === targetHidden.id) {
                return false;
            }

            // Source hidden, target is collapsed parent
            if (sourceHidden && link.target === sourceHidden.id) {
                return false;
            }

            return true;
        });
    }

    findCollapsedAncestor(nodeId, nodes) {
        // First check direct parent
        for (const parent of nodes) {
            if ((parent.type === 'compound' || parent.type === 'parallel') &&
                parent.collapsed &&
                parent.children &&
                parent.children.includes(nodeId)) {
                return parent;
            }
        }
        
        // Recursive check: check if parent has a collapsed ancestor
        for (const parent of nodes) {
            if ((parent.type === 'compound' || parent.type === 'parallel') &&
                parent.children &&
                parent.children.includes(nodeId)) {
                const grandparent = this.visualizer.findCollapsedAncestor(parent.id, nodes);
                if (grandparent) {
                    return grandparent; // Return the collapsed ancestor, not the parent
                }
            }
        }
        
        return null;
    }
}
