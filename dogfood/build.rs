//! Generate the instruction client at build time from the program's own source.
//!
//! `testsvm-quasar-idl` reads `../programs/quasar-dice/src` with `syn` (the
//! `#[program]` instructions and the `#[derive(Accounts)]` structs they name),
//! so there is no `quasar idl-build` step and no IDL JSON: the shape comes
//! straight from the declaration site. `cargo test` runs this; `rerun-if-changed`
//! on every program source file regenerates only when the program changes. The
//! client lands in OUT_DIR (src/lib.rs `include!`s it), so it cannot drift.

use {
    std::{env, fs, path::Path},
    testsvm_quasar_idl::{emit_client, QuasarSource},
};

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src = Path::new(&manifest).join("../programs/quasar-dice/src");

    // Watch every program source file: the shape is extracted from them now.
    rerun_on_rs(&src);

    let idl =
        QuasarSource::from_crate(&src).unwrap_or_else(|e| panic!("extract {}: {e}", src.display()));
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("generated_client.rs");
    fs::write(&out, emit_client(&idl)).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
}

/// Emit `rerun-if-changed` for every `.rs` under `dir` (a bare directory watch
/// does not reliably catch edits to files within it).
fn rerun_on_rs(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        panic!("read program source dir {}", dir.display());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rerun_on_rs(&path);
        } else if path.extension().is_some_and(|e| e == "rs") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
