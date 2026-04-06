/**
 * Pyodide-based SCXML Code Generator
 *
 * Runs Python codegen.py directly in browser using Pyodide (Python WebAssembly).
 * Zero code duplication - uses the same Python code as CLI.
 * Supports C++, Kotlin, and Rust code generation.
 */

// Configuration constants
const PYODIDE_VERSION = 'v0.25.0';
const PYODIDE_CDN_URL = `https://cdn.jsdelivr.net/pyodide/${PYODIDE_VERSION}/full/`;

// Detect environment: GitHub Pages vs local development
const isGitHubPages = window.location.hostname.includes('github.io');
const BASE_PATH = isGitHubPages ? '' : '../';

// Language configuration
const LANGUAGE_CONFIG = {
    cpp: {
        label: 'C++',
        templateDir: '/templates',
        manifestPath: 'tools/codegen/templates/manifest.json',
        templateBase: 'tools/codegen/templates/',
        outputPattern: '_sm.h',
        downloadExt: '_sm.h',
        subdirs: ['actions']
    },
    kotlin: {
        label: 'Kotlin',
        templateDir: '/templates/kotlin',
        manifestPath: 'tools/codegen/templates/kotlin/manifest.json',
        templateBase: 'tools/codegen/templates/kotlin/',
        outputPattern: '.kt',
        downloadExt: '.kt',
        subdirs: ['actions']
    },
    rust: {
        label: 'Rust',
        templateDir: '/templates/rust',
        manifestPath: 'tools/codegen/templates/rust/manifest.json',
        templateBase: 'tools/codegen/templates/rust/',
        outputPattern: '.rs',
        downloadExt: '.rs',
        subdirs: ['actions']
    }
};

class PyodideCodegen {
    constructor() {
        this.pyodide = null;
        this.loading = false;
        this.loaded = false;
        this.loadError = null;
        this._loadedLanguages = new Set();
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
            if (progressCallback) progressCallback('Loading codegen modules...', 50);

            // Fetch and load Python modules (top-level)
            const modules = [
                'tools/codegen/codegen.py',
                'tools/codegen/scxml_parser.py',
                'tools/codegen/license_config.py',
                'tools/codegen/ecmascript_to_lua.py'
            ];

            for (const modulePath of modules) {
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
                'tools/codegen/generators/kotlin_generator.py',
                'tools/codegen/generators/rust_generator.py'
            ];

            for (const modulePath of generatorModules) {
                const response = await fetch(`${BASE_PATH}${modulePath}`);
                const code = await response.text();
                const filename = modulePath.split('/').pop();
                this.pyodide.FS.writeFile(`/generators/${filename}`, code);
            }

            // Load C++ templates by default (always needed for base)
            if (progressCallback) progressCallback('Loading C++ templates...', 70);
            await this._loadLanguageTemplates('cpp');

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
     * Load templates for a specific language
     */
    async _loadLanguageTemplates(language) {
        if (this._loadedLanguages.has(language)) return;

        const config = LANGUAGE_CONFIG[language];
        if (!config) throw new Error(`Unknown language: ${language}`);

        // Load manifest
        const manifestResponse = await fetch(`${BASE_PATH}${config.manifestPath}`);
        if (!manifestResponse.ok) {
            throw new Error(`Failed to load ${language} template manifest: ${manifestResponse.statusText}`);
        }
        const manifest = await manifestResponse.json();
        const templates = manifest.templates;

        if (!templates || templates.length === 0) {
            throw new Error(`${language} template manifest is empty or invalid`);
        }

        // Create template directories
        const templateDir = config.templateDir;
        this._mkdirp(templateDir);
        for (const subdir of config.subdirs) {
            this._mkdirp(`${templateDir}/${subdir}`);
        }

        // Load each template
        for (const template of templates) {
            const response = await fetch(`${BASE_PATH}${config.templateBase}${template}`);
            if (!response.ok) {
                throw new Error(`Failed to load critical template '${template}': ${response.statusText}`);
            }
            const content = await response.text();
            this.pyodide.FS.writeFile(`${templateDir}/${template}`, content);
        }

        this._loadedLanguages.add(language);
    }

    /**
     * Recursively create directories (mkdir -p equivalent)
     */
    _mkdirp(path) {
        const parts = path.split('/').filter(Boolean);
        let current = '';
        for (const part of parts) {
            current += '/' + part;
            try {
                this.pyodide.FS.mkdir(current);
            } catch (e) {
                // Directory already exists
            }
        }
    }

    /**
     * Load Jinja2 templates into Pyodide filesystem (legacy - C++ only)
     */
    async _loadTemplates() {
        await this._loadLanguageTemplates('cpp');
    }

    /**
     * Generate code from SCXML for the specified language
     *
     * @param {string} scxmlContent - SCXML file content
     * @param {string} language - Target language ('cpp', 'kotlin', 'rust')
     * @param {string} filename - Original SCXML filename (optional)
     * @returns {Promise<string>} Generated code
     */
    async generate(scxmlContent, language = 'cpp', filename = 'input.scxml') {
        if (!this.loaded) {
            throw new Error('Pyodide not initialized. Call init() first.');
        }

        const config = LANGUAGE_CONFIG[language];
        if (!config) {
            throw new Error(`Unsupported language: '${language}'. Supported: ${Object.keys(LANGUAGE_CONFIG).join(', ')}`);
        }

        // Ensure templates for this language are loaded
        await this._loadLanguageTemplates(language);

        try {
            // Write SCXML to virtual filesystem
            this.pyodide.FS.writeFile('/input.scxml', scxmlContent);

            // Clean output directory
            await this.pyodide.runPythonAsync(`
import os, shutil
if os.path.exists('/output'):
    shutil.rmtree('/output')
os.makedirs('/output', exist_ok=True)
            `);

            const templateDir = config.templateDir;
            const outputPattern = config.outputPattern;

            // Run Python codegen
            const result = await this.pyodide.runPythonAsync(`
from generators import get_generator

# Initialize ${language} generator
generator = get_generator('${language}', template_dir='${templateDir}')

# Generate code (writes to /output/)
generator.generate('/input.scxml', '/output', as_child=False)

# Read generated file
import os
files = [f for f in os.listdir('/output') if f.endswith('${outputPattern}')]
if not files:
    # Try any file in output
    files = [f for f in os.listdir('/output') if not f.startswith('.')]
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
    module.exports = { PyodideCodegen, pyodideCodegen, LANGUAGE_CONFIG };
}
