// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart's map or set variable (docs/adr/0108, D1 to D5), judged by the
//! `sce-static` checker: `check` accepts a well-formed document
//! (`fixtures/map/map_faces.scxml`, which writes every read, statement and loop
//! the checker reads once), and refuses each read, statement and loop that is
//! no use of a map by the code D11 names, saying what is accepted. The
//! Interpreter's lowering, the first to hold a map ("Order of work", step 4),
//! lowers the document, Rust's `generate`, the second (step 5), and C11's, the
//! third (step 6), generate it, and every other generated lane's `generate`
//! refuses it by name, naming the record that lands a map lane by lane (D10).
//!
//! Each refusal is the well-formed document with one thing changed, so the
//! refusal is the change's and nothing else's.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/map")
}

/// The schemas the statechart imports, copied beside a variant of it.
const SIBLINGS: [&str; 2] = ["map_peer_key.scxml", "map_face.scxml"];

/// The well-formed statechart, as committed.
fn faces() -> String {
    std::fs::read_to_string(fixtures().join("map_faces.scxml")).expect("read the statechart")
}

/// `text` with the one occurrence of `from` replaced by `to`.
fn edited(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "`{from}` stands once in the statechart"
    );
    text.replacen(from, to, 1)
}

/// `sce-codegen --error-format=json <args> <path>`: whether it passed, and
/// what it printed.
fn run(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `check` of `document`, written beside the schemas it imports.
fn check(document: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    let path = dir.path().join("map_faces.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    run(&["check"], &path)
}

/// `document` is refused by `check` with `code`, its message holding `part`.
fn refused(document: &str, code: &str, part: &str) {
    let (ok, out) = check(document);
    assert!(!ok, "expected a refusal holding `{part}`:\n{out}");
    assert!(
        out.contains(&format!("\"code\":\"{code}\"")) && out.contains(part),
        "expected {code} holding `{part}`:\n{out}"
    );
}

/// docs/adr/0108, D3 to D5 and D10: the document that writes every read,
/// statement and loop of a map the checker reads is accepted by `check`,
/// refused by name by every lane that generates a statechart but Rust and C11,
/// and lowered by the Interpreter's lowering, every statement of a map replaced
/// and every read of one a call of the library.
#[test]
fn a_map_document_is_checked_and_refused_by_every_lane_by_name() {
    let path = fixtures().join("map_faces.scxml");
    let (ok, out) = run(&["check"], &path);
    assert!(ok, "check accepts the statechart:\n{out}");
    let (ok, out) = check(&faces());
    assert!(ok, "check accepts it beside copies of its schemas:\n{out}");

    let place = "`map<record:PeerKey, record:Face>[face_max=4]` in a statechart's variable";
    for &language in sce_build::generator::Language::ALL {
        // Rust holds a map ("Order of work", step 5), and C11 (step 6), as the
        // tests below hold.
        if matches!(
            language,
            sce_build::generator::Language::Rust | sce_build::generator::Language::C11
        ) {
            continue;
        }
        let lane = language.canonical_name();
        let out_dir = tempfile::tempdir().expect("an output directory");
        let out_dir = out_dir.path().to_str().expect("a utf-8 path").to_string();
        let mut args = vec!["generate", "-l", lane, "-o", out_dir.as_str()];
        if language == sce_build::generator::Language::Go {
            args.extend(["--go-module-prefix", "example.test/generated"]);
        }
        let (ok, out) = run(&args, &path);
        assert!(!ok, "{lane} generates no map yet:\n{out}");
        assert!(
            out.contains("\"code\":\"generate/unsupported-feature\"")
                && out.contains(place)
                && out.contains(&format!(
                    "has no {lane} lowering of its map yet (docs/adr/0108, D10)"
                )),
            "{lane}:\n{out}"
        );
    }

    // The Interpreter's lowering writes the document, which holds the
    // statechart's every map as the array of its entries.
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|refusal| panic!("the Interpreter lowers a map: {refusal:?}"));
    for held in [
        r#"<data id="faces" expr="[]"/>"#,
        "SceStatic.mapPut(faces, {key: {record: [[&apos;low&apos;, &apos;integer&apos;], \
         [&apos;high&apos;, &apos;integer&apos;]]}}, 4, peer, face)",
        "SceStatic.mapHas(faces, ",
        "SceStatic.mapGet(faces, ",
        "SceStatic.mapRemove(faces, ",
        "SceStatic.mapPutOrFail(byId, ",
        "faces.length",
        "f[1].id",
        "f[0].low",
    ] {
        assert!(lowered.contains(held), "no `{held}` in:\n{lowered}");
    }
    // Asked of the elements, not the text: the fixture's own comment, which the
    // lowering keeps as the author wrote it, names each statement.
    const SCE: &str = "http://sce.dev/ext";
    let document = roxmltree::Document::parse(&lowered).expect("the lowered document is XML");
    for element in document.descendants().filter(|node| node.is_element()) {
        let name = element.tag_name();
        assert!(
            !(name.namespace() == Some(SCE) && ["put", "remove", "clear"].contains(&name.name())),
            "<sce:{}> is left in the document the Interpreter runs:\n{lowered}",
            name.name()
        );
        assert!(
            element.attribute((SCE, "key")).is_none(),
            "a <{}> keeps its sce:key in the document the Interpreter runs:\n{lowered}",
            name.name()
        );
    }
}

/// The Rust lowering of `document`, written beside the schemas it imports:
/// whether `generate` passed, what it printed, and every Rust file it wrote,
/// by name.
fn generate_rust_files(document: &str) -> (bool, String, Vec<(String, String)>) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    let path = dir.path().join("map_faces.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "rust", "-o", out.as_str()], &path);
    (ok, printed, rust_files_in(out_dir.path()))
}

/// Every Rust file `dir` holds, by name.
fn rust_files_in(dir: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read the output") {
        let path = entry.expect("an output file").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a utf-8 file name")
                .to_string();
            files.push((
                name,
                std::fs::read_to_string(&path).expect("read a generated file"),
            ));
        }
    }
    files
}

/// [`generate_rust_files`], every file joined.
fn generate_rust(document: &str) -> (bool, String, String) {
    let (ok, printed, files) = generate_rust_files(document);
    let code = files.into_iter().map(|(_, text)| text).collect();
    (ok, printed, code)
}

/// What a generated Rust machine declares, read off its syntax tree: the type
/// of each field of its structs by name, each function by name with its
/// parameters' types and its return type, and each `use` it writes, each type
/// and path as its tokens print.
#[derive(Default)]
struct Declared {
    fields: Vec<(String, syn::Type)>,
    functions: Vec<(String, Vec<String>, String)>,
    uses: Vec<String>,
}

impl<'ast> syn::visit::Visit<'ast> for Declared {
    fn visit_field(&mut self, field: &'ast syn::Field) {
        if let Some(ident) = &field.ident {
            self.fields.push((ident.to_string(), field.ty.clone()));
        }
        syn::visit::visit_field(self, field);
    }
    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        use quote::ToTokens;
        let params = function
            .sig
            .inputs
            .iter()
            .filter_map(|input| match input {
                syn::FnArg::Typed(typed) => Some(typed.ty.to_token_stream().to_string()),
                syn::FnArg::Receiver(_) => None,
            })
            .collect();
        let output = match &function.sig.output {
            syn::ReturnType::Type(_, ty) => ty.to_token_stream().to_string(),
            syn::ReturnType::Default => String::new(),
        };
        self.functions
            .push((function.sig.ident.to_string(), params, output));
        syn::visit::visit_item_fn(self, function);
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        use quote::ToTokens;
        self.uses.push(item.tree.to_token_stream().to_string());
        syn::visit::visit_item_use(self, item);
    }
}

/// The type arguments of `ty`, a path whose last segment is `BoundedMap`, each
/// as its tokens print; `None` for any other type.
fn bounded_map_arguments(ty: &syn::Type) -> Option<Vec<String>> {
    use quote::ToTokens;
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    if last.ident != "BoundedMap" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    Some(
        arguments
            .args
            .iter()
            .map(|argument| argument.to_token_stream().to_string())
            .collect(),
    )
}

/// The machine among `files`, read off its syntax tree ([`Declared`]), with
/// the path of every function it calls ([`Called`]) and its text.
fn declared_machine(files: &[(String, String)]) -> (Declared, Called, String) {
    let (_, machine) = files
        .iter()
        .find(|(name, _)| name.ends_with("_sm.rs"))
        .unwrap_or_else(|| panic!("no machine among the generated files: {files:?}"));
    let tree = syn::parse_file(machine).expect("the generated machine parses as Rust");
    let mut declared = Declared::default();
    syn::visit::Visit::visit_file(&mut declared, &tree);
    let mut called = Called::default();
    syn::visit::Visit::visit_file(&mut called, &tree);
    (declared, called, machine.clone())
}

impl Declared {
    /// The type of the field `name` the machine holds a variable in, a type
    /// named by its path: a view a host is lent of the same variable is a
    /// reference to it. `machine` is the text a failure shows.
    fn field(&self, name: &str, machine: &str) -> syn::Type {
        self.fields
            .iter()
            .find(|(field, ty)| field == name && matches!(ty, syn::Type::Path(_)))
            .map(|(_, ty)| ty.clone())
            .unwrap_or_else(|| panic!("the machine declares no field `{name}`:\n{machine}"))
    }

    /// The name of the function from the type `from` to the type `to`, each as
    /// its tokens print.
    fn function(&self, from: &str, to: &str, machine: &str) -> String {
        self.functions
            .iter()
            .find(|(_, params, output)| params == &[from.to_string()] && output == to)
            .map(|(name, _, _)| name.clone())
            .unwrap_or_else(|| panic!("no function from `{from}` to `{to}`:\n{machine}"))
    }
}

/// The path of each function a generated Rust machine calls by its path, as
/// its tokens print.
#[derive(Default)]
struct Called(Vec<String>);

impl<'ast> syn::visit::Visit<'ast> for Called {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        use quote::ToTokens;
        if let syn::Expr::Path(path) = call.func.as_ref() {
            self.0.push(path.path.to_token_stream().to_string());
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// docs/adr/0108, D6 item 4 and "Order of work", step 7: the Rust machine
/// holds a map's record key and record value in the types their units declare
/// (`<schema>::<Schema>Payload`), the types an algorithm's map slot holds, so a
/// map is lent to an algorithm whole. A machine that calls no algorithm names
/// each unit all the same, and declares the functions its own record crosses
/// into the unit's and back by, which a `<sce:put>` of a record variable and a
/// value taken whole out of the map call. Read off the syntax tree of the
/// generated machine, not its text.
#[test]
fn rust_holds_a_maps_record_entries_in_their_units_types() {
    use quote::ToTokens;
    let (ok, printed, files) = generate_rust_files(&faces());
    assert!(ok, "Rust generates a map:\n{printed}");
    let (declared, called, machine) = declared_machine(&files);
    let faces = bounded_map_arguments(&declared.field("faces", &machine))
        .unwrap_or_else(|| panic!("`faces` is no BoundedMap:\n{machine}"));
    assert_eq!(
        faces[..2],
        [
            "map_peer_key :: MapPeerKeyPayload".to_string(),
            "map_face :: MapFacePayload".to_string()
        ],
        "`faces` holds its key and its value in their units' types:\n{machine}"
    );
    let by_id = bounded_map_arguments(&declared.field("by_id", &machine))
        .unwrap_or_else(|| panic!("`byId` is no BoundedMap:\n{machine}"));
    assert_eq!(
        by_id[..2],
        ["u32".to_string(), "map_face :: MapFacePayload".to_string()],
        "`byId` holds its number key as it is and its value in its unit's type:\n{machine}"
    );
    // The record variable `last` is of the machine's own type, which a put of
    // `face` crosses into the unit's, and a value taken whole back out of.
    let record = declared
        .field("last", &machine)
        .to_token_stream()
        .to_string();
    for (from, to) in [
        (record.as_str(), "map_face :: MapFacePayload"),
        ("map_face :: MapFacePayload", record.as_str()),
    ] {
        let name = declared.function(from, to, &machine);
        assert!(
            called.0.contains(&name),
            "no statement of the machine crosses through `{name}`:\n{machine}"
        );
    }
    for unit in ["super :: map_peer_key", "super :: map_face"] {
        assert!(
            declared.uses.iter().any(|used| used == unit),
            "the machine does not bring `{unit}` in:\n{machine}"
        );
    }
}

/// The paths of the calls each call of the function `name` holds in its
/// arguments, as their tokens print ([`Called`]).
struct CallsAround {
    name: String,
    within: Vec<Vec<String>>,
}

impl<'ast> syn::visit::Visit<'ast> for CallsAround {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        use quote::ToTokens;
        if let syn::Expr::Path(path) = call.func.as_ref() {
            if path.path.to_token_stream().to_string() == self.name {
                let mut inner = Called::default();
                for argument in &call.args {
                    syn::visit::Visit::visit_expr(&mut inner, argument);
                }
                self.within.push(inner.0);
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// docs/adr/0108, D6 item 4: a map's enum value is held in the type the
/// enum's unit declares, as a record entry is. A value of the machine's own
/// enum is put through the function that crosses it into the unit's type, and
/// the saved state writes each value the map holds back through the other, so
/// it is saved as a variable of the machine's enum is. A guard that reads a
/// value out of the map compares it as a value of the machine's enum, crossed
/// back where it is read. Read off the syntax tree.
#[test]
fn rust_holds_a_maps_enum_entries_in_their_units_type() {
    use quote::ToTokens;
    let document = edited(
        &faces(),
        "<data id=\"order\"",
        "<data id=\"outcomes\" sce:type=\"map&lt;uint32, enum:PutOutcome&gt;[2]\"/>\n    \
         <data id=\"order\"",
    );
    let document = edited(
        &document,
        "<sce:clear target=\"flags\"/>",
        "<sce:clear target=\"flags\"/>\n      \
         <sce:put target=\"outcomes\" key=\"1\" expr=\"putResult\"/>\n      \
         <if cond=\"outcomes[1] === PutOutcome.inserted\">\n        \
         <assign location=\"refused\" expr=\"refused + 1\"/>\n      \
         </if>",
    );
    let (ok, printed, files) = generate_rust_files(&document);
    assert!(ok, "Rust generates a map of enum values:\n{printed}");
    let (declared, called, machine) = declared_machine(&files);
    let outcomes = bounded_map_arguments(&declared.field("outcomes", &machine))
        .unwrap_or_else(|| panic!("`outcomes` is no BoundedMap:\n{machine}"));
    assert_eq!(
        outcomes[..2],
        ["u32".to_string(), "put_outcome :: PutOutcome".to_string()],
        "`outcomes` holds its value in the type the enum's unit declares:\n{machine}"
    );
    // `putResult` is of the machine's own enum, and crosses each way: into the
    // unit's type where it is put, and back where the map is saved.
    let own = declared
        .field("put_result", &machine)
        .to_token_stream()
        .to_string();
    for (from, to) in [
        (own.as_str(), "put_outcome :: PutOutcome"),
        ("put_outcome :: PutOutcome", own.as_str()),
    ] {
        let name = declared.function(from, to, &machine);
        assert!(
            called.0.contains(&name),
            "the put and the saved form of `outcomes` do not cross through `{name}`:\n{machine}"
        );
    }
    // The guard reads the value at 1, of the unit's type, and crosses it back
    // into the machine's enum its variant literal is of: the function back is
    // called on the map's read itself, which the saved form's call is not.
    let back = declared.function("put_outcome :: PutOutcome", &own, &machine);
    let tree = syn::parse_file(&machine).expect("the generated machine parses as Rust");
    let mut around = CallsAround {
        name: back.clone(),
        within: Vec::new(),
    };
    syn::visit::Visit::visit_file(&mut around, &tree);
    assert!(
        around.within.iter().any(|inner| inner
            .iter()
            .any(|path| path.ends_with("map :: value_cloned"))),
        "the read of `outcomes[1]` in the guard does not cross back through `{back}`:\n{machine}"
    );
    assert!(
        declared
            .uses
            .iter()
            .any(|used| used == "super :: put_outcome"),
        "the machine does not bring the enum's unit in:\n{machine}"
    );
}

/// docs/adr/0108, "Order of work", step 5: Rust generates the document, each
/// map held in the runtime's `BoundedMap` or `BoundedSet` built under a seed,
/// each read a call of `portable::map`'s, each record key and the payload a key
/// is looked up by made a key, and each statement a write of the map. D6 item 3:
/// a `<foreach>` whose body does not write the map it walks walks the map
/// itself, and one whose body does walks a copy.
#[test]
fn rust_holds_a_map_and_copies_one_only_for_a_loop_that_writes_it() {
    let (ok, printed, code) = generate_rust(&faces());
    assert!(ok, "Rust generates a map:\n{printed}");
    for held in [
        "::sce_rust_runtime::BoundedMap<",
        "::sce_rust_runtime::BoundedSet<u32, 4>",
        "::sce_rust_runtime::MapSeeds::new(::sce_rust_runtime::map_seed())",
        "::sce_rust_runtime::FromSeed::from_seed(sce_map_seeds.draw())",
        "::sce_rust_runtime::portable::map::holds(",
        "::sce_rust_runtime::portable::map::count(",
        "::sce_rust_runtime::portable::map::value(",
        "::sce_rust_runtime::portable::map::value_cloned(",
        "::sce_rust_runtime::BoundedMap::widened_map(",
        "impl ::sce_rust_runtime::MapKey for",
        "::sce_rust_runtime::PutOutcome::Full",
        "::sce_rust_runtime::RemoveOutcome::Removed",
        ".first();",
        ".next_after(",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated Rust:\n{code}"
        );
    }
    // No loop of the document writes the map it walks, and the walk of
    // `faces`, whose body reaches the machine through its fields alone, lends
    // each face where the map holds it.
    assert!(!code.contains("sce_walk_"), "a loop copies a map:\n{code}");
    assert!(
        code.contains("let f = self.faces.value_at(sce_at_0);"),
        "the walk of `faces` lends no face:\n{code}"
    );

    // docs/adr/0108, D6: a body with a statement that takes the machine whole
    // (a `<send>`) beside an `<assign>` is handed a clone of each face, so the
    // map is not borrowed while the machine is.
    let sends = edited(
        &faces(),
        "<assign location=\"walked\" expr=\"i\"/>",
        "<assign location=\"walked\" expr=\"i\"/>\n        <send event=\"face.walked\"/>",
    );
    let (ok, printed, code) = generate_rust(&sends);
    assert!(ok, "Rust generates a walk whose body sends:\n{printed}");
    assert!(
        code.contains("let f = self.faces.value_at(sce_at_0).clone();"),
        "the walk of `faces` whose body sends lends its face:\n{code}"
    );

    // The walk of `seen` that removes each key it meets walks a copy of it.
    let writes_its_map = edited(
        &faces(),
        "<sce:put target=\"counts\" key=\"s\" expr=\"total\"/>",
        "<sce:remove target=\"seen\" key=\"s\"/>",
    );
    let (ok, printed, code) = generate_rust(&writes_its_map);
    assert!(ok, "Rust generates a loop that writes its map:\n{printed}");
    assert!(
        code.contains("let sce_walk_0 = self.seen.clone();"),
        "the loop that writes `seen` walks no copy of it:\n{code}"
    );
}

/// docs/adr/0108, D1 and D8: Rust holds a map keyed by a byte string and one
/// whose value is a byte string, and a set of byte strings, and its saved state
/// writes and reads each byte string as its Latin-1 text, as it does a
/// byte-string variable (§2.15, "A byte string"), the rest by their own form.
/// The runtime holds that a zero byte and a byte above 0x7f cross whole
/// (`saved_state::tests::a_byte_string_in_a_map_is_saved_as_its_latin1_text`).
#[test]
fn rust_saves_a_byte_string_of_a_map_as_its_latin1_text() {
    let document = edited(
        &faces(),
        "<data id=\"order\"",
        "<data id=\"blobs\" sce:type=\"map&lt;bytes[16], bytes[4]&gt;[2]\"/>\n    \
         <data id=\"tags\" sce:type=\"map&lt;bytes[16], uint8&gt;[2]\"/>\n    \
         <data id=\"ids\" sce:type=\"set&lt;bytes[16]&gt;[2]\"/>\n    <data id=\"order\"",
    );
    let (ok, printed, code) = generate_rust(&document);
    assert!(ok, "Rust generates a map of byte strings:\n{printed}");
    let bytes_writer = "|bytes| ::sce_rust_runtime::saved_state::bytes_to_saved(bytes)";
    let bytes_reader = "::sce_rust_runtime::saved_state::bounded_bytes";
    let own_writer = "::sce_rust_runtime::saved_state::SavedValue::to_saved";
    let own_reader = "::sce_rust_runtime::saved_state::SavedValue::from_saved";
    for held in [
        "::sce_rust_runtime::BoundedMap<::sce_rust_runtime::portable::SceBytes<16>, \
         ::sce_rust_runtime::portable::SceBytes<4>, 2>"
            .to_string(),
        format!(
            "::sce_rust_runtime::saved_state::map_to_saved(&policy.blobs, {bytes_writer}, \
             {bytes_writer})"
        ),
        format!(
            "::sce_rust_runtime::saved_state::map_from_saved(saved.variable(\"blobs\")?, \
             \"blobs\", {bytes_reader}, {bytes_reader})?"
        ),
        format!(
            "::sce_rust_runtime::saved_state::map_to_saved(&policy.tags, {bytes_writer}, \
             {own_writer})"
        ),
        format!(
            "::sce_rust_runtime::saved_state::map_from_saved(saved.variable(\"tags\")?, \
             \"tags\", {bytes_reader}, {own_reader})?"
        ),
        format!("::sce_rust_runtime::saved_state::set_to_saved(&policy.ids, {bytes_writer})"),
        format!(
            "::sce_rust_runtime::saved_state::set_from_saved(saved.variable(\"ids\")?, \
             \"ids\", {bytes_reader})?"
        ),
        // A map of no byte string writes each part by its own form.
        format!(
            "::sce_rust_runtime::saved_state::map_to_saved(&policy.counts, {own_writer}, \
             {own_writer})"
        ),
    ] {
        assert!(
            code.contains(&held),
            "no `{held}` in the generated Rust:\n{code}"
        );
    }
}

/// The one Rust file the forge document at `path` generates, alone.
fn rust_unit(path: &Path) -> String {
    let output = sce_build::compile_forge_file(
        path,
        sce_build::generator::Language::Rust,
        &[],
        &sce_build::ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|refusal| panic!("{} generates on Rust: {}", path.display(), refusal.error));
    let mut files: Vec<String> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".rs"))
        .map(|(_, text)| text)
        .collect();
    assert_eq!(files.len(), 1, "{}: one Rust unit", path.display());
    files.remove(0)
}

/// The fields of `self` an impl reads, in the order it reads them.
#[derive(Default)]
struct SelfFields(Vec<String>);

impl<'ast> syn::visit::Visit<'ast> for SelfFields {
    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        use quote::ToTokens;
        if let syn::Expr::Path(base) = field.base.as_ref() {
            if base.path.is_ident("self") {
                self.0.push(field.member.to_token_stream().to_string());
            }
        }
        syn::visit::visit_expr_field(self, field);
    }
}

/// Each type `text`, a Rust file, makes a key of the runtime's map
/// (`impl ::sce_portable_bytes::MapKey for <type>`), with the fields of it the
/// impl feeds, in order; read off the syntax tree.
fn map_key_impls(text: &str) -> Vec<(String, Vec<String>)> {
    use quote::ToTokens;
    let tree = syn::parse_file(text).expect("the unit parses as Rust");
    tree.items
        .iter()
        .filter_map(|item| {
            let syn::Item::Impl(implemented) = item else {
                return None;
            };
            let (_, trait_path, _) = implemented.trait_.as_ref()?;
            let named: Vec<String> = trait_path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            if named != ["sce_portable_bytes", "MapKey"] {
                return None;
            }
            let mut fed = SelfFields::default();
            syn::visit::Visit::visit_item_impl(&mut fed, implemented);
            Some((implemented.self_ty.to_token_stream().to_string(), fed.0))
        })
        .collect()
}

/// docs/adr/0108, D6 item 4: a statechart's map holds a record key and an enum
/// key in the types their units declare, so each Rust unit makes its type a key
/// of the runtime's map itself: a record whose every field is a key, field by
/// field in the order its schema declares them, and an enum by its number. A
/// machine of the same crate could not, since two machines keyed by one record
/// would each make it one. A record with a field that is no key (a float) is
/// none, and its unit names no `MapKey`.
#[test]
fn a_rust_record_or_enum_unit_is_a_key_of_the_runtimes_map() {
    assert_eq!(
        map_key_impls(&rust_unit(&fixtures().join("map_peer_key.scxml"))),
        [(
            "MapPeerKeyPayload".to_string(),
            vec!["low".to_string(), "high".to_string()]
        )],
        "the record of two integers is a key by both, low first"
    );
    assert_eq!(
        map_key_impls(&rust_unit(&fixtures().join("map_face.scxml"))),
        [(
            "MapFacePayload".to_string(),
            vec!["id".to_string(), "active".to_string(), "label".to_string()]
        )],
        "a record of a number, a truth value and a bounded text is a key by each"
    );
    let outcome = rust_unit(&repo_root().join("stdlib/collection/put_outcome.scxml"));
    let impls = map_key_impls(&outcome);
    assert_eq!(
        impls.iter().map(|(ty, _)| ty.as_str()).collect::<Vec<_>>(),
        ["PutOutcome"],
        "an enum's unit is a key by its number:\n{outcome}"
    );
    let dir = tempfile::tempdir().expect("a scratch directory");
    let reading = dir.path().join("map_reading.scxml");
    std::fs::write(
        &reading,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\"\n\
         \x20      version=\"1.0\" sce:kind=\"event-schema\" name=\"map_reading\"\n\
         \x20      sce:event-name=\"reading.taken\">\n\
         \x20 <datamodel>\n\
         \x20   <data id=\"sensor\" sce:type=\"uint8\" sce:direction=\"in\"/>\n\
         \x20   <data id=\"celsius\" sce:type=\"float64\" sce:direction=\"in\"/>\n\
         \x20 </datamodel>\n\
         </scxml>\n",
    )
    .expect("write the schema");
    let unit = rust_unit(&reading);
    assert!(
        map_key_impls(&unit).is_empty(),
        "a record with a float field is no key:\n{unit}"
    );
}

/// The C11 lowering of `document`, written beside the schemas it imports:
/// whether `generate` passed, what it printed, and every header and source it
/// wrote, joined.
fn generate_c11(document: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    let path = dir.path().join("map_faces.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "c11", "-o", out.as_str()], &path);
    let mut code = String::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(out_dir.path())
        .expect("read the output")
        .map(|entry| entry.expect("an output file").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "h" || ext == "c"))
        .collect();
    files.sort();
    for path in files {
        code.push_str(&std::fs::read_to_string(&path).expect("read a generated file"));
    }
    (ok, printed, code)
}

/// docs/adr/0108, "Order of work", step 6: C11 generates the document, each map
/// and set the type `sce/forge/map.h` declares for its parts and its bound,
/// started under a seed drawn from the one every construction takes; each read a
/// call of the header's over the map's address with a probe of the key, a record
/// key and the payload taken whole each by the function that writes its
/// canonical bytes; each statement a write of the map; a whole map taken into
/// another's storage; and the saved form written and read by each map's type.
/// D6 item 3: a `<foreach>` whose body does not write the map it walks walks the
/// map itself, and one whose body does walks a copy the machine holds.
#[test]
fn c11_holds_a_map_and_copies_one_only_for_a_loop_that_writes_it() {
    let (ok, printed, code) = generate_c11(&faces());
    assert!(ok, "C11 generates a map:\n{printed}");
    for held in [
        "#include \"sce/forge/map.h\"",
        "SCE_MAP_DECLARE(sce_static_map_uint32_uint64_8, uint32_t, uint64_t, 8u, \
         sce_static_map_uint32_uint64_8_key_feed)",
        "SCE_MAP_DECLARE(sce_static_map_uint32_uint64_16, uint32_t, uint64_t, 16u, \
         sce_static_map_uint32_uint64_16_key_feed)",
        "SCE_SET_DECLARE(sce_static_set_uint32_4, uint32_t, 4u, sce_static_set_uint32_4_key_feed)",
        "sce_map_key_text(key, sink);",
        "static inline void map_faces_record_peer_key_map_key(const void *key, sce_map_sink_t *sink)",
        "static inline void map_faces_peer_seen_payload_map_key(const void *key, sce_map_sink_t *sink)",
        "sce_map_seed_t map_seed",
        "sce_map_seed_derive(map_seed, 0u)",
        "sce_map_holds(&(sm->policy.v_faces), &(sm->policy.v_peer), \
         map_faces_record_peer_key_map_key)",
        "map_faces_peer_seen_payload_map_key)",
        "sce_map_count(&(sm->policy.v_faces))",
        "sce_map_lookup_take(&sce_failure_, sce_map_lookup(&(sm->policy.v_counts), SCE_MAP_KEY_UINT(",
        "SCE_MAP_KEY_TEXT(",
        "SCE_MAP_VALUE_OF(sm->policy.v_faces, ",
        "sce_map_assign(&(sm->policy.v_spare), &(sm->policy.v_counts));",
        "sce_map_clear(&(sm->policy.v_flags));",
        "SCE_MAP_PUT_FULL",
        "SCE_MAP_REMOVE_REMOVED",
        "sce_static_map_uint32_uint64_8_t_saved_write(w, &",
        "sce_static_map_uint32_uint64_8_t_saved_read(v, what, key_at, value_at, out, check, err)",
        "&check->of_v_counts",
        "sm->scratch.saved_check",
        // A record key and a record value are read at their saved revisions, a
        // map of numbers at none.
        "\"faces\", record_at[",
        "\"byId\", 1u, record_at[",
        "\"counts\", 1u, 1u,",
        "sce_map_first(&(sm->policy.v_faces))",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated C11:\n{code}"
        );
    }
    // No loop of the document writes the map it walks.
    assert!(
        !code.contains("sce_map_copy_for_walk("),
        "a loop copies a map:\n{code}"
    );

    // The walk of `seen` that removes each key it meets walks a copy of it, held
    // in the machine's scratch at the loop's depth.
    let writes_its_map = edited(
        &faces(),
        "<sce:put target=\"counts\" key=\"s\" expr=\"total\"/>",
        "<sce:remove target=\"seen\" key=\"s\"/>",
    );
    let (ok, printed, code) = generate_c11(&writes_its_map);
    assert!(ok, "C11 generates a loop that writes its map:\n{printed}");
    for held in [
        "sce_map_copy_for_walk(sce_walk_",
        "&sm->scratch.loop_0.of_sce_static_set_uint32_4",
        "sce_static_set_uint32_4_t of_sce_static_set_uint32_4;",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated C11:\n{code}"
        );
    }
}

/// The arguments of each `SCE_MAP_DECLARE(prefix, K, V, B, FEED)` the generated
/// C11 `code` makes, read apart: the declarations a machine's map types are, and
/// not a comment that names the macro.
fn c11_map_declarations(code: &str) -> Vec<Vec<String>> {
    code.lines()
        .filter_map(|line| line.trim().strip_prefix("SCE_MAP_DECLARE("))
        .filter_map(|rest| rest.strip_suffix(')'))
        .map(|args| args.split(',').map(|arg| arg.trim().to_string()).collect())
        .collect()
}

/// docs/adr/0108, D6 item 4: a C11 map holds a record key and a record value in
/// the type the record's unit declares (`<Schema>Payload_t`), never in the
/// machine's own record type, and the machine's header includes each such unit's
/// header though the machine calls no algorithm. A put builds the unit's record
/// from the machine's; the value at a key and a walk's item read whole are the
/// machine's record built from the unit's, and a key the machine holds in its own
/// record is looked up by that record's feed, one a walk binds by the map's own.
#[test]
fn a_c11_map_holds_its_records_in_the_types_their_units_declare() {
    let document = edited(
        &faces(),
        "<assign location=\"walked\" expr=\"i\"/>",
        "<assign location=\"last\" expr=\"f\"/>\n        \
         <if cond=\"has(faces, k)\"><assign location=\"walked\" expr=\"i\"/></if>",
    );
    let (ok, printed, code) = generate_c11(&document);
    assert!(ok, "C11 generates a map of records:\n{printed}");
    let declared = c11_map_declarations(&code);
    let holds = |key: &str, value: &str| {
        declared
            .iter()
            .any(|args| args.len() == 5 && args[1] == key && args[2] == value)
    };
    assert!(
        holds("MapPeerKeyPayload_t", "MapFacePayload_t"),
        "`faces` holds its units' records: {declared:?}"
    );
    assert!(
        holds("uint32_t", "MapFacePayload_t"),
        "`byId` holds its unit's record: {declared:?}"
    );
    assert!(
        declared
            .iter()
            .all(|args| !args.iter().any(|arg| arg.starts_with("map_faces_record_"))),
        "a map holds the machine's own record: {declared:?}"
    );
    for held in [
        "#include \"map_peer_key.h\"",
        "#include \"map_face.h\"",
        "static inline void MapPeerKeyPayload_map_key(const void *key, sce_map_sink_t *sink)",
        "static inline void MapFacePayload_t_saved_write(sce_jw_t *w, const MapFacePayload_t *value)",
        "MapFacePayload_t_saved_write(w, &(slot->value));",
        "static inline map_faces_record_face_t map_faces_record_face_of_unit(MapFacePayload_t from)",
        "static inline MapPeerKeyPayload_t \
         map_faces_record_peer_key_to_unit(map_faces_record_peer_key_t from)",
        "return (MapPeerKeyPayload_t){ .low = from.low, .high = from.high };",
        "sce_key = map_faces_record_peer_key_to_unit(sm->policy.v_peer);",
        "sce_value = map_faces_record_face_to_unit(sm->policy.v_face);",
        "map_faces_record_face_of_unit(SCE_MAP_VALUE_OF(sm->policy.v_faces, ",
        "map_faces_record_face_of_unit(f)",
        "sce_map_holds(&(sm->policy.v_faces), &(k), NULL)",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated C11:\n{code}"
        );
    }
}

/// The statechart that keeps a map of records whose field is a bounded byte
/// string, and the schema of those records, as committed, each by the name it
/// is imported under.
fn frame_documents() -> Vec<(&'static str, String)> {
    ["map_frame.scxml", "map_frames.scxml"]
        .into_iter()
        .map(|name| {
            let text = std::fs::read_to_string(fixtures().join(name))
                .unwrap_or_else(|e| panic!("read {name}: {e}"));
            (name, text)
        })
        .collect()
}

/// The frame documents with the field `body` of the schema retyped `to`, and the
/// statechart cut to its map alone, so that the map's record is the one thing
/// that differs from a machine that generates.
fn frame_documents_with_body(to: &str) -> Vec<(&'static str, String)> {
    let schema = std::fs::read_to_string(fixtures().join("map_frame.scxml"))
        .expect("read the schema of the frames");
    let from = "sce:type=\"bytes[16]\"";
    assert_eq!(
        schema.matches(from).count(),
        1,
        "`{from}` stands once in the schema"
    );
    vec![
        ("map_frame.scxml", schema.replacen(from, to, 1)),
        (
            "map_frames.scxml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\"\n\
             \x20      version=\"1.0\" initial=\"holding\" datamodel=\"sce-static\" \
             name=\"map_frames\">\n\
             \x20 <sce:import kind=\"event-schema\" src=\"map_frame.scxml\" as=\"Frame\"/>\n\
             \x20 <datamodel>\n\
             \x20   <data id=\"frames\" sce:type=\"map&lt;uint32, record:Frame&gt;[2]\"/>\n\
             \x20 </datamodel>\n\
             \x20 <state id=\"holding\"/>\n\
             </scxml>\n"
                .to_string(),
        ),
    ]
}

/// The text of `tokens` with no whitespace, so a path or an expression is
/// compared whatever spacing its tokens print with.
fn unspaced(tokens: &str) -> String {
    tokens.split_whitespace().collect()
}

/// docs/adr/0108, D6 item 4: a Rust map holds a record whose field is a byte
/// string its schema bounds in the type the record's unit declares, as any
/// record. The unit holds the bytes in a `Vec<u8>` and the machine in the
/// `SceBytes<16>` of the field's bound: a put copies the bytes whole out of the
/// machine's record, and the value at a key read whole holds them to the bound
/// again on the way back. Read off the syntax tree of the generated machine.
#[test]
fn rust_holds_a_maps_record_with_a_bounded_byte_string_in_its_units_type() {
    use quote::ToTokens;
    let (ok, printed, files) = generate_rust_beside(&frame_documents(), "map_frames.scxml", false);
    assert!(
        ok,
        "Rust generates a map of records with a bounded byte string:\n{printed}"
    );
    let (declared, called, machine) = declared_machine(&files);
    let frames = bounded_map_arguments(&declared.field("frames", &machine))
        .unwrap_or_else(|| panic!("`frames` is no BoundedMap:\n{machine}"));
    assert_eq!(
        frames[..2],
        [
            "u32".to_string(),
            "map_frame :: MapFramePayload".to_string()
        ],
        "`frames` holds its value in its unit's type:\n{machine}"
    );
    let record = declared
        .field("last", &machine)
        .to_token_stream()
        .to_string();
    let into = declared.function(&record, "map_frame :: MapFramePayload", &machine);
    let back = declared.function("map_frame :: MapFramePayload", &record, &machine);
    for name in [&into, &back] {
        assert!(
            called.0.contains(name),
            "no statement of the machine crosses through `{name}`:\n{machine}"
        );
    }
    let functions = functions_of(&files);
    // Into the unit: the bytes of the machine's bounded storage, copied whole.
    let mut into_calls = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut into_calls, function_named(&functions, &into));
    assert!(
        into_calls
            .0
            .iter()
            .any(|(receiver, method)| method == "to_vec"
                && unspaced(receiver) == "v.body.as_slice()"),
        "`{into}` does not copy the body whole into the unit: {:?}\n{machine}",
        into_calls.0
    );
    // Back: the unit's bytes held to the field's bound again.
    let from_slice = "::sce_rust_runtime::portable::SceBytes::<16>::from_slice";
    let mut back_paths = Called::default();
    syn::visit::Visit::visit_item_fn(&mut back_paths, function_named(&functions, &back));
    assert!(
        back_paths.0.iter().any(|path| unspaced(path) == from_slice),
        "`{back}` does not hold the body to its bound: {:?}\n{machine}",
        back_paths.0
    );
    let mut back_calls = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut back_calls, function_named(&functions, &back));
    assert!(
        back_calls
            .0
            .iter()
            .any(|(receiver, method)| method == "expect"
                && unspaced(receiver) == format!("{from_slice}(&v.body)")),
        "`{back}` does not build the body from the unit's: {:?}\n{machine}",
        back_calls.0
    );
}

/// docs/adr/0108, D6 item 4: a C11 map holds a record whose field is a byte
/// string its schema bounds in the type the record's unit declares, and the
/// unit holds the bytes in the buffer of the bound the machine's record holds
/// them in, so the field crosses as it is both ways.
#[test]
fn c11_copies_a_maps_record_with_a_bounded_byte_string_as_it_is() {
    let (ok, printed, code) = generate_c11_among(&frame_documents(), "map_frames.scxml");
    assert!(
        ok,
        "C11 generates a map of records with a bounded byte string:\n{printed}"
    );
    for held in [
        "#include \"map_frame.h\"",
        "    sce_static_bytes_16_t body;",
        "static inline map_frames_record_frame_t map_frames_record_frame_of_unit(MapFramePayload_t from)",
        "static inline MapFramePayload_t \
         map_frames_record_frame_to_unit(map_frames_record_frame_t from)",
        "return (map_frames_record_frame_t){ .id = from.id, .body = from.body };",
        "return (MapFramePayload_t){ .id = from.id, .body = from.body };",
        "sce_value = map_frames_record_frame_to_unit(sm->policy.v_frame);",
        "map_frames_record_frame_of_unit(SCE_MAP_VALUE_OF(sm->policy.v_frames, ",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated C11:\n{code}"
        );
    }
}

/// docs/adr/0108, D6 item 4: a map whose record has a field Rust does not copy
/// between the machine's record and the unit's (an optional byte string, which
/// no Rust unit holds yet) is refused on Rust naming that field and its type,
/// and not as a field with no bound.
#[test]
fn a_maps_record_rust_does_not_copy_is_refused_naming_the_field() {
    let documents = frame_documents_with_body("sce:type=\"optional&lt;bytes[16]&gt;\"");
    let (ok, printed, _) = generate_rust_beside(&documents, "map_frames.scxml", false);
    assert!(
        !ok,
        "Rust copies no optional byte string into a unit:\n{printed}"
    );
    for part in [
        "\"code\":\"generate/unsupported-feature\"",
        "a map holds record:Frame in the type its unit declares (docs/adr/0108, D6 item 4), \
         and Rust copies no record of it there: its field `body` of type optional<bytes>",
    ] {
        assert!(printed.contains(part), "expected `{part}`:\n{printed}");
    }
    assert!(
        !printed.contains("with no bound"),
        "the refusal says a bounded field has no bound:\n{printed}"
    );
}

/// docs/adr/0108, D1 and D6 item 4: a byte string its schema bounds is a key, so
/// a record that holds one keys a map. C11 holds the key in the unit's record and
/// feeds the bytes of the field with their length; the Rust unit holds the bytes
/// in a `Vec<u8>`, which is no key of the runtime's map, so Rust refuses the map
/// keyed by the record by name, naming the field, and not as a record it cannot
/// copy.
#[test]
fn a_map_keyed_by_a_record_with_a_byte_string_is_refused_by_rust_naming_the_field() {
    let documents: Vec<(&str, String)> = frame_documents()
        .into_iter()
        .map(|(name, text)| match name {
            "map_frames.scxml" => (
                name,
                edited(
                    &text,
                    "<data id=\"held\"",
                    "<data id=\"byFrame\" sce:type=\"map&lt;record:Frame, uint32&gt;[2]\"/>\n    \
                     <data id=\"held\"",
                ),
            ),
            _ => (name, text),
        })
        .collect();
    let (ok, printed, _) = generate_rust_beside(&documents, "map_frames.scxml", false);
    assert!(
        !ok,
        "Rust keys no map by a record with a byte string:\n{printed}"
    );
    for part in [
        "\"code\":\"generate/unsupported-feature\"",
        "a map keyed by record:Frame holds the key in the type its unit declares \
         (docs/adr/0108, D6 item 4), and Rust's unit of it is no key of a map: its field \
         `body` of type bytes is held in the unit in a type that is no key",
    ] {
        assert!(printed.contains(part), "expected `{part}`:\n{printed}");
    }
    assert!(
        !printed.contains("copies no record"),
        "the refusal is the key's, not the copy's:\n{printed}"
    );
    let (ok, printed, code) = generate_c11_among(&documents, "map_frames.scxml");
    assert!(ok, "C11 keys a map by the record:\n{printed}");
    for held in [
        "static inline void MapFramePayload_map_key(const void *key, sce_map_sink_t *sink)",
        "sce_map_feed_bytes(sink, ",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated C11:\n{code}"
        );
    }
}

/// docs/adr/0086, section 1.4: a byte string field with no bound is refused
/// where its schema is read, by every lane and by `check` alike, naming the
/// type that states none, before any map asks whether its record crosses.
#[test]
fn a_maps_record_with_an_unbounded_byte_string_is_refused_at_its_type() {
    let documents = frame_documents_with_body("sce:type=\"bytes\"");
    let dir = tempfile::tempdir().expect("a scratch directory");
    for (name, text) in &documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let (ok, checked) = run(&["check"], &dir.path().join("map_frames.scxml"));
    let (generated, printed, _) = generate_rust_beside(&documents, "map_frames.scxml", false);
    assert!(
        checked.contains("validation/attribute-rule-violated"),
        "`check` refuses the field at its type:\n{checked}"
    );
    for (ok, out) in [(ok, &checked), (generated, &printed)] {
        assert!(!ok, "a byte string with no bound is refused:\n{out}");
        assert!(
            out.contains("`bytes` has no bound") && !out.contains("copies no record"),
            "expected the type's refusal, not the map's:\n{out}"
        );
    }
}

/// docs/adr/0108, D8: a C11 parent that starts a child holding a map takes a
/// seed at its own start though it holds no map itself, keeps it, and hands the
/// child a seed derived from it (`sce_map_seed_of_child`) where it begins the
/// child; the child starts its maps from the seed it is begun with.
#[test]
fn c11_hands_a_child_that_holds_a_map_a_seed_derived_from_its_own() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    std::fs::write(dir.path().join("map_faces.scxml"), faces()).expect("write the child");
    let parent = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="map_parent">
  <state id="running">
    <invoke id="faces" type="scxml" src="map_faces.scxml"/>
  </state>
</scxml>
"#;
    let path = dir.path().join("map_parent.scxml");
    std::fs::write(&path, parent).expect("write the parent");
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "c11", "-o", out.as_str()], &path);
    assert!(
        ok,
        "C11 generates a parent of a child that holds a map:\n{printed}"
    );
    let mut parent_code = String::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("read the output") {
        let path = entry.expect("an output file").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("map_parent") && (name.ends_with(".h") || name.ends_with(".c")) {
            parent_code.push_str(&std::fs::read_to_string(&path).expect("read a generated file"));
        }
    }
    for held in [
        "map_parent_init(map_parent_t *sm, sce_map_seed_t map_seed)",
        "sce_map_seed_t map_seed;",
        "sm->map_seed = map_seed;",
        "sce_map_seed_of_child(sm->map_seed, p->invoke_id)",
    ] {
        assert!(
            parent_code.contains(held),
            "no `{held}` in the generated parent:\n{parent_code}"
        );
    }
    // The parent holds no map: it starts none of its own.
    assert!(
        !parent_code.contains("sce_map_seed_derive(map_seed,"),
        "the parent starts a map:\n{parent_code}"
    );
    // The child is begun with the seed, and starts its maps from it.
    let (ok, printed, child_code) = generate_c11(&faces());
    assert!(ok, "C11 generates the child:\n{printed}");
    for held in [
        "sce_clock_t clock, sce_map_seed_t map_seed)",
        "sce_map_seed_derive(map_seed, 0u)",
    ] {
        assert!(
            child_code.contains(held),
            "no `{held}` in the generated child:\n{child_code}"
        );
    }
}

/// docs/adr/0108, D2, D3 and D11: a map is read through `len`, `has` and the
/// value at a key, and is no operand; a key fits the map's key as a value
/// assigned to a place of its type does; a set holds no value at a key.
#[test]
fn a_read_of_a_map_that_is_no_read_of_it_is_refused() {
    let faces = faces();
    let unsupported = "expression/unsupported-construct";
    for (from, to, code, part) in [
        // A map as an operand, and two maps compared.
        (
            "expr=\"refused + 1\"",
            "expr=\"refused + counts\"",
            unsupported,
            "reading the map or set `counts` as a value",
        ),
        (
            "cond=\"putResult === PutOutcome.full\"",
            "cond=\"counts === spare\"",
            unsupported,
            "reading the map or set `counts` as a value",
        ),
        // `has` of other than two arguments, and of no map.
        (
            "has(counts, 7)",
            "has(counts)",
            "expression/argument-count-mismatch",
            "has",
        ),
        (
            "has(counts, 7)",
            "has(total, 7)",
            unsupported,
            "whose first argument names no map or set variable",
        ),
        // A key of another kind than the map's.
        (
            "has(counts, 7)",
            "has(counts, 'seven')",
            "expression/type-mismatch",
            "uint32",
        ),
        (
            "has(counts, 7)",
            "has(counts, PutOutcome.full)",
            unsupported,
            "a value of an enum is no number, bool, text or byte string",
        ),
        (
            "flags['on']",
            "flags[7]",
            "expression/type-mismatch",
            "string",
        ),
        // A record key is named whole, by a record of its schema.
        (
            "has(faces, peer) &amp;&amp;",
            "has(faces, face) &amp;&amp;",
            unsupported,
            "a record key is named whole, by a record of its schema",
        ),
        (
            "has(faces, peer) &amp;&amp;",
            "has(faces, 1) &amp;&amp;",
            unsupported,
            "a record key is named whole, by a record of its schema",
        ),
        // A field the value's schema does not declare.
        (
            "faces[peer].active",
            "faces[peer].nope",
            unsupported,
            "a field the element's schema does not declare",
        ),
        // A set's entries carry no value.
        (
            "expr=\"len(seen)\"",
            "expr=\"seen[3]\"",
            unsupported,
            "a value read at a key of the set `seen`",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D3: the value at a key of a map of enum values is a value of
/// that enum where an equality judges it, as a variable of the enum is: compared
/// with a variant of its own enum it is accepted, and with a variant of another
/// enum refused, as a variable of the enum compared with one is.
#[test]
fn a_maps_enum_value_is_compared_as_a_value_of_its_enum() {
    let document = edited(
        &faces(),
        "<data id=\"order\"",
        "<data id=\"outcomes\" sce:type=\"map&lt;uint32, enum:PutOutcome&gt;[2]\"/>\n    \
         <data id=\"order\"",
    );
    let compared = |with: &str| {
        edited(
            &document,
            "<sce:clear target=\"flags\"/>",
            &format!(
                "<sce:clear target=\"flags\"/>\n      \
                 <sce:put target=\"outcomes\" key=\"1\" expr=\"putResult\"/>\n      \
                 <if cond=\"outcomes[1] === {with}\"><assign location=\"refused\" \
                 expr=\"refused\"/></if>"
            ),
        )
    };
    let (ok, out) = check(&compared("PutOutcome.inserted"));
    assert!(
        ok,
        "check compares a map's enum value with a variant of its enum:\n{out}"
    );
    refused(
        &compared("RemoveOutcome.absent"),
        "expression/unsupported-construct",
        "a comparison of an enum value with anything but a value of the same enum",
    );
}

/// docs/adr/0108, D1 and D3: a map keyed by an enum is read at a variant of
/// that enum, which is its key and no enum value used as an index; a variant of
/// another enum is no key of it, and is refused.
#[test]
fn a_map_keyed_by_an_enum_is_read_at_a_variant_of_it() {
    let document = edited(
        &faces(),
        "<data id=\"order\"",
        "<data id=\"byOutcome\" sce:type=\"map&lt;enum:PutOutcome, uint32&gt;[3]\"/>\n    \
         <data id=\"order\"",
    );
    let read_at = |key: &str| {
        edited(
            &document,
            "<sce:clear target=\"flags\"/>",
            &format!(
                "<sce:clear target=\"flags\"/>\n      \
                 <sce:put target=\"byOutcome\" key=\"PutOutcome.inserted\" expr=\"1\"/>\n      \
                 <assign location=\"walked\" expr=\"byOutcome[{key}]\"/>"
            ),
        )
    };
    let (ok, out) = check(&read_at("PutOutcome.inserted"));
    assert!(
        ok,
        "check reads a map keyed by an enum at a variant:\n{out}"
    );
    let (ok, out) = check(&read_at("RemoveOutcome.absent"));
    assert!(
        !ok && out.contains("\"code\":\"expression/unsupported-construct\""),
        "a variant of another enum is no key of the map:\n{out}"
    );
}

/// docs/adr/0108, D2 and D3: an entry is written by `<sce:put>`, a map takes
/// whole only another map variable whose type it fits, and the value at a key
/// taken whole is the record its key reads once, of the variable's schema.
#[test]
fn a_map_is_written_by_its_statements_and_taken_whole_only_from_a_map() {
    let faces = faces();
    let unsupported = "expression/unsupported-construct";
    let rule = "scxml/static-datamodel-rule";
    for (from, to, code, part) in [
        (
            "<assign location=\"spare\" expr=\"counts\"/>",
            "<assign location=\"counts[7]\" expr=\"1\"/>",
            unsupported,
            "a part of an entry of the map `counts`",
        ),
        (
            "<assign location=\"spare\" expr=\"counts\"/>",
            "<assign location=\"spare\" expr=\"total\"/>",
            unsupported,
            "an assignment to the whole map `spare` from `total`",
        ),
        (
            "<assign location=\"spare\" expr=\"counts\"/>",
            "<assign location=\"counts\" expr=\"spare\"/>",
            unsupported,
            "an assignment to the whole map `counts` from `spare`: ",
        ),
        (
            "expr=\"byId[face.id]\"",
            "expr=\"byId[last.id]\"",
            rule,
            "the key of the entry reads `last`",
        ),
        (
            "<assign location=\"last\" expr=\"byId[face.id]\"/>",
            "<assign location=\"peer\" expr=\"byId[face.id]\"/>",
            rule,
            "a value of the map `byId` is a record of the schema Face",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D3, D4 and D11: `<sce:put>`, `<sce:remove>` and
/// `<sce:clear>` name a map or a set; a put's key and value fit the map's, the
/// value required of a map and refused on a set; and an outcome is a variable
/// of the standard enum of the statement.
#[test]
fn a_statement_of_a_map_is_refused_naming_what_it_takes() {
    let faces = faces();
    let rule = "scxml/static-datamodel-rule";
    for (from, to, code, part) in [
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"total\" key=\"7\" expr=\"1\"/>",
            rule,
            "target names a map or a set variable: one of faces, counts",
        ),
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"counts\" key=\"7\"/>",
            rule,
            "a put into the map `counts` names the value it puts at the key",
        ),
        (
            "<sce:put target=\"seen\" key=\"3\" outcome=\"putResult\"/>",
            "<sce:put target=\"seen\" key=\"3\" expr=\"1\" outcome=\"putResult\"/>",
            rule,
            "the set `seen` holds keys that carry no value",
        ),
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"counts\" key=\"'x'\" expr=\"1\"/>",
            "expression/type-mismatch",
            "uint32",
        ),
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"counts\" key=\"7\" expr=\"true\"/>",
            "expression/type-mismatch",
            "uint64",
        ),
        (
            "key=\"peer\" expr=\"face\" outcome=\"putResult\"",
            "key=\"face\" expr=\"face\" outcome=\"putResult\"",
            rule,
            "a whole record of the schema PeerKey is taken by name",
        ),
        (
            "key=\"peer\" expr=\"face\" outcome=\"putResult\"",
            "key=\"peer\" expr=\"peer\" outcome=\"putResult\"",
            rule,
            "a whole record of the schema Face is taken by name",
        ),
        (
            "key=\"peer\" expr=\"face\" outcome=\"putResult\"",
            "key=\"peer\" expr=\"face\" outcome=\"total\"",
            rule,
            "`total` is a variable of `uint64`, and outcome names a variable of the standard \
             enum sce:std/collection/put_outcome.scxml (inserted, replaced, full)",
        ),
        (
            "outcome=\"removeResult\"",
            "outcome=\"putResult\"",
            rule,
            "outcome names a variable of the standard enum \
             sce:std/collection/remove_outcome.scxml (removed, absent)",
        ),
        (
            "<sce:remove target=\"seen\" key=\"3\"/>",
            "<sce:remove target=\"seen\" key=\"3\" expr=\"1\"/>",
            rule,
            "a remove from `seen` names the key of the entry it removes, and takes no expr",
        ),
        (
            "<sce:remove target=\"seen\" key=\"3\"/>",
            "<sce:remove target=\"order\" key=\"3\"/>",
            rule,
            "target names a map or a set variable",
        ),
        (
            "<sce:clear target=\"flags\"/>",
            "<sce:clear target=\"total\"/>",
            rule,
            "target names a list, a map or a set variable: one of",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D5: a `<foreach>` over a map binds its key by `sce:key`,
/// held to the rules its item and index are; a set's item is its key and a
/// list's walk binds none; and the key is read, never written.
#[test]
fn a_walk_over_a_map_binds_its_key_by_the_rules_of_a_loop() {
    let faces = faces();
    let rule = "scxml/static-datamodel-rule";
    for (from, to, code, part) in [
        (
            "<foreach array=\"seen\" item=\"s\">",
            "<foreach array=\"seen\" item=\"s\" sce:key=\"t\">",
            rule,
            "the set `seen` holds keys that carry no value",
        ),
        (
            "<foreach array=\"order\" item=\"o\">",
            "<foreach array=\"order\" item=\"o\" sce:key=\"t\">",
            rule,
            "a <foreach> over a list binds each element and its index",
        ),
        (
            "sce:key=\"k\"",
            "sce:key=\"f\"",
            rule,
            "a <foreach> sce:key is a name other than the item's",
        ),
        (
            "index=\"i\"",
            "index=\"k\"",
            rule,
            "a <foreach> index is a name other than the key's",
        ),
        (
            "sce:key=\"k\"",
            "sce:key=\"total\"",
            rule,
            "a <foreach> sce:key is a name nothing in scope already means",
        ),
        (
            "<assign location=\"walked\" expr=\"i\"/>",
            "<assign location=\"k.low\" expr=\"1\"/>",
            "expression/unsupported-construct",
            "which a <foreach> binds",
        ),
        (
            "<sce:put target=\"counts\" key=\"f.id\" expr=\"k.low\"/>",
            "<sce:put target=\"counts\" key=\"f.id\" expr=\"k.nope\"/>",
            "expression/unknown-member",
            "nope",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D1 and D10: a map whose value is no value an entry's read
/// reads (a list, another map, an optional value) is the language's, and is
/// refused by name; and a map, its statements and its loop are a `sce-static`
/// statechart's alone.
#[test]
fn a_map_the_checker_does_not_read_is_refused_by_name() {
    refused(
        &edited(
            &faces(),
            "map&lt;uint32, uint64&gt;[16]",
            "map&lt;uint32, list&lt;uint8&gt;[2]&gt;[16]",
        ),
        "generate/unsupported-feature",
        "as the value of `map<uint32, list<uint8>[2]>[16]`",
    );
    for (statement, part) in [
        (
            "<sce:put target=\"m\" key=\"1\"/>",
            "a map or a set variable, and so the statements that write one, exist only under",
        ),
        (
            "<foreach array=\"m\" item=\"v\" sce:key=\"k\"/>",
            "a <foreach> binds the key of a map's entry only where a variable holds a map",
        ),
    ] {
        let document = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <datamodel>
    <data id="m" expr="0"/>
  </datamodel>
  <state id="s">
    <onentry>{statement}</onentry>
  </state>
</scxml>
"#
        );
        refused(&document, "scxml/static-datamodel-rule", part);
    }
}

/// The fixture directory the algorithm that keeps each peer's latest
/// generation stands in, beside the schemas, the enum and the union it imports.
fn static_fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The documents `algorithm_take_generation.scxml` imports from the fixture
/// directory, copied beside a variant of it.
const TAKE_SIBLINGS: [&str; 4] = [
    "schema_map_peer.scxml",
    "enum_take_kind.scxml",
    "schema_take_duplicate.scxml",
    "union_take_result.scxml",
];

/// The algorithm that keeps each peer's latest generation in an `inout` map
/// (docs/adr/0108, D6 item 2), as committed.
fn take_generation() -> String {
    std::fs::read_to_string(static_fixtures().join("algorithm_take_generation.scxml"))
        .expect("read the algorithm")
}

/// `text` with the one occurrence of `from` replaced by `to`, for a document
/// other than the statechart [`edited`] names in its message.
fn edited_algorithm(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "`{from}` stands once in the algorithm"
    );
    text.replacen(from, to, 1)
}

/// The forge parse of the algorithm `document`: its model, or the refusal the
/// parser placed.
fn parse_algorithm(
    document: &str,
) -> Result<sce_build::forge::model::AlgorithmModel, sce_build::forge::error::ForgeError> {
    let label = sce_build::DocumentLabel::symmetric("algorithm_take_generation");
    match sce_build::forge::parser::parse_forge_with_imports(document, label) {
        Ok(Some(parsed)) => match parsed.document {
            sce_build::forge::model::ForgeDocument::Algorithm(m) => Ok(m),
            other => panic!("the document is an algorithm, not a {}", other.kind()),
        },
        Ok(None) => panic!("the document is a forge document"),
        Err(located) => Err(located.error),
    }
}

/// `document` is refused where the parser reads it, its message holding
/// `part`.
fn parse_refused(document: &str, part: &str) -> sce_build::forge::error::ForgeError {
    let refusal = parse_algorithm(document)
        .err()
        .unwrap_or_else(|| panic!("expected a refusal holding `{part}`"));
    assert!(
        refusal.to_string().contains(part),
        "expected a refusal holding `{part}`, got: {refusal}"
    );
    refusal
}

/// docs/adr/0108, D5 and D6 item 2: an algorithm's `inout` map slot, its
/// put, its `has` and its read at a key are read and judged where the document
/// is read: the parameter is written in place, the put names its outcome, and
/// the body's judges, run before any backend renders it, prove the read at the
/// peer's key inside the map where `has` holds it. The Interpreter's lowering
/// holds an algorithm's map ("Order of work", step 7, D10): the `inout` table
/// arrives in the holder its caller lends, is read out of it into a local of
/// its name, a put into it is held to the holder's bound, and the table is
/// left in the holder however the function returns. Every generated backend
/// that lowers no algorithm's map yet
/// (`codegen_matrix::ALGORITHM_MAP_BACKENDS`) refuses the algorithm by name,
/// naming the record that lands it, rather than by any judge of its body.
#[test]
fn an_algorithm_holds_an_inout_map_the_interpreter_lowers_and_every_other_backend_refuses() {
    let m = parse_algorithm(&take_generation()).expect("the algorithm is read");
    let resets = &m.signature.params[0];
    assert!(resets.inout && resets.written_in_place(), "{resets:?}");
    assert!(m.signature.writes_in_place());
    assert!(!m.signature.params[1].inout);
    assert_eq!(m.first_map().map(|(name, _)| name), Some("resets"));

    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in TAKE_SIBLINGS {
        std::fs::copy(static_fixtures().join(name), dir.path().join(name)).expect("copy an import");
    }
    let path = dir.path().join("algorithm_take_generation.scxml");
    std::fs::write(&path, take_generation()).expect("write the algorithm");
    let lowered =
        sce_build::forge::static_js::lower_algorithm_document(path.to_str().expect("a utf-8 path"))
            .unwrap_or_else(|e| panic!("the Interpreter lowers the algorithm: {e:?}"));
    assert_eq!(lowered.symbol, "take_generation");
    for part in [
        "SceStatic.algorithms.take_generation = function (__lent0, peer, generation) { var \
         resets = __lent0.map; try { ",
        "SceStatic.mapHas(resets, ",
        "SceStatic.mapPut(resets, ",
        ", __lent0.capacity, peer, generation); resets = __wrote.map; stored = __wrote.outcome; }",
        " } finally { __lent0.map = resets; } }",
    ] {
        assert!(
            lowered.install.contains(part),
            "the installed algorithm holds `{part}`:\n{}",
            lowered.install
        );
    }
    let unlowered = sce_build::generator::Language::ALL
        .iter()
        .filter(|language| {
            !sce_build::forge::codegen_matrix::ALGORITHM_MAP_BACKENDS.contains(language)
        });
    for &language in unlowered {
        let lane = language.canonical_name();
        let out_dir = tempfile::tempdir().expect("an output directory");
        let out_dir = out_dir.path().to_str().expect("a utf-8 path").to_string();
        let mut args = vec!["generate", "-l", lane, "-o", out_dir.as_str()];
        if language == sce_build::generator::Language::Go {
            args.extend(["--go-module-prefix", "example.test/generated"]);
        }
        let (ok, out) = run(&args, &path);
        assert!(!ok, "{lane} lowers no algorithm's map yet:\n{out}");
        assert!(
            out.contains("\"code\":\"generate/unsupported-feature\"")
                && out.contains(&format!(
                    "holds `resets` of map<record:Zid, uint64>, and {lane} does not lower a map \
                     or a set in an algorithm yet (docs/adr/0108, D10)"
                )),
            "{lane}:\n{out}"
        );
    }
}

/// docs/adr/0108, D6 item 2: an `inout` slot only on an algorithm that cannot
/// fail, by the code of its own; and the rules of where `inout` stands and
/// what a non-failing algorithm's put names.
#[test]
fn an_inout_slot_is_given_only_to_an_algorithm_that_cannot_fail() {
    let may_fail = edited_algorithm(
        &take_generation(),
        "<sce:return type=\"union:TakeResult\"/>",
        "<sce:return type=\"union:TakeResult\" may-fail=\"true\"/>",
    );
    let refusal = parse_refused(
        &may_fail,
        "<sce:param name=\"resets\" inout=\"true\"> is written in place, and the algorithm \
         declares <sce:return may-fail=\"true\">",
    );
    assert!(
        matches!(
            &refusal,
            sce_build::forge::error::ForgeError::Validation(v)
                if matches!(**v, sce_build::forge::error::ValidationError::AlgorithmInoutOnMayFail { .. })
        ),
        "{refusal:?}"
    );

    // `inout` on a parameter that is no map or set is read by nothing.
    parse_refused(
        &edited_algorithm(
            &take_generation(),
            "<sce:param name=\"generation\" type=\"uint64\"/>",
            "<sce:param name=\"generation\" type=\"uint64\" inout=\"true\"/>",
        ),
        "only a map or a set parameter is written in place",
    );

    // A put with no outcome fails on a full map, which an algorithm that does
    // not declare may-fail cannot.
    parse_refused(
        &edited_algorithm(&take_generation(), " outcome=\"stored\"", ""),
        "an algorithm that does not declare may-fail names an outcome on every put",
    );

    // A map parameter not declared inout is a read-only view.
    parse_refused(
        &edited_algorithm(&take_generation(), " inout=\"true\"", ""),
        "a map or a set parameter is written only where it is declared inout=\"true\"",
    );
}

/// docs/adr/0108, D3 to D5: what a map's statements and loop in an algorithm
/// name, each refused where the document is read, naming what is accepted.
#[test]
fn an_algorithm_map_statement_is_refused_naming_what_it_takes() {
    let body_with = |statement: &str| {
        edited_algorithm(
            &take_generation(),
            "    <sce:var name=\"taken\" type=\"union:TakeResult\">",
            &format!("    {statement}\n    <sce:var name=\"taken\" type=\"union:TakeResult\">"),
        )
    };
    for (statement, part) in [
        (
            "<sce:clear target=\"stored\"/>",
            "the name of a map or a set local, or of an inout map or set parameter",
        ),
        (
            "<sce:put target=\"resets\" key=\"peer\" outcome=\"stored\"/>",
            "the value the put writes at the key",
        ),
        (
            "<sce:remove target=\"resets\" key=\"peer\" expr=\"generation\"/>",
            "a remove names the key of the entry it removes, and takes no value",
        ),
        (
            "<sce:remove target=\"resets\" key=\"peer\" outcome=\"stored\"/>",
            "sce:std/collection/remove_outcome.scxml",
        ),
        (
            "<sce:foreach item=\"g\" in=\"generation\" key=\"k\"><sce:clear target=\"resets\"/></sce:foreach>",
            "only a walk over a map binds each entry's key beside its value",
        ),
    ] {
        parse_refused(&body_with(statement), part);
    }

    // A map returned is built by puts and taken whole by a variable that holds
    // its bound: it declares may-fail (D5).
    let returns_a_map = edited_algorithm(
        &take_generation(),
        "<sce:return type=\"union:TakeResult\"/>",
        "<sce:return type=\"map&lt;uint32, uint64&gt;[4]\"/>",
    );
    parse_refused(
        &returns_a_map,
        "a map or a set an algorithm returns declares may-fail",
    );
}

/// What `generate -l c11` of the document `name`, written as `document` beside
/// `siblings` copied from the fixture directory, prints and writes: whether it
/// passed, what it printed, and every header and source it wrote, in the order
/// of their names.
fn generate_c11_beside(name: &str, document: &str, siblings: &[&str]) -> (bool, String, String) {
    let mut documents: Vec<(&str, String)> = siblings
        .iter()
        .map(|sibling| {
            let text = std::fs::read_to_string(static_fixtures().join(sibling))
                .unwrap_or_else(|e| panic!("read {sibling}: {e}"));
            (*sibling, text)
        })
        .collect();
    documents.push((name, document.to_string()));
    generate_c11_among(&documents, name)
}

/// What `generate -l c11` of `main`, written in a scratch directory beside
/// `documents`, prints and writes: whether it passed, what it printed, and
/// every header and source it wrote, in the order of their names.
fn generate_c11_among(documents: &[(&str, String)], main: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let path = dir.path().join(main);
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "c11", "-o", out.as_str()], &path);
    let mut files: Vec<PathBuf> = std::fs::read_dir(out_dir.path())
        .expect("read the output")
        .map(|entry| entry.expect("an output file").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "h" || ext == "c"))
        .collect();
    files.sort();
    let mut code = String::new();
    for path in files {
        code.push_str(&std::fs::read_to_string(&path).expect("read a generated file"));
    }
    (ok, printed, code)
}

/// The line of the generated `code` that opens the definition of a C function
/// whose declaration holds `marker`.
fn c11_definition<'c>(code: &'c str, marker: &str) -> &'c str {
    code.lines()
        .find(|line| line.starts_with("static inline ") && line.contains(marker))
        .unwrap_or_else(|| panic!("no definition holding `{marker}` in:\n{code}"))
}

/// The parameters the definition line `line` declares, each as written, read
/// apart at the commas of its parameter list.
fn c11_parameters(line: &str) -> Vec<String> {
    let list = &line[line.find('(').expect("a parameter list") + 1..];
    let list = &list[..list.rfind(')').expect("a parameter list is closed")];
    list.split(", ").map(str::to_string).collect()
}

/// The text of the function the definition line `line` opens in `code`, from
/// that line to the brace that closes it at the start of a line.
fn c11_function<'c>(code: &'c str, line: &str) -> &'c str {
    let rest = &code[code.find(line).expect("the line is of the code")..];
    &rest[..rest.find("\n}").expect("a function is closed") + 2]
}

/// docs/adr/0108, D6 items 1, 2 and 4 on C11: an algorithm is lent each map by
/// the address of its caller's storage, the core every map type begins with,
/// which views a map of any bound and is never copied. The `inout` table of
/// `algorithm_take_generation.scxml` is that address, read by `has` and at a
/// key through it, the value read as the type the algorithm names since the
/// address names no type of the map, and put into where it stands with its
/// outcome written as the standard enum's variant. A read-only slot is a
/// `const` view, and a walk of it whose body writes nothing of the map walks the
/// map itself, binding one entry at a time, every map reached through the
/// address its name reads it at.
#[test]
fn c11_lends_an_algorithm_its_map_by_address_and_copies_none() {
    let (ok, printed, code) = generate_c11_beside(
        "algorithm_take_generation.scxml",
        &take_generation(),
        &TAKE_SIBLINGS,
    );
    assert!(ok, "C11 generates the algorithm:\n{printed}");
    let line = c11_definition(&code, " take_generation(");
    assert_eq!(
        c11_parameters(line).first().map(String::as_str),
        Some("sce_map_core_t * resets"),
        "the inout table is the address of the caller's storage: {line}"
    );
    let body = c11_function(&code, line);
    for held in [
        "sce_map_holds(&((*resets))",
        "SCE_MAP_VALUE_AS(uint64_t, sce_map_lookup_found(sce_map_lookup(&((*resets))",
        "sce_map_put(&(*resets), &sce_key_, sizeof(sce_key_), &sce_value_, sizeof(sce_value_))",
        "sce_put_ == SCE_MAP_PUT_INSERTED ? ",
    ] {
        assert!(
            body.contains(held),
            "no `{held}` in the generated algorithm:\n{body}"
        );
    }
    assert!(
        code.contains("#include \"sce/forge/map.h\""),
        "the header of the map's functions is not included:\n{code}"
    );
    // A body that does not declare may-fail reads the value the lookup found,
    // with no failure to record.
    assert!(
        !body.contains("sce_map_lookup_take") && !body.contains("sce_failure_"),
        "the proven read records a failure:\n{body}"
    );
    for copy in ["sce_map_copy_for_walk", "sce_map_assign", "memcpy"] {
        assert!(!body.contains(copy), "`{copy}` copies the map:\n{body}");
    }

    let walks = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="any_above" version="1.0">
  <sce:signature>
    <sce:param name="m" type="map&lt;uint32, uint64&gt;"/>
    <sce:param name="least" type="uint64"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:var name="found" type="bool" init="false"/>
    <sce:foreach in="m" item="v" key="k">
      <sce:if cond="v &gt; least &amp;&amp; k &gt; 0">
        <sce:assign target="found" expr="true"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="found"/>
  </sce:body>
</scxml>"#;
    let (ok, printed, code) = generate_c11_beside("algorithm_any_above.scxml", walks, &[]);
    assert!(ok, "C11 generates a walk of a read-only map:\n{printed}");
    let line = c11_definition(&code, " any_above(");
    assert_eq!(
        c11_parameters(line).first().map(String::as_str),
        Some("const sce_map_core_t * m"),
        "a read-only slot is a const view of the caller's storage: {line}"
    );
    let body = c11_function(&code, line);
    for held in [
        "for (uint32_t sce_at_v_ = sce_map_first(&(*m)); sce_at_v_ != SCE_MAP_NO_ENTRY; \
         sce_at_v_ = sce_map_next(&(*m), sce_at_v_)) {",
        "const uint64_t v = *(const uint64_t *)sce_map_value_at(&(*m), sce_at_v_);",
        "const uint32_t k = *(const uint32_t *)sce_map_key_at(&(*m), sce_at_v_);",
    ] {
        assert!(body.contains(held), "no `{held}` in the walk:\n{body}");
    }
    for copy in ["sce_map_copy_for_walk", "sce_map_assign", "memcpy"] {
        assert!(!body.contains(copy), "`{copy}` copies the map:\n{body}");
    }
}

/// docs/adr/0108, D6 item 3 and D4 "Storage" on C11: a walk whose body writes
/// the map it walks walks a copy of the map as it was when the loop began, taken
/// as the loop begins into storage the caller hands the call, of the caller's
/// bound: the copy is no frame's. A walk of the same map whose body writes
/// nothing of it walks the map itself, as the walk of a read-only map does.
#[test]
fn a_c11_walk_that_writes_its_map_walks_a_copy_its_caller_holds() {
    let writes_its_map = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="drop_below" version="1.0">
  <sce:signature>
    <sce:param name="m" type="map&lt;uint32, uint64&gt;" inout="true"/>
    <sce:param name="least" type="uint64"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:var name="found" type="bool" init="false"/>
    <sce:foreach in="m" item="w" key="j">
      <sce:if cond="w &gt; least &amp;&amp; j &gt; 0">
        <sce:assign target="found" expr="true"/>
      </sce:if>
    </sce:foreach>
    <sce:foreach in="m" item="v" key="k">
      <sce:if cond="v &lt; least">
        <sce:remove target="m" key="k"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="found"/>
  </sce:body>
</scxml>"#;
    let (ok, printed, code) =
        generate_c11_beside("algorithm_drop_below.scxml", writes_its_map, &[]);
    assert!(ok, "C11 generates a walk that writes its map:\n{printed}");
    let line = c11_definition(&code, " drop_below(");
    assert_eq!(
        c11_parameters(line),
        [
            "sce_map_core_t * m",
            "uint64_t least",
            "sce_map_core_t *sce_walk_0_"
        ],
        "the copy's storage is the caller's, handed after the arguments: {line}"
    );
    let body = c11_function(&code, line);
    for held in [
        // The reading walk walks the map itself.
        "for (uint32_t sce_at_w_ = sce_map_first(&(*m)); ",
        // The writing walk copies the map as the loop begins and walks the copy.
        "sce_map_copy_for_walk(sce_walk_0_, &(*m));",
        "for (uint32_t sce_at_v_ = sce_map_first(sce_walk_0_); ",
        "sce_map_remove(&(*m), &sce_key_, NULL)",
    ] {
        assert!(body.contains(held), "no `{held}` in the walks:\n{body}");
    }
    assert_eq!(
        body.matches("sce_map_copy_for_walk(").count(),
        1,
        "only the walk that writes its map copies it:\n{body}"
    );
}

/// docs/adr/0108, D4 "Storage" and D8 on C11: a map local of an algorithm's own
/// is a member of the struct of its maps the algorithm declares and its caller
/// holds, handed by its address after the arguments, of the type a machine
/// names for a map of the same parts and bound, declared under the guard every
/// header that holds one shares. It is started where it is declared under a
/// seed derived, as a child machine's is, from the seed of the map the
/// algorithm is lent, and where it is lent none and returns none, from the seed
/// its caller hands it after the declared parameters, before the struct of its
/// maps: never a fixed one.
#[test]
fn a_c11_map_local_is_held_in_its_callers_storage_under_a_derived_seed() {
    let holds_its_own = edited_algorithm(
        &take_generation(),
        "    <sce:var name=\"taken\" type=\"union:TakeResult\">",
        "    <sce:var name=\"seen\" type=\"set&lt;uint32&gt;[4]\"/>\n    \
         <sce:clear target=\"seen\"/>\n    <sce:var name=\"taken\" type=\"union:TakeResult\">",
    );
    let (ok, printed, code) = generate_c11_beside(
        "algorithm_take_generation.scxml",
        &holds_its_own,
        &TAKE_SIBLINGS,
    );
    assert!(ok, "C11 generates an algorithm's map local:\n{printed}");
    let line = c11_definition(&code, " take_generation(");
    assert_eq!(
        c11_parameters(line).last().map(String::as_str),
        Some("take_generation_maps_t *sce_maps_"),
        "the struct of the algorithm's maps is handed last: {line}"
    );
    for held in [
        "SCE_SET_DECLARE(sce_static_set_uint32_4, uint32_t, 4u, sce_static_set_uint32_4_key_feed)",
        "    sce_static_set_uint32_4_t seen;\n} take_generation_maps_t;",
        "sce_static_set_uint32_4_start(&sce_maps_->seen, \
         sce_map_seed_of_child(resets->seed, \"take_generation.seen\"));",
        "sce_map_clear(&(sce_maps_->seen));",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated algorithm:\n{code}"
        );
    }

    let counts = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="count_distinct" version="1.0">
  <sce:signature>
    <sce:param name="ids" type="list&lt;uint32&gt;"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="seen" type="set&lt;uint32&gt;[4]"/>
    <sce:foreach in="ids" item="id">
      <sce:put target="seen" key="id"/>
    </sce:foreach>
    <sce:return expr="len(seen)"/>
  </sce:body>
</scxml>"#;
    let (ok, printed, code) = generate_c11_beside("algorithm_count_distinct.scxml", counts, &[]);
    assert!(
        ok,
        "C11 generates a map local of an algorithm lent none:\n{printed}"
    );
    let line = c11_definition(&code, " count_distinct(");
    let parameters = c11_parameters(line);
    assert_eq!(
        &parameters[parameters.len().saturating_sub(2)..],
        [
            "sce_map_seed_t sce_seed_",
            "count_distinct_maps_t *sce_maps_"
        ],
        "an algorithm lent no map is handed the seed after its arguments: {line}"
    );
    assert!(
        code.contains(
            "sce_static_set_uint32_4_start(&sce_maps_->seen, \
             sce_map_seed_of_child(sce_seed_, \"count_distinct.seen\"));"
        ),
        "the local is not keyed under the seed it is handed:\n{code}"
    );
}

/// docs/adr/0108, D5 on C11: a map an algorithm returns is built in the
/// destination its caller hands it last, with the caller's failure record, as a
/// list return is (docs/adr/0082): a map of the caller's own bound and seed,
/// reached by its core, emptied where the local it returns is declared and put
/// into where it stands. A put into it that finds it full fails the call, which
/// answers whether it built the map and passes its failure on into the caller's
/// record; the algorithm declares no result type of its own.
#[test]
fn a_c11_map_return_is_built_in_the_destination_its_caller_hands_it() {
    let keeps = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="keep_above" version="1.0">
  <sce:signature>
    <sce:param name="m" type="map&lt;uint32, uint64&gt;"/>
    <sce:param name="least" type="uint64"/>
    <sce:return type="map&lt;uint32, uint64&gt;[8]" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="kept" type="map&lt;uint32, uint64&gt;[8]"/>
    <sce:foreach in="m" item="v" key="k">
      <sce:if cond="v &gt; least">
        <sce:put target="kept" key="k" expr="v"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="kept"/>
  </sce:body>
</scxml>"#;
    let (ok, printed, code) = generate_c11_beside("algorithm_keep_above.scxml", keeps, &[]);
    assert!(ok, "C11 generates a map return:\n{printed}");
    let line = c11_definition(&code, " keep_above(");
    assert!(
        line.starts_with("static inline bool keep_above("),
        "the call answers whether it built the map: {line}"
    );
    assert_eq!(
        c11_parameters(line),
        [
            "const sce_map_core_t * m",
            "uint64_t least",
            "sce_map_core_t * kept",
            "sce_forge_algorithm_failure_t *sce_caller_failure_"
        ],
        "the destination and the caller's failure record are handed last: {line}"
    );
    let body = c11_function(&code, line);
    for held in [
        "sce_map_clear(kept);",
        "if (sce_map_put(&(*kept), &sce_key_, sizeof(sce_key_), &sce_value_, \
         sizeof(sce_value_)) == SCE_MAP_PUT_FULL) {",
        "SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED",
        "sce_forge_algorithm_fail(sce_caller_failure_, sce_failure_.error); return false;",
        "return true;",
    ] {
        assert!(
            body.contains(held),
            "no `{held}` in the map return:\n{body}"
        );
    }
    assert!(
        !code.contains("keep_above_result_t"),
        "a map return declares a result type of its own:\n{code}"
    );
}

/// docs/adr/0108, D6 items 2 and 4 on C11: the statechart that lends
/// `algorithm_take_generation.scxml` its table (`static_map_take.scxml`)
/// generates, the `<sce:call>` a call of the machine's adapter with the table
/// lent by the address of the machine's own storage and the answer stored into
/// `took`. The adapter takes that address and hands it on as it is, so nothing
/// of the map crosses and nothing is written back.
#[test]
fn a_c11_statechart_lends_its_map_to_an_algorithm_where_it_holds_it() {
    let mut siblings = TAKE_SIBLINGS.to_vec();
    siblings.push("algorithm_take_generation.scxml");
    let statechart = std::fs::read_to_string(static_fixtures().join("static_map_take.scxml"))
        .expect("read the statechart");
    let (ok, printed, code) = generate_c11_beside("static_map_take.scxml", &statechart, &siblings);
    assert!(ok, "C11 generates the statechart:\n{printed}");
    assert!(
        code.contains("&(sm->policy.v_resets).core"),
        "the table is not lent by its address:\n{code}"
    );
    let line = c11_definition(&code, "(sce_map_core_t * p0");
    assert!(
        line.contains(" call_"),
        "the table's address is taken by no adapter: {line}"
    );
    let body = c11_function(&code, line);
    assert!(
        body.contains("(p0, "),
        "the adapter hands the address on as it is:\n{body}"
    );
    for copy in ["sce_map_copy_for_walk", "sce_map_assign", "memcpy"] {
        assert!(!body.contains(copy), "`{copy}` copies the map:\n{body}");
    }
}

/// The start of the body of a variant of `algorithm_take_generation.scxml`
/// that holds a set of its own and walks the table it is lent with a body that
/// removes from it.
const TAKE_WITH_STORAGE: &str = "  <sce:body>\n    \
    <sce:var name=\"seen\" type=\"set&lt;uint32&gt;[4]\"/>\n    \
    <sce:clear target=\"seen\"/>\n    \
    <sce:foreach in=\"resets\" item=\"held\" key=\"who\">\n      \
    <sce:if cond=\"held === 0\">\n        \
    <sce:remove target=\"resets\" key=\"who\"/>\n      \
    </sce:if>\n    \
    </sce:foreach>";

/// docs/adr/0108, D4 "Storage" and D6 item 3 on C11: an algorithm a statechart
/// lends its table to that holds a map of its own and walks the table with a
/// body that writes it is handed the machine's storage for both, members of the
/// storage the machine holds for the call in its `scratch`: the struct of the
/// algorithm's maps, and for the copy, a map of the largest bound the machine
/// holds that the parameter could be lent. The adapter hands each on by its
/// address after the arguments.
#[test]
fn a_c11_statechart_hands_an_algorithm_the_storage_of_its_maps() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for sibling in TAKE_SIBLINGS {
        std::fs::copy(static_fixtures().join(sibling), dir.path().join(sibling))
            .expect("copy an import");
    }
    let algorithm = edited_algorithm(&take_generation(), "  <sce:body>", TAKE_WITH_STORAGE);
    std::fs::write(
        dir.path().join("algorithm_take_generation.scxml"),
        algorithm,
    )
    .expect("write the algorithm");
    let statechart = std::fs::read_to_string(static_fixtures().join("static_map_take.scxml"))
        .expect("read the statechart");
    let path = dir.path().join("static_map_take.scxml");
    std::fs::write(&path, statechart).expect("write the statechart");
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "c11", "-o", out.as_str()], &path);
    assert!(ok, "C11 generates the statechart:\n{printed}");
    let mut code = String::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(out_dir.path())
        .expect("read the output")
        .map(|entry| entry.expect("an output file").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "h" || ext == "c"))
        .collect();
    files.sort();
    for file in files {
        code.push_str(&std::fs::read_to_string(&file).expect("read a generated file"));
    }
    for held in [
        "take_generation_maps_t maps;",
        "sce_static_map_SchemaMapPeerPayload_uint64_2_t w0;",
        ", &s->maps, &s->w0.core)",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated machine:\n{code}"
        );
    }
}

/// `sce-codegen generate -l rust` of `main`, written in a scratch directory
/// beside `documents`, and of every document it imports where `with_imports`:
/// whether it passed, what it printed, and every Rust file it wrote, by name.
fn generate_rust_beside(
    documents: &[(&str, String)],
    main: &str,
    with_imports: bool,
) -> (bool, String, Vec<(String, String)>) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let mut args = vec!["generate", "-l", "rust", "-o", out.as_str()];
    if with_imports {
        args.push("--with-imports");
    }
    let (ok, printed) = run(&args, &dir.path().join(main));
    (ok, printed, rust_files_in(out_dir.path()))
}

/// The documents of the static fixture directory `names` names, as committed.
fn static_documents(names: &[&'static str]) -> Vec<(&'static str, String)> {
    names
        .iter()
        .map(|name| {
            let text = std::fs::read_to_string(static_fixtures().join(name))
                .unwrap_or_else(|e| panic!("read {name}: {e}"));
            (*name, text)
        })
        .collect()
}

/// Every function the Rust files `files` declare, read off their syntax
/// trees; each file is held to parse as Rust.
fn functions_of(files: &[(String, String)]) -> Vec<syn::ItemFn> {
    #[derive(Default)]
    struct Functions(Vec<syn::ItemFn>);
    impl<'ast> syn::visit::Visit<'ast> for Functions {
        fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
            self.0.push(function.clone());
            syn::visit::visit_item_fn(self, function);
        }
    }
    let mut functions = Functions::default();
    for (name, text) in files {
        let tree = syn::parse_file(text)
            .unwrap_or_else(|e| panic!("{name} does not parse as Rust: {e}\n{text}"));
        syn::visit::Visit::visit_file(&mut functions, &tree);
    }
    functions.0
}

/// The function named `name` among `functions`.
fn function_named<'f>(functions: &'f [syn::ItemFn], name: &str) -> &'f syn::ItemFn {
    functions
        .iter()
        .find(|function| function.sig.ident == name)
        .unwrap_or_else(|| panic!("no function `{name}` was generated"))
}

/// Whether `name` is the name of the adapter a Rust machine declares for a
/// call of an algorithm with state in a slot: `call_` and the name the call's
/// rewrite gives it, the callee's alias and the codes of its state arguments
/// (`call_TakeGeneration__vpv`, `type_ctx::state_call_name`).
fn is_adapter(name: &str) -> bool {
    name.strip_prefix("call_")
        .is_some_and(|coded| coded.contains("__"))
}

/// The adapter a machine declares for an algorithm call ([`is_adapter`]).
fn adapter_of(functions: &[syn::ItemFn]) -> &syn::ItemFn {
    functions
        .iter()
        .find(|function| is_adapter(&function.sig.ident.to_string()))
        .expect("the machine declares the call's adapter")
}

/// The generic parameters and the type of each parameter of `function`, each
/// as its tokens print.
fn signature_of(function: &syn::ItemFn) -> (Vec<String>, Vec<String>) {
    use quote::ToTokens;
    let generics = function
        .sig
        .generics
        .params
        .iter()
        .map(|param| param.to_token_stream().to_string())
        .collect();
    let inputs = function
        .sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(typed) => Some(typed.ty.to_token_stream().to_string()),
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    (generics, inputs)
}

/// The syntax tree of the machine among `files`.
fn machine_tree(files: &[(String, String)]) -> (syn::File, String) {
    let (_, machine) = files
        .iter()
        .find(|(name, _)| name.ends_with("_sm.rs"))
        .expect("the machine is generated");
    let tree = syn::parse_file(machine).expect("the machine parses as Rust");
    (tree, machine.clone())
}

/// Each method a syntax tree calls, with the receiver it calls it on, each as
/// its tokens print.
#[derive(Default)]
struct MethodCalls(Vec<(String, String)>);

impl<'ast> syn::visit::Visit<'ast> for MethodCalls {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        use quote::ToTokens;
        self.0.push((
            call.receiver.to_token_stream().to_string(),
            call.method.to_string(),
        ));
        syn::visit::visit_expr_method_call(self, call);
    }
}

impl MethodCalls {
    /// Each receiver a `.clone()` is called on.
    fn cloned(&self) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(_, method)| method == "clone")
            .map(|(receiver, _)| receiver.as_str())
            .collect()
    }
}

/// The arguments of each call a machine makes of the adapter of an algorithm
/// call ([`is_adapter`]), each as its tokens print.
#[derive(Default)]
struct AdapterCalls(Vec<Vec<String>>);

impl<'ast> syn::visit::Visit<'ast> for AdapterCalls {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        use quote::ToTokens;
        if let syn::Expr::Path(path) = call.func.as_ref() {
            let adapter = path
                .path
                .segments
                .last()
                .is_some_and(|segment| is_adapter(&segment.ident.to_string()));
            if adapter {
                self.0.push(
                    call.args
                        .iter()
                        .map(|arg| arg.to_token_stream().to_string())
                        .collect(),
                );
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// The documents the statechart that lends its table imports, and the
/// algorithm it lends it to, as committed.
const TAKE_DOCUMENTS: [&str; 5] = [
    "schema_map_peer.scxml",
    "enum_take_kind.scxml",
    "schema_take_duplicate.scxml",
    "union_take_result.scxml",
    "algorithm_take_generation.scxml",
];

/// docs/adr/0108, D5, D6 items 1, 2 and 4, and "Order of work", step 7: Rust
/// lowers an algorithm that writes the table it is lent in place. The
/// function is generic over the bound of the caller's map and takes it `&mut`
/// in the type a machine's variable of the same parts is held in, the record
/// key in the type its unit declares; `has` and the value at the peer's key are
/// the reads of the leaf crate, the value read where the build proved the key
/// held, since the algorithm declares no `may-fail` and has no failure to hand
/// on; and the put is the map's own. Read off the syntax tree of the generated
/// function.
#[test]
fn rust_lowers_an_algorithm_that_writes_a_map_it_is_lent_in_place() {
    let documents = static_documents(&TAKE_DOCUMENTS);
    let (ok, printed, files) =
        generate_rust_beside(&documents, "algorithm_take_generation.scxml", false);
    assert!(ok, "Rust generates the algorithm:\n{printed}");
    let functions = functions_of(&files);
    let take = function_named(&functions, "take_generation");
    let (generics, inputs) = signature_of(take);
    assert_eq!(
        generics,
        [
            "const N0 : usize",
            "S0 : :: sce_portable_bytes :: MapStorage"
        ]
    );
    assert_eq!(
        inputs.first().map(String::as_str),
        Some(
            "& mut :: sce_portable_bytes :: BoundedMap < schema_map_peer :: \
             SchemaMapPeerPayload , u64 , N0 , S0 >"
        ),
        "{inputs:?}"
    );
    let mut called = Called::default();
    syn::visit::Visit::visit_item_fn(&mut called, take);
    for read in [
        ":: sce_portable_bytes :: map :: holds",
        ":: sce_portable_bytes :: map :: held_cloned",
    ] {
        assert!(
            called.0.iter().any(|path| path == read),
            "no `{read}` in {:?}",
            called.0
        );
    }
    assert!(
        !called.0.iter().any(|path| path.ends_with("found")),
        "a read proven held does not fail: {:?}",
        called.0
    );
    let mut methods = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut methods, take);
    assert!(
        methods
            .0
            .iter()
            .any(|(receiver, method)| receiver == "resets" && method == "put"),
        "the table is put into where it is lent: {:?}",
        methods.0
    );
    // The enum the document names only as the tag of the union locals it
    // builds is spelled nowhere in the function, so its module is not `use`d:
    // an unused import is refused under `-D warnings`.
    let (_, text) = files
        .iter()
        .find(|(_, text)| text.contains("pub fn take_generation"))
        .expect("the algorithm's file");
    assert!(
        !text.contains("use super::enum_take_kind;"),
        "the tag's enum is imported though nothing names it:\n{text}"
    );
}

/// docs/adr/0108, D6 items 2 and 4: the Rust machine lends its table to the
/// algorithm by a `<sce:call>`: the call's adapter is generic over the bound
/// of the table and takes it `&mut` in the type the machine holds it in, the
/// type the algorithm's slot names, and the machine hands it its own variable,
/// `&mut self.resets`, with no copy.
#[test]
fn a_rust_machine_lends_its_table_to_be_written_in_place() {
    let mut documents = static_documents(&TAKE_DOCUMENTS);
    documents.extend(static_documents(&["static_map_take.scxml"]));
    let (ok, printed, files) = generate_rust_beside(&documents, "static_map_take.scxml", true);
    assert!(
        ok,
        "Rust generates the statechart and what it imports:\n{printed}"
    );
    let functions = functions_of(&files);
    function_named(&functions, "take_generation");
    let (generics, inputs) = signature_of(adapter_of(&functions));
    assert_eq!(generics, ["const N0 : usize"]);
    assert_eq!(
        inputs.first().map(String::as_str),
        Some(
            "& mut :: sce_rust_runtime :: BoundedMap < schema_map_peer :: \
             SchemaMapPeerPayload , u64 , N0 >"
        ),
        "{inputs:?}"
    );
    let (tree, _) = machine_tree(&files);
    let mut calls = AdapterCalls::default();
    syn::visit::Visit::visit_file(&mut calls, &tree);
    assert_eq!(calls.0.len(), 1, "{:?}", calls.0);
    assert_eq!(
        calls.0[0].first().map(String::as_str),
        Some("& mut self . resets")
    );
}

/// An algorithm that is lent the table `b` to write in place and the table `a`
/// to read: the walk of `a` writes `b` only, and the walk of `b` removes each
/// key it meets.
const REFILL_BY_WALKS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="refill"
       version="1.0">
  <sce:signature>
    <sce:param name="b" type="map&lt;uint32, uint64&gt;" inout="true"/>
    <sce:param name="a" type="map&lt;uint32, uint64&gt;"/>
  </sce:signature>
  <sce:body>
    <sce:foreach in="a" item="v" key="k">
      <sce:remove target="b" key="k"/>
    </sce:foreach>
    <sce:foreach in="b" item="w" key="j">
      <sce:remove target="b" key="j"/>
    </sce:foreach>
  </sce:body>
</scxml>
"#;

/// A statechart whose `<foreach>` over `a` calls `Refill(b, a)`: `b` is lent
/// to be written in place, and `a`, the map the loop walks, to be read.
const WALK_LEND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="walk_lend">
  <sce:import kind="algorithm" src="algorithm_refill.scxml" as="Refill"/>
  <datamodel>
    <data id="a" sce:type="map&lt;uint32, uint64&gt;[4]" sce:direction="out"/>
    <data id="b" sce:type="map&lt;uint32, uint64&gt;[4]" sce:direction="out"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <foreach array="a" item="v" sce:key="k">
        <sce:call expr="Refill(b, a)"/>
      </foreach>
    </transition>
  </state>
</scxml>
"#;

/// docs/adr/0108, D6 items 1 and 3, read off the syntax trees of the
/// generated Rust: a map lent to be read is handed over by reference and
/// copied nowhere, by the machine (`&self.a`, its walk over `a` taking no
/// copy) or by its adapter (`&BoundedMap`); and in the algorithm a
/// `<sce:foreach>` over a map its body does not write walks the map itself,
/// while one whose body writes the map it walks walks a copy of it.
#[test]
fn rust_copies_no_map_it_reads_and_only_the_map_a_walk_writes() {
    let documents = [
        ("algorithm_refill.scxml", REFILL_BY_WALKS.to_string()),
        ("walk_lend.scxml", WALK_LEND.to_string()),
    ];
    let (ok, printed, files) = generate_rust_beside(&documents, "walk_lend.scxml", true);
    assert!(
        ok,
        "Rust generates the statechart and the algorithm:\n{printed}"
    );
    let functions = functions_of(&files);

    let refill = function_named(&functions, "refill");
    let (generics, inputs) = signature_of(refill);
    // Each map lent is the caller's variable, generic over its own bound and
    // its own storage (docs/adr/0108, D4 "Storage").
    assert_eq!(
        generics,
        [
            "const N0 : usize",
            "S0 : :: sce_portable_bytes :: MapStorage",
            "const N1 : usize",
            "S1 : :: sce_portable_bytes :: MapStorage",
        ]
    );
    assert_eq!(
        inputs,
        [
            "& mut :: sce_portable_bytes :: BoundedMap < u32 , u64 , N0 , S0 >",
            "& :: sce_portable_bytes :: BoundedMap < u32 , u64 , N1 , S1 >",
        ]
    );
    let mut methods = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut methods, refill);
    assert_eq!(
        methods.cloned(),
        ["b"],
        "only the walk of `b`, whose body writes `b`, copies its map: {:?}",
        methods.0
    );

    let (_, inputs) = signature_of(adapter_of(&functions));
    assert_eq!(
        inputs,
        [
            "& mut :: sce_rust_runtime :: BoundedMap < u32 , u64 , N0 >",
            "& :: sce_rust_runtime :: BoundedMap < u32 , u64 , N1 >",
        ]
    );
    let (tree, machine) = machine_tree(&files);
    let mut calls = AdapterCalls::default();
    syn::visit::Visit::visit_file(&mut calls, &tree);
    assert_eq!(calls.0, [["& mut self . b", "& self . a"]]);
    let mut methods = MethodCalls::default();
    syn::visit::Visit::visit_file(&mut methods, &tree);
    assert!(
        !methods.cloned().contains(&"self . a"),
        "the machine copies the map it lends to be read:\n{machine}"
    );
    assert!(
        !machine.contains("sce_walk_"),
        "the walk of `a`, whose body writes only `b`, copies a map:\n{machine}"
    );
}

/// An algorithm that declares `may-fail` and returns the entries of the map it
/// is lent, keeping the keys it met in a set of its own: a map local it
/// returns, a set local it does not, and puts that name no outcome.
const DISTINCT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="distinct"
       version="1.0">
  <sce:signature>
    <sce:param name="m" type="map&lt;uint32, uint64&gt;"/>
    <sce:return type="map&lt;uint32, uint64&gt;[4]" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="seen" type="set&lt;uint32&gt;[4]"/>
    <sce:var name="out" type="map&lt;uint32, uint64&gt;[4]"/>
    <sce:foreach in="m" item="v" key="k">
      <sce:put target="seen" key="k"/>
      <sce:put target="out" key="k" expr="v"/>
    </sce:foreach>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// docs/adr/0108, D5 and D8: a Rust algorithm writes the map it returns into
/// its caller's destination, a parameter of the returned local's name after
/// the declared ones, `&mut` the map of the local's bound, emptied where the
/// local is declared, the return saying only that it was written; builds any
/// other map local under a seed drawn from the map it is lent; and fails a put
/// that names no outcome on a full map with `capacity-exceeded`.
#[test]
fn a_rust_algorithm_returns_a_map_into_its_callers_destination() {
    use quote::ToTokens;
    let documents = [("algorithm_distinct.scxml", DISTINCT.to_string())];
    let (ok, printed, files) = generate_rust_beside(&documents, "algorithm_distinct.scxml", false);
    assert!(ok, "Rust generates the algorithm:\n{printed}");
    let functions = functions_of(&files);
    let distinct = function_named(&functions, "distinct");
    let (generics, inputs) = signature_of(distinct);
    assert_eq!(
        generics,
        [
            "const N0 : usize",
            "S0 : :: sce_portable_bytes :: MapStorage",
            "SR : :: sce_portable_bytes :: MapStorage",
        ]
    );
    assert_eq!(
        inputs,
        [
            "& :: sce_portable_bytes :: BoundedMap < u32 , u64 , N0 , S0 >",
            "& mut :: sce_portable_bytes :: BoundedMap < u32 , u64 , 4 , SR >",
        ]
    );
    assert_eq!(
        distinct.sig.output.to_token_stream().to_string(),
        "-> Result < () , sce_forge_runtime :: algorithm :: AlgorithmError >"
    );
    let mut methods = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut methods, distinct);
    for (receiver, method) in [
        ("out", "clear"),
        ("m", "derived_seed"),
        ("seen", "put"),
        ("out", "put"),
    ] {
        assert!(
            methods
                .0
                .iter()
                .any(|(r, called)| r == receiver && called == method),
            "no `{receiver}.{method}` in {:?}",
            methods.0
        );
    }
    assert!(
        methods.cloned().is_empty(),
        "the walk of the map lent to be read copies it: {:?}",
        methods.0
    );
    let mut called = Called::default();
    syn::visit::Visit::visit_item_fn(&mut called, distinct);
    assert!(
        called
            .0
            .iter()
            .any(|path| path == ":: sce_portable_bytes :: FromSeed :: from_seed"),
        "the set local is built under a seed: {:?}",
        called.0
    );
    let (_, text) = files
        .iter()
        .find(|(_, text)| text.contains("pub fn distinct"))
        .expect("the algorithm's file");
    // docs/adr/0108, D4 "Storage": the set the body builds for itself is kept
    // in the storage of the map its seed is drawn from, so a caller that lends
    // an inline map has nothing allocated for it.
    assert!(
        text.contains("seen: ::sce_portable_bytes::BoundedSet<u32, 4, S0> ="),
        "the set local is kept in the storage of the map it is lent:\n{text}"
    );
    assert!(
        text.contains("== ::sce_portable_bytes::PutOutcome::Full {")
            && text.contains(
                "return Err(sce_forge_runtime::algorithm::AlgorithmError::CapacityExceeded);"
            )
            && text.contains("return Ok(());"),
        "a put with no outcome fails a full map, and the return says the map was written:\n{text}"
    );
}

/// An algorithm that is lent a table of names to write in place and one to
/// read: each name it reads is put into the first, a literal name is put and
/// then removed.
const KEEP_NAMES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="keep"
       version="1.0">
  <sce:import kind="enum" src="sce:std/collection/put_outcome.scxml" as="PutOutcome"/>
  <sce:signature>
    <sce:param name="names" type="map&lt;string[8], uint32&gt;" inout="true"/>
    <sce:param name="seen" type="map&lt;string[8], uint32&gt;"/>
  </sce:signature>
  <sce:body>
    <sce:var name="stored" type="enum:PutOutcome" init="PutOutcome.inserted"/>
    <sce:foreach in="seen" item="count" key="name">
      <sce:put target="names" key="name" expr="count" outcome="stored"/>
    </sce:foreach>
    <sce:put target="names" key="'zz'" expr="0" outcome="stored"/>
    <sce:remove target="names" key="'zz'"/>
  </sce:body>
</scxml>
"#;

/// A statechart that lends its two tables of names to `Keep(names, seen)`.
const TEXT_LEND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="text_lend">
  <sce:import kind="algorithm" src="algorithm_keep.scxml" as="Keep"/>
  <datamodel>
    <data id="names" sce:type="map&lt;string[8], uint32&gt;[4]" sce:direction="out"/>
    <data id="seen" sce:type="map&lt;string[8], uint32&gt;[4]" sce:direction="out"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <sce:call expr="Keep(names, seen)"/>
    </transition>
  </state>
</scxml>
"#;

/// docs/adr/0108, D1, D6 items 1, 2 and 4: a map keyed by a text is lent to a
/// Rust algorithm read-only and in place. Each text key is held in
/// `SceString` of the bound the slot's type states, in the algorithm, the
/// adapter and the machine alike, so the map crosses whole; a walk's key is put
/// by a widening the compiler judges, a literal by a construction it judges,
/// and a remove looks the literal up as it is.
#[test]
fn a_map_keyed_by_a_text_is_lent_to_a_rust_algorithm() {
    let documents = [
        ("algorithm_keep.scxml", KEEP_NAMES.to_string()),
        ("text_lend.scxml", TEXT_LEND.to_string()),
    ];
    let (ok, printed, files) = generate_rust_beside(&documents, "text_lend.scxml", true);
    assert!(
        ok,
        "Rust generates the statechart and the algorithm:\n{printed}"
    );
    let functions = functions_of(&files);
    let keep = function_named(&functions, "keep");
    let (generics, inputs) = signature_of(keep);
    assert_eq!(
        generics,
        [
            "const N0 : usize",
            "S0 : :: sce_portable_bytes :: MapStorage",
            "const N1 : usize",
            "S1 : :: sce_portable_bytes :: MapStorage",
        ]
    );
    assert_eq!(
        inputs,
        [
            "& mut :: sce_portable_bytes :: BoundedMap < :: sce_portable_bytes :: SceString < 8 \
             > , u32 , N0 , S0 >",
            "& :: sce_portable_bytes :: BoundedMap < :: sce_portable_bytes :: SceString < 8 > , \
             u32 , N1 , S1 >",
        ]
    );
    let (_, text) = files
        .iter()
        .find(|(_, text)| text.contains("pub fn keep"))
        .expect("the algorithm's file");
    for part in [
        "::sce_portable_bytes::SceString::<8>::widened(&name)",
        "::sce_portable_bytes::SceString::<8>::proven::<{ \"zz\".len() }>(\"zz\")",
    ] {
        assert!(text.contains(part), "no `{part}` in:\n{text}");
    }
    let mut methods = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut methods, keep);
    assert!(
        methods
            .0
            .iter()
            .any(|(receiver, method)| receiver == "names" && method == "remove"),
        "{:?}",
        methods.0
    );

    let (_, inputs) = signature_of(adapter_of(&functions));
    assert_eq!(
        inputs,
        [
            "& mut :: sce_rust_runtime :: BoundedMap < :: sce_rust_runtime :: portable :: \
             SceString < 8 > , u32 , N0 >",
            "& :: sce_rust_runtime :: BoundedMap < :: sce_rust_runtime :: portable :: SceString \
             < 8 > , u32 , N1 >",
        ]
    );
    let (tree, _) = machine_tree(&files);
    let mut calls = AdapterCalls::default();
    syn::visit::Visit::visit_file(&mut calls, &tree);
    assert_eq!(calls.0, [["& mut self . names", "& self . seen"]]);
}

/// An algorithm that counts the distinct numbers of a list in a set of its
/// own, lent no map and returning none: it takes its caller's seed.
const DISTINCT_COUNT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="distinct_count"
       version="1.0">
  <sce:signature>
    <sce:param name="xs" type="list&lt;uint32&gt;"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="seen" type="set&lt;uint32&gt;[4]"/>
    <sce:foreach in="xs" item="x">
      <sce:put target="seen" key="x"/>
    </sce:foreach>
    <sce:return expr="len(seen)"/>
  </sce:body>
</scxml>
"#;

/// A statechart that counts the distinct numbers of its list by
/// `DistinctCount(xs)`.
const SEED_LEND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="seed_lend">
  <sce:import kind="algorithm" src="algorithm_distinct_count.scxml" as="DistinctCount"/>
  <datamodel>
    <data id="xs" sce:type="list&lt;uint32&gt;[4]" sce:direction="out"/>
    <data id="n" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="n" expr="DistinctCount(xs)"/>
    </transition>
  </state>
</scxml>
"#;

/// docs/adr/0108, D8: an algorithm that builds a map local and is lent no map
/// and returns none takes the seed of the local's keys from its caller, as a
/// parameter after its declared ones, and draws the local's from it; a Rust
/// machine that calls one keeps a seed drawn where it is built from the seed
/// of its maps, and hands it to each call, through the call's adapter.
#[test]
fn a_rust_machine_hands_its_seed_to_an_algorithm_that_builds_a_map() {
    let documents = [
        ("algorithm_distinct_count.scxml", DISTINCT_COUNT.to_string()),
        ("seed_lend.scxml", SEED_LEND.to_string()),
    ];
    let (ok, printed, files) = generate_rust_beside(&documents, "seed_lend.scxml", true);
    assert!(
        ok,
        "Rust generates the statechart and the algorithm:\n{printed}"
    );
    let functions = functions_of(&files);
    let distinct = function_named(&functions, "distinct_count");
    let (generics, inputs) = signature_of(distinct);
    assert_eq!(
        inputs.last().map(String::as_str),
        Some(":: sce_portable_bytes :: MapSeed"),
        "{inputs:?}"
    );
    // Lent no map, it has no storage to follow: its set is kept in the
    // default storage, and the function is generic over none.
    assert!(
        !generics
            .iter()
            .any(|generic| generic.contains("MapStorage")),
        "{generics:?}"
    );
    let (_, text) = files
        .iter()
        .find(|(_, text)| text.contains("pub fn distinct_count"))
        .expect("the algorithm's file");
    assert!(
        text.contains("seen: ::sce_portable_bytes::BoundedSet<u32, 4> ="),
        "the set local is kept in the default storage:\n{text}"
    );
    let mut methods = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut methods, distinct);
    assert!(
        methods
            .0
            .iter()
            .any(|(receiver, method)| receiver == "sce_map_seed" && method == "derive"),
        "the set local draws its seed from its caller's: {:?}",
        methods.0
    );

    let (_, inputs) = signature_of(adapter_of(&functions));
    assert_eq!(
        inputs.last().map(String::as_str),
        Some(":: sce_rust_runtime :: MapSeed"),
        "{inputs:?}"
    );
    let (tree, machine) = machine_tree(&files);
    let mut calls = AdapterCalls::default();
    syn::visit::Visit::visit_file(&mut calls, &tree);
    assert_eq!(calls.0.len(), 1, "{:?}", calls.0);
    assert_eq!(
        calls.0[0].last().map(String::as_str),
        Some("self . sce_algorithm_seed")
    );
    let mut declared = Declared::default();
    syn::visit::Visit::visit_file(&mut declared, &tree);
    {
        use quote::ToTokens;
        assert_eq!(
            declared
                .field("sce_algorithm_seed", &machine)
                .to_token_stream()
                .to_string(),
            ":: sce_rust_runtime :: MapSeed"
        );
    }
    assert!(
        machine.contains("sce_algorithm_seed: sce_map_seeds.draw(),"),
        "the machine draws its algorithms' seed where it is built:\n{machine}"
    );
}

/// An algorithm that puts two numbers into a set of its own and answers how
/// many distinct ones it holds, lent no map and returning none: it takes its
/// caller's seed for its own map.
const DISTINCT_PAIR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="distinct_pair"
       version="1.0">
  <sce:signature>
    <sce:param name="a" type="uint32"/>
    <sce:param name="b" type="uint32"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="seen" type="set&lt;uint32&gt;[2]"/>
    <sce:put target="seen" key="a"/>
    <sce:put target="seen" key="b"/>
    <sce:return expr="len(seen)"/>
  </sce:body>
</scxml>
"#;

/// An algorithm that builds no map and is lent none, and answers what
/// `DistinctPair` answers: it takes its caller's seed all the same, since an
/// algorithm it calls does.
const DISTINCT_THROUGH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="distinct_through"
       version="1.0">
  <sce:import kind="algorithm" src="algorithm_distinct_pair.scxml" as="DistinctPair"/>
  <sce:signature>
    <sce:param name="a" type="uint32"/>
    <sce:param name="b" type="uint32"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="DistinctPair(a, b)"/>
  </sce:body>
</scxml>
"#;

/// docs/adr/0108, D8: whether an algorithm takes its caller's seed is a fixed
/// point over the algorithms it calls. `DistinctThrough` builds no map, is
/// lent none and returns none, and calls `DistinctPair`, which builds a set,
/// so both take a seed after their declared parameters, and the caller hands
/// the callee one derived from its own: on Rust `sce_map_seed.derive(..)`; on
/// C11 `sce_map_seed_of_child` of its `sce_seed_` under the callee's alias,
/// followed by the struct of the callee's own maps, a member of the caller's.
#[test]
fn an_algorithm_that_calls_one_that_takes_a_seed_takes_one_itself() {
    let documents = [
        ("algorithm_distinct_pair.scxml", DISTINCT_PAIR.to_string()),
        (
            "algorithm_distinct_through.scxml",
            DISTINCT_THROUGH.to_string(),
        ),
    ];
    let mut functions = Vec::new();
    for (main, name) in [
        ("algorithm_distinct_pair.scxml", "distinct_pair"),
        ("algorithm_distinct_through.scxml", "distinct_through"),
    ] {
        let (ok, printed, files) = generate_rust_beside(&documents, main, false);
        assert!(ok, "Rust generates `{name}`:\n{printed}");
        functions.extend(functions_of(&files));
        let (_, inputs) = signature_of(function_named(&functions, name));
        assert_eq!(
            inputs.last().map(String::as_str),
            Some(":: sce_portable_bytes :: MapSeed"),
            "`{name}` takes no seed from its caller: {inputs:?}"
        );
    }
    let mut methods = MethodCalls::default();
    syn::visit::Visit::visit_item_fn(&mut methods, function_named(&functions, "distinct_through"));
    assert!(
        methods
            .0
            .iter()
            .any(|(receiver, method)| receiver == "sce_map_seed" && method == "derive"),
        "the call is handed no seed derived from the caller's: {:?}",
        methods.0
    );

    let (ok, printed, code) = generate_c11_among(&documents, "algorithm_distinct_pair.scxml");
    assert!(ok, "C11 generates the callee:\n{printed}");
    let line = c11_definition(&code, " distinct_pair(");
    let parameters = c11_parameters(line);
    assert_eq!(
        &parameters[parameters.len().saturating_sub(2)..],
        [
            "sce_map_seed_t sce_seed_",
            "distinct_pair_maps_t *sce_maps_"
        ],
        "the callee takes no seed before the struct of its maps: {line}"
    );
    let (ok, printed, code) = generate_c11_among(&documents, "algorithm_distinct_through.scxml");
    assert!(ok, "C11 generates the caller:\n{printed}");
    let line = c11_definition(&code, " distinct_through(");
    let parameters = c11_parameters(line);
    assert_eq!(
        &parameters[parameters.len().saturating_sub(2)..],
        [
            "sce_map_seed_t sce_seed_",
            "distinct_through_maps_t *sce_maps_"
        ],
        "the caller takes no seed though it calls one that does: {line}"
    );
    for held in [
        "    distinct_pair_maps_t sce_call_0_;\n} distinct_through_maps_t;",
        "distinct_pair(a, b, sce_map_seed_of_child(sce_seed_, \
         \"distinct_through.DistinctPair\"), &sce_maps_->sce_call_0_)",
    ] {
        assert!(code.contains(held), "no `{held}` in the caller:\n{code}");
    }
}

/// A statechart that holds no map and counts the distinct numbers of two of
/// its variables by `DistinctPair(a, b)`, an algorithm of scalar slots called
/// by its own name, which takes its caller's seed for a set of its own.
const SEED_BY_NAME: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="seed_by_name">
  <sce:import kind="algorithm" src="algorithm_distinct_pair.scxml" as="DistinctPair"/>
  <datamodel>
    <data id="a" sce:type="uint32" expr="1" sce:direction="out"/>
    <data id="b" sce:type="uint32" expr="2" sce:direction="out"/>
    <data id="n" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="n" expr="DistinctPair(a, b)"/>
    </transition>
  </state>
</scxml>
"#;

/// What `generate -l c11` of [`SEED_BY_NAME`], beside `DistinctPair`, writes.
fn seed_by_name_c11() -> String {
    let documents = [
        ("algorithm_distinct_pair.scxml", DISTINCT_PAIR.to_string()),
        ("seed_by_name.scxml", SEED_BY_NAME.to_string()),
    ];
    let (ok, printed, code) = generate_c11_among(&documents, "seed_by_name.scxml");
    assert!(
        ok,
        "C11 generates a machine that holds no map and calls an algorithm that takes a \
         seed:\n{printed}"
    );
    code
}

/// docs/adr/0108, D8 on C11: a machine that holds no map but calls an
/// algorithm that takes its caller's seed takes a seed at every start, its
/// construction's and its restore's, and keeps it to hand each call; its
/// saved-state judge puts no map's entries anywhere, so it is handed no
/// storage to judge them in. The header's and `saved.c`'s seed is the
/// machine's maps' or its algorithms', and their judge only its maps'.
#[test]
fn a_c11_machine_that_holds_no_map_takes_a_seed_for_the_algorithm_it_calls() {
    let code = seed_by_name_c11();
    for held in [
        "seed_by_name_init(seed_by_name_t *sm, sce_map_seed_t map_seed)",
        "    sce_map_seed_t map_seed;\n",
        "sce_map_seed_t map_seed, sce_saved_error_t *err) {",
        "bool seed_by_name_judge(const char *text, size_t len, sce_saved_error_t *err)",
    ] {
        assert!(code.contains(held), "no `{held}` in the machine:\n{code}");
    }
    assert!(
        !code.contains("seed_by_name_saved_check_t"),
        "a machine that holds no map judges no map of a saved state:\n{code}"
    );
}

/// docs/adr/0108, D4 "Storage" and D8 on C11: an algorithm a statechart calls
/// by its own name that takes its caller's seed holds maps of its own, in
/// storage the machine holds for the call: a member of its `scratch` of the
/// struct of the algorithm's maps, handed after the seed the machine keeps for
/// its algorithms (`ScratchSite::Maps`).
#[test]
fn a_c11_machine_holds_the_maps_of_an_algorithm_it_calls_by_its_own_name() {
    let code = seed_by_name_c11();
    for held in [
        "distinct_pair_maps_t maps_distinct_pair;",
        "sm->map_seed, &sm->scratch.call.maps_distinct_pair",
    ] {
        assert!(code.contains(held), "no `{held}` in the machine:\n{code}");
    }
}
