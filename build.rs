use std::path::Path;
use std::process::Command;
use std::env;

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Note that there are a number of downsides to this approach, the comments
    // below detail how to improve the portability of these commands.
    Command::new("m68k-linux-gnu-gcc").args(&["src/sys/libc.S", "-c", "-o"])
        .arg(&format!("{}/libc.o", out_dir))
        .status().unwrap();
    Command::new("m68k-linux-gnu-gcc").args(&["src/ba_video_decode.S", "-c", "-o"])
        .arg(&format!("{}/ba_video_decode.o", out_dir))
        .status().unwrap();
    Command::new("m68k-linux-gnu-ar").args(&["crus", "libmdrsasm.a", "libc.o", "ba_video_decode.o"])
        .current_dir(&Path::new(&out_dir))
        .status().unwrap();

    println!("cargo::rustc-link-search=native={}", out_dir);
    println!("cargo::rustc-link-lib=static=mdrsasm");
    println!("cargo::rerun-if-changed=src/sys/libc_a.S");
    println!("cargo::rerun-if-changed=src/ba_video_decode.S");
    println!("cargo::rerun-if-changed=build.rs");
}