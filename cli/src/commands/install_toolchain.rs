use anyhow::Result;
use clap::Parser;
use dirs::home_dir;
use flate2::read::GzDecoder;
use std::{fs::File as SyncFile, process::Command};
use tar::Archive;

use crate::{download_file, CommandExecutor, RUSTUP_TOOLCHAIN_NAME};

#[derive(Parser)]
#[command(
    name = "install-toolchain",
    about = "Install the cargo-prove toolchain."
)]
pub struct InstallToolchainCmd {}

impl InstallToolchainCmd {
    pub fn run(&self) -> Result<()> {
        // Setup variables.
        let root_dir = home_dir().unwrap();
        let target = get_target();
        let toolchain_name = format!("rust-toolchain-{}.tar.gz", target);
        let toolchain_archive_path = root_dir.join(toolchain_name);
        let toolchain_dir = root_dir.join(target);

        // Download the toolchain.
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(download_file("", toolchain_archive_path.to_str().unwrap()))
            .unwrap();

        // Unpack the toolchain.
        let tar_gz = SyncFile::open(&toolchain_archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(&toolchain_dir)?;

        // Remove the existing toolchain from rustup, if it exists.
        match Command::new("rustup")
            .args(["toolchain", "remove", RUSTUP_TOOLCHAIN_NAME])
            .run()
        {
            Ok(_) => println!("Succesfully removed existing toolchain."),
            Err(_) => println!("No existing toolchain to remove."),
        }

        // Link the toolchain to rustup.
        Command::new("rustup")
            .args(["toolchain", "link", RUSTUP_TOOLCHAIN_NAME])
            .arg(toolchain_dir)
            .run()?;
        println!("Succesfully linked toolchain to rustup.");

        Ok(())
    }
}

#[allow(unreachable_code)]
fn get_target() -> &'static str {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    return "x86_64-unknown-linux-gnu";

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    return "x86_64-apple-darwin";

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    return "aarch64-apple-darwin";

    panic!("Unsupported architecture. Please build the toolchain from source.")
}
