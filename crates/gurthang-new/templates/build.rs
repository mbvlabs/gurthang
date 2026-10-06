use std::{
    env, fs,
    io::ErrorKind,
    path::PathBuf,
};

const SSR_BUNDLE: &str = "dist-ssr/ssr.mjs";

fn main() {
    println!("cargo:rerun-if-changed={SSR_BUNDLE}");
    println!("cargo:rerun-if-changed=assets");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"))
        .join("inertia-ssr.mjs");
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
}
