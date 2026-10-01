// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Breadcrumb Manager - Handles breadcrumb navigation for sub-SCXML
 */

// DOM Element IDs
const DOM_IDS = {
    STATE_DIAGRAM_CONTAINER: 'state-diagram-single',
    BREADCRUMB_CONTAINER: 'breadcrumb-container',
    BACK_BUTTON: 'btn-back'
};

// CSS Classes
const CSS_CLASSES = {
    BREADCRUMB_ITEM: 'breadcrumb-item',
    BREADCRUMB_SEPARATOR: 'breadcrumb-separator',
    ACTIVE: 'active'
};

// Display Styles
const DISPLAY_STYLES = {
    BLOCK: 'block',
    NONE: 'none'
};

// UI Constants
const UI_CONSTANTS = {
    BREADCRUMB_SEPARATOR_CHAR: '›',
    BREADCRUMB_DEPTH_ATTR: 'data-depth'
};

class BreadcrumbManager {
    constructor(controller) {
        this.controller = controller;
    }

    /**
     * Helper: Create and initialize a new visualizer instance
     * @param {Object} structure - SCXML structure
     * @returns {Promise<SCXMLVisualizer|null>} Initialized visualizer or null if container not found
     */
    async _createVisualizer(structure) {
        const containerId = DOM_IDS.STATE_DIAGRAM_CONTAINER;
        const container = document.getElementById(containerId);
        
        if (!container) {
            console.error(`${containerId} container not found`);
            return null;
        }

        // Clear container
        container.innerHTML = '';
        
        // Create new visualizer
        const visualizer = new SCXMLVisualizer(containerId, structure);
        
        // Wait for initialization to complete
        if (visualizer.initPromise) {
            await visualizer.initPromise;
        } else {
            logger.warn('Visualizer has no initPromise, assuming synchronous initialization');
        }
        
        return visualizer;
    }

    /**
     * Helper: Cleanup previous visualizer to prevent memory leaks
     * @param {SCXMLVisualizer} visualizer - Visualizer to cleanup
     */
    _cleanupVisualizer(visualizer) {
        if (!visualizer) return;
        
        // Check if visualizer has destroy method
        if (typeof visualizer.destroy === 'function') {
            visualizer.destroy();
        }
        
        // Note: D3 selections and event listeners should be cleaned up by visualizer.destroy()
        // If destroy() doesn't exist, we rely on garbage collection
    }

    extractSubSCXMLInfo(structure) {
        const subSCXMLs = [];
        
        logger.debug('[extractSubSCXMLInfo] Analyzing structure:', structure);
        
        // Get all sub-SCXMLs from runner (includes nested children)
        const allSubSCXMLs = this.controller.runner.getSubSCXMLStructures ? this.controller.runner.getSubSCXMLStructures() : [];
        logger.debug('[extractSubSCXMLInfo] All sub-SCXMLs from runner:', allSubSCXMLs);
        
        // Recursively find states with invoke in current structure
        const findInvokeStates = (states, parentPath = '') => {
            for (const state of states) {
                const statePath = parentPath ? `${parentPath}.${state.id}` : state.id;

                // W3C SCXML 6.4: States can have multiple invoke elements
                if (state.hasInvoke && state.invokes && state.invokes.length > 0) {
                    state.invokes.forEach(invoke => {
                        if (invoke.invokeSrc || invoke.invokeSrcExpr) {
                            logger.debug(`[extractSubSCXMLInfo] Found invoke in state: ${state.id}, src: ${invoke.invokeSrc}, srcexpr: ${invoke.invokeSrcExpr}`);

                            // W3C SCXML 6.4: Handle both static src and dynamic srcexpr
                            const srcPath = invoke.invokeSrc || invoke.invokeSrcExpr;

                            // Find matching sub-SCXML from runner's list
                            // Helper: Extract filename from path for comparison
                            const getFilename = (path) => path?.replace(/^file:/, '').split('/').pop() || '';

                            const matchingSubSCXML = allSubSCXMLs.find(sub => {
                                if (sub.parentStateId === state.id) return true;
                                if (sub.srcPath === srcPath) return true;

                                // For srcexpr, compare filenames (not substring match to avoid false positives)
                                if (invoke.invokeSrcExpr && sub.srcPath) {
                                    const subFilename = getFilename(sub.srcPath);
                                    const exprFilename = getFilename(invoke.invokeSrcExpr);
                                    return subFilename === exprFilename;
                                }

                                return false;
                            });

                            if (matchingSubSCXML) {
                                subSCXMLs.push(matchingSubSCXML);
                            } else {
                                // Store metadata even if structure not available
                                // For srcexpr, we'll need to evaluate at navigation time
                                subSCXMLs.push({
                                    parentStateId: state.id,
                                    srcPath: invoke.invokeSrc,
                                    srcExpr: invoke.invokeSrcExpr,  // Store srcexpr for later evaluation
                                    invokeId: invoke.invokeId || srcPath,
                                    structure: null  // Structure not available (will be resolved dynamically)
                                });
                            }
                        }
                    });
                }
                
                // Recursively check children
                if (state.children && state.children.length > 0) {
                    findInvokeStates(state.children, statePath);
                }
            }
        };
        
        if (structure.states) {
            findInvokeStates(structure.states);
        }
        
        logger.debug(`Extracted ${subSCXMLs.length} sub-SCXML(s) from structure`);
        return subSCXMLs;
    }

    updateBreadcrumb() {
        const breadcrumbContainer = document.getElementById(DOM_IDS.BREADCRUMB_CONTAINER);
        if (!breadcrumbContainer) {
            logger.warn(`${DOM_IDS.BREADCRUMB_CONTAINER} not found`);
            return;
        }

        // Build breadcrumb path: [stack items] + current
        const path = [
            ...this.controller.navigationStack.map(m => m.label),
            this.controller.currentMachine.label
        ];

        // Render breadcrumb with separators
        const breadcrumbHTML = path.map((label, i) => {
            const isLast = (i === path.length - 1);
            if (isLast) {
                // Current (active) item
                return `<span class="${CSS_CLASSES.BREADCRUMB_ITEM} ${CSS_CLASSES.ACTIVE}">${this.controller.escapeHtml(label)}</span>`;
            } else {
                // Clickable parent items
                return `<a href="#" class="${CSS_CLASSES.BREADCRUMB_ITEM}" ${UI_CONSTANTS.BREADCRUMB_DEPTH_ATTR}="${i}">${this.controller.escapeHtml(label)}</a>`;
            }
        }).join(` <span class="${CSS_CLASSES.BREADCRUMB_SEPARATOR}">${UI_CONSTANTS.BREADCRUMB_SEPARATOR_CHAR}</span> `);

        breadcrumbContainer.innerHTML = breadcrumbHTML;

        // Add click handlers for breadcrumb navigation
        breadcrumbContainer.querySelectorAll(`a.${CSS_CLASSES.BREADCRUMB_ITEM}`).forEach(item => {
            item.addEventListener('click', async (e) => {
                e.preventDefault();
                const depth = parseInt(e.target.dataset.depth);
                await this.controller.navigateToDepth(depth);
            });
        });

        logger.debug(`Breadcrumb updated: ${path.join(' > ')}`);
    }

    async handleStateNavigation(stateId, invokeSrc, invokeSrcExpr, invokeId) {
        logger.debug(`[handleStateNavigation] Handling navigation from state: ${stateId}`);
        logger.debug(`[handleStateNavigation] Current machine:`, this.controller.currentMachine);

        const subSCXMLs = this.controller.currentMachine.subSCXMLs || [];
        logger.debug(`[handleStateNavigation] Available subSCXMLs:`, subSCXMLs);

        subSCXMLs.forEach((sub, idx) => {
            logger.debug(`  [${idx}] parentStateId: "${sub.parentStateId}", srcPath: "${sub.srcPath}"`);
        });

        logger.debug(`[handleStateNavigation] Looking for parentStateId: "${stateId}"`);
        const childInfo = subSCXMLs.find(info => info.parentStateId === stateId);

        if (childInfo) {
            logger.debug(`Found static child SCXML: ${childInfo.srcPath}`);
            // The engine says WHICH child this is and where its file is; the
            // structure is built from the Rust model, as the parent's was.
            let childStructure = null;
            try {
                childStructure = await this._childStructure(childInfo.srcPath);
            } catch (error) {
                console.error(`Child structure not available for ${childInfo.srcPath}:`, error);
                alert(`Cannot navigate to child SCXML: ${childInfo.srcPath}\n\n${error}`);
                return;
            }
            await this.navigateToChild(stateId, childStructure, { ...childInfo, structure: childStructure });
        } else {
            logger.warn(`No child SCXML found for state ${stateId}`);

            const stateHasInvoke = invokeSrc || invokeSrcExpr ||
                this.controller.currentMachine.structure?.states?.find(s => s.id === stateId && s.hasInvoke);

            if (invokeSrcExpr) {
                // W3C SCXML 6.4: Evaluate srcexpr using native JSEngine
                try {
                    logger.debug(`[handleStateNavigation] Dynamic invoke detected: ${invokeSrcExpr}`);

                    // Zero Duplication: Use native JSEngine::evaluateExpression() as Single Source of Truth
                    // Supports full ECMAScript expressions: "pathVar", "'file:' + pathVar", etc.
                    const evaluatedSrc = this.controller.runner.evaluateExpression(invokeSrcExpr);

                    if (!evaluatedSrc) {
                        logger.warn(`Dynamic invoke: Expression evaluation returned empty result`);
                        alert(`Dynamic invoke failed\n\nExpression: ${invokeSrcExpr}\nEvaluation returned empty result`);
                        return;
                    }

                    logger.debug(`[handleStateNavigation] Expression "${invokeSrcExpr}" evaluated to: ${evaluatedSrc}`);

                    // Remove 'file:' prefix if present
                    const cleanPath = evaluatedSrc.replace(/^file:/, '');
                    logger.debug(`[handleStateNavigation] Clean path: ${cleanPath}`);

                    // Refresh child structures from runner (invoke may have loaded child at runtime)
                    logger.debug(`[handleStateNavigation] Refreshing child structures from runner...`);
                    const runtimeSubSCXMLs = this.controller.runner.getSubSCXMLStructures ?
                        this.controller.runner.getSubSCXMLStructures() : [];
                    logger.debug(`[handleStateNavigation] Runtime subSCXMLs:`, runtimeSubSCXMLs);

                    // Merge with existing subSCXMLs
                    const allSubSCXMLs = [...subSCXMLs, ...runtimeSubSCXMLs];

                    // Try to find child SCXML with evaluated path
                    const dynamicChildInfo = allSubSCXMLs.find(info => {
                        const infoCleanPath = (info.srcPath || '').replace(/^file:/, '');
                        return infoCleanPath === cleanPath ||
                               info.srcPath === evaluatedSrc ||
                               info.parentStateId === stateId;
                    });

                    if (dynamicChildInfo && dynamicChildInfo.srcPath) {
                        logger.debug(`[handleStateNavigation] Found dynamic child SCXML: ${dynamicChildInfo.srcPath}`);
                        const childStructure = await this._childStructure(dynamicChildInfo.srcPath);
                        await this.navigateToChild(stateId, childStructure,
                            { ...dynamicChildInfo, structure: childStructure });
                        return;
                    }

                    // W3C SCXML 6.4: Child not found in cached structures, try to parse it directly
                    logger.debug(`[handleStateNavigation] Attempting to parse child SCXML directly from file: ${cleanPath}`);

                    try {
                        // Read the child file the engine can see, and build its
                        // structure from the Rust model — no second engine
                        // instance just to read a document.
                        const childStructure = await this._childStructure(cleanPath);
                        await this.navigateToChild(stateId, childStructure, {
                            parentStateId: stateId,
                            srcPath: evaluatedSrc,
                            structure: childStructure
                        });
                        return;
                    } catch (parseError) {
                        console.error(`[handleStateNavigation] Failed to parse child SCXML:`, parseError);
                        logger.warn(`[handleStateNavigation] Available children:`, allSubSCXMLs);
                        alert(`Dynamic invoke failed\n\nChild SCXML could not be loaded: ${evaluatedSrc}\n\nError: ${parseError.message}\n\nCheck console for details.`);
                    }
                } catch (e) {
                    console.error(`Error evaluating srcexpr ${invokeSrcExpr}:`, e);
                    alert(`Error evaluating dynamic invoke\n\nExpression: ${invokeSrcExpr}\nError: ${e.message}`);
                }
            } else if (stateHasInvoke) {
                alert(`No sub-SCXML found for state "${stateId}"\n\nThis state has an invoke element, but it was not loaded.`);
            } else {
                alert(`No sub-SCXML found for state "${stateId}"`);
            }
        }
    }

    /**
     * The text of a child document the engine resolved, read from its
     * virtual filesystem: under the runner's base path when it has one,
     * otherwise at the first of the usual places that holds it.
     */
    _readChildFile(srcPath) {
        if (typeof window.Module === 'undefined') {
            throw new Error('WASM Module not available');
        }
        const Module = window.Module;
        const cleanPath = (srcPath || '').replace(/^file:/, '');

        let virtualPath = null;
        if (this.controller.runner.getBasePath) {
            virtualPath = `${this.controller.runner.getBasePath()}${cleanPath}`;
        } else {
            const searchPaths = [`/resources/${cleanPath}`, `/${cleanPath}`];
            const params = new URLSearchParams(window.location.hash.substring(1));
            const testId = params.get('test');
            if (testId) {
                searchPaths.unshift(`/resources/${testId}/${cleanPath}`);
            }
            for (const path of searchPaths) {
                try {
                    Module.FS.stat(path);
                    virtualPath = path;
                    break;
                } catch (e) {
                    // Not at this path; try the next.
                }
            }
            if (!virtualPath) {
                throw new Error(`File not found in virtual FS: ${cleanPath}`);
            }
        }
        logger.debug(`[_readChildFile] Reading virtual file: ${virtualPath}`);
        return Module.FS.readFile(virtualPath, { encoding: 'utf8' });
    }

    /**
     * A child document's structure, built from the Rust model like the
     * parent's (`SceBuildWasm.guiStructure`) — never from the engine that
     * runs it, so a child is drawn from the same reading as its parent.
     */
    async _childStructure(srcPath) {
        return SceBuildWasm.guiStructure(this._readChildFile(srcPath), srcPath);
    }

    async navigateToChild(stateId, childStructure, childInfo = null) {
        logger.debug(`Navigating to child state machine from state: ${stateId}`);

        // Save current node positions for layout restoration
        // Note: Only save x, y positions. Width and height will be recalculated
        // to ensure text content fits properly (e.g., entry/exit actions)
        const savedNodePositions = this.controller.currentMachine.visualizer?.nodes?.map(node => ({
            id: node.id,
            x: node.x,
            y: node.y
        })) || [];

        // Push current machine to stack
        this.controller.navigationStack.push({
            id: this.controller.currentMachine.id,
            label: this.controller.currentMachine.label,
            structure: this.controller.currentMachine.structure,
            visualizer: this.controller.currentMachine.visualizer,
            subSCXMLs: this.controller.currentMachine.subSCXMLs,
            nodePositions: savedNodePositions  // Save layout state
        });

        // Cleanup previous visualizer to prevent memory leaks
        this._cleanupVisualizer(this.controller.visualizer);

        // Create new visualizer for child
        const childVisualizer = await this._createVisualizer(childStructure);
        if (!childVisualizer) {
            console.error('Failed to create child visualizer');
            return;
        }
        
        const childSubSCXMLs = this.extractSubSCXMLInfo(childStructure);

        // Generate label
        let label = `Child of ${stateId}`;
        if (childInfo && childInfo.srcPath) {
            if (childInfo.srcPath.startsWith('inline-content:')) {
                label = `${stateId} (inline content)`;
            } else {
                const filename = childInfo.srcPath.split('/').pop();
                label = `${stateId} (${filename})`;
            }
        }

        // Update current machine
        this.controller.currentMachine = {
            id: stateId,
            label: label,
            structure: childStructure,
            visualizer: childVisualizer,
            subSCXMLs: childSubSCXMLs
        };

        // CRITICAL: Update controller.visualizer to point to current active visualizer
        // ui-updater.js uses controller.visualizer.highlightActiveStates()
        this.controller.visualizer = childVisualizer;

        this.updateBreadcrumb();

        const backButton = document.getElementById(DOM_IDS.BACK_BUTTON);
        if (backButton) {
            backButton.style.display = DISPLAY_STYLES.BLOCK;
        }

        await this.controller.updateState();
        logger.debug(`Navigation complete. Stack depth: ${this.controller.navigationStack.length}`);
    }

    async navigateBack() {
        if (this.controller.navigationStack.length === 0) {
            logger.debug('Already at root level');
            return;
        }

        logger.debug('Navigating back to parent');
        const parent = this.controller.navigationStack.pop();

        // Cleanup current visualizer to prevent memory leaks
        this._cleanupVisualizer(this.controller.visualizer);

        // Create new visualizer for parent
        const parentVisualizer = await this._createVisualizer(parent.structure);
        if (!parentVisualizer) {
            console.error('Failed to create parent visualizer');
            // Restore stack state
            this.controller.navigationStack.push(parent);
            return;
        }

        // Restore saved node positions to preserve layout
        if (parent.nodePositions && parent.nodePositions.length > 0) {
            logger.debug(`Restoring ${parent.nodePositions.length} node positions from saved state`);

            parent.nodePositions.forEach(savedPos => {
                const node = parentVisualizer.nodes.find(n => n.id === savedPos.id);
                if (node) {
                    node.x = savedPos.x;
                    node.y = savedPos.y;
                    // Do NOT restore width and height - they should be recalculated
                    // to reflect the current text content (e.g., after action formatting changes)
                    // node.width = savedPos.width;
                    // node.height = savedPos.height;
                }
            });

            // Recalculate compound bounds based on restored child positions
            logger.debug('Recalculating compound bounds to fit restored child positions...');
            parentVisualizer.nodes.forEach(node => {
                if (SCXMLVisualizer.isCompoundOrParallel(node)) {
                    parentVisualizer.updateCompoundBounds(node);
                }
            });

            // Re-optimize transition snap points with restored node positions
            logger.debug('Re-optimizing transition layout with restored positions...');
            parentVisualizer.layoutOptimizer.optimizeSnapPointAssignments(
                parentVisualizer.allLinks,
                parentVisualizer.nodes
            );

            // Re-render with restored positions and updated transitions
            parentVisualizer.render();
        }

        this.controller.currentMachine = {
            id: parent.id,
            label: parent.label,
            structure: parent.structure,
            visualizer: parentVisualizer,
            subSCXMLs: parent.subSCXMLs
        };

        // CRITICAL: Update controller.visualizer to point to current active visualizer
        // ui-updater.js uses controller.visualizer.highlightActiveStates()
        this.controller.visualizer = parentVisualizer;

        this.updateBreadcrumb();

        if (this.controller.navigationStack.length === 0) {
            const backButton = document.getElementById(DOM_IDS.BACK_BUTTON);
            if (backButton) {
                backButton.style.display = DISPLAY_STYLES.NONE;
            }
        }

        await this.controller.updateState();
        logger.debug(`Navigation back complete. Stack depth: ${this.controller.navigationStack.length}`);
    }

    async navigateToDepth(depth) {
        if (depth < 0 || depth > this.controller.navigationStack.length) {
            console.error(`Invalid depth: ${depth}`);
            return;
        }

        const stepsBack = this.controller.navigationStack.length - depth;
        for (let i = 0; i < stepsBack; i++) {
            await this.navigateBack();
        }
    }
}
