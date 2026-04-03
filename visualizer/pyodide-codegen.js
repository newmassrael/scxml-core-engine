/**
 * Pyodide-based SCXML Code Generator
 *
 * Runs Python codegen.py directly in browser using Pyodide (Python WebAssembly).
 * Zero code duplication - uses the same Python code as CLI.
 */

// Configuration constants
const PYODIDE_VERSION = 'v0.25.0';
const PYODIDE_CDN_URL = `https://cdn.jsdelivr.net/pyodide/${PYODIDE_VERSION}/full/`;

// Detect environment: GitHub Pages vs local development
const isGitHubPages = window.location.hostname.includes('github.io');
const BASE_PATH = isGitHubPages ? '' : '../';

class PyodideCodegen {
    constructor() {
        this.pyodide = null;
        this.loading = false;
        this.loaded = false;
        this.loadError = null;
    }

    /**
     * Initialize Pyodide and load Python codegen
     */
    async init(progressCallback) {
        if (this.loaded) return;
        if (this.loading) {
            // Wait for existing load to complete
            while (this.loading) {
                await new Promise(resolve => setTimeout(resolve, 100));
            }
            return;
        }

        this.loading = true;

        try {
            // Load Pyodide
            if (progressCallback) progressCallback('Loading Python runtime...', 0);
            this.pyodide = await loadPyodide({
                indexURL: PYODIDE_CDN_URL
            });

            // Load required packages
            if (progressCallback) progressCallback('Loading Python packages...', 30);
            await this.pyodide.loadPackage(['jinja2', 'lxml', 'micropip']);

            // Load Python files
            if (progressCallback) progressCallback('Loading codegen modules...', 60);

            // Fetch and load Python modules (top-level)
            const modules = [
                'tools/codegen/codegen.py',
                'tools/codegen/scxml_parser.py',
                'tools/codegen/license_config.py'
            ];

            for (const modulePath of modules) {
                // Adapt path based on environment (local uses ../, GitHub Pages uses current dir)
                const response = await fetch(`${BASE_PATH}${modulePath}`);
                const code = await response.text();
                const filename = modulePath.split('/').pop();
                this.pyodide.FS.writeFile(`/${filename}`, code);
            }

            // Load generators package
            this.pyodide.FS.mkdir('/generators');
            const generatorModules = [
                'tools/codegen/generators/__init__.py',
                'tools/codegen/generators/base.py',
                'tools/codegen/generators/cpp_generator.py',
                'tools/codegen/generators/kotlin_generator.py'
            ];

            for (const modulePath of generatorModules) {
                const response = await fetch(`${BASE_PATH}${modulePath}`);
                const code = await response.text();
                const filename = modulePath.split('/').pop();
                this.pyodide.FS.writeFile(`/generators/${filename}`, code);
            }

            // Load templates directory structure
            if (progressCallback) progressCallback('Loading templates...', 80);
            await this._loadTemplates();

            // Initialize Python environment
            await this.pyodide.runPythonAsync(`
import sys
import os
sys.path.insert(0, '/')

# Create output directory
os.makedirs('/output', exist_ok=True)

# Import modules
from generators import get_generator
from scxml_parser import SCXMLParser
            `);

            if (progressCallback) progressCallback('Ready!', 100);
            this.loaded = true;

        } catch (error) {
            this.loadError = error;
            throw new Error(`Failed to initialize Pyodide: ${error.message}`);
        } finally {
            this.loading = false;
        }
    }

    /**
     * Load Jinja2 templates into Pyodide filesystem
     */
    async _loadTemplates() {
        // Load template manifest (adapt path based on environment)
        const manifestResponse = await fetch(`${BASE_PATH}tools/codegen/templates/manifest.json`);
        if (!manifestResponse.ok) {
            throw new Error(`Failed to load template manifest: ${manifestResponse.statusText}`);
        }

        const manifest = await manifestResponse.json();
        const templates = manifest.templates;

        if (!templates || templates.length === 0) {
            throw new Error('Template manifest is empty or invalid');
        }

        // Create template directories
        this.pyodide.FS.mkdir('/templates');
        this.pyodide.FS.mkdir('/templates/actions');

        // Load each template (fail fast on errors)
        for (const template of templates) {
            const response = await fetch(`${BASE_PATH}tools/codegen/templates/${template}`);

            if (!response.ok) {
                throw new Error(`Failed to load critical template '${template}': ${response.statusText}`);
            }

            const content = await response.text();
            this.pyodide.FS.writeFile(`/templates/${template}`, content);
        }
    }

    /**
     * Generate C++ code from SCXML
     *
     * @param {string} scxmlContent - SCXML file content
     * @param {string} filename - Original SCXML filename (optional)
     * @returns {Promise<string>} Generated C++ code
     */
    async generate(scxmlContent, filename = 'input.scxml') {
        if (!this.loaded) {
            throw new Error('Pyodide not initialized. Call init() first.');
        }

        try {
            // Write SCXML to virtual filesystem
            this.pyodide.FS.writeFile('/input.scxml', scxmlContent);

            // Run Python codegen
            const result = await this.pyodide.runPythonAsync(`
from generators import get_generator

# Initialize C++ generator
generator = get_generator('cpp', template_dir='/templates')

# Generate code (writes to /output/)
generator.generate('/input.scxml', '/output', as_child=False)

# Read generated file
import os
files = [f for f in os.listdir('/output') if f.endswith('_sm.h')]
if not files:
    raise Exception('No output file generated')

with open('/output/' + files[0], 'r') as f:
    code = f.read()

code
            `);

            return result;

        } catch (error) {
            // Extract Python error details (with multiline support)
            let errorMessage = error.message;
            if (error.message.includes('PythonError')) {
                // Extract actual Python error (multiline support with [\s\S])
                const match = error.message.match(/PythonError: ([\s\S]+?)(?:\n\n|$)/);
                if (match) {
                    errorMessage = match[1].trim();
                }
            }

            throw new Error(`Code generation failed: ${errorMessage}`);
        }
    }

    /**
     * Get state machine name from SCXML
     *
     * @param {string} scxmlContent - SCXML file content
     * @returns {Promise<string>} State machine name
     */
    async getStateMachineName(scxmlContent) {
        if (!this.loaded) {
            throw new Error('Pyodide not initialized. Call init() first.');
        }

        try {
            this.pyodide.FS.writeFile('/input.scxml', scxmlContent);

            const name = await this.pyodide.runPythonAsync(`
from scxml_parser import SCXMLParser
parser = SCXMLParser()
model = parser.parse_file('/input.scxml')
model.name
            `);

            return name;
        } catch (error) {
            return 'StateMachine';
        }
    }
}

// Global instance
const pyodideCodegen = new PyodideCodegen();

// Export for use in codegen.html
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { PyodideCodegen, pyodideCodegen };
}
