use colored::Colorize;
use std::process::Command;
use crate::helpers::path;
use std::os::windows::process::CommandExt;
use std::path::Path;



#[cfg(target_os = "windows")]
pub fn start() -> Result<(), Box<dyn std::error::Error>> {
    //check if nginx is already running by checking pid file at logs/

    let pid_file_path = format!("{}/logs/nginx.pid", path::get_nginx_path()?);
    let pid_file = Path::new(&pid_file_path);
    if pid_file.exists() {
        println!("{}", "✔ Nginx service is already running.".green());
        return Ok(()); // Nginx is already running
    }




    let nginx_path = path::get_nginx_path()?;
    let nginx_exe = format!("{}/nginx.exe", &nginx_path);

    Command::new(&nginx_exe)
        .current_dir(&nginx_path) // Sets working dir so relative paths like "conf/nginx.conf" work
        .arg("-p")
        .arg(".")                 // Use current dir (set above) as Nginx prefix
        .arg("-c")
        .arg("conf/nginx.conf")
        .creation_flags(0x08000000) // DETACHED_PROCESS on Windows
        .spawn()?; // Don't wait on it

    println!("{}", "✔ Nginx service started successfully.".green());
    Ok(())
}



pub fn stop() -> Result<(), Box<dyn std::error::Error>> {
    let pid_file_path = format!("{}/logs/nginx.pid", path::get_nginx_path()?);
    let pid_file = Path::new(&pid_file_path);
    if !pid_file.exists() {
        println!("{}", "❌ Nginx service is not running.".green());
        return Ok(()); // Nginx is already running
    }

    println!("Stopping Nginx service...");

    let nginx_path = path::get_nginx_path()?;
    let nginx_exe = format!("{}/nginx.exe", &nginx_path);
    let output = Command::new(&nginx_exe)
        .current_dir(&nginx_path)   
        .arg("-p")
        .arg(".")
        .arg("-c")
        .arg("conf/nginx.conf")
        .arg("-s")
        .arg("stop")
        .spawn()?
        .wait()?;

    if output.success() {
        println!("{}", "✔ Nginx service stopped successfully.".green());
    } else {
        eprintln!("Failed to stop Nginx service.");
        return Err("Failed to stop Nginx service".into());
    }

    Ok(())
}

pub fn reload() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Reloading Nginx service...".yellow());
    let nginx_path = path::get_nginx_path()?;
    let nginx_exe = format!("{}/nginx.exe", &nginx_path);
    let output = Command::new(&nginx_exe)
        .current_dir(&nginx_path)   
        .arg("-p")
        .arg(".")
        .arg("-c")
        .arg("conf/nginx.conf")
        .arg("-s")
        .arg("reload")
        .spawn()?
        .wait()?;
    if output.success() {
        println!("{}", "✔ Nginx service reloaded successfully.".green());
    } else {
        eprintln!("Failed to reload Nginx service.");
        return Err("Failed to reload Nginx service".into());
    }

    Ok(())
}