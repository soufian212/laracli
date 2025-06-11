use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use laracli::helpers::{self, config, nginx};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::{fs, time::Duration};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
};
use std::fs::File;
use winreg::RegKey;
use winreg::enums::*;
use zip::ZipArchive;
use std::fs::OpenOptions;


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
        let install_output = Command::new("sc").args(&sc_args).output()?;

        if !install_output.status.success() {
            let stderr = String::from_utf8_lossy(&install_output.stderr);
            println!("Failed to install {} service: {}", service_name, stderr);
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
        let start_output = Command::new("sc").args(&["start", service_name]).output()?;

        if !start_output.status.success() {
            println!(
                "Failed to start {} service: {}",
                service_name,
                String::from_utf8_lossy(&start_output.stderr)
            );
            return Err(format!("Failed to start {} service", service_name).into());
        }

        println!(
            "Successfully installed and started {} service",
            service_name
        );
    }

    // Create default config if it doesn't exist
    let config_path = Path::new(r"C:\laracli\config.json");
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

pub async fn download_with_progress_async(
    url: &str,
    out_path: &str,
    label: &str,
    max_retries: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a more robust HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .pool_max_idle_per_host(1)
        .user_agent("laracli/1.0")
        .build()?;

    for attempt in 1..=max_retries {
        println!(
            "{}",
            format!("Downloading {} (Attempt {}/{})", label, attempt, max_retries).yellow()
        );

        // Check if partial file exists for resume
        let mut resume_from = 0u64;
        if attempt > 1 && std::path::Path::new(out_path).exists() {
            if let Ok(metadata) = std::fs::metadata(out_path) {
                resume_from = metadata.len();
                println!("Resuming download from {} bytes", resume_from);
            }
        }

        // Build request with range header for resume
        let mut request = client.get(url);
        if resume_from > 0 {
            request = request.header("Range", format!("bytes={}-", resume_from));
        }

        let res = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                println!("❌ Request failed: {}", e);
                if attempt == max_retries {
                    return Err(format!("Failed to GET from '{}' after {} attempts: {}", url, max_retries, e).into());
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        if !res.status().is_success() && res.status().as_u16() != 206 {
            println!("❌ HTTP error: {}", res.status());
            if attempt == max_retries {
                return Err(format!("HTTP error {}: {}", res.status(), res.status().canonical_reason().unwrap_or("Unknown")).into());
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
            continue;
        }

        let total_size = if res.status().as_u16() == 206 {
            // Partial content - parse Content-Range header
            if let Some(content_range) = res.headers().get("content-range") {
                if let Ok(range_str) = content_range.to_str() {
                    if let Some(total_str) = range_str.split('/').nth(1) {
                        total_str.parse::<u64>().unwrap_or(0)
                    } else {
                        res.content_length().unwrap_or(0) + resume_from
                    }
                } else {
                    res.content_length().unwrap_or(0) + resume_from
                }
            } else {
                res.content_length().unwrap_or(0) + resume_from
            }
        } else {
            res.content_length().unwrap_or(0)
        };

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap(),
        );

        // Open file for writing (append if resuming)
        let mut file = if resume_from > 0 {
            match OpenOptions::new().create(true).append(true).open(out_path) {
                Ok(f) => f,
                Err(e) => {
                    println!("❌ Failed to open file for append '{}': {}", out_path, e);
                    if attempt == max_retries {
                        return Err(format!("Failed to open file '{}': {}", out_path, e).into());
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        } else {
            match File::create(out_path) {
                Ok(f) => f,
                Err(e) => {
                    println!("❌ Failed to create file '{}': {}", out_path, e);
                    if attempt == max_retries {
                        return Err(format!("Failed to create file '{}': {}", out_path, e).into());
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        };

        let mut downloaded = resume_from;
        pb.set_position(downloaded);
        let mut stream = res.bytes_stream();
        let mut download_success = true;
        let mut last_progress = std::time::Instant::now();

        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => {
                    if let Err(e) = file.write_all(&chunk) {
                        println!("❌ Error writing to file: {}", e);
                        download_success = false;
                        break;
                    }
                    downloaded += chunk.len() as u64;
                    pb.set_position(downloaded);

                    // Flush every 10MB or every 5 seconds
                    if downloaded % (10 * 1024 * 1024) == 0 || last_progress.elapsed() > Duration::from_secs(5) {
                        if let Err(e) = file.flush() {
                            println!("❌ Error flushing file: {}", e);
                            download_success = false;
                            break;
                        }
                        last_progress = std::time::Instant::now();
                    }
                }
                Err(e) => {
                    println!("❌ Error while downloading chunk: {}", e);
                    download_success = false;
                    break;
                }
            }
        }

        // Final flush
        if let Err(e) = file.flush() {
            println!("❌ Error during final flush: {}", e);
            download_success = false;
        }

        if download_success && (total_size == 0 || downloaded >= total_size) {
            pb.finish_with_message(format!("✅ {} downloaded successfully", label));
            println!("✅ {} downloaded ({} bytes)", label, downloaded);
            return Ok(());
        } else {
            pb.finish_with_message(format!("❌ {} download failed", label));
            println!("❌ Download failed. Expected: {} bytes, Downloaded: {} bytes", total_size, downloaded);
            
            if attempt == max_retries {
                // Clean up partial file only on final failure
                if let Err(e) = std::fs::remove_file(out_path) {
                    println!("Warning: Failed to remove partial file: {}", e);
                }
                return Err(format!("Download incomplete after {} attempts. Last attempt downloaded {} of {} bytes", 
                    max_retries, downloaded, total_size).into());
            }
            
            println!("Retrying in 10 seconds...");
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }

    Err("Download failed after all retries".into())
}


pub async fn setup_tools() -> Result<(), Box<dyn std::error::Error>> {
    let tools_dir = Path::new("tools");

    // Create tools directory if it doesn't exist
    fs::create_dir_all(&tools_dir)?;

    // --- Download and extract Nginx ---
    let nginx_url = "http://nginx.org/download/nginx-1.23.3.zip";
    let nginx_zip = "nginx-1.23.3.zip";
    download_with_progress_async(nginx_url, nginx_zip, "Nginx", 3).await?;

    println!("{}", "Extracting Nginx".yellow());
    let nginx_file = fs::File::open(nginx_zip)?;
    let mut nginx_archive = ZipArchive::new(nginx_file)?;
    nginx_archive.extract(&tools_dir)?;
    fs::remove_file(nginx_zip)?;
    println!("{}", "✅ Nginx extracted successfully".green());

    
    // --- Download and extract Php ---
    let php_url = "https://files04.tchspt.com/down/php-8.4.8-Win32-vs17-x64.zip";
    let php_zip = "php-8.4.8-nts-Win32-vs17-x64.zip";
    println!("{}", "Downloading PHP (approx. 32 MB, may take a few minutes)...".yellow());
    download_with_progress_async(php_url, php_zip, "PHP", 3).await?;
    println!("{}", "Extracting PHP".yellow());
    let php_file = fs::File::open(php_zip)?;
    let mut php_archive = ZipArchive::new(php_file)?;
    php_archive.extract(&tools_dir.join(&php_zip.replace(".zip", "")))?;
    fs::remove_file(php_zip)?;
    println!("{}", "✅ PHP extracted successfully".green());

    //--- Download and extract MySQL ---
    let mysql_url = "https://cdn.mysql.com//Downloads/MySQL-8.4/mysql-8.4.5-winx64.zip";
    let mysql_zip = "mysql-8.4.5-winx64_2.zip";
    println!("{}", "Downloading MySQL (approx. 247 MB, may take a few minutes)...".yellow());
    download_with_progress_async(mysql_url, mysql_zip, "MySQL", 3).await?;

    println!("{}", "Extracting MySQL".yellow());
    let mysql_file = fs::File::open(mysql_zip)?;
    let mut mysql_archive = ZipArchive::new(mysql_file)?;
    mysql_archive.extract(&tools_dir)?;
    fs::remove_file(mysql_zip)?;
    println!("{}", "✅ MySQL extracted successfully".green());

    // --- Create Global Nginx Config ---
    println!("{}", "Creating config files".yellow());
    match nginx::create_global_nginx_config() {
        Ok(_) => println!("{}", "✅ Global Nginx config created".green()),
        Err(e) => println!("{}", format!("❌ Error creating global Nginx config: {}", e).red()),
    }

    // --- Create Default Config File ---
    config::create_config_file();
    println!("{}", "✅ Config file created".green());

    // --- Create my.ini for MySQL ---
    println!("{}", "Creating my.ini".yellow());
    helpers::mysql::create_my_ini_file();
    println!("{}", "✅ my.ini created".green());

    // --- Initialize MySQL Data Directory ---
    println!("{}", "Creating MySQL data directory".yellow());
    let mysql_path = helpers::path::get_mysql_path().unwrap();
    let mysqld_path = Path::new(&mysql_path).join("bin").join("mysqld.exe");

    let output = Command::new(&mysqld_path)
        .arg("--initialize-insecure")
        .arg("--basedir")
        .arg(&mysql_path)
        .arg("--datadir")
        .arg(Path::new(&mysql_path).join("data"))
        .output();

    let mysql_data_dir = Path::new(&mysql_path).join("data");

    if mysql_data_dir.exists() {
        println!("{}", "✅ MySQL data directory created".green());
    } else {
        println!("{}", "❌ Error creating MySQL data directory".red());
    }

    Ok(())
}


pub fn add_exe_to_path() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Adding current executable to PATH".yellow());
    let new_path = helpers::path::get_current_exe_dir().unwrap();

    // Open the user environment variables
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .expect("Failed to open Environment key");

    // Read the existing PATH value
    let current_path: String = env.get_value("Path").unwrap_or_default();

    // Only append if it's not already there
    if !current_path
        .to_lowercase()
        .contains(&new_path.to_lowercase())
    {
        let updated_path = format!("{};{}", current_path, new_path);
        env.set_value("Path", &updated_path)?;
        println!("{}", "✅ Current executable added to PATH".green());
    } else {
        println!("{}", "ℹ️ Path already contains the directory.".blue());
    }

    notify_environment_change();

    println!(
        "{}",
        r#"✅ You are ready to star type "laracli run-dev"  "#.green()
    );

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
