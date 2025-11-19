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
                // W3C SCXML 3.6: Initial attribute for compound/parallel states
                initial: state.initial || '',
                // W3C SCXML 3.4: Parallel states expanded by default to show concurrent regions
                collapsed: (state.type === 'compound'),  // Only compound states collapsed, parallel expanded
                onentry: state.onentry || [],
                onexit: state.onexit || [],
                // W3C SCXML 6.4: Invoke metadata for child SCXML/service invocation
                // States can have multiple invoke elements (array)
                invokes: state.hasInvoke && state.invokes ? state.invokes.map(invoke => ({
                    hasInvoke: true,
                    invokeType: invoke.invokeType || null,
                    invokeTypeExpr: invoke.invokeTypeExpr || null,
                    invokeSrc: invoke.invokeSrc || null,
                    invokeSrcExpr: invoke.invokeSrcExpr || null,
                    invokeId: invoke.invokeId || null,
                    invokeIdLocation: invoke.invokeIdLocation || null,
                    invokeNamelist: invoke.invokeNamelist || null,
                    invokeAutoForward: invoke.invokeAutoForward || false,
                    invokeParams: invoke.invokeParams || [],
                    invokeContent: invoke.invokeContent || null,
                    invokeContentExpr: invoke.invokeContentExpr || null,
                    invokeFinalize: invoke.invokeFinalize || null
                })) : []
            };

            // Debug: Log children arrays for compound/parallel states
            if (SCXMLVisualizer.isCompoundOrParallel(state) && node.children.length > 0) {
                logger.debug(`[buildNodes] ${state.id} (${state.type}) has children: ${node.children.join(', ')}`);
            }

            nodes.push(node);
        });

        return nodes;
    }

    /**
     * W3C SCXML 3.6: Auto-expand ancestors of initial targets
     * 
     * When a compound state has an initial attribute targeting deeply nested states,
     * all ancestors on the path must be expanded to show the initial configuration.
     * 
     * Example: <state id="s2"><initial><transition target="s21p112 s21p122"/></initial></state>
     * - s2.initial = "s21p112 s21p122"
     * - Must expand: s21, s21p1, s21p11, s21p12 to show the initial targets
     * 
     * @param {Array} nodes - All nodes from buildNodes()
     */
    expandInitialPaths(nodes) {
        const nodeMap = new Map(nodes.map(n => [n.id, n]));

        // Performance optimization: Build parent map once (O(n) instead of O(n²))
        // Maps each child ID to its parent ID for fast ancestor lookup
        const parentMap = new Map();
        nodes.forEach(node => {
            if (node.children) {
                node.children.forEach(childId => {
                    parentMap.set(childId, node.id);
                });
            }
        });

        // Helper: Find all ancestors of a node (O(depth) instead of O(n²))
        // Walks up the parent chain from child to root
        const findAncestors = (nodeId) => {
            const ancestors = [];
            let currentId = nodeId;

            while (parentMap.has(currentId)) {
                const parentId = parentMap.get(currentId);
                ancestors.push(parentId);
                currentId = parentId;
            }

            return ancestors;
        };

        // Process each node with initial attribute
        nodes.forEach(node => {
            if (node.initial && node.initial.trim()) {
                // W3C SCXML 3.6: Parse space-separated initial targets
                const initialTargets = node.initial.trim().split(/\s+/);

                if (this.visualizer.debugMode) {
                    logger.debug(`[EXPAND INITIAL] ${node.id} has initial targets: ${initialTargets.join(', ')}`);
                }

                // Expand all ancestors of each initial target
                initialTargets.forEach(targetId => {
                    const ancestors = findAncestors(targetId);
                    ancestors.forEach(ancestorId => {
                        const ancestorNode = nodeMap.get(ancestorId);
                        if (ancestorNode && ancestorNode.collapsed) {
                            if (this.visualizer.debugMode) {
                                logger.debug(`  Expanding ancestor ${ancestorId} for initial target ${targetId}`);
                            }
                            ancestorNode.collapsed = false;
                        }
                    });
                });
            }
        });
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

        // Count invoke lines if present (W3C SCXML 6.4)
        // States can have multiple invoke elements
        if (node.invokes && node.invokes.length > 0) {
            node.invokes.forEach(invoke => {
                const formatted = InvokeFormatter.formatInvokeInfo(invoke);
                if (formatted.main) {
                    mainLines += 1;
                    detailLines += (formatted.details?.length || 0);
                    if (formatted.details && formatted.details.length > 0) {
                        actionsWithDetails += 1;
                    }
                }
            });
        }

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
        logger.debug('[getVisibleNodes] Checking visibility for all nodes...');
        const visibleIds = new Set();

        this.visualizer.nodes.forEach(node => {
            // Recursive check: node is hidden if ANY ancestor is collapsed
            const collapsedAncestor = this.visualizer.findCollapsedAncestor(node.id, this.visualizer.nodes);
            
            if (collapsedAncestor) {
                logger.debug(`  ${node.id}: HIDDEN (ancestor ${collapsedAncestor.id} collapsed)`);
            } else {
                visibleIds.add(node.id);
                logger.debug(`  ${node.id}: VISIBLE (type=${node.type})`);
            }
        });

        const result = this.visualizer.nodes.filter(n => visibleIds.has(n.id));
        logger.debug(`[getVisibleNodes] Result: ${result.map(n => n.id).join(', ')}`);
        
        // Debug: Show compound/parallel nodes and their children
        this.visualizer.nodes.filter(n => n.type === 'compound' || n.type === 'parallel').forEach(n => {
            logger.debug(`  ${n.id} (${n.type}, collapsed=${n.collapsed}): children=${n.children ? n.children.join(', ') : 'none'}`);
        });
        
        return result;
    }
}
