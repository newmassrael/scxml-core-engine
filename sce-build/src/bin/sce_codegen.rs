// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// sce-codegen — Unified SCXML code generator CLI.
//
// Replaces Python scripts: codegen.py, generate_kotlin_w3c.py,
// fix_scxml_name.py, read_test_metadata.py
//
// Subcommands:
//   generate       — Single SCXML → code (replaces codegen.py)
//   generate-w3c   — Batch W3C test generation (replaces generate_kotlin_w3c.py)
//   fix-scxml-name — Fix SCXML name attribute (replaces fix_scxml_name.py)
//   read-metadata  — Extract metadata description (replaces read_test_metadata.py)

use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sce_build::analyzer;
use sce_build::filters;
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

// ── CLI Definition ──────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "sce-codegen", about = "SCE SCXML Code Generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from a single SCXML file
    Generate {
        /// Input SCXML file path
        scxml: String,
        /// Target language (rust, cpp, kotlin, go)
        #[arg(short, long, default_value = "cpp")]
        language: String,
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output_dir: String,
        /// Generate as child state machine (C++ invoke support)
        #[arg(long)]
        as_child: bool,
        /// Write CMake DEPFILE for incremental builds
        #[arg(long)]
        write_deps: Option<String>,
        /// Go module path hosting the generated forge packages, e.g.
        /// `github.com/acme/proj/generated`. When set, each Go
        /// `<sce:import>` emits `import "{prefix}/{snake}"` instead of
        /// the bare `"{snake}"` form (invalid outside GOPATH). Required
        /// for any Go crossfile fixture; ignored for other languages.
        #[arg(long)]
        go_module_prefix: Option<String>,
    },
    /// Batch generate W3C test state machines and test classes
    GenerateW3c {
        /// Target language (rust, cpp, kotlin, go)
        #[arg(short, long)]
        language: String,
        /// Path to tests/CMakeLists.txt (test registry)
        #[arg(long)]
        registry: Option<String>,
        /// Path to resources directory
        #[arg(long)]
        resources: Option<String>,
        /// Generate single test by ID
        #[arg(short, long)]
        test: Option<String>,
        /// Remove all generated files
        #[arg(long)]
        clean: bool,
        /// List tests without generating
        #[arg(long)]
        list: bool,
    },
    /// Fix SCXML name attribute (ensure <scxml name="testXXX">)
    FixScxmlName {
        /// SCXML file path
        scxml: String,
        /// Desired name value
        name: String,
    },
    /// Read metadata.txt description field
    ReadMetadata {
        /// Path to metadata.txt
        metadata_file: String,
    },
    /// Build forge dependency manifest (JSON)
    Manifest {
        /// Directory containing forge SCXML files
        dir: String,
    },
    /// Generate a cross-language numerical conformance test harness from a
    /// fixture catalog (single source of truth for all 5 languages).
    GenerateConformance {
        /// Target language (rust, cpp, kotlin, go, python)
        #[arg(short, long)]
        language: String,
        /// Path to fixture catalog JSON (tests/forge/conformance/fixtures.json)
        #[arg(short, long)]
        manifest: String,
        /// Output directory for the generated harness file
        #[arg(short, long)]
        output_dir: String,
    },
    /// Print the conformance fixture name list from a manifest. Build
    /// systems consume this so they don't need a native JSON parser
    /// (CMake, Gradle, plain Bash) to enumerate fixtures.
    ListFixtures {
        /// Path to fixture catalog JSON (tests/forge/conformance/fixtures.json)
        #[arg(short, long)]
        manifest: String,
        /// Output format. `plain` (default) is one fixture name per line,
        /// suitable for `for fixture in $(sce-codegen list-fixtures ...)`.
        /// `cmake` emits a single semicolon-separated CMake list literal.
        /// `space` emits a single space-separated line.
        #[arg(short, long, default_value = "plain")]
        format: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate {
            scxml,
            language,
            output_dir,
            as_child,
            write_deps,
            go_module_prefix,
        } => cmd_generate(
            &scxml,
            &language,
            &output_dir,
            as_child,
            write_deps.as_deref(),
            go_module_prefix.as_deref(),
        ),
        Commands::GenerateW3c {
            language,
            registry,
            resources,
            test,
            clean,
            list,
        } => cmd_generate_w3c(&language, registry.as_deref(), resources.as_deref(), test.as_deref(), clean, list),
        Commands::FixScxmlName { scxml, name } => cmd_fix_scxml_name(&scxml, &name),
        Commands::ReadMetadata { metadata_file } => cmd_read_metadata(&metadata_file),
        Commands::Manifest { dir } => cmd_manifest(&dir),
        Commands::GenerateConformance {
            language,
            manifest,
            output_dir,
        } => cmd_generate_conformance(&language, &manifest, &output_dir),
        Commands::ListFixtures { manifest, format } => cmd_list_fixtures(&manifest, &format),
    }
}

// ── Subcommand: generate ────────────────────────────────────────

fn cmd_generate(
    scxml_path: &str,
    language: &str,
    output_dir: &str,
    as_child: bool,
    depfile_path: Option<&str>,
    go_module_prefix: Option<&str>,
) {
    let lang: Language = language.parse().unwrap_or_else(|_| {
        eprintln!("Unknown language: {language}. Use rust, cpp, kotlin, or go.");
        std::process::exit(1);
    });

    // SCE Forge: detect non-statechart kind and route to forge pipeline.
    // Read the file once; the same content is reused for both detection and compilation.
    let scxml_content = fs::read_to_string(scxml_path).unwrap_or_else(|e| {
        eprintln!("Cannot read {scxml_path}: {e}");
        std::process::exit(1);
    });

    if sce_build::is_forge_document(&scxml_content) {
        let input_stem = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let base_dir = Path::new(scxml_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let forge_opts = sce_build::ForgeCompileOptions {
            go_module_prefix: go_module_prefix.map(str::to_owned),
        };
        match sce_build::compile_forge_with_imports(
            &scxml_content,
            input_stem,
            lang,
            base_dir,
            &forge_opts,
        ) {
            Ok(output) => {
                let out = Path::new(output_dir);
                for (filename, code) in &output.files {
                    let path = out.join(filename);
                    fs::write(&path, code).unwrap_or_else(|e| {
                        eprintln!("Write error: {e}");
                        std::process::exit(1);
                    });
                    println!("Generated: {}", path.display());
                }
                if let Some(dep_path) = depfile_path {
                    let out = Path::new(output_dir);
                    let targets: Vec<String> = output
                        .files
                        .iter()
                        .map(|(f, _)| out.join(f).display().to_string())
                        .collect();
                    let dep_content = format!("{}: {}\n", targets.join(" "), scxml_path);
                    let _ = fs::write(dep_path, dep_content);
                }
                println!("Needs ScriptEngine: false");
                return;
            }
            Err(e) => {
                eprintln!("Forge codegen error: {e}");
                std::process::exit(1);
            }
        }
    }

    let template_dir = sce_build::find_template_dir_for(lang);

    let mut parser = SCXMLParser::new();
    let mut model = match parser.parse_file(scxml_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Parse error: {e}");
            std::process::exit(1);
        }
    };

    if as_child {
        model.has_parent_communication = true;
    }

    analyzer::analyze(&mut model, scxml_path);

    // W3C SCXML 5.8: Document rejected at parse time (e.g., unloadable external script)
    // Generate a language-appropriate rejection stub so AOT test reports PASS.
    if model.document_rejected {
        let input_stem = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let out = Path::new(output_dir);
        let pascal = crate::filters::to_pascal_case(input_stem.to_string());

        match lang {
            Language::Cpp => {
                let header = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     #pragma once\n\
                     #define SCE_DOCUMENT_REJECTED 1\n\
                     namespace SCE::Generated::{name} {{\n\
                     struct {pascal} {{\n\
                     }};\n\
                     }}  // namespace SCE::Generated::{name}\n",
                    name = input_stem, pascal = pascal
                );
                let inl = "// W3C SCXML 5.8: Document rejected\n";
                fs::write(out.join(format!("{input_stem}_sm.h")), &header)
                    .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
                fs::write(out.join(format!("{input_stem}_sm.inl")), inl)
                    .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
            }
            Language::Rust => {
                let stub = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // This state machine was rejected at parse time.\n"
                );
                fs::write(out.join(format!("{input_stem}_sm.rs")), &stub)
                    .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
            }
            Language::Kotlin => {
                let stub = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     package com.sce.generated.{name}\n",
                    name = input_stem
                );
                fs::write(out.join(format!("{input_stem}Sm.kt")), &stub)
                    .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
            }
            Language::Go => {
                let stub = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     package {name}\n",
                    name = input_stem
                );
                fs::write(out.join(format!("{input_stem}_sm.go")), &stub)
                    .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
            }
            Language::Python => {
                let stub = "# W3C SCXML 5.8: Document rejected\n";
                fs::write(out.join(format!("{input_stem}_sm.py")), stub)
                    .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
            }
        }
        println!("Document rejected (W3C SCXML 5.8): {}", model.name);
        println!("Needs ScriptEngine: false");
        return;
    }

    if !analyzer::can_generate_static(&model) {
        eprintln!("Cannot generate static code for '{}' (dynamic features)", model.name);
        println!("Reason: static generation not possible");
        std::process::exit(1);
    }

    resolve_source_path(&mut model, Path::new(scxml_path));

    let input_stem = Path::new(scxml_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let output = match lang {
        Language::Rust => {
            let code = sce_build::generator::generate(&model, &template_dir).unwrap_or_else(|e| {
                eprintln!("Generation error: {e}");
                std::process::exit(1);
            });
            GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.rs"), code)],
            }
        }
        Language::Cpp => {
            sce_build::generator::generate_cpp(&model, &template_dir, input_stem).unwrap_or_else(|e| {
                eprintln!("Generation error: {e}");
                std::process::exit(1);
            })
        }
        Language::Kotlin => {
            let code = sce_build::generator::generate_kotlin(&model, &template_dir).unwrap_or_else(|e| {
                eprintln!("Generation error: {e}");
                std::process::exit(1);
            });
            GeneratedOutput {
                files: vec![(format!("{input_stem}Sm.kt"), code)],
            }
        }
        Language::Go => {
            let code = sce_build::generator::generate_go(&model, &template_dir).unwrap_or_else(|e| {
                eprintln!("Generation error: {e}");
                std::process::exit(1);
            });
            GeneratedOutput {
                files: vec![(format!("{input_stem}_sm.go"), code)],
            }
        }
        Language::Python => {
            eprintln!("Python statechart codegen is not yet supported");
            std::process::exit(1);
        }
    };

    let out_path = Path::new(output_dir);
    fs::create_dir_all(out_path).unwrap_or_else(|e| {
        eprintln!("Cannot create output directory: {e}");
        std::process::exit(1);
    });

    let mut output_paths = Vec::new();
    for (filename, code) in &output.files {
        let file_path = out_path.join(filename);
        fs::write(&file_path, code).unwrap_or_else(|e| {
            eprintln!("Cannot write {}: {e}", file_path.display());
            std::process::exit(1);
        });
        println!("  Generated: {}", file_path.display());
        output_paths.push(file_path);
    }

    println!("  Needs ScriptEngine: {}", model.needs_script_engine);

    // W3C SCXML 6.4: Generate children metadata + hybrid SCXML stubs for all languages.
    // C++ uses _children.txt for CMake post-processing; all languages need hybrid stubs.
    let children = collect_invoke_child_names(&model);
    if lang == Language::Cpp && !children.is_empty() {
        let children_file = out_path.join(format!("{input_stem}_children.txt"));
        fs::write(&children_file, children.join("\n") + "\n").unwrap_or_else(|e| {
            eprintln!("Cannot write children file: {e}");
            std::process::exit(1);
        });
    }
    generate_hybrid_child_scxmls(&model, Path::new(scxml_path), out_path);

    // Write DEPFILE for CMake incremental builds
    if let Some(depfile) = depfile_path {
        write_depfile(depfile, &output_paths, &template_dir, lang, Path::new(scxml_path));
    }
}

/// Collect child SCXML names from model's static/hybrid invokes.
fn collect_invoke_child_names(model: &SCXMLModel) -> Vec<String> {
    let mut children = Vec::new();
    for invoke in &model.static_invokes {
        if !invoke.child_name.is_empty() {
            children.push(invoke.child_name.clone());
        }
    }
    for invoke in &model.hybrid_invokes {
        if !invoke.child_name.is_empty() {
            children.push(invoke.child_name.clone());
        }
    }
    children
}

/// W3C SCXML 6.4: Generate SCXML files for hybrid invoke children (srcexpr/contentexpr).
/// For srcexpr: scan the SCXML source directory for a non-parent .scxml file.
/// For contentexpr or no match: generate a trivial stub that immediately reaches final state.
fn generate_hybrid_child_scxmls(model: &SCXMLModel, scxml_path: &Path, output_dir: &Path) {
    let parent_dir = scxml_path.parent().unwrap_or(Path::new("."));
    let parent_name = scxml_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    for invoke in &model.hybrid_invokes {
        if invoke.child_name.is_empty() {
            continue;
        }
        let child_name = &invoke.child_name;
        let dest = output_dir.join(format!("{child_name}.scxml"));

        // Already exists (from a previous run or static child)
        if dest.exists() {
            continue;
        }

        let srcexpr = invoke.srcexpr.as_deref().unwrap_or("");

        let mut found_child = false;
        if !srcexpr.is_empty() {
            // Scan resource directory for child SCXML (first non-parent .scxml file)
            if let Ok(entries) = std::fs::read_dir(parent_dir) {
                let mut candidates: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        name.ends_with(".scxml") && name != parent_name
                    })
                    .collect();
                candidates.sort_by_key(|e| e.file_name());
                if let Some(child_entry) = candidates.first() {
                    let _ = std::fs::copy(child_entry.path(), &dest);
                    found_child = true;
                }
            }
        }

        if !found_child {
            // Generate trivial stub that immediately reaches final state
            let stub = format!(
                "<?xml version=\"1.0\"?>\n\
                 <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                 name=\"{child_name}\" initial=\"final\" version=\"1.0\">\n\
                 \x20 <final id=\"final\"/>\n\
                 </scxml>\n"
            );
            let _ = std::fs::write(&dest, stub);
        }
    }
}

/// Write CMake DEPFILE (Makefile-format dependency file).
fn write_depfile(depfile_path: &str, output_paths: &[PathBuf], template_dir: &Path, lang: Language, scxml_input: &Path) {
    let mut deps = Vec::new();

    // Add the SCXML input file itself as a dependency
    deps.push(scxml_input.to_path_buf());

    // Add template dependencies (language-specific only)
    if let Ok(entries) = glob_jinja2_files(template_dir) {
        if lang == Language::Cpp {
            // C++ templates are at the base level which also contains rust/ and kotlin/ subdirs.
            // Filter out other languages' templates to avoid spurious rebuilds.
            for entry in entries {
                // Filter using path components (portable across OS)
                let is_other_lang = entry.components().any(|c| {
                    let s = c.as_os_str().to_string_lossy();
                    s == "rust" || s == "kotlin" || s == "go"
                });
                if !is_other_lang {
                    deps.push(entry);
                }
            }
        } else {
            deps.extend(entries);
        }
    }

    // Add sce-codegen binary as a dependency (rebuilds if binary itself changes,
    // which covers all Rust source changes). More precise than listing all .rs files.
    let exe_path = std::env::current_exe().ok();
    if let Some(ref exe) = exe_path {
        if exe.exists() {
            deps.push(exe.clone());
        }
    }

    if !output_paths.is_empty() {
        // List all outputs as targets (e.g., C++ produces both .h and .inl)
        let targets: String = output_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let mut content = format!("{targets}: ");
        for (i, dep) in deps.iter().enumerate() {
            if i > 0 {
                content.push_str(" \\\n    ");
            }
            content.push_str(&dep.display().to_string());
        }
        content.push('\n');
        fs::write(depfile_path, content).unwrap_or_else(|e| {
            eprintln!("Cannot write depfile {depfile_path}: {e}");
            std::process::exit(1);
        });
    }
}

fn glob_jinja2_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(glob_jinja2_files(&path)?);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jinja2") {
            files.push(path);
        }
    }
    Ok(files)
}

// ── Subcommand: generate-w3c ────────────────────────────────────

/// Test info parsed from CMakeLists.txt
#[derive(Debug, Clone)]
struct TestInfo {
    test_type: String,
    comment: String,
}

/// Metadata from resources/XXX/metadata.txt
#[derive(Debug, Default)]
struct TestMetadata {
    description: String,
    specnum: String,
}

fn cmd_generate_w3c(language: &str, registry: Option<&str>, resources: Option<&str>, single_test: Option<&str>, clean: bool, list: bool) {
    let lang: Language = language.parse().unwrap_or_else(|_| {
        eprintln!("Unknown language: {language}. Use cpp, rust, kotlin, or go.");
        std::process::exit(1);
    });

    // Resolve project root
    let project_root = find_project_root();
    let resources_dir = resources
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join("resources"));
    let cmake_file = registry
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join("tests/CMakeLists.txt"));

    let backend: Box<dyn W3cBackend> = match lang {
        Language::Rust => Box::new(RustBackend::new(&project_root)),
        Language::Go => Box::new(GoBackend::new(&project_root)),
        Language::Kotlin => Box::new(KotlinBackend::new(&project_root)),
        Language::Cpp => Box::new(CppBackend::new(&project_root)),
        Language::Python => {
            eprintln!("Python W3C test generation is not yet supported");
            std::process::exit(1);
        }
    };

    generate_w3c_unified(backend.as_ref(), &resources_dir, &cmake_file, single_test, clean, list);
}

fn find_project_root() -> PathBuf {
    // Try CARGO_MANIFEST_DIR ancestor, then CWD
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = crate_dir.join("..");
    if candidate.join("tests/CMakeLists.txt").exists() {
        return fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.to_path_buf());
    }
    let cwd = std::env::current_dir().expect("Cannot get CWD");
    if cwd.join("tests/CMakeLists.txt").exists() {
        return cwd;
    }
    eprintln!("Cannot find project root. Run from project directory or set --registry/--resources.");
    std::process::exit(1);
}

/// Parse test registrations from CMakeLists.txt.
fn parse_cmake_tests(cmake_file: &Path) -> BTreeMap<String, TestInfo> {
    let content = fs::read_to_string(cmake_file).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {e}", cmake_file.display());
        std::process::exit(1);
    });

    let re = regex::Regex::new(
        r"sce_generate_static_w3c_test\((\S+)\s+\$\{STATIC_W3C_OUTPUT_DIR\}(?:\s+TYPE\s+(\w+))?\)\s*#\s*(.*)"
    ).unwrap();

    let mut tests = BTreeMap::new();
    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            let test_id = caps[1].to_string();
            let test_type = caps.get(2).map(|m| m.as_str()).unwrap_or("SIMPLE").to_string();
            let comment = caps[3].trim().to_string();
            tests.insert(test_id, TestInfo { test_type, comment });
        }
    }
    tests
}

/// Read metadata.txt for a test.
fn read_metadata(resources_dir: &Path, test_id: &str) -> TestMetadata {
    let num_prefix = extract_num_prefix(test_id);
    let meta_file = resources_dir.join(&num_prefix).join("metadata.txt");
    if !meta_file.exists() {
        return TestMetadata::default();
    }

    let content = fs::read_to_string(&meta_file).unwrap_or_default();
    let mut metadata = TestMetadata::default();
    for line in content.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            match key.trim() {
                "description" => metadata.description = value.trim().to_string(),
                "specnum" => metadata.specnum = value.trim().to_string(),
                _ => {}
            }
        }
    }
    metadata
}

/// Find the SCXML file for a test ID.
fn find_scxml(resources_dir: &Path, test_id: &str) -> Option<PathBuf> {
    let num_prefix = extract_num_prefix(test_id);
    let scxml = resources_dir.join(&num_prefix).join(format!("test{test_id}.scxml"));
    if scxml.exists() { Some(scxml) } else { None }
}

/// Extract numeric prefix from test ID (e.g., "144" -> "144", "553b" -> "553")
fn extract_num_prefix(test_id: &str) -> String {
    test_id.chars().take_while(|c| c.is_ascii_digit()).collect()
}

/// Convert to PascalCase (matches Python to_pascal_case and Rust filters::to_pascal_case)
fn to_pascal_case(name: &str) -> String {
    filters::to_pascal_case(name.to_string())
}

/// Detect pass state from analyzed model.
fn detect_pass_state(model: &SCXMLModel) -> Option<String> {
    let final_states: Vec<&str> = model
        .states
        .iter()
        .filter(|(_, s)| s.is_final)
        .map(|(id, _)| id.as_str())
        .collect();

    // Priority: pass > final > single non-fail final state
    for state in &final_states {
        if state.eq_ignore_ascii_case("pass") {
            return Some("Pass".to_string());
        }
    }
    for state in &final_states {
        if state.eq_ignore_ascii_case("final") {
            return Some("Final".to_string());
        }
    }
    let non_fail: Vec<_> = final_states
        .iter()
        .filter(|s| !s.eq_ignore_ascii_case("fail"))
        .collect();
    if non_fail.len() == 1 {
        return Some(to_pascal_case(non_fail[0]));
    }
    None
}

/// Check if model uses BasicHTTP send (performHttpSend in generated code).
fn model_uses_http_send(model: &SCXMLModel) -> bool {
    for state in model.states.values() {
        for trans in &state.transitions {
            if action_uses_http_send(&trans.actions) {
                return true;
            }
        }
        for block in state.on_entry_blocks.iter().chain(state.on_exit_blocks.iter()) {
            if action_uses_http_send(block) {
                return true;
            }
        }
    }
    false
}

fn action_uses_http_send(actions: &[sce_build::model::Action]) -> bool {
    for action in actions {
        if action.action_type == "send"
            && action.send_type.contains("BasicHTTPEventProcessor")
        {
            return true;
        }
        if action_uses_http_send(&action.then_actions)
            || action_uses_http_send(&action.else_actions)
            || action_uses_http_send(&action.actions)
        {
            return true;
        }
        for branch in &action.elseif_branches {
            if action_uses_http_send(&branch.actions) {
                return true;
            }
        }
    }
    false
}

// ── W3cBackend Trait ───────────────────────────────────────────

/// Trait abstracting language-specific W3C test generation behavior.
/// Each backend implements language-specific SM generation, test file creation,
/// child processing, and cleanup logic.
trait W3cBackend {
    /// Human-readable language name for summary output.
    fn language_name(&self) -> &str;

    /// Base directory for generated state machine code.
    fn sm_output_base(&self) -> &Path;

    /// Directory for generated test files (may differ from sm_output_base).
    fn test_output_dir(&self) -> &Path;

    /// Generate SM code. Returns Vec of (filename, code) pairs.
    /// C++ returns .h + .inl, others return one file.
    fn generate_sm(&self, model: &SCXMLModel, input_stem: &str) -> Result<Vec<(String, String)>, String>;

    /// Hook after writing parent SM (e.g. Rust writes mod.rs).
    fn post_write_parent(&self, _test_id: &str, _test_mod_dir: &Path, _input_stem: &str) {}

    /// Process a successfully generated child SM: fix package, register module, etc.
    fn process_child(&self, test_id: &str, child_name: &str, code: String, test_mod_dir: &Path);

    /// Handle a child that failed codegen (Kotlin generates stubs, others skip).
    fn process_child_failure(&self, _test_id: &str, _child_name: &str, _test_mod_dir: &Path) {}

    /// Generate test file content. Default returns empty (C++ uses CMake-managed test headers).
    fn generate_test_file(
        &self,
        _test_id: &str,
        _input_stem: &str,
        _machine_name: &str,
        _pass_state: &str,
        _needs_script: bool,
        _uses_http: bool,
        _test_type: &str,
        _metadata: &TestMetadata,
    ) -> String { String::new() }

    /// Test filename (relative to test_output_dir or test_mod_dir).
    /// Default returns empty (backends that generate test files must override).
    fn test_filename(&self, _test_id: &str, _input_stem: &str) -> String { String::new() }

    /// Test file lives in test_mod_dir (Go) vs test_output_dir (Rust, Kotlin).
    fn test_in_sm_dir(&self) -> bool { false }

    /// Whether this backend writes SM files into per-test subdirectories (testNNN/).
    /// C++ writes to a flat output directory; others use subdirectories.
    fn uses_per_test_subdirs(&self) -> bool { true }

    /// Whether this backend generates test files alongside SM code.
    /// C++ test headers are managed by CMake, not by sce-codegen.
    fn generates_test_files(&self) -> bool { true }

    /// Called after main loop to write module indices (Rust writes root mod.rs).
    fn finalize(&self, _generated_ids: &[String]) {}

    /// Clean all generated files.
    fn clean(&self);

    /// Clean stale generated files for tests no longer in registry.
    /// Returns number of stale entries removed. Default is no-op.
    fn clean_stale(&self, _valid_ids: &BTreeSet<String>) -> usize { 0 }

    /// Child name matching for the standard naming convention.
    /// Default checks: test{num_prefix}_, test{num_prefix}sub, test{test_id}_
    fn child_name_matches(&self, child_name: &str, test_id: &str, num_prefix: &str) -> bool {
        child_name.starts_with(&format!("test{num_prefix}_"))
            || child_name.starts_with(&format!("test{num_prefix}sub"))
            || child_name.starts_with(&format!("test{test_id}_"))
    }

    /// Check if parent code references this child. Default checks Policy, State, StateMachine.
    fn parent_references_child(&self, parent_code: &str, child_machine: &str) -> bool {
        parent_code.contains(&format!("{child_machine}Policy"))
            || parent_code.contains(&format!("{child_machine}State"))
            || parent_code.contains(&format!("{child_machine}StateMachine"))
    }
}

// ── Shared Utilities for W3C Generation ────────────────────────

/// Collect child SCXML files from both resource dir AND output dir (for hybrid stubs).
/// Returns deduplicated, sorted paths.
fn collect_child_scxml_entries(resource_dir: &Path, output_dir: &Path) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for dir in [resource_dir, output_dir] {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("scxml") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if seen.insert(name) {
                    entries.push(path);
                }
            }
        }
    }
    entries.sort();
    entries
}

/// Unified child SM generation for all backends.
/// Handles: child discovery (resource + output dir), filtering by naming convention,
/// parent code reference check (direct or hybrid), parse + analyze + generate,
/// and backend-specific post-processing.
fn generate_child_sms(
    backend: &dyn W3cBackend,
    test_id: &str,
    scxml_path: &Path,
    test_mod_dir: &Path,
    parent_code: &str,
) {
    let resource_dir = scxml_path.parent().unwrap_or(Path::new("."));
    let parent_stem = scxml_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let num_prefix = extract_num_prefix(test_id);

    // Track used hybrid names to handle multiple hybrid children (hybrid0, hybrid1, etc.)
    let mut used_hybrid_names = BTreeSet::new();

    for path in collect_child_scxml_entries(resource_dir, test_mod_dir) {
        let child_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if child_name == parent_stem {
            continue;
        }

        if !backend.child_name_matches(&child_name, test_id, &num_prefix) {
            continue;
        }

        // Determine effective name: direct reference or hybrid name mapping.
        // Direct: parent code contains "{ChildMachine}Policy" or "{ChildMachine}StateMachine"
        // Hybrid: parent code contains "Hybrid" and we find the next unused testNNN_hybridM name
        let child_machine = to_pascal_case(&child_name);
        let effective_name = if backend.parent_references_child(parent_code, &child_machine) {
            child_name.clone()
        } else {
            match find_next_hybrid_name(parent_code, &num_prefix, &used_hybrid_names) {
                Some(name) => {
                    used_hybrid_names.insert(name.clone());
                    name
                }
                None => continue,
            }
        };

        // Parse + analyze + generate child SM
        let mut parser = SCXMLParser::new();
        let child_str = path.to_str().unwrap_or("");
        match parser.parse_file(child_str) {
            Ok(mut child_model) => {
                analyzer::analyze(&mut child_model, child_str);

                if !analyzer::can_generate_static(&child_model) {
                    backend.process_child_failure(test_id, &effective_name, test_mod_dir);
                    continue;
                }

                resolve_source_path(&mut child_model, &path);

                // Override model name for hybrid children so generated types match parent
                if effective_name != child_name {
                    child_model.name = effective_name.clone();
                }

                match backend.generate_sm(&child_model, &effective_name) {
                    Ok(files) => {
                        // Child SM always produces a single file; use the code from the first entry
                        if let Some((_, code)) = files.into_iter().next() {
                            backend.process_child(test_id, &effective_name, code, test_mod_dir);
                        }
                    }
                    Err(_) => {
                        backend.process_child_failure(test_id, &effective_name, test_mod_dir);
                    }
                }
            }
            Err(_) => {
                backend.process_child_failure(test_id, &effective_name, test_mod_dir);
            }
        }
    }
}

/// Find the next unused hybrid invoke name (e.g. "test191_hybrid0") referenced by parent code.
/// Scans for testNNN_hybrid0..testNNN_hybrid9, skipping names already in `used`.
fn find_next_hybrid_name(parent_code: &str, num_prefix: &str, used: &BTreeSet<String>) -> Option<String> {
    if !parent_code.contains("Hybrid") {
        return None;
    }
    for i in 0..10 {
        let hybrid_name = format!("test{num_prefix}_hybrid{i}");
        if used.contains(&hybrid_name) {
            continue;
        }
        let hybrid_machine = to_pascal_case(&hybrid_name);
        if parent_code.contains(&format!("{hybrid_machine}Policy"))
            || parent_code.contains(&format!("{hybrid_machine}StateMachine"))
        {
            return Some(hybrid_name);
        }
    }
    None
}

/// The single unified W3C test generation loop shared by all backends.
fn generate_w3c_unified(
    backend: &dyn W3cBackend,
    resources_dir: &Path,
    cmake_file: &Path,
    single_test: Option<&str>,
    clean: bool,
    list: bool,
) {
    if clean {
        backend.clean();
        return;
    }

    let cmake_tests = parse_cmake_tests(cmake_file);
    println!("C++ test registry: {} tests", cmake_tests.len());

    if list {
        for (tid, info) in &cmake_tests {
            let scxml = find_scxml(resources_dir, tid);
            let status = if scxml.is_some() { "OK" } else { "MISSING" };
            let comment_trunc: String = info.comment.chars().take(70).collect();
            println!("  {tid:6} [{:9}] {status} -- {comment_trunc}", info.type_str());
        }
        return;
    }

    let test_ids: Vec<String> = if let Some(tid) = single_test {
        vec![tid.to_string()]
    } else {
        let mut ids: Vec<String> = cmake_tests.keys().cloned().collect();
        ids.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        ids
    };

    let mut generated_static = Vec::new();
    let mut generated_script = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for test_id in &test_ids {
        let scxml_path = match find_scxml(resources_dir, test_id) {
            Some(p) => p,
            None => {
                skipped.push((test_id.clone(), "SCXML not found".to_string()));
                continue;
            }
        };

        let mut parser = SCXMLParser::new();
        let scxml_str = scxml_path.to_str().unwrap_or("");
        match parser.parse_file(scxml_str) {
            Ok(mut model) => {
                analyzer::analyze(&mut model, scxml_str);

                // W3C SCXML 5.8: document_rejected models have initial->pass already
                // redirected by the parser, so they CAN be generated. Only skip
                // truly dynamic models.
                if !analyzer::can_generate_static(&model) && !model.document_rejected {
                    skipped.push((test_id.clone(), "dynamic features".to_string()));
                    continue;
                }

                resolve_source_path(&mut model, &scxml_path);

                let input_stem = scxml_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

                match backend.generate_sm(&model, input_stem) {
                    Ok(files) => {
                        // Determine write directory: per-test subdir or flat output dir
                        let test_mod_dir = if backend.uses_per_test_subdirs() {
                            backend.sm_output_base().join(format!("test{test_id}"))
                        } else {
                            backend.sm_output_base().to_path_buf()
                        };
                        fs::create_dir_all(&test_mod_dir).unwrap_or_else(|e| {
                            eprintln!("Cannot create dir: {e}");
                            std::process::exit(1);
                        });

                        // Collect parent code for child reference checking
                        let parent_code: String = files.iter().map(|(_, c)| c.as_str()).collect::<Vec<_>>().join("\n");

                        // Write SM files
                        for (filename, code) in &files {
                            let file_path = test_mod_dir.join(filename);
                            write_if_changed(&file_path, code);
                        }

                        // Post-write hook (e.g. Rust writes initial mod.rs)
                        backend.post_write_parent(test_id, &test_mod_dir, input_stem);

                        // W3C SCXML 6.4: Generate hybrid SCXML stubs + child state machines
                        // (only for backends that use per-test subdirs; C++ handles children via CMake)
                        if backend.uses_per_test_subdirs() {
                            generate_hybrid_child_scxmls(&model, &scxml_path, &test_mod_dir);
                            generate_child_sms(backend, test_id, &scxml_path, &test_mod_dir, &parent_code);
                        }

                        // Detect pass state and generate test file (if backend supports it)
                        if backend.generates_test_files() {
                            let pass_state = detect_pass_state(&model);
                            if let Some(ref pass) = pass_state {
                                let machine = to_pascal_case(input_stem);
                                let metadata = read_metadata(resources_dir, test_id);
                                let test_type = cmake_tests.get(test_id.as_str()).map(|i| i.test_type.as_str()).unwrap_or("SIMPLE");
                                let uses_http = model_uses_http_send(&model);
                                let needs_script = model.needs_script_engine;

                                let test_code = backend.generate_test_file(
                                    test_id, input_stem, &machine, pass, needs_script, uses_http, test_type, &metadata,
                                );
                                let test_filename = backend.test_filename(test_id, input_stem);
                                let test_file_dir = if backend.test_in_sm_dir() {
                                    &test_mod_dir
                                } else {
                                    backend.test_output_dir()
                                };
                                let test_file = test_file_dir.join(&test_filename);
                                fs::create_dir_all(test_file_dir).ok();
                                write_if_changed(&test_file, &test_code);

                                if needs_script {
                                    generated_script.push(test_id.clone());
                                } else {
                                    generated_static.push(test_id.clone());
                                }
                            } else {
                                failed.push((test_id.clone(), "pass state not detected".to_string()));
                            }
                        } else {
                            // Backend doesn't generate test files (C++); count as generated
                            if model.needs_script_engine {
                                generated_script.push(test_id.clone());
                            } else {
                                generated_static.push(test_id.clone());
                            }
                        }
                    }
                    Err(e) => {
                        failed.push((test_id.clone(), format!("codegen failed: {e}")));
                    }
                }
            }
            Err(e) => {
                failed.push((test_id.clone(), format!("parse error: {e}")));
            }
        }
    }

    // Clean stale files (backends that override clean_stale, e.g. Kotlin)
    let mut stale_removed = 0;
    if single_test.is_none() {
        let valid_ids: BTreeSet<String> = generated_static.iter().chain(generated_script.iter()).cloned().collect();
        if !valid_ids.is_empty() {
            stale_removed = backend.clean_stale(&valid_ids);
        }
    }

    // Finalize (Rust writes root mod.rs)
    if single_test.is_none() {
        let mut all_ids: Vec<String> = generated_static.iter().chain(generated_script.iter()).cloned().collect();
        all_ids.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        backend.finalize(&all_ids);
    }

    // Summary
    let total_generated = generated_static.len() + generated_script.len();
    println!("\n{}", "=".repeat(60));
    println!("{} W3C Test Generation Summary", backend.language_name());
    println!("{}", "=".repeat(60));
    println!("  Generated (pure static):    {}", generated_static.len());
    println!("  Generated (script engine):  {}", generated_script.len());
    println!("  Generated (total):          {total_generated}");
    println!("  Skipped:                    {}", skipped.len());
    println!("  Failed:                     {}", failed.len());
    if stale_removed > 0 {
        println!("  Stale removed:              {stale_removed}");
    }
    println!("  Total:                      {}", test_ids.len());

    if !skipped.is_empty() {
        println!("\nSkipped:");
        for (tid, reason) in &skipped {
            println!("  {tid}: {reason}");
        }
    }

    if !failed.is_empty() {
        println!("\nFailed tests:");
        for (tid, reason) in &failed {
            println!("  {tid}: {reason}");
        }
    }

    if total_generated > 0 {
        println!("\nGenerated SM classes: {}", backend.sm_output_base().display());
        if !backend.test_in_sm_dir() {
            println!("Generated test classes: {}", backend.test_output_dir().display());
        }
        println!("\nGenerated test IDs (static): {}", generated_static.join(" "));
        if !generated_script.is_empty() {
            println!("Generated test IDs (script): {}", generated_script.join(" "));
        }
    }

    if !failed.is_empty() {
        std::process::exit(1);
    }
}

// ── RustBackend ────────────────────────────────────────────────

struct RustBackend {
    sm_base: PathBuf,
    test_dir: PathBuf,
    tmpl_dir: PathBuf,
}

impl RustBackend {
    fn new(project_root: &Path) -> Self {
        let tests_crate = project_root.join("sce-rust-tests");
        Self {
            sm_base: tests_crate.join("src/generated"),
            test_dir: tests_crate.join("tests"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Rust),
        }
    }
}

impl W3cBackend for RustBackend {
    fn language_name(&self) -> &str { "Rust" }
    fn sm_output_base(&self) -> &Path { &self.sm_base }
    fn test_output_dir(&self) -> &Path { &self.test_dir }

    fn generate_sm(&self, model: &SCXMLModel, input_stem: &str) -> Result<Vec<(String, String)>, String> {
        let code = sce_build::generator::generate(model, &self.tmpl_dir)?;
        Ok(vec![(format!("{input_stem}_sm.rs"), code)])
    }

    fn post_write_parent(&self, _test_id: &str, test_mod_dir: &Path, input_stem: &str) {
        let mod_content = format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\n\
             #[allow(dead_code, unused_variables, unused_imports, clippy::all)]\n\
             mod {input_stem}_sm;\n\
             pub use {input_stem}_sm::*;\n"
        );
        write_if_changed(&test_mod_dir.join("mod.rs"), &mod_content);
    }

    fn process_child(&self, _test_id: &str, child_name: &str, code: String, test_mod_dir: &Path) {
        let child_sm_file = test_mod_dir.join(format!("{child_name}_sm.rs"));
        write_if_changed(&child_sm_file, &code);

        // Add child module to the test's mod.rs
        let mod_file = test_mod_dir.join("mod.rs");
        if let Ok(existing) = fs::read_to_string(&mod_file) {
            if !existing.contains(&format!("mod {child_name}_sm;")) {
                let addition = format!(
                    "#[allow(dead_code, unused_variables, unused_imports, clippy::all)]\n\
                     mod {child_name}_sm;\n\
                     pub use {child_name}_sm::*;\n"
                );
                write_if_changed(&mod_file, &format!("{existing}{addition}"));
            }
        }
    }

    fn generate_test_file(
        &self,
        test_id: &str,
        _input_stem: &str,
        machine_name: &str,
        pass_state: &str,
        needs_script: bool,
        uses_http: bool,
        test_type: &str,
        _metadata: &TestMetadata,
    ) -> String {
        let timeout_secs = if test_type == "SCHEDULED" || test_type == "HTTP" { 5 } else { 3 };
        let lua_register = if needs_script {
            "    let _ = sce_rust_lua::register();\n"
        } else {
            ""
        };
        let is_http = test_type == "HTTP" && uses_http;
        let http_setup = if is_http {
            "    sce_rust_tests::harness::setup_http_test(&mut engine);\n"
        } else {
            ""
        };
        let pass_variant = to_pascal_case(pass_state);

        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             use std::time::Duration;\n\
             \n\
             #[test]\n\
             fn test_{test_id}() {{\n\
             {lua_register}\
             \x20   let policy = sce_rust_tests::generated::test{test_id}::{machine_name}Policy::new();\n\
             \x20   let mut engine = sce_rust_runtime::Engine::new(policy);\n\
             {http_setup}\
             \x20   engine.initialize();\n\
             \x20   let completed = engine.run_until_completion(\n\
             \x20       Duration::from_secs({timeout_secs}),\n\
             \x20       Duration::from_millis(10),\n\
             \x20   );\n\
             \x20   assert!(completed, \"Test {test_id} timed out\");\n\
             \x20   assert_eq!(\n\
             \x20       engine.get_current_state(),\n\
             \x20       sce_rust_tests::generated::test{test_id}::{machine_name}State::{pass_variant},\n\
             \x20       \"Test {test_id} reached wrong final state\"\n\
             \x20   );\n\
             }}\n"
        )
    }

    fn test_filename(&self, test_id: &str, _input_stem: &str) -> String {
        format!("test_{test_id}.rs")
    }

    fn finalize(&self, generated_ids: &[String]) {
        if generated_ids.is_empty() {
            return;
        }
        let mut mod_lines = vec![
            "// GENERATED -- DO NOT EDIT (sce-codegen)".to_string(),
            format!("//! Generated W3C SCXML conformance test state machines ({} tests).\n", generated_ids.len()),
        ];
        for id in generated_ids {
            mod_lines.push(format!("pub mod test{id};"));
        }
        mod_lines.push(String::new());
        write_if_changed(&self.sm_base.join("mod.rs"), &mod_lines.join("\n"));
    }

    fn clean(&self) {
        // Remove generated dirs but not handcrafted files
        for entry in fs::read_dir(&self.sm_base).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with("test")) {
                fs::remove_dir_all(&path).ok();
            }
        }
        for entry in fs::read_dir(&self.test_dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("test_") && name.ends_with(".rs") {
                    fs::remove_file(&path).ok();
                }
            }
        }
        println!("Cleaned Rust generated files");
    }
}

// ── GoBackend ──────────────────────────────────────────────────

struct GoBackend {
    sm_base: PathBuf,
    tmpl_dir: PathBuf,
}

impl GoBackend {
    fn new(project_root: &Path) -> Self {
        let tests_module = project_root.join("sce-go-tests");
        Self {
            sm_base: tests_module.join("generated"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Go),
        }
    }
}

impl W3cBackend for GoBackend {
    fn language_name(&self) -> &str { "Go" }
    fn sm_output_base(&self) -> &Path { &self.sm_base }
    fn test_output_dir(&self) -> &Path { &self.sm_base } // Go tests live in sm dir

    fn generate_sm(&self, model: &SCXMLModel, input_stem: &str) -> Result<Vec<(String, String)>, String> {
        let code = sce_build::generator::generate_go(model, &self.tmpl_dir)?;
        Ok(vec![(format!("{input_stem}_sm.go"), code)])
    }

    fn process_child(&self, test_id: &str, child_name: &str, code: String, test_mod_dir: &Path) {
        let parent_package = format!("test{test_id}");
        let child_pkg = sce_build::filters::to_snake_case(child_name.to_string());
        let fixed_code = code.replace(
            &format!("package {child_pkg}"),
            &format!("package {parent_package}"),
        );
        let child_file = test_mod_dir.join(format!("{child_name}_sm.go"));
        write_if_changed(&child_file, &fixed_code);
    }

    fn generate_test_file(
        &self,
        test_id: &str,
        _input_stem: &str,
        machine_name: &str,
        pass_state: &str,
        needs_script: bool,
        uses_http: bool,
        test_type: &str,
        metadata: &TestMetadata,
    ) -> String {
        let is_http = test_type == "HTTP" && uses_http;
        let timeout = if test_type == "SCHEDULED" { "5 * time.Second" } else { "3 * time.Second" };

        let engine_setup = if needs_script {
            format!(
                "\tpolicy := New{machine_name}Policy()\n\
                 \tpolicy.SessionID = sce.GenerateSessionID()\n\
                 \tscegotest.RegisterLuaEngine()\n\
                 \tengine := sce.NewEngine[{machine_name}State, {machine_name}Event](&policy)"
            )
        } else {
            format!(
                "\tpolicy := New{machine_name}Policy()\n\
                 \tengine := sce.NewEngine[{machine_name}State, {machine_name}Event](&policy)"
            )
        };

        let http_setup = if is_http {
            "\n\tscegotest.SetupHTTPTest(engine)\n"
        } else {
            ""
        };

        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             // W3C SCXML {specnum}: {description}\n\
             package test{test_id}\n\
             \n\
             import (\n\
             \t\"testing\"\n\
             \t\"time\"\n\
             \n\
             \tsce \"github.com/newmassrael/sce-go-runtime\"\n\
             \tscegotest \"github.com/newmassrael/sce-go-tests/harness\"\n\
             )\n\
             \n\
             func TestW3C{test_id}(t *testing.T) {{\n\
             {engine_setup}{http_setup}\n\
             \tengine.Initialize()\n\
             \tcompleted := engine.RunUntilCompletion({timeout}, 10*time.Millisecond)\n\
             \tif !completed {{\n\
             \t\tt.Fatalf(\"Test {test_id} timed out\")\n\
             \t}}\n\
             \tscegotest.AssertFinalState(t, engine.GetCurrentState(), {machine_name}State{pass_state}, \"{test_id}\")\n\
             }}\n",
            specnum = metadata.specnum,
            description = metadata.description,
        )
    }

    fn test_filename(&self, _test_id: &str, input_stem: &str) -> String {
        format!("{input_stem}_test.go")
    }

    fn test_in_sm_dir(&self) -> bool { true }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            println!("Cleaned: {}", self.sm_base.display());
        }
    }
}

// ── KotlinBackend ──────────────────────────────────────────────

struct KotlinBackend {
    sm_base: PathBuf,
    test_dir: PathBuf,
    tmpl_dir: PathBuf,
}

impl KotlinBackend {
    fn new(project_root: &Path) -> Self {
        let tests_module = project_root.join("sce-kotlin-tests");
        Self {
            sm_base: tests_module.join("src/main/kotlin/com/sce/generated"),
            test_dir: tests_module.join("src/test/kotlin/com/sce/w3c"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Kotlin),
        }
    }
}

impl W3cBackend for KotlinBackend {
    fn language_name(&self) -> &str { "Kotlin" }
    fn sm_output_base(&self) -> &Path { &self.sm_base }
    fn test_output_dir(&self) -> &Path { &self.test_dir }

    fn generate_sm(&self, model: &SCXMLModel, input_stem: &str) -> Result<Vec<(String, String)>, String> {
        let code = sce_build::generator::generate_kotlin(model, &self.tmpl_dir)?;
        Ok(vec![(format!("{input_stem}Sm.kt"), code)])
    }

    fn child_name_matches(&self, child_name: &str, _test_id: &str, num_prefix: &str) -> bool {
        // Kotlin uses only num_prefix patterns (no test{test_id}_ pattern)
        child_name.starts_with(&format!("test{num_prefix}_"))
            || child_name.starts_with(&format!("test{num_prefix}sub"))
    }

    fn parent_references_child(&self, parent_code: &str, child_machine: &str) -> bool {
        // Kotlin checks StateMachine only
        parent_code.contains(&format!("{child_machine}StateMachine"))
    }

    fn process_child(&self, test_id: &str, child_name: &str, code: String, test_mod_dir: &Path) {
        let parent_package = format!("test{test_id}");
        let child_package = child_name.to_lowercase();
        let fixed_code = code.replace(
            &format!("package com.sce.generated.{child_package}"),
            &format!("package com.sce.generated.{parent_package}"),
        );
        let child_sm_file = test_mod_dir.join(format!("{child_name}Sm.kt"));
        write_if_changed(&child_sm_file, &fixed_code);
    }

    fn process_child_failure(&self, test_id: &str, child_name: &str, test_mod_dir: &Path) {
        let parent_package = format!("test{test_id}");
        let child_class = to_pascal_case(child_name);
        let stub = format!(
            "// GENERATED STUB -- child codegen failed (no-op)\n\
             package com.sce.generated.{parent_package}\n\n\
             import com.sce.runtime.*\n\n\
             sealed interface {child_class}State : State {{\n\
             \x20   data object Initial : {child_class}State\n\
             }}\n\
             sealed interface {child_class}Event : Event\n\n\
             class {child_class}StateMachine(\n\
             \x20   scriptEngine: ScxmlScriptEngine? = null\n\
             ) : StateMachineEngine<{child_class}State, {child_class}Event>(scriptEngine) {{\n\
             \x20   override val initialState = {child_class}State.Initial\n\
             \x20   override fun processEvent(state: {child_class}State, event: {child_class}Event) = TransitionResult.Ignored as TransitionResult<{child_class}State>\n\
             \x20   override fun onEntry(state: {child_class}State) {{}}\n\
             \x20   override fun onExit(state: {child_class}State) {{}}\n\
             \x20   override fun executeTransitionActions(source: {child_class}State, event: {child_class}Event?) {{}}\n\
             }}\n"
        );
        let child_sm_file = test_mod_dir.join(format!("{child_name}Sm.kt"));
        write_if_changed(&child_sm_file, &stub);
    }

    fn generate_test_file(
        &self,
        test_id: &str,
        _input_stem: &str,
        _machine_name: &str,
        pass_state: &str,
        needs_script: bool,
        uses_http: bool,
        test_type: &str,
        metadata: &TestMetadata,
    ) -> String {
        let sm_class = format!("Test{}", to_pascal_case(test_id));
        let sm_package = format!("test{test_id}");

        // W3C SCXML C.2: HTTP tests use W3CHttpTestBase only when SM actually uses performHttpSend()
        let is_http = test_type == "HTTP" && uses_http;
        let base_class = if is_http { "W3CHttpTestBase" } else { "W3CTestBase" };

        // W3C SCXML 6.2: SCHEDULED tests need longer timeout
        let timeout_override = if test_type == "SCHEDULED" {
            "    override val timeoutMs: Long = 5000L\n"
        } else {
            ""
        };

        let create_sm = if needs_script {
            format!("    override fun createStateMachine() = {sm_class}StateMachine(createEngine())\n")
        } else {
            format!("    override fun createStateMachine() = {sm_class}StateMachine()\n")
        };

        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             package com.sce.w3c\n\
             \n\
             import com.sce.generated.{sm_package}.{sm_class}Event\n\
             import com.sce.generated.{sm_package}.{sm_class}State\n\
             import com.sce.generated.{sm_package}.{sm_class}StateMachine\n\
             import org.junit.jupiter.api.DisplayName\n\
             \n\
             // W3C SCXML {specnum}: {description}\n\
             @DisplayName(\"Test {test_id} -- W3C SCXML {specnum}\")\n\
             class {sm_class} : {base_class}<{sm_class}State, {sm_class}Event>() {{\n\
             {create_sm}\
             \x20   override val expectedPassState: {sm_class}State = {sm_class}State.{pass_state}\n\
             {timeout_override}\
             }}\n",
            specnum = metadata.specnum,
            description = metadata.description,
        )
    }

    fn test_filename(&self, test_id: &str, _input_stem: &str) -> String {
        format!("Test{}.kt", to_pascal_case(test_id))
    }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            println!("Cleaned: {}", self.sm_base.display());
        }
        for entry in fs::read_dir(&self.test_dir).into_iter().flatten() {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("Test") && name.ends_with(".kt") {
                    fs::remove_file(entry.path()).ok();
                }
            }
        }
        println!("Cleaned test classes in: {}", self.test_dir.display());
    }

    fn clean_stale(&self, valid_ids: &BTreeSet<String>) -> usize {
        let mut removed = 0;

        // Clean stale SM directories
        if self.sm_base.exists() {
            for entry in fs::read_dir(&self.sm_base).into_iter().flatten().flatten() {
                if !entry.path().is_dir() { continue; }
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("test") { continue; }
                let dir_test_id = &name[4..];
                if !valid_ids.contains(dir_test_id) {
                    fs::remove_dir_all(entry.path()).ok();
                    println!("  Removed stale SM dir: {name}");
                    removed += 1;
                }
            }
        }

        // Clean stale test classes
        if self.test_dir.exists() {
            let valid_lower: BTreeSet<String> = valid_ids.iter().map(|s| s.to_lowercase()).collect();
            for entry in fs::read_dir(&self.test_dir).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("Test") || !name.ends_with(".kt") { continue; }
                if name == "W3CTestBase.kt" || name == "W3CHttpTestBase.kt" { continue; }
                let stem = &name[..name.len() - 3];
                let file_test_id = &stem[4..];
                if !valid_ids.contains(file_test_id) && !valid_lower.contains(&file_test_id.to_lowercase()) {
                    fs::remove_file(entry.path()).ok();
                    println!("  Removed stale test: {name}");
                    removed += 1;
                }
            }
        }

        removed
    }
}

// ── CppBackend ─────────────────────────────────────────────────

struct CppBackend {
    output_dir: PathBuf,
    tmpl_dir: PathBuf,
}

impl CppBackend {
    fn new(project_root: &Path) -> Self {
        Self {
            output_dir: project_root.join("build/tests/w3c_static_generated"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Cpp),
        }
    }
}

impl W3cBackend for CppBackend {
    fn language_name(&self) -> &str { "C++" }
    fn sm_output_base(&self) -> &Path { &self.output_dir }
    fn test_output_dir(&self) -> &Path { &self.output_dir }

    fn generate_sm(&self, model: &SCXMLModel, input_stem: &str) -> Result<Vec<(String, String)>, String> {
        let output = sce_build::generator::generate_cpp(model, &self.tmpl_dir, input_stem)?;
        Ok(output.files)
    }

    fn uses_per_test_subdirs(&self) -> bool { false }
    fn generates_test_files(&self) -> bool { false }

    fn process_child(&self, _test_id: &str, _child_name: &str, _code: String, _test_mod_dir: &Path) {
        // C++ children are handled by CMake via _children.txt, not the W3C generator
    }

    fn clean(&self) {
        if self.output_dir.exists() {
            fs::remove_dir_all(&self.output_dir).ok();
            println!("Cleaned: {}", self.output_dir.display());
        }
    }
}

// ── Subcommand: fix-scxml-name ──────────────────────────────────

fn cmd_fix_scxml_name(scxml_path: &str, name: &str) {
    let content = fs::read_to_string(scxml_path).unwrap_or_else(|e| {
        eprintln!("Cannot read {scxml_path}: {e}");
        std::process::exit(1);
    });

    // Find first <scxml...> tag
    let re_scxml = regex::Regex::new(r"(?s)<scxml[^>]*?>").unwrap();
    let fixed = match re_scxml.find(&content) {
        Some(m) => {
            let first_scxml = m.as_str();
            // Remove existing name attribute
            let re_name = regex::Regex::new(r#"\s+name="[^"]*""#).unwrap();
            let cleaned = re_name.replace(first_scxml, "").to_string();
            // Add new name attribute
            let with_name = cleaned.replacen("<scxml", &format!("<scxml name=\"{name}\""), 1);
            format!("{}{}{}", &content[..m.start()], with_name, &content[m.end()..])
        }
        None => {
            eprintln!("No <scxml> tag found in {scxml_path}");
            std::process::exit(1);
        }
    };

    fs::write(scxml_path, fixed).unwrap_or_else(|e| {
        eprintln!("Cannot write {scxml_path}: {e}");
        std::process::exit(1);
    });
}

// ── Subcommand: read-metadata ───────────────────────────────────

fn cmd_read_metadata(metadata_file: &str) {
    let content = fs::read_to_string(metadata_file).unwrap_or_else(|e| {
        eprintln!("ERROR: Cannot read {metadata_file}: {e}");
        std::process::exit(1);
    });

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("description:") {
            let description = line.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");
            println!("{description}");
            return;
        }
    }

    eprintln!("ERROR: No description field found in {metadata_file}");
    std::process::exit(1);
}

// ── Subcommand: manifest ───────────────────────────────────────

fn cmd_manifest(dir: &str) {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        eprintln!("ERROR: Not a directory: {dir}");
        std::process::exit(1);
    }

    let manifest = sce_build::build_forge_manifest(dir_path).unwrap_or_else(|e| {
        eprintln!("ERROR: {e}");
        std::process::exit(1);
    });

    let json = serde_json::to_string_pretty(&manifest).unwrap_or_else(|e| {
        eprintln!("ERROR: JSON serialization failed: {e}");
        std::process::exit(1);
    });

    println!("{json}");
}

// ── Subcommand: generate-conformance ───────────────────────────

fn cmd_generate_conformance(language: &str, manifest_path: &str, output_dir: &str) {
    let lang: Language = language.parse().unwrap_or_else(|_| {
        eprintln!("Unknown language: {language}. Use rust, cpp, kotlin, go, or python.");
        std::process::exit(1);
    });

    let manifest = sce_build::conformance::Manifest::load(Path::new(manifest_path))
        .unwrap_or_else(|e| {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        });

    let template_base = sce_build::find_template_base();
    // Resource dir is the sibling of the manifest's parent (manifest lives at
    // tests/forge/conformance/, SCXML files at tests/forge/resources/).
    let resource_dir = Path::new(manifest_path)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("resources"))
        .unwrap_or_else(|| {
            eprintln!("ERROR: cannot derive resource_dir from manifest path {manifest_path}");
            std::process::exit(1);
        });
    let rendered =
        sce_build::conformance::render_harness(&manifest, lang, &template_base, &resource_dir)
            .unwrap_or_else(|e| {
                eprintln!("ERROR: {e}");
                std::process::exit(1);
            });

    let out_dir = Path::new(output_dir);
    fs::create_dir_all(out_dir).unwrap_or_else(|e| {
        eprintln!("ERROR: create {}: {e}", out_dir.display());
        std::process::exit(1);
    });
    let out_path = out_dir.join(sce_build::conformance::harness_filename(lang));
    fs::write(&out_path, rendered).unwrap_or_else(|e| {
        eprintln!("ERROR: write {}: {e}", out_path.display());
        std::process::exit(1);
    });
    println!("Generated conformance harness: {}", out_path.display());
}

// ── Subcommand: list-fixtures ──────────────────────────────────

fn cmd_list_fixtures(manifest_path: &str, format: &str) {
    let manifest = sce_build::conformance::Manifest::load(Path::new(manifest_path))
        .unwrap_or_else(|e| {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        });
    let names: Vec<&str> = manifest.fixtures.iter().map(|f| f.name.as_str()).collect();
    match format {
        "plain" => {
            for n in &names {
                println!("{n}");
            }
        }
        "cmake" => println!("{}", names.join(";")),
        "space" => println!("{}", names.join(" ")),
        other => {
            eprintln!("ERROR: unknown --format {other}; expected plain|cmake|space");
            std::process::exit(1);
        }
    }
}

// ── Utility functions ───────────────────────────────────────────

/// Resolve SCXML source path to project-relative path (delegates to lib).
fn resolve_source_path(model: &mut SCXMLModel, scxml_path: &Path) {
    sce_build::resolve_source_path(model, scxml_path.to_str().unwrap_or(""));
}

/// Write file only if content differs (preserves timestamps).
fn write_if_changed(path: &Path, content: &str) -> bool {
    if path.exists() {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == content {
                return false;
            }
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("Cannot write {}: {e}", path.display());
        std::process::exit(1);
    });
    true
}

impl TestInfo {
    fn type_str(&self) -> &str {
        &self.test_type
    }
}
