use std::fs::{self, File};
use std::io;
use std::io::Write;
use std::path::Path;

fn process_files(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        // recurse and register paths as rebuild conditions
        println!("cargo:rerun-if-changed={}", path.display());

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            process_files(&path)?;
        }
    } else {
        // compile any *.sass and *.scss files
        if let Some(ext) = path.extension() {
            if ext == "sass" || ext == "scss" {
                // register any scss and sass files found as triggers for a rebuild
                println!("cargo:rerun-if-changed={}", path.display());

                let is_source_file = if let Some(os_filename) = path.file_name() {
                    if let Some(filename) = os_filename.to_str() {
                        !filename.starts_with('_')
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_source_file {
                    // run the compilation abd write to css
                    let output_path = path.with_extension("css");
                    match sass_rs::compile_file(path, sass_rs::Options::default()) {
                        Ok(compiled) => {
                            let mut css_file = File::create(output_path)?;
                            css_file.write_all(compiled.as_bytes())?;
                        }
                        Err(msg) => panic!("Compilation of sass failed: {}", msg),
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() {
    // build any sass files found in the stylesheets folder
    let sass_path = Path::new("static/stylesheets/");
    if !sass_path.exists() {
        panic!(
            "The static stylesheets path at {} could not be found. Please make sure it exists.",
            sass_path.canonicalize().expect("Invalid path").display()
        );
    } else if let Err(e) = process_files(sass_path) {
        panic!("{}", e);
    }
}
