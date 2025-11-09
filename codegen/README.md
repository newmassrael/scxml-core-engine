# SCXML Web Tools

Online tools for SCXML state machine development.

## Tools

### 1. Code Generator (index.html)
Generate optimized C++ code from SCXML state machine definitions.

**Features:**
- SCXML Validation against W3C SCXML 1.0 specification
- Type-safe, zero-overhead C++ code generation
- Real-time feedback and validation
- Download generated code

**Usage:**
1. Paste your SCXML code in the input pane
2. Click "Generate C++ Code"
3. Download the generated C++ header file

### 2. Interactive Visualizer (visualizer.html)
Step-by-step SCXML execution with real-time state diagram visualization.

**Features:**
- Time-travel debugging (step forward/backward)
- Real-time state diagram with D3.js
- Event queue and data model inspection
- Execution log with W3C SCXML spec references
- Parent-child state machine visualization (invoke support)

**Usage:**
- From Code Generator: Click "Visualize" button
- Direct access: `visualizer.html#test=226` (W3C test number)
- Custom SCXML: `visualizer.html#scxml=<base64>`

## Built with

- Frontend: HTML, CSS, JavaScript
- Visualization: D3.js force-directed graphs
- SCXML Engine: W3C SCXML 1.0 compliant (WASM)

## About scxml-core-engine

These tools are powered by [scxml-core-engine](https://github.com/newmassrael/scxml-core-engine),
a C++ SCXML library with W3C compliance and code generation capabilities.
