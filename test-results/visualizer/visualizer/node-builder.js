// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Node Builder - Handles node creation and sizing
 */

class NodeBuilder {
    constructor(visualizer) {
        this.visualizer = visualizer;
    }

    buildNodes() {
        const nodes = [];

        // Add initial pseudo-node
        if (this.visualizer.initialState) {
            nodes.push({
                id: '__initial__',
                type: 'initial-pseudo',
                label: '',
                children: [],
                collapsed: false
            });
        }

        this.visualizer.states.forEach(state => {
            const node = {
                id: state.id,
                type: state.type,
                label: state.id,
                children: state.children || [],
                // W3C SCXML 3.4: Parallel states expanded by default to show concurrent regions
                collapsed: (state.type === 'compound'),  // Only compound states collapsed, parallel expanded
                onentry: state.onentry || [],
                onexit: state.onexit || [],
                // W3C SCXML 6.3: Invoke metadata for child SCXML navigation
                hasInvoke: state.hasInvoke || false,
                invokeSrc: state.invokeSrc || null,
                invokeSrcExpr: state.invokeSrcExpr || null,
                invokeId: state.invokeId || null
            };

            // Debug: Log children arrays for compound/parallel states
            if (SCXMLVisualizer.isCompoundOrParallel(state) && node.children.length > 0) {
                console.log(`[buildNodes] ${state.id} (${state.type}) has children: ${node.children.join(', ')}`);
            }

            nodes.push(node);
        });

        return nodes;
    }

    getNodeWidth(node) {
        if (node.type === 'initial-pseudo') return 20;
        if (SCXMLVisualizer.isCompoundOrParallel(node)) {
            return node.collapsed ? LAYOUT_CONSTANTS.STATE_MIN_WIDTH : 300;
        }

        // Calculate width based on text content (generous estimates for ELK layout)
        let maxWidth = LAYOUT_CONSTANTS.STATE_MIN_WIDTH;

        // State ID length
        const idWidth = (node.id || '').length * 10 + 60;
        if (idWidth > maxWidth) maxWidth = idWidth;

        // Check onentry/onexit actions for text length (use canvas measurement for accuracy)
        const actions = [...(node.onentry || []), ...(node.onexit || [])];
        if (actions.length > 0) {
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            ctx.font = '13px sans-serif';

            actions.forEach(action => {
                const text = this.visualizer.formatActionText(action);
                if (text) {
                    // "↓ entry / " = ~10 chars + action text
                    const fullText = `↓ entry / ${text}`;
                    const metrics = ctx.measureText(fullText);
                    const estimatedWidth = metrics.width + 80; // Add padding for margins, box, and scrollbar
                    if (estimatedWidth > maxWidth) {
                        maxWidth = estimatedWidth;
                    }
                }
            });
        }


        return Math.min(maxWidth, LAYOUT_CONSTANTS.STATE_MAX_WIDTH);
    }

    getNodeHeight(node) {
        if (node.type === 'initial-pseudo') return 20;
        if (SCXMLVisualizer.isCompoundOrParallel(node)) {
            return node.collapsed ? 50 : 200;
        }

        // Calculate height based on number of actions
        const entryActions = (node.onentry || []).length;
        const exitActions = (node.onexit || []).length;
        const totalActions = entryActions + exitActions;

        if (totalActions === 0) return LAYOUT_CONSTANTS.STATE_MIN_HEIGHT;

        return LAYOUT_CONSTANTS.STATE_BASE_HEIGHT + (totalActions * LAYOUT_CONSTANTS.ACTION_HEIGHT);
    }

    getVisibleNodes() {
        console.log('[getVisibleNodes] Checking visibility for all nodes...');
        const visibleIds = new Set();

        this.visualizer.nodes.forEach(node => {
            // Recursive check: node is hidden if ANY ancestor is collapsed
            const collapsedAncestor = this.visualizer.findCollapsedAncestor(node.id, this.visualizer.nodes);
            
            if (collapsedAncestor) {
                console.log(`  ${node.id}: HIDDEN (ancestor ${collapsedAncestor.id} collapsed)`);
            } else {
                visibleIds.add(node.id);
                console.log(`  ${node.id}: VISIBLE (type=${node.type})`);
            }
        });

        const result = this.visualizer.nodes.filter(n => visibleIds.has(n.id));
        console.log(`[getVisibleNodes] Result: ${result.map(n => n.id).join(', ')}`);
        
        // Debug: Show compound/parallel nodes and their children
        this.visualizer.nodes.filter(n => n.type === 'compound' || n.type === 'parallel').forEach(n => {
            console.log(`  ${n.id} (${n.type}, collapsed=${n.collapsed}): children=${n.children ? n.children.join(', ') : 'none'}`);
        });
        
        return result;
    }
}
