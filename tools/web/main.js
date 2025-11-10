// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Simplified main.js - Engine handles all invoke logic automatically
 */

/**
 * Parse URL hash parameters
 */
function parseHashParams() {
    const params = {};
    const hash = window.location.hash.substring(1);

    hash.split('&').forEach(param => {
        const [key, value] = param.split('=');
        if (key && value) {
            params[key] = decodeURIComponent(value);
        }
    });

    return params;
}

/**
 * Load SCXML content from various sources
 */
async function loadSCXMLContent() {
    const params = parseHashParams();

    // Source 1: W3C test number (#test=226)
    if (params.test) {
        const testId = params.test;

        // Auto-detect environment: GitHub Pages vs Local Development
        // GitHub Pages: hostname contains 'github.io'
        // Local Dev: localhost or 127.0.0.1
        const isGitHubPages = window.location.hostname.includes('github.io');
        const isLocalhost = window.location.hostname === 'localhost' ||
                           window.location.hostname === '127.0.0.1' ||
                           window.location.hostname === '';

        // Path resolution:
        // - GitHub Pages: ../resources (served from docs/)
        // - Localhost: resources (symlink in tools/web/ → ../../resources)
        // - Project root server: ../../resources (fallback)
        const resourcesPrefix = isGitHubPages ? '../resources' :
                               isLocalhost ? 'resources' :
                               '../../resources';

        const url = `${resourcesPrefix}/${testId}/test${testId}.scxml`;
        const environment = isGitHubPages ? 'GitHub Pages' : isLocalhost ? 'Localhost' : 'Local Dev';
        console.log(`Loading W3C test ${testId} from ${url} (${environment})`);

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Failed to fetch test ${testId}: ${response.statusText}`);
        }
        return await response.text();
    }

    // Source 2: Base64 encoded SCXML (#scxml=<base64>)
    if (params.scxml) {
        console.log('Loading SCXML from base64');
        return atob(params.scxml);
    }

    // Source 3: External URL (#url=<url>)
    if (params.url) {
        console.log(`Loading SCXML from URL: ${params.url}`);
        const response = await fetch(params.url);
        if (!response.ok) {
            throw new Error(`Failed to fetch ${params.url}: ${response.statusText}`);
        }
        return await response.text();
    }

    throw new Error('No SCXML source specified. Use #test=144, #scxml=<base64>, or #url=<url>');
}

/**
 * Initialize visualizer (simplified - engine handles invoke automatically)
 */
async function initVisualizer(scxmlContent) {
    showLoading(true);

    try {
        // Load WASM module
        console.log('Loading WASM module...');
        const Module = await createVisualizer();
        console.log('WASM module loaded');

        // Setup Emscripten virtual file system for invoke resolution
        const params = parseHashParams();
        if (params.test) {
            const testId = params.test;

            // Auto-detect environment for sub-SCXML fetch
            const isGitHubPages = window.location.hostname.includes('github.io');
            const basePath = isGitHubPages
                ? `../resources/${testId}/`
                : `../../resources/${testId}/`;
            
            // Create directory in virtual FS
            try {
                Module.FS.mkdir('/resources');
            } catch (e) {
                // Directory might already exist
            }
            
            try {
                Module.FS.mkdir(`/resources/${testId}`);
            } catch (e) {
                // Directory might already exist
            }
            
            // Write parent SCXML to virtual FS
            Module.FS.writeFile(`/resources/${testId}/test${testId}.scxml`, scxmlContent);
            console.log(`Created virtual FS: /resources/${testId}/test${testId}.scxml`);

            // Try to load sub-SCXML files only if parent has <invoke> elements (W3C SCXML 6.3)
            if (scxmlContent.includes('<invoke')) {
                try {
                    const subResponse = await fetch(`${basePath}test${testId}sub1.scxml`);
                    if (subResponse.ok) {
                        const subContent = await subResponse.text();
                        Module.FS.writeFile(`/resources/${testId}/test${testId}sub1.scxml`, subContent);
                        console.log(`Created virtual FS: /resources/${testId}/test${testId}sub1.scxml`);
                    }
                } catch (e) {
                    console.log(`Parent has <invoke> but sub-SCXML not found: test${testId}sub1.scxml`);
                }
            } else {
                console.log(`No <invoke> elements - skipping sub-SCXML fetch`);
            }
        }

        // Create single InteractiveTestRunner (engine handles children automatically)
        console.log('Initializing state machine...');
        const runner = new Module.InteractiveTestRunner();

        // Set base path BEFORE loadSCXML for static sub-SCXML analysis
        if (params.test) {
            runner.setBasePath(`/resources/${params.test}/`);
            console.log(`Base path set for invoke resolution: /resources/${params.test}/`);
        }

        if (!runner.loadSCXML(scxmlContent, false)) {
            throw new Error('Failed to load SCXML content');
        }

        if (!runner.initialize()) {
            throw new Error('Failed to initialize state machine');
        }

        const structure = runner.getSCXMLStructure();
        console.log(`  State machine initialized: ${structure.states.length} states`);

        // W3C SCXML 6.3: Get statically detected sub-SCXML structures
        const subSCXMLStructures = runner.getSubSCXMLStructures();
        const hasChildren = subSCXMLStructures.length > 0;

        console.log(`Sub-SCXML detection: ${hasChildren ? subSCXMLStructures.length + ' file(s) found' : 'none'}`);
        if (hasChildren) {
            for (let i = 0; i < subSCXMLStructures.length; i++) {
                const subInfo = subSCXMLStructures[i];
                console.log(`  Child ${i}: ${subInfo.srcPath} (invoked from '${subInfo.parentStateId}')`);
            }
        }

        // Determine container ID based on sub-SCXML presence
        const containerIdToUse = hasChildren ? 'state-diagram-parent-split' : 'state-diagram-single';
        console.log(`Container ID to use: ${containerIdToUse} (hasChildren: ${hasChildren})`);

        // Setup view layout
        if (hasChildren) {
            const singleView = document.getElementById('single-view-container');
            const splitView = document.getElementById('split-view-container');
            console.log(`Setting up split view - singleView: ${singleView}, splitView: ${splitView}`);

            if (singleView) {
                singleView.style.display = 'none';
                console.log(`  ✓ Single view hidden: ${singleView.style.display}`);
            } else {
                console.error('  ✗ single-view-container NOT FOUND!');
            }

            if (splitView) {
                splitView.style.display = 'block';
                console.log(`  ✓ Split view shown: ${splitView.style.display}`);

                // Verify setting applied
                const computed = window.getComputedStyle(splitView);
                console.log(`  ✓ Split view computed display: ${computed.display}, height: ${computed.height}`);
            } else {
                console.error('  ✗ split-view-container NOT FOUND!');
            }

            console.log(`Split view enabled`);
        } else {
            document.getElementById('single-view-container').style.display = 'block';
            document.getElementById('split-view-container').style.display = 'none';
            console.log(`Single view enabled`);
        }

        // Extract available events for UI buttons
        const availableEvents = new Set();
        if (structure.transitions) {
            structure.transitions.forEach(trans => {
                if (trans.event && trans.event.trim() !== '') {
                    availableEvents.add(trans.event);
                }
            });
        }

        // Create parent visualizer
        const visualizer = new SCXMLVisualizer(containerIdToUse, structure);

        // Wait for visualizer to render (initGraph is async)
        console.log('[INIT] Waiting for visualizer to render...');
        await visualizer.initPromise;
        console.log('[INIT] Visualizer render complete');

        const controller = new ExecutionController(runner, visualizer, Array.from(availableEvents).sort(), visualizerManager);

        // Register parent visualizer with manager
        visualizerManager.setParent(visualizer);

        // Create child visualizers if sub-SCXML files exist
        if (hasChildren) {
            const childTabsContainer = document.getElementById('child-tabs');
            const childDiagramsContainer = document.getElementById('child-diagrams-container');
            
            // Show tabs only if multiple children
            if (subSCXMLStructures.length > 1 && childTabsContainer) {
                childTabsContainer.style.display = 'flex';
            }
            
            for (let i = 0; i < subSCXMLStructures.length; i++) {
                const subInfo = subSCXMLStructures[i];
                
                // Create child diagram container
                const childDiagramId = `child-diagram-${i}`;
                const childDiv = document.createElement('div');
                childDiv.id = childDiagramId;
                childDiv.className = `child-diagram diagram-container-split ${i === 0 ? 'active' : ''}`;
                childDiagramsContainer.appendChild(childDiv);
                
                // Create child visualizer and wait for render
                const childVisualizer = new SCXMLVisualizer(childDiagramId, subInfo.structure);
                await childVisualizer.initPromise;
                visualizerManager.addChild(i, childVisualizer);
                
                // Create tab button
                if (subSCXMLStructures.length > 1 && childTabsContainer) {
                    const tabButton = document.createElement('button');
                    tabButton.className = `child-tab ${i === 0 ? 'active' : ''}`;
                    tabButton.textContent = subInfo.invokeId;
                    tabButton.onclick = () => switchChildTab(i);
                    childTabsContainer.appendChild(tabButton);
                }
            }
        }

        // Initial state render
        console.log('[INIT] Before updateState()');
        await controller.updateState();
        console.log('[INIT] After updateState()');

        // W3C SCXML 3.2: Before first macrostep, no state is active yet
        // But we highlight initial state for better UX (after updateState clears it)
        console.log('[INIT] structure.initial:', structure.initial);
        console.log('[INIT] visualizer.initialState:', visualizer.initialState);

        const initialStateId = structure.initial;
        if (initialStateId) {
            console.log(`[INIT] Highlighting initial state: ${initialStateId}`);
            visualizer.highlightActiveStates([initialStateId]);
            console.log('[INIT] highlightActiveStates() called');
        } else {
            console.warn('[INIT] No initial state found!');
        }

        console.log('Visualizer ready!');

        showLoading(false);

        // Update test ID in header (reuse params from above)
        if (params.test) {
            const testIdElement = document.getElementById('test-id');
            if (testIdElement) {
                testIdElement.textContent = params.test;
            }
        }

    } catch (error) {
        console.error('❌ Initialization error:', error);
        showFatalError(error.message);
        showLoading(false);
    }
}

/**
 * Show/hide loading overlay
 */
function showLoading(show) {
    const loadingElement = document.getElementById('loading-state');
    const mainContent = document.getElementById('main-content');
    
    if (loadingElement) {
        loadingElement.style.display = show ? 'block' : 'none';
    }
    
    if (mainContent) {
        mainContent.style.display = show ? 'none' : 'block';
    }
}

/**
 * Show fatal error to user (alert dialog for critical errors)
 * For debug messages, see ExecutionController.showMessage()
 */
function showFatalError(message) {
    console.error(`[FATAL ERROR] ${message}`);
    alert(message);
}

/**
 * Global visualizer manager instance
 */
const visualizerManager = new VisualizerManager();

/**
 * Handle window resize
 * Note: Resize handling is now managed by VisualizerManager
 */
// Resize handling is now managed by VisualizerManager (see visualizer-manager.js)

/**
 * Switch active child tab
 */
function switchChildTab(index) {
    const tabs = document.querySelectorAll('.child-tab');
    const diagrams = document.querySelectorAll('.child-diagram');
    
    tabs.forEach((tab, i) => {
        tab.classList.toggle('active', i === index);
    });
    
    diagrams.forEach((diagram, i) => {
        diagram.classList.toggle('active', i === index);
    });
    
    console.log(`Switched to child ${index}`);
}

/**
 * Main entry point
 */
window.addEventListener('DOMContentLoaded', async () => {
    console.log('SCXML Interactive Visualizer Starting...');

    try {
        const scxmlContent = await loadSCXMLContent();
        await initVisualizer(scxmlContent);
    } catch (error) {
        console.error('❌ Fatal error:', error);
        showFatalError(`Fatal error: ${error.message}`);
        showLoading(false);
    }
});
