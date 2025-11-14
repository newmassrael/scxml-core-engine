// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Breadcrumb Manager - Handles breadcrumb navigation for sub-SCXML
 */

class BreadcrumbManager {
    constructor(controller) {
        this.controller = controller;
    }

    extractSubSCXMLInfo(structure) {
        const subSCXMLs = [];
        
        console.log('[extractSubSCXMLInfo] Analyzing structure:', structure);
        
        // Get all sub-SCXMLs from runner (includes nested children)
        const allSubSCXMLs = this.controller.runner.getSubSCXMLStructures ? this.controller.runner.getSubSCXMLStructures() : [];
        console.log('[extractSubSCXMLInfo] All sub-SCXMLs from runner:', allSubSCXMLs);
        
        // Recursively find states with invoke in current structure
        const findInvokeStates = (states, parentPath = '') => {
            for (const state of states) {
                const statePath = parentPath ? `${parentPath}.${state.id}` : state.id;
                
                if (state.hasInvoke && state.invokeSrc) {
                    console.log(`[extractSubSCXMLInfo] Found invoke state: ${state.id}, src: ${state.invokeSrc}`);
                    
                    // Find matching sub-SCXML from runner's list
                    const matchingSubSCXML = allSubSCXMLs.find(sub => 
                        sub.parentStateId === state.id || 
                        sub.srcPath === state.invokeSrc
                    );
                    
                    if (matchingSubSCXML) {
                        subSCXMLs.push(matchingSubSCXML);
                    } else {
                        // Store metadata even if structure not available
                        subSCXMLs.push({
                            parentStateId: state.id,
                            srcPath: state.invokeSrc,
                            invokeId: state.invokeId || state.invokeSrc,
                            structure: null  // Structure not available
                        });
                    }
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
        
        console.log(`Extracted ${subSCXMLs.length} sub-SCXML(s) from structure`);
        return subSCXMLs;
    }

    updateBreadcrumb() {
        const breadcrumbContainer = document.getElementById('breadcrumb-container');
        if (!breadcrumbContainer) {
            console.warn('breadcrumb-container not found');
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
                return `<span class="breadcrumb-item active">${this.controller.escapeHtml(label)}</span>`;
            } else {
                // Clickable parent items
                return `<a href="#" class="breadcrumb-item" data-depth="${i}">${this.controller.escapeHtml(label)}</a>`;
            }
        }).join(' <span class="breadcrumb-separator">›</span> ');

        breadcrumbContainer.innerHTML = breadcrumbHTML;

        // Add click handlers for breadcrumb navigation
        breadcrumbContainer.querySelectorAll('a.breadcrumb-item').forEach(item => {
            item.addEventListener('click', async (e) => {
                e.preventDefault();
                const depth = parseInt(e.target.dataset.depth);
                await this.controller.navigateToDepth(depth);
            });
        });

        console.log(`Breadcrumb updated: ${path.join(' > ')}`);
    }

    async handleStateNavigation(stateId, invokeSrc, invokeSrcExpr, invokeId) {
        console.log(`[handleStateNavigation] Handling navigation from state: ${stateId}`);
        console.log(`[handleStateNavigation] Current machine:`, this.controller.currentMachine);

        const subSCXMLs = this.controller.currentMachine.subSCXMLs || [];
        console.log(`[handleStateNavigation] Available subSCXMLs:`, subSCXMLs);

        subSCXMLs.forEach((sub, idx) => {
            console.log(`  [${idx}] parentStateId: "${sub.parentStateId}", srcPath: "${sub.srcPath}"`);
        });

        console.log(`[handleStateNavigation] Looking for parentStateId: "${stateId}"`);
        const childInfo = subSCXMLs.find(info => info.parentStateId === stateId);

        if (childInfo) {
            console.log(`Found static child SCXML: ${childInfo.srcPath}`);
            const childStructure = childInfo.structure;

            if (childStructure) {
                await this.navigateToChild(stateId, childStructure, childInfo);
            } else {
                console.error(`Child structure not available for ${childInfo.srcPath}`);
                alert(`Cannot navigate to child SCXML: ${childInfo.srcPath}\n\nThe child SCXML structure was not loaded.`);
            }
        } else {
            console.warn(`No child SCXML found for state ${stateId}`);

            const stateHasInvoke = invokeSrc || invokeSrcExpr ||
                this.controller.currentMachine.structure?.states?.find(s => s.id === stateId && s.hasInvoke);

            if (invokeSrcExpr) {
                console.warn(`Dynamic invoke (srcexpr) not yet supported: ${invokeSrcExpr}`);
                alert(`Dynamic invoke not supported for navigation\n\nState "${stateId}" uses srcexpr attribute.`);
            } else if (stateHasInvoke) {
                alert(`No sub-SCXML found for state "${stateId}"\n\nThis state has an invoke element, but it was not loaded.`);
            } else {
                alert(`No sub-SCXML found for state "${stateId}"`);
            }
        }
    }

    async navigateToChild(stateId, childStructure, childInfo = null) {
        console.log(`Navigating to child state machine from state: ${stateId}`);

        // Push current machine to stack
        this.controller.navigationStack.push({
            id: this.controller.currentMachine.id,
            label: this.controller.currentMachine.label,
            structure: this.controller.currentMachine.structure,
            visualizer: this.controller.currentMachine.visualizer,
            subSCXMLs: this.controller.currentMachine.subSCXMLs
        });

        // Create new visualizer for child
        const containerId = 'state-diagram-single';
        const container = document.getElementById(containerId);
        if (!container) {
            console.error('state-diagram-single container not found');
            return;
        }

        container.innerHTML = '';
        const childVisualizer = new SCXMLVisualizer(containerId, childStructure);
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

        this.updateBreadcrumb();

        const backButton = document.getElementById('btn-back');
        if (backButton) {
            backButton.style.display = 'block';
        }

        await this.controller.updateState();
        console.log(`Navigation complete. Stack depth: ${this.controller.navigationStack.length}`);
    }

    async navigateBack() {
        if (this.controller.navigationStack.length === 0) {
            console.log('Already at root level');
            return;
        }

        console.log('Navigating back to parent');
        const parent = this.controller.navigationStack.pop();

        const containerId = 'state-diagram-single';
        const container = document.getElementById(containerId);
        if (!container) {
            console.error('state-diagram-single container not found');
            return;
        }

        container.innerHTML = '';
        const parentVisualizer = new SCXMLVisualizer(containerId, parent.structure);

        this.controller.currentMachine = {
            id: parent.id,
            label: parent.label,
            structure: parent.structure,
            visualizer: parentVisualizer,
            subSCXMLs: parent.subSCXMLs
        };

        this.updateBreadcrumb();

        if (this.controller.navigationStack.length === 0) {
            const backButton = document.getElementById('btn-back');
            if (backButton) {
                backButton.style.display = 'none';
            }
        }

        await this.controller.updateState();
        console.log(`Navigation back complete. Stack depth: ${this.controller.navigationStack.length}`);
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
