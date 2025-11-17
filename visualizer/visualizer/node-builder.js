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
                // Use ActionFormatter to get both main and detail lines
                const formatted = ActionFormatter.formatAction(action);

                // Measure main line
                if (formatted.main) {
                    const fullText = `↓ entry / ${formatted.main}`;
                    const metrics = ctx.measureText(fullText);

                    // Calculate required state width to fit this text
                    // Text rendering position calculation:
                    // - textX = -width/2 + (width * TEXT_LEFT_MARGIN_PERCENT) + TEXT_PADDING
                    // - textX = -width/2 + (width * 0.10) + 8 = -0.4*width + 8
                    // - Right boundary = width/2 - (width * 0.05) = 0.45*width (5% right margin)
                    // - Available text space = 0.45*width - (-0.4*width + 8) = 0.85*width - 8
                    // - Therefore: textWidth = 0.85*width - 8
                    // - Solving for width: width = (textWidth + 8) / 0.85

                    const textWidth = metrics.width;
                    const paddingLeft = 6;  // Background box padding
                    const paddingRight = 6;
                    const requiredWidth = (textWidth + paddingLeft + paddingRight + 20) / 0.85;

                    if (requiredWidth > maxWidth) {
                        maxWidth = requiredWidth;
                    }
                }

                // Measure detail lines (indented)
                if (formatted.details && formatted.details.length > 0) {
                    const detailCtx = canvas.getContext('2d');
                    detailCtx.font = '11px sans-serif'; // Detail lines use 11px font

                    formatted.details.forEach(detail => {
                        const detailText = detail; // Detail already includes "↳ " prefix
                        const detailMetrics = detailCtx.measureText(detailText);

                        // Detail lines are indented by 10px from main text position
                        // So effective text width is detail width + 10px indent
                        const textWidth = detailMetrics.width + 10;
                        const paddingLeft = 6;
                        const paddingRight = 6;
                        const requiredWidth = (textWidth + paddingLeft + paddingRight + 20) / 0.85;

                        if (requiredWidth > maxWidth) {
                            maxWidth = requiredWidth;
                        }
                    });
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

        // Precise height calculation based on actual content rendering
        // Matches renderer.js spacing exactly (lines 757-1544)

        const STATE_ID_HEIGHT = 26;        // Initial offset + state ID text (renderer.js:757)
        const SEPARATOR_SPACING = 36;      // 14 (before) + 2 (line) + 20 (after) (renderer.js:776-784)
        const MAIN_LINE_HEIGHT = 25;       // 15 (text) + 6 (padding) + 4 (spacing) (renderer.js:1527)
        const DETAIL_LINE_HEIGHT = 16;     // Detail line spacing (renderer.js:1542)
        const DETAIL_GROUP_SPACING = 8;    // Additional spacing after detail lines (renderer.js:1544)
        const BOTTOM_PADDING = 10;         // Bottom margin

        let totalHeight = STATE_ID_HEIGHT;

        // Count all action lines (main + details)
        let mainLines = 0;
        let detailLines = 0;
        let actionsWithDetails = 0;        // Actions that have detail lines

        (node.onentry || []).forEach(action => {
            const formatted = ActionFormatter.formatAction(action);
            mainLines += 1;
            detailLines += (formatted.details?.length || 0);
            if (formatted.details && formatted.details.length > 0) {
                actionsWithDetails += 1;
            }
        });

        (node.onexit || []).forEach(action => {
            const formatted = ActionFormatter.formatAction(action);
            mainLines += 1;
            detailLines += (formatted.details?.length || 0);
            if (formatted.details && formatted.details.length > 0) {
                actionsWithDetails += 1;
            }
        });

        // If has actions, add separator
        if (mainLines > 0) {
            totalHeight += SEPARATOR_SPACING;
        }

        // Add space for all lines
        totalHeight += (mainLines * MAIN_LINE_HEIGHT);
        totalHeight += (detailLines * DETAIL_LINE_HEIGHT);
        totalHeight += (actionsWithDetails * DETAIL_GROUP_SPACING);  // Additional spacing after each action with details
        totalHeight += BOTTOM_PADDING;

        return Math.max(totalHeight, LAYOUT_CONSTANTS.STATE_MIN_HEIGHT);
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
