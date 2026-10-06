use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

const SSR_BUNDLE: &str = "dist-ssr/ssr.mjs";

fn main() {
    println!("cargo:rerun-if-changed={SSR_BUNDLE}");
    println!("cargo:rerun-if-changed=dist");
    println!("cargo:rerun-if-changed=assets");

    let output_root = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let output = output_root.join("inertia-ssr.mjs");
    let bundle = match fs::read(SSR_BUNDLE) {
        Ok(bundle) => bundle,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            println!(
                "cargo:warning=Inertia SSR bundle is absent; run `npm run build` before the production Cargo build"
            );
            Vec::new()
        }
        Err(error) => panic!("could not read {SSR_BUNDLE}: {error}"),
    };
    fs::write(output, bundle).expect("could not stage the Inertia SSR bundle");
    stage_directory(Path::new("dist"), &output_root.join("dist"));
    stage_directory(Path::new("assets"), &output_root.join("assets"));
}

fn stage_directory(source: &Path, destination: &Path) {
    if destination.exists() {
        fs::remove_dir_all(destination).expect("could not clear staged frontend assets");
    }
    fs::create_dir_all(destination).expect("could not create staged frontend asset directory");
    if !source.is_dir() {
        return;
    }
    copy_directory(source, destination);
}

fn copy_directory(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).expect("could not read frontend asset directory") {
        let entry = entry.expect("could not inspect frontend asset");
        let target = destination.join(entry.file_name());
        if entry
            .file_type()
            .expect("could not inspect asset type")
            .is_dir()
        {
            fs::create_dir_all(&target).expect("could not create staged asset directory");
            copy_directory(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("could not stage frontend asset");
        }
    }
}
