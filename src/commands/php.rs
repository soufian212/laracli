use colored::Colorize;
use std::{
    fs::File,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    process::Command,
};

pub fn start_php_cgi() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Starting PHP service...".yellow());

    // Check if port 9000 is already used
    if TcpStream::connect("127.0.0.1:9000").is_ok() {
        println!("{}", "ℹ PHP is already running on port 9000.".blue());
        return Ok(());
    }

    let php_path = crate::helpers::path::get_php_path()?;

    let output = Command::new(&php_path.join("php-cgi.exe"))
        .arg("-b")
        .arg("127.0.0.1:9000")
        .spawn();

    match output {
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

pub fn enable_php_extension(extension: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        format!("Enabling PHP extension: {}", extension).yellow()
    );

    let php_path = crate::helpers::path::get_php_path()?;
    let php_ini_path = php_path.join("php.ini");
    let ext_dir = php_path.join("ext");

    if !php_ini_path.exists() {
        return Err("php.ini file not found".into());
    }

    // Check if the extension DLL exists
    let dll_name = format!("php_{}.dll", extension);
    let dll_path = ext_dir.join(&dll_name);
    if !dll_path.exists() {
        return Err(format!(
            "DLL file {} not found in {}. Please ensure the PHP extension is installed.",
            dll_name,
            ext_dir.display()
        )
        .into());
    }

    // Read php.ini content
    let mut content = String::new();
    File::open(&php_ini_path)?.read_to_string(&mut content)?;

    let extension_line = format!("extension={}", extension);
    let commented_extension_line = format!(";extension={}", extension);
    let ext_dir_line = "extension_dir = \"ext\"";
    let commented_ext_dir_line = ";extension_dir = \"ext\"";

    let mut new_content = String::new();
    let mut changes_made = false;
    let mut extension_found = false;

    // Check and uncomment or set extension_dir
    if content.contains(commented_ext_dir_line) {
        content = content.replace(commented_ext_dir_line, ext_dir_line);
        println!("{}", "ℹ Uncommented extension_dir in php.ini.".blue());
        changes_made = true;
    } else if !content.contains(ext_dir_line) {
        content = format!("{}\n{}", content.trim_end(), ext_dir_line);
        println!("{}", "ℹ Added extension_dir to php.ini.".blue());
        changes_made = true;
    }

    // Process each line to enable the extension
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed == extension_line {
            new_content.push_str(line);
            new_content.push('\n');
            extension_found = true;
        } else if trimmed == commented_extension_line {
            new_content.push_str(&extension_line);
            new_content.push('\n');
            extension_found = true;
            changes_made = true;
            println!("{}", "ℹ Uncommented extension in php.ini.".blue());
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    // If not found, add it
    if !extension_found {
        new_content = format!("{}\n{}", new_content.trim_end(), extension_line);
        println!("{}", "ℹ Added extension to php.ini.".blue());
        changes_made = true;
    }

    // Write back to php.ini if changes were made
    if changes_made {
        File::create(&php_ini_path)?.write_all(new_content.as_bytes())?;
    }

    println!(
        "{}",
        format!("✔ PHP extension {} enabled successfully.", extension).green()
    );

    // Restart PHP service to apply changes
    restart_php_service()?;

    Ok(())
}

pub fn disable_php_extension(extension: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        format!("Disabling PHP extension: {}", extension).yellow()
    );

    let php_path = crate::helpers::path::get_php_path()?;
    let php_ini_path = php_path.join("php.ini");

    if !php_ini_path.exists() {
        return Err("php.ini file not found".into());
    }

    // Read php.ini content
    let mut content = String::new();
    File::open(&php_ini_path)?.read_to_string(&mut content)?;

    let extension_line = format!("extension={}", extension);
    let commented_line = format!(";{}", extension_line);

    let mut new_content = String::new();
    let mut found = false;
    for line in content.lines() {
        if line.trim_start().starts_with(&extension_line) {
            new_content.push_str(&commented_line);
            new_content.push('\n');
            found = true;
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }
    if !found {
        println!("{}", "ℹ Extension already disabled or not present.".blue());
    }

    // Write back to php.ini
    File::create(&php_ini_path)?.write_all(new_content.as_bytes())?;
    println!(
        "{}",
        format!("✔ PHP extension {} disabled successfully.", extension).green()
    );

    // Restart PHP service to apply changes
    restart_php_service()?;

    Ok(())
}

pub fn restart_php_service() -> Result<(), Box<dyn std::error::Error>> {
    // Stop PHP service
    let output = Command::new("taskkill")
        .args(&["/F", "/IM", "php-cgi.exe"])
        .output();

    // Wait for PHP service to stop
    std::thread::sleep(std::time::Duration::from_secs(1));
    match output {
        Ok(_) => println!("✔ PHP service stopped successfully."),
        Err(e) => println!("Error stopping PHP service: {}", e),
    }

    // Start PHP service
    start_php_cgi()?;

    Ok(())
}


pub fn stop_php_cgi() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Stopping PHP service...".yellow());
    Command::new("taskkill")
        .args(&["/F", "/IM", "php-cgi.exe"])
        .output()?;
    println!("{}", "✔ PHP service stopped successfully.".green());
    Ok(())
}