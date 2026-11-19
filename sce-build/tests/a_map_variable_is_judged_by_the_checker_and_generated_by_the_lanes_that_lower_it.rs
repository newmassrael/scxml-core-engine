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
    let mut files = Vec::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("read the output") {
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
    (ok, printed, files)
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
            called.0.iter().any(|path| *path == name),
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

/// docs/adr/0108, D6 item 4: a map's enum value is held in the type the
/// enum's unit declares, as a record entry is. A value of the machine's own
/// enum is put through the function that crosses it into the unit's type, and
/// the saved state writes each value the map holds back through the other, so
/// it is saved as a variable of the machine's enum is. Read off the syntax
/// tree.
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
         <sce:put target=\"outcomes\" key=\"1\" expr=\"putResult\"/>",
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
            called.0.iter().any(|path| *path == name),
            "the put and the saved form of `outcomes` do not cross through `{name}`:\n{machine}"
        );
    }
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
/// peer's key inside the map where `has` holds it. No backend lowers an
/// algorithm's map yet ("Order of work", step 7), so each refuses the
/// algorithm by name, naming the record that lands it, rather than by any
/// judge of its body.
#[test]
fn an_algorithm_holds_an_inout_map_and_every_backend_refuses_it_by_name() {
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
    for &language in sce_build::generator::Language::ALL {
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
