use std::path::Path;
use std::process::Command;
use std::env;

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Note that there are a number of downsides to this approach, the comments
    // below detail how to improve the portability of these commands.
    Command::new("m68k-linux-gnu-gcc").args(&["src/header.S", "-c", "-o"])
        .arg(&format!("{}/header.o", out_dir))
        .status().unwrap();
    Command::new("m68k-linux-gnu-gcc").args(&["src/sys/libc.S", "-c", "-o"])
        .arg(&format!("{}/libc.o", out_dir))
        .status().unwrap();
    Command::new("m68k-linux-gnu-ar").args(&["crus", "libheader.a", "header.o", "libc.o"])
        .current_dir(&Path::new(&out_dir))
        .status().unwrap();

    println!("cargo::rustc-link-search=native={}", out_dir);
    println!("cargo::rustc-link-lib=static=header");
    println!("cargo::rerun-if-changed=src/header.S");
    println!("cargo::rerun-if-changed=src/sys/libc_a.S");
    println!("cargo::rerun-if-changed=build.rs");
}