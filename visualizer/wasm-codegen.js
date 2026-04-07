/**
 * WASM-based SCXML Code Generator
 *
 * Native Rust codegen compiled to WebAssembly.
 * All templates (Rust, C++, Kotlin) embedded in ~3MB WASM binary.
 * No Python runtime needed.
 */

// Detect environment: GitHub Pages vs local development
const isGitHubPages = window.location.hostname.includes('github.io');
const WASM_BASE = isGitHubPages ? 'wasm/' : 'wasm/';

class WasmCodegen {
    constructor() {
        this.module = null;
        this.loading = false;
        this.loaded = false;
        this.loadError = null;
    }

    /**
     * Initialize the WASM module.
     * Downloads and compiles the WASM binary with all templates embedded.
     */
    async init(progressCallback) {
        if (this.loaded) return;
        if (this.loading) {
            while (this.loading) {
                await new Promise(resolve => setTimeout(resolve, 50));
            }
            if (this.loadError) throw this.loadError;
            return;
        }

        this.loading = true;

        try {
            if (progressCallback) progressCallback('Loading WASM codegen...', 30);

            // Dynamic import of the WASM ES module
            this.module = await import(`./${WASM_BASE}sce_build.js`);
            await this.module.default(`./${WASM_BASE}sce_build_bg.wasm`);

            if (progressCallback) progressCallback('Ready!', 100);
            this.loaded = true;
        } catch (error) {
            this.loadError = error;
            throw new Error(`Failed to initialize WASM codegen: ${error.message}`);
        } finally {
            this.loading = false;
        }
    }

    /**
     * Generate code from SCXML content for any supported language.
     *
     * @param {string} scxmlContent - SCXML file content
     * @param {string} language - Target language: 'rust', 'cpp', 'kotlin'
     * @param {string} filename - Original SCXML filename (for naming)
     * @returns {{ files: Array<[string, string]>, primaryCode: string }}
     */
    generate(scxmlContent, language = 'rust', filename = 'input.scxml') {
        if (!this.loaded) {
            throw new Error('WASM codegen not initialized. Call init() first.');
        }

        // Extract machine name from filename (strip extension and path)
        const name = filename.replace(/\.scxml$/i, '').replace(/.*[\\/]/, '');

        try {
            const jsonResult = this.module.compile_scxml_lang(scxmlContent, name, language);
            const files = JSON.parse(jsonResult);
            // files is [[filename, code], ...] — C++ returns 2 files (.h + .inl), others return 1
            return {
                files,
                primaryCode: files.map(([, code]) => code).join('\n'),
            };
        } catch (error) {
            throw new Error(`${language} code generation failed: ${error}`);
        }
    }

    /**
     * Get state machine name from SCXML content.
     *
     * @param {string} scxmlContent - SCXML file content
     * @returns {string} State machine name
     */
    getStateMachineName(scxmlContent) {
        if (!this.loaded) return 'untitled';

        try {
            return this.module.get_machine_name(scxmlContent);
        } catch {
            return 'untitled';
        }
    }
}

// Global instance
const wasmCodegen = new WasmCodegen();

// Export for use in codegen.html
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { WasmCodegen, wasmCodegen };
}
