#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod install;
mod integration;
mod payload;
mod ui;
use payload::Result;
use windows::{core::PCWSTR, Win32::UI::WindowsAndMessaging::*};
fn message(text: &str, flags: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    let text: Vec<_> = text.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            windows::core::w!("Dictado Lite · Instalación"),
            flags,
        )
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let test = args.iter().any(|a| a == "--test");
    let quiet = args.iter().any(|a| a == "--quiet");
    if args.iter().any(|a| a == "--help") {
        println!("Dictado Lite setup: [--quiet] [--test] [--uninstall]. Test uses a separate fixed per-user directory, without shortcuts or registry integration.");
        return Ok(());
    }
    if args.iter().any(|a| a == "--remove") {
        install::uninstall(test)?;
        if !quiet {
            message(
                "Dictado Lite se ha desinstalado. Tus ajustes se conservan.",
                MB_OK | MB_ICONINFORMATION,
            );
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--uninstall") {
        if quiet
            || message(
                "¿Desinstalar Dictado Lite? Se conservarán tus ajustes.",
                MB_YESNO | MB_ICONQUESTION,
            ) == IDYES
        {
            install::launch_uninstall(test, quiet)?;
        }
        return Ok(());
    }
    if !args
        .iter()
        .all(|a| matches!(a.as_str(), "--test" | "--quiet"))
    {
        return Err("Unknown setup argument".into());
    }
    if quiet {
        let root = install::install(test, |_, _| true)?;
        println!("{}", serde_json::json!({"installed":root,"offline":true}));
    } else if let Some(root) = ui::run(test)? {
        if message("Instalación completa.\n\nMantén Ctrl+Alt+Space para hablar y suelta para pegar. Escape cancela.\n\n¿Abrir Dictado Lite ahora?",MB_YESNO|MB_ICONINFORMATION)==IDYES{std::process::Command::new(root.join("dictado-lite.exe")).spawn()?;}
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        if !std::env::args().any(|a| a == "--quiet") {
            message(
                &format!("No se pudo completar la operación.\n\n{e}"),
                MB_OK | MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
}
