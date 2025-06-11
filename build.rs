use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use chrono::Local;
use zip::write::{FileOptions, ZipWriter};

fn main() -> std::io::Result<()> {
    // Set build date
    let build_date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // Try to get git hash
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .map(|output| String::from_utf8(output.stdout).unwrap_or_default().trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    
    // Rerun if git changes
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Get the version from Cargo.toml
    let version = env::var("CARGO_PKG_VERSION").unwrap_or("0.1.3".to_string());
    
    // Define the output zip file name
    let zip_name = format!("laracli-{}.zip", version);
    let out_dir = env::var("OUT_DIR").unwrap_or("target/release".to_string());
    let zip_path = Path::new(&out_dir).parent().unwrap().parent().unwrap().join(&zip_name);
    
    // Create the zip file
    let zip_file = File::create(&zip_path)?;
    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // List of binaries to include
    let binaries = vec!["laracli.exe", "laracli-service.exe"];
    
    // Add each binary to the zip
    for binary in binaries {
        let binary_path = Path::new("target/release").join(binary);
        if binary_path.exists() {
            let mut file = File::open(&binary_path)?;
            zip.start_file(binary, options)?;
            std::io::copy(&mut file, &mut zip)?;
        } else {
            eprintln!("Warning: Binary {} not found in target/release", binary);
        }
    }
    
    zip.finish()?;
    
    println!("cargo:warning=Created zip file: {}", zip_path.display());
    Ok(())
}