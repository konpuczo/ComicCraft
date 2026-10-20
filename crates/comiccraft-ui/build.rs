use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let rc_path = out_dir.join("comiccraft.rc");
    let res_path = out_dir.join("comiccraft.res.o");

    let rc_content = r#"1 VERSIONINFO
FILEVERSION 0,1,0,0
PRODUCTVERSION 0,1,0,0
FILEOS 0x40004
FILETYPE 0x1
{
    BLOCK "StringFileInfo"
    {
        BLOCK "040904B0"
        {
            VALUE "CompanyName", "Learning Machines LLC\0"
            VALUE "FileDescription", "ComicCraft — Comics, manga and digital illustration\0"
            VALUE "FileVersion", "0.1.0\0"
            VALUE "InternalName", "comiccraft\0"
            VALUE "OriginalFilename", "comiccraft.exe\0"
            VALUE "ProductName", "ComicCraft\0"
            VALUE "ProductVersion", "0.1.0\0"
        }
    }
    BLOCK "VarFileInfo"
    {
        VALUE "Translation", 0x0409, 1200
    }
}
"#;

    if fs::write(&rc_path, rc_content).is_ok() {
        let status = Command::new("windres")
            .args([
                rc_path.to_str().unwrap_or(""),
                "-O",
                "coff",
                "-o",
                res_path.to_str().unwrap_or(""),
            ])
            .status();

        if let Ok(s) = status {
            if s.success() {
                println!("cargo:rustc-link-arg-bins={}", res_path.display());
            }
        }
    }
}
