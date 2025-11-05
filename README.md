# Rust Sega Genesis Demo

## Usage:
This demo requires Nightly Rust (because m68k support is very unstable in both LLVM and Rust).

This command will enable Nightly.
```
$ rustup toolchain install nightly
```

Next, you need to install [cargo-binutils](https://crates.io/crates/cargo-binutils).

```
$ cargo install cargo-binutils

$ rustup component add llvm-tools
```

Now, use `cargo objcopy` to first compile the program normally, then turn that output file into a raw binary.

This command will do exactly that:
```
$ cargo objcopy --release -- -O binary target/m68k-none-eabi/release/mdrs.bin
```

If you want debug symbols (which drastically increases binary size), omit the `--release` parameter:
```
$ cargo objcopy -- -O binary target/m68k-none-eabi/release/mdrs.bin
```

## What's going to be in the demo?

> *Whatever I want.* 

> If you want to try something different, feel free to clone this repository.

## Wait, so m68k is supported on Rust now?
![](https://i.imgur.com/fEyWqUs.jpeg)

> It compiles, but so far both LLVM and Rust's m68k backend are **very unfinished and unstable**. 

> The LLVM backend doesn't even support every instruction the m68k has to offer, *even in inline assembly!* 

> LLVM sometimes even straight up generates the wrong opcode for an instruction! ([here's a particularly infuriating example of this](https://github.com/llvm/llvm-project/issues/165077))

> So yeah, technically it *works*, but it doesn't *work very well*.


