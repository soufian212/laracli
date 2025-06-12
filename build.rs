use std::env;
use std::fs::{File};
use std::path::Path;
use std::process::Command;
use chrono::Local;
use zip::write::{FileOptions, ZipWriter};

fn main() {
    // Set build date
    let build_date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // Try to get git hash
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .map(|output| String::from_utf8(output.stdout).unwrap_or_default().trim().to_string())
        .unwrap_or_else(|e| {
            println!("cargo:warning=Failed to get git hash: {}", e);
            "unknown".to_string()
        });
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Rerun if git changes
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Get the version from Cargo.toml
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|e| {
        println!("cargo:warning=Failed to get CARGO_PKG_VERSION: {}. Using default 0.1.3", e);
        "0.1.3".to_string()
    });

    // Determine the target directory based on build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let bin_dir = Path::new("target").join(&profile);
    let zip_name = format!("laracli-{}.zip", version);
    let zip_path = Path::new("target").join(&zip_name);

    println!("cargo:warning=Attempting to create zip file: {}", zip_path.display());

    // Create the zip file
    let zip_file = match File::create(&zip_path) {
        Ok(file) => file,
        Err(e) => {
            println!("cargo:warning=Failed to create zip file {}: {}", zip_path.display(), e);
            return;
        }
    };

    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // List of binaries to include
    let binaries = vec!["laracli.exe", "laracli-service.exe", "laracli-service-config.exe"];

    // Add each binary to the zip
    for binary in binaries {
        let binary_path = bin_dir.join(binary);
        println!("cargo:warning=Checking for binary: {}", binary_path.display());
        if binary_path.exists() {
            match File::open(&binary_path) {
                Ok(mut file) => {
                    if let Err(e) = zip.start_file(binary, options) {
                        println!("cargo:warning=Failed to start zip entry for {}: {}", binary, e);
                        continue;
                    }
                    if let Err(e) = std::io::copy(&mut file, &mut zip) {
                        println!("cargo:warning=Failed to add {} to zip: {}", binary, e);
                        continue;
                    }
                    println!("cargo:warning=Added {} to zip", binary);
                }
                Err(e) => {
                    println!("cargo:warning=Failed to open binary {}: {}", binary_path.display(), e);
                }
            }
        } else {
            println!("cargo:warning=Binary {} not found in {}", binary, bin_dir.display());
        }
    }

    // Finalize the zip file
    if let Err(e) = zip.finish() {
        println!("cargo:warning=Failed to finalize zip file {}: {}", zip_path.display(), e);
        return;
    }

    println!("cargo:warning=Successfully created zip file: {}", zip_path.display());
}