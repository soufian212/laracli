use std::{fs, time::Duration};
use std::path::Path;
use std::process::Command;
use colored::Colorize;
use laracli::helpers::{self, config, nginx};
use reqwest::blocking::Client;
use zip::ZipArchive;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{Read, Write};
use winreg::enums::*;
use winreg::RegKey;
use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::core::PCWSTR;



pub fn setup_services() -> Result<(), Box<dyn std::error::Error>> {
    let services = vec![
        ("laracli", "laracli-service.exe"),
        ("laracli_config", "laracli-service-config.exe"),
    ];

    // Ensure we're running with admin privileges
    if !is_elevated() {
        println!("This command requires administrative privileges. Please run as administrator.");
        return Err("Administrative privileges required".into());
    }

    // Get the directory of the current executable
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Could not determine executable directory")?;
    println!("Using executable directory: {:?}", exe_dir);

    // Verify binaries exist in the executable directory
    for (_, binary_name) in services.iter() {
        let binary_path = exe_dir.join(binary_name);
        if !binary_path.exists() {
            println!("Binary not found: {:?}", binary_path);
            return Err(format!("Binary {} not found in {:?}", binary_name, exe_dir).into());
        }
    }

    // Install and configure services
    for (service_name, binary_name) in services.iter() {
        // Construct the full path for the service binary
        let binary_path = exe_dir.join(binary_name).canonicalize()?;
        let binary_path_str = binary_path.to_string_lossy();
        let formatted_path = format!(r#""{}""#, binary_path_str);

        // Build and log the sc create command
        let sc_args = vec![
            "create",
            service_name,
            "binPath=",
            &formatted_path,
            "start=",
            "auto",
        ];
        println!("Executing sc command: sc {}", sc_args.join(" "));

        // Install service
        let install_output = Command::new("sc")
            .args(&sc_args)
            .output()?;

        if !install_output.status.success() {
            let stderr = String::from_utf8_lossy(&install_output.stderr);
            println!(
                "Failed to install {} service: {}",
                service_name,
                stderr
            );
            return Err(format!("Failed to install {} service: {}", service_name, stderr).into());
        }

        // Set service permissions
        let perm_output = Command::new("sc")
            .args(&[
                "sdset",
                service_name,
                "D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)",
            ])
            .output()?;

        if !perm_output.status.success() {
            println!(
                "Failed to set permissions for {} service: {}",
                service_name,
                String::from_utf8_lossy(&perm_output.stderr)
            );
            return Err(format!("Failed to set permissions for {} service", service_name).into());
        }

        // Grant permissions to hosts file
        let hosts_path = r"C:\Windows\System32\drivers\etc\hosts";
        let icacls_output = Command::new("icacls")
            .args(&[hosts_path, "/grant", "*S-1-5-19:F", "/T"])
            .output()?;

        if !icacls_output.status.success() {
            println!(
                "Failed to set hosts file permissions: {}",
                String::from_utf8_lossy(&icacls_output.stderr)
            );
            return Err("Failed to set hosts file permissions".into());
        }

        // Start the service
        let start_output = Command::new("sc")
            .args(&["start", service_name])
            .output()?;

        if !start_output.status.success() {
            println!(
                "Failed to start {} service: {}",
                service_name,
                String::from_utf8_lossy(&start_output.stderr)
            );
            return Err(format!("Failed to start {} service", service_name).into());
        }

        println!("Successfully installed and started {} service", service_name);
    }

    // Create default config if it doesn't exist
    let config_path = Path::new(r"C:\ProgramData\laracli\config.json");
    if !config_path.exists() {
        fs::create_dir_all(config_path.parent().unwrap())?;
        let default_config = r#"{
            "watched_paths": [],
            "linked_paths": []
        }"#;
        fs::write(config_path, default_config)?;
        println!("Created default config at {:?}", config_path);
    }


    Ok(())
}



pub fn setup_tools() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let tools_dir = Path::new("tools");

    // Create tools directory if it doesn't exist
    fs::create_dir_all(&tools_dir)?;

    // Helper closure for download with progress and size check
    let download_with_progress = |url: &str, out_path: &str, label: &str, max_retries: usize| -> Result<(), Box<dyn std::error::Error>> {
        let mut attempt = 0;
        while attempt < max_retries {
            attempt += 1;
            println!("{}", format!("Downloading {} (Attempt {}/{})", label, attempt, max_retries).yellow());
            let mut response = client.get(url)
                .header("User-Agent", "laracli/1.0")
                .send()?;

            let total_size = response
                .content_length()
                .ok_or(format!("Failed to get content length for {} download", label))?;

            let pb = ProgressBar::new(total_size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));

            let mut temp_out_path = format!("{}.tmp", out_path);
            let mut out = fs::File::create(&temp_out_path)?;
            let mut downloaded: u64 = 0;
            let mut buffer = [0; 8192];
            loop {
                let n = response.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                out.write_all(&buffer[..n])?;
                downloaded += n as u64;
                pb.set_position(downloaded);
            }
            pb.finish_with_message(format!("✅ {} downloaded successfully", label));

            if downloaded != total_size {
                fs::remove_file(&temp_out_path)?;
                if attempt == max_retries {
                    return Err(format!("{} download incomplete after {} attempts: expected {} bytes, got {}", label, max_retries, total_size, downloaded).into());
                }
                std::thread::sleep(Duration::from_secs(2)); // Wait before retry
                continue;
            }

            // Validate ZIP before renaming
            let file = fs::File::open(&temp_out_path)?;
            let mut archive = ZipArchive::new(file);
            if archive.is_err() {
                fs::remove_file(&temp_out_path)?;
                if attempt == max_retries {
                    return Err(format!("Invalid ZIP archive for {} after {} attempts: {}", label, max_retries, archive.unwrap_err()).into());
                }
                std::thread::sleep(Duration::from_secs(2)); // Wait before retry
                continue;
            }
            fs::rename(&temp_out_path, out_path)?;
            break;
        }
        Ok(())
    };

    // --- Download and extract Nginx ---
    let nginx_url = "http://nginx.org/download/nginx-1.23.3.zip";
    let nginx_zip = "nginx-1.23.3.zip";
    download_with_progress(nginx_url, nginx_zip, "Nginx", 3)?;

    println!("{}", "Extracting Nginx".yellow());
    let nginx_file = fs::File::open(nginx_zip)?;
    let mut nginx_archive = ZipArchive::new(nginx_file)?;
    nginx_archive.extract(&tools_dir.join("nginx-1.23.3"))?;
    fs::remove_file(nginx_zip)?;
    println!("{}", "✅ Nginx extracted successfully".green());

    // --- Download and extract MySQL ---
    let mysql_url = "https://cdn.mysql.com//Downloads/MySQL-8.4/mysql-8.4.5-winx64.zip"; // Verified 247 MB
    let mysql_zip = "mysql-8.4.5-winx64_2.zip";
    println!("{}", "Downloading MySQL (approx. 247 MB, may take a few minutes)...".yellow());
    download_with_progress(mysql_url, mysql_zip, "MySQL", 3)?;

    println!("{}", "Extracting MySQL".yellow());
    let mysql_file = fs::File::open(mysql_zip)?;
    let mut mysql_archive = ZipArchive::new(mysql_file)?;
    mysql_archive.extract(&tools_dir.join("mysql-8.4.5-winx64"))?;
    fs::remove_file(mysql_zip)?;
    println!("{}", "✅ MySQL extracted successfully".green());

    // Create global Nginx config and config file
    println!("{}", "Creating config files".yellow());
    match nginx::create_global_nginx_config() {
        Ok(_) => println!("{}", "✅ Global Nginx config created".green()),
        Err(e) => println!("{}", format!("❌ Error creating global Nginx config: {}", e).red()),
    }
    config::create_config_file();
    println!("{}", "✅ Config file created".green());

    println!("{}", "creating my.ini".yellow());
    helpers::mysql::create_my_ini_file();
    println!("{}", "✅ my.ini created".green());

    println!("{}", "Creating MySQL data directory".yellow());

    let let_mysql_path = helpers::path::get_mysql_path().unwrap();
    let mysqld_path = std::path::Path::new(&let_mysql_path).join("bin").join("mysqld.exe");

    let mysqld_command = Command::new(&mysqld_path)
    .arg("--initialize-insecure")
    .arg("/data")
    .output();

    let mysql_data_dir = std::path::Path::new(&let_mysql_path).join("data");

    if mysql_data_dir.exists() {
        println!("{}", "✅ MySQL data directory created".green());
    }else {
        println!("{}", "❌ Error creating MySQL data directory".red());
    }



    Ok(())
}


pub fn add_exe_to_path() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Adding current executable to PATH".yellow());
    let new_path = helpers::path::get_current_exe_dir().unwrap();

    // Open the user environment variables
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE).expect("Failed to open Environment key");

    // Read the existing PATH value
    let current_path: String = env.get_value("Path").unwrap_or_default();

    // Only append if it's not already there
    if !current_path.to_lowercase().contains(&new_path.to_lowercase()) {
        let updated_path = format!("{};{}", current_path, new_path);
        env.set_value("Path", &updated_path)?;
        println!("{}", "✅ Current executable added to PATH".green());
    } else {
        println!("{}", "ℹ️ Path already contains the directory.".blue());
    }

    notify_environment_change();

    println!("{}", r#"✅ You are ready to star type "laracli run"  "#.green());

    Ok(())


}

fn is_elevated() -> bool {
    Command::new("net")
        .args(&["session"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn notify_environment_change() {
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }
}