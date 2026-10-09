#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod install;
mod integration;
mod payload;
mod ui;
use dictado_lite::{
    locale::text,
    platform::windows::{pw, wide},
};
use payload::Result;
use windows::{core::PCWSTR, Win32::UI::WindowsAndMessaging::*};
fn message(value: &str, flags: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    let message: Vec<_> = value.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            pw(&wide(text(
                "Dictado Lite · Instalación",
                "Dictado Lite · Setup",
            ))),
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
                text("Dictado Lite se ha desinstalado. Tus ajustes se conservan.", "Dictado Lite has been uninstalled. Your settings and model cache are preserved."),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        return Ok(());
    }
    let uninstall_image = std::env::current_exe()?
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("dictado-uninstall.exe"));
    if args.iter().any(|a| a == "--uninstall") || (args.is_empty() && uninstall_image) {
        if quiet
            || message(
                text(
                    "¿Desinstalar Dictado Lite? Se conservarán tus ajustes.",
                    "Uninstall Dictado Lite? Your settings and model cache will be preserved.",
                ),
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
        println!(
            "{}",
            serde_json::json!({"installed":root,"models_included":false})
        );
    } else if let Some((root, launch)) = ui::run(test)? {
        if launch {
            std::process::Command::new(root.join("dictado-lite.exe")).spawn()?;
        }
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
