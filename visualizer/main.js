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

        // DRY Principle: Use shared path resolution from utils.js
        const resourcesPrefix = getResourcesPath();
        const environment = getEnvironmentName();

        const url = `${resourcesPrefix}/${testId}/test${testId}.scxml`;
        logger.debug(`Loading W3C test ${testId} from ${url} (${environment})`);

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Failed to fetch test ${testId}: ${response.statusText}`);
        }
        return await response.text();
    }

    // Source 2: Base64 encoded SCXML (#scxml=<base64>)
    if (params.scxml) {
        logger.debug('Loading SCXML from base64');
        return atob(params.scxml);
    }

    // Source 3: External URL (#url=<url>)
    if (params.url) {
        logger.debug(`Loading SCXML from URL: ${params.url}`);
        const response = await fetch(params.url);
        if (!response.ok) {
            throw new Error(`Failed to fetch ${params.url}: ${response.statusText}`);
        }
        return await response.text();
    }

    throw new Error('No SCXML source specified. Use #test=144, #scxml=<base64>, or #url=<url>');
}

/**
 * Create a validation error message for <script> element violations
 * @param {string} violation - Description of the violation
 * @param {string} details - Specific details about what was found
 * @param {number} elementIndex - 0-based index of the script element
 * @returns {Error} Formatted error object
 */
function createScriptValidationError(violation, details, elementIndex) {
    return new Error([
        'SCXML Document Rejected (W3C SCXML 5.8.2)',
        '',
        violation,
        '',
        details,
        `Location: <script> element #${elementIndex + 1}`,
        '',
        'The SCXML processor cannot execute this document.'
    ].join('\n'));
}

/**
 * W3C SCXML 5.8.2: Validate <script> elements
 * @param {Document} xmlDoc - Parsed SCXML document
 * @throws {Error} If document violates W3C SCXML script requirements
 */
function validateScriptElements(xmlDoc) {
    // W3C SCXML 5.8.2: Validate all <script> elements
    const scripts = xmlDoc.querySelectorAll('script');
    
    for (let i = 0; i < scripts.length; i++) {
        const script = scripts[i];
        const hasSrc = script.hasAttribute('src');
        const hasContent = script.textContent.trim().length > 0;
        
        // W3C SCXML 5.8.2: Must specify either src or child content (not both, not neither)
        if (!hasSrc && !hasContent) {
            throw createScriptValidationError(
                '<script> element must specify either \'src\' attribute or child content.',
                'Found: Empty <script/> element',
                i
            );
        }
        
        if (hasSrc && hasContent) {
            throw createScriptValidationError(
                '<script> element cannot specify both \'src\' attribute and child content.',
                `Found: <script src="${script.getAttribute('src')}">...</script>`,
                i
            );
        }
        
        // W3C SCXML 5.8.2: If src is specified, download must succeed
        // Note: Modern browsers deprecated synchronous XHR (blocks main thread)
        // Browser will automatically handle script loading errors at runtime
        // and throw appropriate errors if download fails
        if (hasSrc) {
            const src = script.getAttribute('src');
            
            // Check for empty src attribute
            if (!src || src.trim().length === 0) {
                throw createScriptValidationError(
                    'Script element has empty src attribute.',
                    'Found: <script src=""/>',
                    i
                );
            }
            
            logger.debug(`Script element with src="${src}" will be validated by browser at runtime (W3C SCXML 5.8.2)`);
            
            // Optional: Basic URL validation to catch obvious errors early
            try {
                new URL(src, window.location.href); // Validates URL format
                logger.debug(`URL format valid: ${src}`);
            } catch (urlError) {
                throw createScriptValidationError(
                    'Invalid script URL format.',
                    `URL: ${src}\nError: ${urlError.message}`,
                    i
                );
            }
        }
    }
}

/**
 * Initialize visualizer (simplified - engine handles invoke automatically)
 */
async function initVisualizer(scxmlContent) {
    showLoading(true);

    try {
        // Load WASM module
        logger.debug('Loading WASM module...');
        window.Module = await createVisualizer();  // Make Module globally accessible
        const Module = window.Module;  // Local alias for convenience
        logger.debug('WASM module loaded');

        // W3C SCXML 5.8.2: Parse and validate <script> elements before execution
        const parser = new DOMParser();
        const xmlDoc = parser.parseFromString(scxmlContent, 'text/xml');
        
        // Check for XML parse errors
        const parseError = xmlDoc.querySelector('parsererror');
        if (parseError) {
            throw new Error('XML parsing failed: ' + parseError.textContent);
        }
        
        // Validate script elements
        try {
            validateScriptElements(xmlDoc);
        } catch (error) {
            // Document rejected - display error and stop execution
            showLoading(false);
            showFatalError(error.message);
            return;
        }

        // Setup Emscripten virtual file system for invoke resolution
        const params = parseHashParams();
        if (params.test) {
            const testId = params.test;

            // DRY Principle: Use shared path resolution from utils.js
            const resourcesPrefix = getResourcesPath();
            const basePath = `${resourcesPrefix}/${testId}/`;
            
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
            logger.debug(`Created virtual FS: /resources/${testId}/test${testId}.scxml`);

            // W3C SCXML 6.3/6.4: Extract child SCXML files from invoke src/srcexpr attributes
            if (scxmlContent.includes('<invoke')) {
                const childFiles = new Set();  // Use Set to avoid duplicates

                try {
                    // Reuse already-parsed xmlDoc (avoid duplicate parsing)
                    
                    // Extract static src="file:..." from invoke elements
                    const invokeElements = xmlDoc.querySelectorAll('invoke[src]');
                    invokeElements.forEach(invoke => {
                        const src = invoke.getAttribute('src');
                        if (src && src.startsWith('file:')) {
                            const filename = src.replace(/^file:/, '');
                            childFiles.add(filename);
                        }
                    });

                    // W3C SCXML 6.4: Extract file references from data/assign expressions
                    // Look for 'file:...' string literals in expr attributes
                    if (scxmlContent.includes('srcexpr')) {
                        logger.debug(`Dynamic invoke (srcexpr) detected - extracting file references from XML`);

                        const exprElements = xmlDoc.querySelectorAll('[expr]');
                        exprElements.forEach(elem => {
                            const expr = elem.getAttribute('expr');
                            // Match 'file:...' or "file:..." in expressions
                            const fileMatches = expr.match(/['"]file:([^'"]+)['"]/g);
                            if (fileMatches) {
                                fileMatches.forEach(match => {
                                    const filename = match.replace(/^['"]file:/, '').replace(/['"]$/, '');
                                    childFiles.add(filename);
                                    logger.debug(`Extracted file reference from expr: ${filename}`);
                                });
                            }
                        });

                        if (childFiles.size > 0) {
                            logger.debug(`XML parsing extracted ${childFiles.size} file reference(s)`);
                        }
                    }
                } catch (e) {
                    // Fallback to regex if XML parsing fails
                    logger.warn('XML parsing failed, using regex fallback:', e.message);

                    // Extract static src="file:..." references
                    const invokePattern = /<invoke[^>]*\ssrc=["']file:([^"']+)["']/g;
                    let match;

                    while ((match = invokePattern.exec(scxmlContent)) !== null) {
                        const filename = match[1];
                        childFiles.add(filename);
                    }

                    // Extract file references from expressions
                    if (scxmlContent.includes('srcexpr')) {
                        const fileRefPattern = /['"']file:([^'"']+)['"']/g;
                        while ((match = fileRefPattern.exec(scxmlContent)) !== null) {
                            const filename = match[1];
                            childFiles.add(filename);
                        }
                    }
                }

                if (childFiles.size > 0) {
                    logger.debug(`Loading ${childFiles.size} potential child SCXML file(s)`);

                    // Load detected/potential child files
                    let loadedCount = 0;
                    for (const childFile of childFiles) {
                        try {
                            const childResponse = await fetch(`${basePath}${childFile}`);
                            if (childResponse.ok) {
                                const childContent = await childResponse.text();
                                Module.FS.writeFile(`/resources/${testId}/${childFile}`, childContent);
                                logger.debug(`Created virtual FS: /resources/${testId}/${childFile}`);
                                loadedCount++;
                            } else {
                                // Log failed HTTP responses at debug level
                                console.debug(`Skipped potential child file ${childFile}: HTTP ${childResponse.status}`);
                            }
                        } catch (e) {
                            // Log errors at debug level (network issues, FS errors, etc.)
                            console.debug(`Skipped potential child file ${childFile}: ${e.message}`);
                        }
                    }

                    logger.debug(`Child SCXML loading complete: ${loadedCount}/${childFiles.size} files loaded`);
                } else {
                    logger.debug(`No file-based invokes detected (content-based invokes may be used)`);
                }
            } else {
                logger.debug(`No <invoke> elements - skipping child SCXML fetch`);
            }
        }

        // Create single InteractiveTestRunner (engine handles children automatically)
        logger.debug('Initializing state machine...');
        const runner = new Module.InteractiveTestRunner();

        // Set base path BEFORE loadSCXML for static sub-SCXML analysis
        if (params.test) {
            runner.setBasePath(`/resources/${params.test}/`);
            logger.debug(`Base path set for invoke resolution: /resources/${params.test}/`);
        }

        if (!runner.loadSCXML(scxmlContent, false)) {
            throw new Error('Failed to load SCXML content');
        }

        if (!runner.initialize()) {
            throw new Error('Failed to initialize state machine');
        }

        const structure = runner.getSCXMLStructure();
        logger.debug(`  State machine initialized: ${structure.states.length} states, ${structure.transitions ? structure.transitions.length : 0} transitions`);
        logger.debug('[DEBUG] Structure object:', structure);

        // W3C SCXML 6.3: Get statically detected sub-SCXML structures
        const subSCXMLStructures = runner.getSubSCXMLStructures();
        const hasChildren = subSCXMLStructures.length > 0;

        logger.debug(`Sub-SCXML detection: ${hasChildren ? subSCXMLStructures.length + ' file(s) found' : 'none'}`);
        if (hasChildren) {
            for (let i = 0; i < subSCXMLStructures.length; i++) {
                const subInfo = subSCXMLStructures[i];
                logger.debug(`  Child ${i}: ${subInfo.srcPath} (invoked from '${subInfo.parentStateId}')`);
            }
        }

        // Single-window navigation for all visualizations
        // Child SCXML navigation is handled via breadcrumb + state click
        const containerIdToUse = 'state-diagram-single';
        logger.debug(`Container ID: ${containerIdToUse} (single-window navigation mode)`);

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
        logger.debug('[INIT] Waiting for visualizer to render...');
        await visualizer.initPromise;
        logger.debug('[INIT] Visualizer render complete');

        const controller = new ExecutionController(runner, visualizer, Array.from(availableEvents).sort(), visualizerManager);

        // Expose controller globally for event deletion buttons
        window.executionController = controller;

        // Register parent visualizer with manager
        visualizerManager.setParent(visualizer);

        // Single-window navigation: Child visualizers are created on-demand when navigating
        // No need to pre-create child visualizers - they will be created in navigateToChild()
        if (hasChildren) {
            logger.debug(`✓ ${subSCXMLStructures.length} child SCXML(s) detected - available for navigation`);
            // Store sub-SCXML info in currentMachine for navigation
            controller.currentMachine.subSCXMLs = subSCXMLStructures;
        }

        // W3C SCXML 3.13: ExecutionController.initializeState() already called in constructor
        // No need to manually update state here - controller handles initial state display
        // (may already be in final state due to eventless transitions)
        logger.debug('[INIT] ExecutionController initialized with actual state');
        logger.debug('Visualizer ready!');

        showLoading(false);

        // Recenter diagram after container is visible (accurate dimensions)
        requestAnimationFrame(() => {
            visualizer.centerDiagram();
            logger.debug('[INIT] Diagram recentered with actual container dimensions');
        });

        // Update test ID in header (reuse params from above)
        if (params.test) {
            const testIdElement = document.getElementById('test-id');
            if (testIdElement) {
                testIdElement.textContent = params.test;
            }
        }

        // Center View button handler
        const btnCenterView = document.getElementById('btn-center-view');
        if (btnCenterView) {
            btnCenterView.addEventListener('click', () => {
                // Center diagram on active states
                const activeStateIds = Array.from(visualizer.activeStates || []);
                
                if (activeStateIds.length > 0) {
                    visualizer.centerDiagram(activeStateIds);
                } else {
                    // No active states, center on all visible nodes
                    visualizer.centerDiagram();
                }
            });
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
    
    logger.debug(`Switched to child ${index}`);
}

/**
 * Main entry point
 */
window.addEventListener('DOMContentLoaded', async () => {
    logger.debug('SCXML Interactive Visualizer Starting...');

    try {
        const scxmlContent = await loadSCXMLContent();
        await initVisualizer(scxmlContent);
    } catch (error) {
        console.error('❌ Fatal error:', error);
        showFatalError(`Fatal error: ${error.message}`);
        showLoading(false);
    }
});
