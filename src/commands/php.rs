use std::{net::TcpStream, process::Command};

use colored::Colorize;

pub fn start_php_cgi() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Starting PHP service...".yellow());

    // Check if port 9000 is already used
    if TcpStream::connect("127.0.0.1:9000").is_ok() {
        println!("{}", "ℹ PHP is already running on port 9000.".blue());
        return Ok(());
    }

    let php_path = crate::helpers::path::get_php_path()?;

    let outout = Command::new(&php_path.join("php-cgi.exe"))
        .arg("-b")
        .arg("127.0.0.1:9000")
        .spawn();

    match outout {
        Ok(output) => {
            println!("{}", "✔ PHP service started successfully.".green());
        }
        Err(e) => {
            println!("{}", "❌ Failed to start PHP service.".red());
            println!("{}", e.to_string());
        }
    }

    Ok(())
}
