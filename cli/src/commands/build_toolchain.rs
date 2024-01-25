use std::{path::PathBuf, process::Command};

use anyhow::Result;
use clap::Parser;

use crate::{CommandExecutor, RUSTUP_TOOLCHAIN_NAME};

#[derive(Parser)]
#[command(name = "build-toolchain", about = "Build the cargo-prove toolchain.")]
pub struct BuildToolchainCmd {}

impl BuildToolchainCmd {
    pub fn run(&self) -> Result<()> {
        // Get enviroment variables.
        let github_access_token = std::env::var("GITHUB_ACCESS_TOKEN");
        let build_dir = std::env::var("SUCCINCT_BUILD_DIR");

        // Clone our rust fork.
        let rust_dir = match build_dir {
            Ok(build_dir) => {
                println!("Detected SUCCINCT_BUILD_DIR, skipping cloning rust.");
                PathBuf::from(build_dir).join("rust")
            }
            Err(_) => {
                println!("No SUCCINCT_BUILD_DIR detected, cloning rust.");
                let repo_url = match github_access_token {
                    Ok(github_access_token) => {
                        println!("Detected GITHUB_ACCESS_TOKEN, using it to clone rust.");
                        format!(
                            "https://{}@github.com/succinctlabs/rust",
                            github_access_token
                        )
                    }
                    Err(_) => {
                        println!("No GITHUB_ACCESS_TOKEN detected. If you get throttled by Github, set it to bypass the rate limit.");
                        "https://github.com/succinctlabs/rust".to_string()
                    }
                };
                Command::new("git").args(["clone", &repo_url]).run()?;

                // Checkout the correct branch.
                Command::new("git")
                    .args(["checkout", "riscv32im-succinct-zkvm"])
                    .current_dir("rust")
                    .run()?;
                Command::new("git")
                    .args(["reset", "--hard"])
                    .current_dir("rust")
                    .run()?;
                Command::new("git")
                    .args(["submodule", "update", "--init", "--recursive", "--progress"])
                    .current_dir("rust")
                    .run()?;
                PathBuf::from("rust")
            }
        };

        // Install our config.toml.
        let config_toml = include_str!("config.toml");
        std::fs::write(rust_dir.join("config.toml"), config_toml)?;

        // Build the toolchain (stage 1).
        Command::new("python3")
            .env(
                "CARGO_TARGET_RISCV32IM_SUCCINCT_ZKVM_ELF_RUSTFLAGS",
                "-Cpasses=loweratomic",
            )
            .args(["x.py", "build"])
            .current_dir(&rust_dir)
            .run()?;

        // Build the toolchain (stage 2).
        Command::new("python3")
            .env(
                "CARGO_TARGET_RISCV32IM_SUCCINCT_ZKVM_ELF_RUSTFLAGS",
                "-Cpasses=loweratomic",
            )
            .args(["x.py", "build", "--stage", "2"])
            .current_dir(&rust_dir)
            .run()?;

        // Remove the existing toolchain from rustup, if it exists.
        match Command::new("rustup")
            .args(["toolchain", "remove", RUSTUP_TOOLCHAIN_NAME])
            .run()
        {
            Ok(_) => println!("Succesfully removed existing toolchain."),
            Err(_) => println!("No existing toolchain to remove."),
        }

        // Find the toolchain directory.
        let mut toolchain_dir = None;
        for wentry in std::fs::read_dir(rust_dir.join("build"))? {
            let entry = wentry?;
            let toolchain_dir_candidate = entry.path().join("stage2");
            if toolchain_dir_candidate.is_dir() {
                toolchain_dir = Some(toolchain_dir_candidate);
                break;
            }
        }
        let toolchain_dir = toolchain_dir.unwrap();
        println!(
            "Found built toolchain directory at {}",
            toolchain_dir.as_path().to_str().unwrap()
        );

        // Link the toolchain to rustup.
        Command::new("rustup")
            .args(["toolchain", "link", RUSTUP_TOOLCHAIN_NAME])
            .arg(toolchain_dir)
            .run()?;
        println!("Succesfully linked the toolchain to rustup.");

        Ok(())
    }
}
