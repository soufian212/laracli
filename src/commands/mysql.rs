use std::io;
use std::path::Path;
use std::process::Command;
use std::process::Child;

// Add this import for Windows-specific process extensions
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use chrono::format;
use colored::Colorize;
use laracli::helpers;

pub fn start() -> Result<Child, std::io::Error>  {
    println!("{}", "Starting MySQL service...".yellow());

    //check if mysql already running

    let mysql_path = helpers::path::get_mysql_path().unwrap();
    let mysql_bin = Path::new(&mysql_path).join("bin/mysqld.exe");
    let ini_file = Path::new(&mysql_path).join("my.ini");

    if !mysql_bin.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("MySQL binary not found at {}", mysql_bin.to_str().unwrap()),
        ));
    } else if !ini_file.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("MySQL ini file not found at {}", ini_file.to_str().unwrap()),
        ));
    }

    let child = Command::new(mysql_bin)
        .arg(format!("--defaults-file={}", ini_file.to_str().unwrap()))
        .arg("--console")
        .creation_flags(0x08000000) // DETACHED_PROCESS on Windows
        .spawn()?;


    println!("{}", "✔ MySQL service started successfully.".green());

    Ok(child)
}

pub fn stop() -> Result<(), io::Error> {
    println!("{}", "Stopping MySQL service...".yellow());
    let output = Command::new("taskkill")
        .args(&["/F", "/IM", "mysqld.exe"])
        .output()?;

    if output.status.success() {
        println!("{}", "✔ MySQL service stopped successfully.".green());
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("{}: {}", "Failed to stop MySQL service".red(), stderr),
        ))
    }
}