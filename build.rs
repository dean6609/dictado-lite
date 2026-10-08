//! Compile owned Windows icon/manifest using the already selected native tools.
use std::{env, path::PathBuf, process::Command};
fn main() {
    for path in ["assets/app.rc", "assets/app.manifest", "assets/dictado.ico"] {
        println!("cargo:rerun-if-changed={path}");
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let gnu = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
    let resource = out.join(if gnu { "app.o" } else { "app.res" });
    let status = if gnu {
        Command::new("x86_64-w64-mingw32-windres")
            .args(["-i", "assets/app.rc", "-O", "coff", "-o"])
            .arg(&resource)
            .status()
    } else {
        let mut compiler = PathBuf::from("rc.exe");
        if Command::new(&compiler).arg("/?").output().is_err() {
            let kits =
                PathBuf::from(env::var_os("ProgramFiles(x86)").expect("Windows SDK directory"))
                    .join("Windows Kits/10/bin");
            let mut candidates: Vec<_> = std::fs::read_dir(kits)
                .expect("Windows SDK")
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("x64/rc.exe"))
                .filter(|path| path.is_file())
                .collect();
            candidates.sort();
            compiler = candidates.pop().expect("Windows SDK resource compiler");
        }
        Command::new(compiler)
            .arg("/nologo")
            .arg("/fo")
            .arg(&resource)
            .arg("assets/app.rc")
            .status()
    }
    .expect("Resource compiler unavailable; run scripts/build-native.ps1 first");
    assert!(status.success(), "Windows resource compilation failed");
    println!("cargo:rustc-link-arg={}", resource.display());
}
