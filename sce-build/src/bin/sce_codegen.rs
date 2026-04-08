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
    },
    /// Batch generate W3C test state machines and test classes
    GenerateW3c {
        /// Target language (kotlin, cpp, go)
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
        } => cmd_generate(&scxml, &language, &output_dir, as_child, write_deps.as_deref()),
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
    }
}

// ── Subcommand: generate ────────────────────────────────────────

fn cmd_generate(scxml_path: &str, language: &str, output_dir: &str, as_child: bool, depfile_path: Option<&str>) {
    let lang: Language = language.parse().unwrap_or_else(|_| {
        eprintln!("Unknown language: {language}. Use rust, cpp, kotlin, or go.");
        std::process::exit(1);
    });

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
    // This is correct spec behavior — generate a rejection stub so AOT test reports PASS.
    // The stub defines SCE_DOCUMENT_REJECTED and a minimal namespace/class that
    // RejectedDocumentTest (base class) uses to auto-pass the test.
    if model.document_rejected {
        let input_stem = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let out = Path::new(output_dir);
        let pascal = crate::filters::to_pascal_case(input_stem.to_string());
        let header = format!(
            "// W3C SCXML 5.8: Document rejected — processor correctly refused non-conformant document\n\
             #pragma once\n\
             #define SCE_DOCUMENT_REJECTED 1\n\
             namespace SCE::Generated::{name} {{\n\
             struct {pascal} {{\n\
             }};\n\
             }}  // namespace SCE::Generated::{name}\n",
            name = input_stem, pascal = pascal
        );
        let inl = "// W3C SCXML 5.8: Document rejected — no executable content generated\n";
        std::fs::write(out.join(format!("{input_stem}_sm.h")), &header)
            .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
        std::fs::write(out.join(format!("{input_stem}_sm.inl")), inl)
            .unwrap_or_else(|e| { eprintln!("Write error: {e}"); std::process::exit(1); });
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

    // Write children metadata and generate hybrid child SCXML stubs (C++ pipeline)
    if lang == Language::Cpp {
        let children = collect_children_names(&model);
        if !children.is_empty() {
            let children_file = out_path.join(format!("{input_stem}_children.txt"));
            fs::write(&children_file, children.join("\n") + "\n").unwrap_or_else(|e| {
                eprintln!("Cannot write children file: {e}");
                std::process::exit(1);
            });
        }
        // W3C SCXML 6.4: Generate SCXML stubs for hybrid invoke children (srcexpr/contentexpr)
        // Matches Python cpp_generator.py behavior: scan resource dir or create trivial stub
        generate_hybrid_child_scxmls(&model, Path::new(scxml_path), out_path);
    }

    // Write DEPFILE for CMake incremental builds
    if let Some(depfile) = depfile_path {
        write_depfile(depfile, &output_paths, &template_dir, lang, Path::new(scxml_path));
    }
}

/// Collect child SCXML names from model's static/hybrid invokes.
fn collect_children_names(model: &SCXMLModel) -> Vec<String> {
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
        let _contentexpr = invoke.contentexpr.as_deref().unwrap_or("");

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
        eprintln!("Unknown language: {language}. Use kotlin, cpp, or go.");
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

    match lang {
        Language::Kotlin => generate_w3c_kotlin(&project_root, &resources_dir, &cmake_file, single_test, clean, list),
        Language::Cpp => generate_w3c_cpp(&project_root, &resources_dir, &cmake_file, single_test, clean, list),
        Language::Go => generate_w3c_go(&project_root, &resources_dir, &cmake_file, single_test, clean, list),
        Language::Rust => {
            eprintln!("Rust W3C generation not yet supported via generate-w3c. Use sce-rust-tests build.rs instead.");
            std::process::exit(1);
        }
    }
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

// ── Kotlin W3C Generation ───────────────────────────────────────

fn generate_w3c_kotlin(
    project_root: &Path,
    resources_dir: &Path,
    cmake_file: &Path,
    single_test: Option<&str>,
    clean: bool,
    list: bool,
) {
    let tests_module = project_root.join("sce-kotlin-tests");
    let sm_output_base = tests_module.join("src/main/kotlin/com/sce/generated");
    let test_output_dir = tests_module.join("src/test/kotlin/com/sce/w3c");
    let template_dir = sce_build::find_template_dir_for(Language::Kotlin);

    if clean {
        if sm_output_base.exists() {
            fs::remove_dir_all(&sm_output_base).ok();
            println!("Cleaned: {}", sm_output_base.display());
        }
        for entry in fs::read_dir(&test_output_dir).into_iter().flatten() {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("Test") && name.ends_with(".kt") {
                    fs::remove_file(entry.path()).ok();
                }
            }
        }
        println!("Cleaned test classes in: {}", test_output_dir.display());
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

        // Generate parent SM
        let (model, generated_ok) = match generate_kotlin_sm(test_id, &scxml_path, &sm_output_base, &template_dir) {
            Ok(result) => result,
            Err(e) => {
                failed.push((test_id.clone(), format!("codegen failed: {e}")));
                continue;
            }
        };

        let needs_script = model.needs_script_engine;

        if !generated_ok {
            failed.push((test_id.clone(), "codegen failed".to_string()));
            continue;
        }

        // Generate child SMs
        generate_kotlin_child_sms(test_id, &scxml_path, &sm_output_base, &template_dir);

        // Detect pass state from model
        let pass_state = detect_pass_state(&model);
        if pass_state.is_none() {
            failed.push((test_id.clone(), "pass state not detected".to_string()));
            continue;
        }

        // Check HTTP send from model
        let metadata = read_metadata(resources_dir, test_id);
        let test_type = cmake_tests.get(test_id.as_str()).map(|i| i.test_type.as_str()).unwrap_or("SIMPLE");
        let uses_http = model_uses_http_send(&model);

        // Generate test class
        let test_written = generate_kotlin_test_class(
            test_id,
            &metadata,
            test_type,
            needs_script,
            &pass_state.unwrap(),
            uses_http,
            &test_output_dir,
        );

        if test_written {
            if needs_script {
                generated_script.push(test_id.clone());
            } else {
                generated_static.push(test_id.clone());
            }
        } else {
            failed.push((test_id.clone(), "test class generation failed".to_string()));
        }
    }

    // Clean stale files (full generation mode only)
    let mut stale_removed = 0;
    if single_test.is_none() {
        let valid_ids: BTreeSet<String> = generated_static.iter().chain(generated_script.iter()).cloned().collect();
        if !valid_ids.is_empty() {
            stale_removed = clean_stale_kotlin(&sm_output_base, &test_output_dir, &valid_ids);
        }
    }

    // Summary
    let total_generated = generated_static.len() + generated_script.len();
    println!("\n{}", "=".repeat(60));
    println!("Kotlin W3C Test Generation Summary");
    println!("{}", "=".repeat(60));
    println!("  Generated (pure static):    {}", generated_static.len());
    println!("  Generated (script engine):  {}", generated_script.len());
    println!("  Generated (total):          {total_generated}");
    println!("  Skipped (other):            {}", skipped.len());
    println!("  Failed:                     {}", failed.len());
    println!("  Stale removed:              {stale_removed}");
    println!("  Total:                      {}", test_ids.len());

    if !skipped.is_empty() {
        println!("\nSkipped (other):");
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
        println!("\nGenerated SM classes: {}", sm_output_base.display());
        println!("Generated test classes: {}", test_output_dir.display());
        println!("\nGenerated test IDs (static): {}", generated_static.join(" "));
        if !generated_script.is_empty() {
            println!("Generated test IDs (script): {}", generated_script.join(" "));
        }
    }

    if !failed.is_empty() {
        std::process::exit(1);
    }
}

/// Generate Kotlin state machine for a single test.
/// Returns (model, success) so caller can inspect model flags.
fn generate_kotlin_sm(
    test_id: &str,
    scxml_path: &Path,
    sm_output_base: &Path,
    template_dir: &Path,
) -> Result<(SCXMLModel, bool), String> {
    let output_dir = sm_output_base.join(format!("test{test_id}"));

    let mut parser = SCXMLParser::new();
    let mut model = parser.parse_file(scxml_path.to_str().unwrap_or(""))?;
    analyzer::analyze(&mut model, scxml_path.to_str().unwrap_or(""));

    if !analyzer::can_generate_static(&model) {
        return Ok((model, false));
    }

    resolve_source_path(&mut model, scxml_path);

    let code = sce_build::generator::generate_kotlin(&model, template_dir)?;

    let input_stem = scxml_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    fs::create_dir_all(&output_dir).map_err(|e| format!("Cannot create dir: {e}"))?;

    let sm_file = output_dir.join(format!("{input_stem}Sm.kt"));
    write_if_changed(&sm_file, &code);

    Ok((model, true))
}

/// Generate child state machines for static invoke tests.
fn generate_kotlin_child_sms(
    test_id: &str,
    scxml_path: &Path,
    sm_output_base: &Path,
    template_dir: &Path,
) {
    let resource_dir = scxml_path.parent().unwrap_or(Path::new("."));
    let parent_stem = scxml_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let parent_package = format!("test{test_id}");
    let output_dir = sm_output_base.join(&parent_package);
    let num_prefix = extract_num_prefix(test_id);

    // Read parent SM to check which child classes are referenced
    let parent_sm_file = output_dir.join(format!("{parent_stem}Sm.kt"));
    let parent_sm_content = fs::read_to_string(&parent_sm_file).unwrap_or_default();

    // Find child SCXML files
    let entries = fs::read_dir(resource_dir).into_iter().flatten();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("scxml") {
            continue;
        }
        let child_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip parent itself
        if child_name == parent_stem {
            continue;
        }

        // Match: test191_child0, test226sub1, etc.
        if !child_name.starts_with(&format!("test{num_prefix}_"))
            && !child_name.starts_with(&format!("test{num_prefix}sub"))
        {
            continue;
        }

        // Skip if parent SM doesn't reference this child class (hybrid invoke)
        let child_class = to_pascal_case(&child_name);
        if !parent_sm_content.contains(&format!("{child_class}StateMachine")) {
            continue;
        }

        // Generate child SM
        let mut parser = SCXMLParser::new();
        let child_path_str = path.to_str().unwrap_or("");
        let child_result = parser.parse_file(child_path_str);

        match child_result {
            Ok(mut child_model) => {
                analyzer::analyze(&mut child_model, child_path_str);

                if !analyzer::can_generate_static(&child_model) {
                    generate_kotlin_child_stub(&child_name, &parent_package, &child_class, &output_dir);
                    continue;
                }

                resolve_source_path(&mut child_model, &path);

                match sce_build::generator::generate_kotlin(&child_model, template_dir) {
                    Ok(code) => {
                        let child_sm_file = output_dir.join(format!("{child_name}Sm.kt"));
                        // Fix package: child SM must be in same package as parent
                        let child_package = child_name.to_lowercase();
                        let fixed_code = code.replace(
                            &format!("package com.sce.generated.{child_package}"),
                            &format!("package com.sce.generated.{parent_package}"),
                        );
                        write_if_changed(&child_sm_file, &fixed_code);
                    }
                    Err(_) => {
                        generate_kotlin_child_stub(&child_name, &parent_package, &child_class, &output_dir);
                    }
                }
            }
            Err(_) => {
                generate_kotlin_child_stub(&child_name, &parent_package, &child_class, &output_dir);
            }
        }
    }
}

/// Generate a no-op stub for a child SM that failed codegen.
fn generate_kotlin_child_stub(child_name: &str, parent_package: &str, child_class: &str, output_dir: &Path) {
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
    let child_sm_file = output_dir.join(format!("{child_name}Sm.kt"));
    write_if_changed(&child_sm_file, &stub);
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

/// Generate Kotlin JUnit5 test class for a W3C test.
fn generate_kotlin_test_class(
    test_id: &str,
    metadata: &TestMetadata,
    test_type: &str,
    needs_script_engine: bool,
    pass_state: &str,
    uses_http: bool,
    test_output_dir: &Path,
) -> bool {
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

    let create_sm = if needs_script_engine {
        format!("    override fun createStateMachine() = {sm_class}StateMachine(createEngine())\n")
    } else {
        format!("    override fun createStateMachine() = {sm_class}StateMachine()\n")
    };

    let content = format!(
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
    );

    fs::create_dir_all(test_output_dir).ok();
    let test_file = test_output_dir.join(format!("{sm_class}.kt"));
    write_if_changed(&test_file, &content);
    true
}

/// Clean stale generated files for tests no longer in registry.
fn clean_stale_kotlin(sm_output_base: &Path, test_output_dir: &Path, valid_ids: &BTreeSet<String>) -> usize {
    let mut removed = 0;

    // Clean stale SM directories
    if sm_output_base.exists() {
        for entry in fs::read_dir(sm_output_base).into_iter().flatten().flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("test") {
                continue;
            }
            let dir_test_id = &name[4..]; // strip "test" prefix
            if !valid_ids.contains(dir_test_id) {
                fs::remove_dir_all(entry.path()).ok();
                println!("  Removed stale SM dir: {name}");
                removed += 1;
            }
        }
    }

    // Clean stale test classes
    if test_output_dir.exists() {
        let valid_lower: BTreeSet<String> = valid_ids.iter().map(|s| s.to_lowercase()).collect();
        for entry in fs::read_dir(test_output_dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("Test") || !name.ends_with(".kt") {
                continue;
            }
            if name == "W3CTestBase.kt" || name == "W3CHttpTestBase.kt" {
                continue;
            }
            let stem = &name[..name.len() - 3]; // strip ".kt"
            let file_test_id = &stem[4..]; // strip "Test"
            if !valid_ids.contains(file_test_id) && !valid_lower.contains(&file_test_id.to_lowercase()) {
                fs::remove_file(entry.path()).ok();
                println!("  Removed stale test: {name}");
                removed += 1;
            }
        }
    }

    removed
}

// ── C++ W3C Generation ──────────────────────────────────────────

fn generate_w3c_cpp(
    project_root: &Path,
    resources_dir: &Path,
    cmake_file: &Path,
    single_test: Option<&str>,
    clean: bool,
    list: bool,
) {
    let output_dir = project_root.join("build/tests/w3c_static_generated");
    let cmake_tests = parse_cmake_tests(cmake_file);
    let template_dir = sce_build::find_template_dir_for(Language::Cpp);

    if clean {
        if output_dir.exists() {
            fs::remove_dir_all(&output_dir).ok();
            println!("Cleaned: {}", output_dir.display());
        }
        return;
    }

    if list {
        println!("C++ test registry: {} tests", cmake_tests.len());
        for (tid, info) in &cmake_tests {
            let scxml = find_scxml(resources_dir, tid);
            let status = if scxml.is_some() { "OK" } else { "MISSING" };
            println!("  {tid:6} [{:9}] {status} -- {}", info.type_str(), info.comment);
        }
        return;
    }

    fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
        eprintln!("Cannot create output directory: {e}");
        std::process::exit(1);
    });

    let test_ids: Vec<String> = if let Some(tid) = single_test {
        vec![tid.to_string()]
    } else {
        cmake_tests.keys().cloned().collect()
    };

    let mut generated = 0;
    let mut failed = 0;

    for test_id in &test_ids {
        let scxml_path = match find_scxml(resources_dir, test_id) {
            Some(p) => p,
            None => {
                eprintln!("  SCXML not found for test {test_id}");
                failed += 1;
                continue;
            }
        };

        let mut parser = SCXMLParser::new();
        let scxml_str = scxml_path.to_str().unwrap_or("");
        match parser.parse_file(scxml_str) {
            Ok(mut model) => {
                analyzer::analyze(&mut model, scxml_str);
                if !analyzer::can_generate_static(&model) {
                    eprintln!("  Cannot generate static code for test {test_id}");
                    failed += 1;
                    continue;
                }

                resolve_source_path(&mut model, &scxml_path);

                let input_stem = scxml_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                match sce_build::generator::generate_cpp(&model, &template_dir, input_stem) {
                    Ok(output) => {
                        for (filename, code) in &output.files {
                            let file_path = output_dir.join(filename);
                            write_if_changed(&file_path, code);
                            println!("  Generated: {filename}");
                        }
                        generated += 1;
                    }
                    Err(e) => {
                        eprintln!("  C++ codegen failed for test {test_id}: {e}");
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("  Parse error for test {test_id}: {e}");
                failed += 1;
            }
        }
    }

    println!("\nC++ W3C Generation: {generated} generated, {failed} failed");
    if failed > 0 {
        std::process::exit(1);
    }
}

// ── Go child SM generation ────────────────────────────────────

/// Generate child state machines for Go invoke tests.
/// Port of generate_kotlin_child_sms() adapted for Go package conventions.
fn generate_go_child_sms(
    test_id: &str,
    scxml_path: &Path,
    test_dir: &Path,
    template_dir: &Path,
    parent_code: &str,
) {
    let resource_dir = scxml_path.parent().unwrap_or(Path::new("."));
    let parent_stem = scxml_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let parent_package = format!("test{test_id}");
    let num_prefix = extract_num_prefix(test_id);

    // Find child SCXML files in the resource directory
    let entries = match fs::read_dir(resource_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("scxml") {
            continue;
        }
        let child_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip parent itself
        if child_name == parent_stem {
            continue;
        }

        // Match: test191_child0, test226sub1, test338_machineName, etc.
        if !child_name.starts_with(&format!("test{num_prefix}_"))
            && !child_name.starts_with(&format!("test{num_prefix}sub"))
            && !child_name.starts_with(&format!("test{test_id}_"))
        {
            continue;
        }

        // Check if parent SM references this child's type directly
        let child_machine = to_pascal_case(&child_name);
        let is_directly_referenced = parent_code.contains(&format!("{child_machine}Policy"))
            || parent_code.contains(&format!("{child_machine}State"));

        // Also check if this is a hybrid invoke child (named as testNNN_hybridM)
        // For hybrid invokes, the SCXML name might not match — we need to find the right mapping
        // by checking if any hybrid invoke references this file via srcexpr
        let is_hybrid_child = !is_directly_referenced && parent_code.contains("Hybrid");

        if !is_directly_referenced && !is_hybrid_child {
            continue;
        }

        // For hybrid children, rename to match the expected hybrid name
        let (effective_child_name, effective_child_machine) = if is_directly_referenced {
            (child_name.clone(), child_machine.clone())
        } else {
            // Find the hybrid index by checking which hybrid type is referenced
            let mut found = None;
            for i in 0..10 {
                let hybrid_name = format!("test{num_prefix}_hybrid{i}");
                let hybrid_machine = to_pascal_case(&hybrid_name);
                if parent_code.contains(&format!("{hybrid_machine}Policy")) {
                    found = Some((hybrid_name, hybrid_machine));
                    break;
                }
            }
            match found {
                Some(f) => f,
                None => continue,
            }
        };

        // Parse and generate child SM
        let mut parser = SCXMLParser::new();
        let child_path_str = path.to_str().unwrap_or("");
        match parser.parse_file(child_path_str) {
            Ok(mut child_model) => {
                analyzer::analyze(&mut child_model, child_path_str);
                resolve_source_path(&mut child_model, &path);

                // Override the model name for hybrid children so types match parent expectations
                if effective_child_name != child_name {
                    child_model.name = effective_child_name.clone();
                }

                match sce_build::generator::generate_go(&child_model, template_dir) {
                    Ok(code) => {
                        // Fix package name to match parent's package
                        let child_pkg = sce_build::filters::to_snake_case(child_model.name.clone());
                        let fixed_code = code.replace(
                            &format!("package {child_pkg}"),
                            &format!("package {parent_package}"),
                        );
                        let child_file = test_dir.join(format!("{effective_child_name}_sm.go"));
                        write_if_changed(&child_file, &fixed_code);
                    }
                    Err(e) => {
                        eprintln!("  Go child codegen failed for {child_name}: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("  Go child parse error for {child_name}: {e}");
            }
        }
    }
}

// ── Go W3C Generation ──────────────────────────────────────────

fn generate_w3c_go(
    project_root: &Path,
    resources_dir: &Path,
    cmake_file: &Path,
    single_test: Option<&str>,
    clean: bool,
    list: bool,
) {
    let tests_module = project_root.join("sce-go-tests");
    let generated_dir = tests_module.join("generated");
    let template_dir = sce_build::find_template_dir_for(Language::Go);

    if clean {
        if generated_dir.exists() {
            fs::remove_dir_all(&generated_dir).ok();
            println!("Cleaned: {}", generated_dir.display());
        }
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

                // W3C SCXML 5.8: document_rejected models have initial→pass already
                // redirected by the parser, so they CAN be generated. Only skip
                // truly dynamic models.
                if !analyzer::can_generate_static(&model) && !model.document_rejected {
                    skipped.push((test_id.clone(), "dynamic features".to_string()));
                    continue;
                }

                resolve_source_path(&mut model, &scxml_path);

                let input_stem = scxml_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

                match sce_build::generator::generate_go(&model, &template_dir) {
                    Ok(code) => {
                        let test_dir = generated_dir.join(format!("test{test_id}"));
                        fs::create_dir_all(&test_dir).unwrap_or_else(|e| {
                            eprintln!("Cannot create dir: {e}");
                            std::process::exit(1);
                        });

                        let sm_file = test_dir.join(format!("{input_stem}_sm.go"));
                        write_if_changed(&sm_file, &code);

                        // W3C SCXML 6.4: Generate child state machines for invoke tests
                        generate_go_child_sms(test_id, &scxml_path, &test_dir, &template_dir, &code);

                        // Detect pass state and generate test file
                        let pass_state = detect_pass_state(&model);
                        if let Some(ref pass) = pass_state {
                            let metadata = read_metadata(resources_dir, test_id);
                            let test_type = cmake_tests.get(test_id.as_str()).map(|i| i.test_type.as_str()).unwrap_or("SIMPLE");
                            let uses_http = model_uses_http_send(&model);
                            let needs_script = model.needs_script_engine;
                            let machine = to_pascal_case(input_stem);

                            let test_code = generate_go_test_file(
                                test_id, input_stem, &machine, pass, needs_script, uses_http, test_type, &metadata,
                            );
                            let test_file = test_dir.join(format!("{input_stem}_test.go"));
                            write_if_changed(&test_file, &test_code);

                            if needs_script {
                                generated_script.push(test_id.clone());
                            } else {
                                generated_static.push(test_id.clone());
                            }
                        } else {
                            failed.push((test_id.clone(), "pass state not detected".to_string()));
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

    // Summary
    let total_generated = generated_static.len() + generated_script.len();
    println!("\n{}", "=".repeat(60));
    println!("Go W3C Test Generation Summary");
    println!("{}", "=".repeat(60));
    println!("  Generated (pure static):    {}", generated_static.len());
    println!("  Generated (script engine):  {}", generated_script.len());
    println!("  Generated (total):          {total_generated}");
    println!("  Skipped:                    {}", skipped.len());
    println!("  Failed:                     {}", failed.len());
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
        println!("\nGenerated files: {}", generated_dir.display());
    }

    if !failed.is_empty() {
        std::process::exit(1);
    }
}

/// Generate Go test file for a W3C test.
fn generate_go_test_file(
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
        "\n\tengine.EnableHTTPLoopback()\n"
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
