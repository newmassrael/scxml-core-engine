fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: gen_cmp <scxml> <lang> <output_dir>");
        std::process::exit(2);
    }
    let lang: sce_build::generator::Language = args[2].parse().unwrap();
    let tdir = sce_build::find_template_dir_for(lang);
    let output = sce_build::compile_scxml_lang(&args[1], &tdir, lang).unwrap();
    for (filename, code) in &output.files {
        let path = std::path::Path::new(&args[3]).join(filename);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, code).unwrap();
        eprintln!("Written: {}", path.display());
    }
}
