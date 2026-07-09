//! Fails the build if the test fixtures drift from the canonical docs/fixtures copies.
use std::path::Path;

fn main() {
    // docs/fixtures lives two levels up from src/core.
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures");
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    println!("cargo:rerun-if-changed={}", docs.display());
    println!("cargo:rerun-if-changed={}", tests.display());
    if let Err(e) = compare_dirs(&docs, &tests) {
        panic!(
            "fixture mirror out of sync: {e}\nRe-copy docs/fixtures into src/core/tests/fixtures"
        );
    }
}

fn compare_dirs(a: &Path, b: &Path) -> Result<(), String> {
    for sub in ["gourmet", "ventopay"] {
        let (da, db) = (a.join(sub), b.join(sub));
        let mut names: Vec<_> = std::fs::read_dir(&da)
            .map_err(|e| format!("read {}: {e}", da.display()))?
            .filter_map(|e| e.ok().map(|e| e.file_name()))
            .collect();
        names.sort();
        for name in names {
            let fa = std::fs::read(da.join(&name)).map_err(|e| e.to_string())?;
            let fb = std::fs::read(db.join(&name))
                .map_err(|_| format!("missing mirror {}/{:?}", sub, name))?;
            if fa != fb {
                return Err(format!("byte mismatch in {}/{:?}", sub, name));
            }
        }
    }
    Ok(())
}
