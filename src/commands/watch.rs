use windows_service::service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{PathBuf};
use std::time::Duration;
use std::env;
use laracli::utils::elevate;
use crate::helpers;




/// Main watch command
pub fn watch_directory(watch_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let watch_dir = PathBuf::from(watch_path);
    println!("Setting up watch for directory: {:?}", watch_dir);

    // Check if directory exists
    if !watch_dir.exists() {
        return Err(format!("Directory does not exist: {:?}", watch_dir).into());
    }

    if !elevate::is_elevated() {
        println!("🔒 Elevation required to modify hosts file. Requesting UAC permission...");
        elevate::run_as_admin()?;
        return Ok(()); // new process will be elevated
    }

    ensure_service_installed()?;

    // Step 1: Save to config.json with normalized path
    
    match helpers::config::add_to_watched_paths(&watch_dir.to_str().unwrap()) {
        Ok(_) => {
            println!("✅ Added directory to watch: {}", watch_dir.display());
        }
        Err(e) => {
            println!("Directory already in config: {}", &e);
        },
    }

    restart_service().expect("Failed to restart service");

    // Step 2: Add existing folders to hosts
    println!("Scanning existing directories...");
    for entry in fs::read_dir(&watch_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                add_host_entry(name)?;
            }
        }
    }

    println!("✅ Directory watcher configuration updated!");
    println!("The service will now monitor: {}", &watch_dir.display());
    println!("New Laravel projects will automatically get .test domain entries.");
    
    Ok(())
}


/// Adds a host entry if not exists
fn add_host_entry(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entry = format!("127.0.0.1\t{}.test #added by laracli", project_name);
    let hosts_path = r"C:\Windows\System32\drivers\etc\hosts";

    // Check if entry already exists
    let file = fs::File::open(hosts_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.contains(&format!("{}.test", project_name)) {
            println!("Host entry for {}.test already exists.", project_name);
            return Ok(());
        }
    }

    // Add new entry
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(hosts_path)?;

    writeln!(file, "{}", entry)?;
    println!("✅ Added host entry for {}.test", project_name);
    Ok(())
}

/// List all watched directories from config
pub fn list_watched_directories() -> Result<(), Box<dyn std::error::Error>> {
    let config = helpers::config::load_config();
    
    if config.watched_paths.is_empty() {
        println!("No directories are currently being watched.");
    } else {
        println!("Currently watched directories:");
        for path in &config.watched_paths {
            println!("  📁 {}", path);
        }
    }
    
    Ok(())
}

/// Remove a directory from watching
pub fn unwatch_directory(watch_path: &str) -> Result<(), Box<dyn std::error::Error>> {    
    match helpers::config::remove_from_watched_paths(watch_path) {
        Ok(_) => {
            println!("Removed directory from watching: {}", watch_path);
            restart_service()?;
        }
        Err(e) => {
            println!("Directory not found in config: {}", &e);
        },
    };
    Ok(())
}

fn ensure_service_installed() -> Result<(), Box<dyn std::error::Error>> {
    let service_name = "laracli";
    let display_name = "Laracli Directory Watcher";
    let service_path = env::current_exe()?.with_file_name("laracli-service.exe");

    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE | ServiceManagerAccess::CONNECT)?;

    // Check if service already exists
    match manager.open_service(service_name, ServiceAccess::QUERY_STATUS) {
        Ok(_) => {
            println!("✅ Service `{}` already installed.", service_name);
        }
        Err(_) => {
            println!("🔧 Installing `{}` service...", service_name);
            let service_info = ServiceInfo {
                name: OsString::from(service_name),
                display_name: OsString::from(display_name),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::AutoStart,
                error_control: ServiceErrorControl::Normal,
                executable_path: service_path.clone(),
                launch_arguments: vec![],
                dependencies: vec![],
                account_name: None, // LocalSystem
                account_password: None,
            };

            let service = manager.create_service(&service_info, ServiceAccess::START)?;
            service.start::<OsString>(&[])?;
            println!("✅ Service `{}` installed and started.", service_name);
        }
    }

    Ok(())
}

pub fn restart_service() -> Result<(), Box<dyn std::error::Error>> {
    if !elevate::is_elevated() {
        println!("🔒 Elevation required to modify hosts file. Requesting UAC permission...");
        elevate::run_as_admin()?;
        return Ok(()); 
    }

    let service_name = "laracli";
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    
    if let Ok(service) = manager.open_service(service_name, ServiceAccess::START | ServiceAccess::STOP | ServiceAccess::QUERY_STATUS) {
        println!("🔄 Restarting service to apply configuration changes...");
        
        // Stop the service
        if let Err(e) = service.stop() {
            // Service might not be running, that's ok
            println!("Note: Error stopping service (might not be running): {}", e);
        }
        
        // Wait a moment for service to stop
        std::thread::sleep(Duration::from_secs(2));
        
        // Start the service
        service.start::<OsString>(&[])?;
        println!("✅ Service restarted successfully.");
    } else {
        println!("⚠️  Could not restart service. You may need to restart it manually.");
    }
    
    Ok(())
}