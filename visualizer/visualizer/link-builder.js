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
        if (this.visualizer.debugMode) {
            console.log(`[GET VISIBLE LINKS] Processing ${allLinks.length} links...`);
        }
        return allLinks.map(link => {
            // Hide containment/delegation
            if (link.linkType === 'containment' || link.linkType === 'delegation') {
                return null;
            }

            // Check hidden states
            const sourceHidden = this.findCollapsedAncestor(link.source, nodes);
            const targetHidden = this.findCollapsedAncestor(link.target, nodes);

            // Both hidden in same compound - completely hide
            if (sourceHidden && targetHidden && sourceHidden.id === targetHidden.id) {
                return null;
            }

            // Create modified link - keep original data, add visual redirect fields
            let modifiedLink = {
                ...link,
                originalLink: link,         // Store reference to original for routing sync
                visualSource: link.source,  // Default to original
                visualTarget: link.target   // Default to original
            };

            // Target hidden - visual redirect to collapsed ancestor (e.g., s1→p visually becomes s1→s2)
            if (targetHidden && !sourceHidden) {
                // Check if redirect would create self-loop (e.g., p→ps1 becomes p→p)
                if (link.source === targetHidden.id) {
                    if (this.visualizer.debugMode) {
                    console.log(`[VISUAL REDIRECT] ${link.source}→${link.target}: would create self-loop ${link.source}→${targetHidden.id}, hiding link`);
                }
                    return null;
                }
                if (this.visualizer.debugMode) {
                    console.log(`[VISUAL REDIRECT] ${link.source}→${link.target}: target hidden, redirect to ${targetHidden.id}`);
                }
                modifiedLink.visualTarget = targetHidden.id;
                modifiedLink.originalTarget = link.target;  // Store for auto-expand
            }

            // Source hidden - visual redirect to collapsed ancestor (e.g., p→fail visually becomes s2→fail)
            if (sourceHidden && !targetHidden) {
                // Check if redirect would create self-loop (e.g., s2→p becomes s2→s2)
                if (link.target === sourceHidden.id) {
                    if (this.visualizer.debugMode) {
                    console.log(`[VISUAL REDIRECT] ${link.source}→${link.target}: would create self-loop ${sourceHidden.id}→${link.target}, hiding link`);
                }
                    return null;
                }
                if (this.visualizer.debugMode) {
                    console.log(`[VISUAL REDIRECT] ${link.source}→${link.target}: source hidden, redirect from ${sourceHidden.id}`);
                }
                modifiedLink.visualSource = sourceHidden.id;
                modifiedLink.originalSource = link.source;
            }

            // Both hidden in different ancestors - redirect both ends
            if (sourceHidden && targetHidden && sourceHidden.id !== targetHidden.id) {
                if (this.visualizer.debugMode) {
                    console.log(`[VISUAL REDIRECT] ${link.source}→${link.target}: both hidden in different ancestors, redirecting ${sourceHidden.id}→${targetHidden.id}`);
                }
                modifiedLink.visualSource = sourceHidden.id;
                modifiedLink.visualTarget = targetHidden.id;
                modifiedLink.originalSource = link.source;
                modifiedLink.originalTarget = link.target;
            }

            // **FINAL VALIDATION: Check if visual nodes exist in visible nodes**
            const visualSourceNode = nodes.find(n => n.id === modifiedLink.visualSource);
            const visualTargetNode = nodes.find(n => n.id === modifiedLink.visualTarget);

            if (!visualSourceNode || !visualTargetNode) {
                console.warn(`[GET VISIBLE LINKS] Skipping ${link.source}→${link.target}: visual nodes not found (${modifiedLink.visualSource}, ${modifiedLink.visualTarget})`);
                return null;
            }

            return modifiedLink;
        }).filter(link => link !== null);
    }

    findCollapsedAncestor(nodeId, nodes) {
        let outermostCollapsed = null;
        
        // Find direct parent first
        const parent = nodes.find(n => 
            SCXMLVisualizer.isCompoundOrParallel(n) &&
            n.children &&
            n.children.includes(nodeId)
        );
        
        if (!parent) {
            return null;
        }
        
        // If parent is collapsed, remember it
        if (parent.collapsed) {
            outermostCollapsed = parent;
        }
        
        // Recursively check if parent has collapsed ancestors
        const grandparent = this.findCollapsedAncestor(parent.id, nodes);
        if (grandparent) {
            outermostCollapsed = grandparent;  // Prefer outermost
        }
        
        return outermostCollapsed;
    }
}
