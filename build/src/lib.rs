use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn embed_elf() {
    // Always rerun the script.
    println!("cargo:rerun-if-changed=src/d");

    // Get enviroment variables.
    let ignore = env::var("SUCCINCT_BUILD_IGNORE")
        .map(|_| true)
        .unwrap_or(false);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let pkg_name = env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set");

    // Only run the build script if thce SUCCINCT_BUILD_IGNORE environment variable is not set.
    if !ignore {
        // Get the output directory.
        let out_dir = Path::new(&manifest_dir);

        // Get the rustc binary.
        let mut cmd = Command::new("rustup");
        for (key, _val) in env::vars().filter(|x| x.0.starts_with("CARGO")) {
            cmd.env_remove(key);
        }
        cmd.env_remove("RUSTUP_TOOLCHAIN");
        let rustc = cmd
            .args(["+succinct", "which", "rustc"])
            .output()
            .expect("failed to find rustc")
            .stdout;
        let rustc = String::from_utf8(rustc).unwrap();
        let rustc = rustc.trim();
        println!("rustc: {}", rustc);

        // Define paths for output artifacts.
        let elf_dir = out_dir.join("elf");
        let elf_path = elf_dir.join("riscv32im-succinct-zkvm-elf");

        // Build the binary using the succinct toolchain.
        let build_target = "riscv32im-succinct-zkvm-elf";
        let rust_flags = [
            "-C",
            "passes=loweratomic",
            "-C",
            "link-arg=-Ttext=0x00200800",
            "-C",
            "panic=abort",
        ];
        let mut cmd = Command::new("cargo");
        for (key, _val) in env::vars().filter(|x| x.0.starts_with("CARGO")) {
            cmd.env_remove(key);
        }
        cmd.env_remove("RUSTUP_TOOLCHAIN");
        cmd.env("RUSTUP_TOOLCHAIN", "succinct")
            .env("CARGO_ENCODED_RUSTFLAGS", rust_flags.join("\x1f"))
            .env("RUSTC", rustc)
            .env("SUCCINCT_BUILD_IGNORE", "1")
            .args([
                "build",
                "--release",
                "--target",
                build_target,
                "--locked",
                "-vvv",
            ])
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .output()
            .unwrap();

        let target_elf_path = out_dir
            .join("target")
            .join("riscv32im-succinct-zkvm-elf")
            .join("release")
            .join(pkg_name);
        std::fs::copy(&target_elf_path, elf_path).unwrap();
        let mut target_elf = File::open(&target_elf_path).unwrap();
        let mut target_elf_bytes = Vec::new();
        target_elf.read_to_end(&mut target_elf_bytes).unwrap();
    }
}
