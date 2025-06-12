use std::path::Path;
use crate::helpers;
use windows_service::service::{ServiceAccess, ServiceState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use std::time::Duration;

pub fn link(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let name = path.file_name().unwrap().to_str().unwrap();

    println!("Linking {}, path: {}", name, path.display());
    let config_path = helpers::config::get_config_path();
    println!("Using config: {}", config_path.display());

    // Add to linked_paths in config
    helpers::config::add_to_linked_paths(path.to_str().unwrap());
    println!("✅ Updated config with linked path: {}", path.display());

    // Create nginx config immediately
    match helpers::nginx::create_nginx_config(path.to_str().unwrap(), None) {
        Ok(()) => println!("✅ Nginx config created"),
        Err(e) => println!("❌ Error creating Nginx config: {}", e),
    }

    // Restart laracli_config service to process config changes
    restart_config_service()?;

    println!("✅ Project linked! The service will now monitor this directory.");
    println!("   - Host entry: {}.test -> 127.0.0.1 (will be added by service)", name);
    println!("   - Nginx config: Created and will be reloaded by service");
    println!("   - Service monitoring: If you move/delete this directory, entries will be automatically cleaned up");

    Ok(())
}

pub fn unlink(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let name = path.file_name().unwrap().to_str().unwrap();

    println!("Unlinking {}, path: {}", name, path.display());
    let config_path = helpers::config::get_config_path();
    println!("Using config: {}", config_path.display());

    // Remove from linked_paths in config
    helpers::config::remove_from_linked_paths(path.to_str().unwrap()).expect("Failed to remove path from config");
    println!("✅ Removed linked path from config: {}", path.display());

    // Remove nginx config immediately
    match helpers::nginx::delete_nginx_config(path.to_str().unwrap()) {
        Ok(()) => println!("✅ Nginx config deleted"),
        Err(e) => println!("❌ Error deleting Nginx config: {}", e),
    }

    // Restart laracli_config service to process config changes
    crate::commands::nginx::reload()?;

    println!("✅ Project unlinked! Service will no longer monitor this directory.");

    Ok(())
}

fn restart_config_service() -> Result<(), Box<dyn std::error::Error>> {
    let service_name = "laracli_config";
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    
    if let Ok(service) = manager.open_service(service_name, ServiceAccess::START | ServiceAccess::STOP | ServiceAccess::QUERY_STATUS) {
        println!("🔄 Restarting laracli_config service to apply configuration changes...");
        
        // Stop the service
        if let Err(e) = service.stop() {
            println!("Note: Error stopping service (might not be running): {}", e);
        }
        
        // Wait for service to stop
        for _ in 0..10 {
            let status = service.query_status()?;
            if status.current_state == ServiceState::Stopped {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        
        // Start the service
        service.start::<&str>(&[])?;
        println!("✅ laracli_config service restarted successfully.");
    } else {
        println!("⚠️ Could not restart laracli_config service. Changes may not take effect until service restarts.");
    }
    
    Ok(())
}