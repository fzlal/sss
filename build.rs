use std::path::{Path, PathBuf};

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        alsa_runtime_shim();
    }
}

// quad-snd links with -lasound, which normally requires the libasound2-dev
// package. Many desktops only ship the runtime library (libasound.so.2), so
// point the linker at that file under the expected name when the dev symlink
// is missing.
fn alsa_runtime_shim() {
    let mut candidates: Vec<PathBuf> = vec![
        "/usr/lib/libasound.so.2".into(),
        "/usr/lib64/libasound.so.2".into(),
        "/lib/libasound.so.2".into(),
        "/lib64/libasound.so.2".into(),
    ];
    for root in ["/usr/lib", "/lib"] {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                candidates.push(entry.path().join("libasound.so.2"));
            }
        }
    }

    let Some(library) = candidates.iter().find(|path| path.is_file()) else {
        return;
    };
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let link = out.join("libasound.so");
    if !link.is_file() {
        let _ = symlink(library, &link);
    }
    if link.is_file() {
        println!("cargo:rustc-link-search=native={}", out.display());
    }
}

#[cfg(unix)]
fn symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[cfg(not(unix))]
fn symlink(_original: &Path, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symlinks unsupported",
    ))
}
