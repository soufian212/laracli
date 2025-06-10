use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use chrono::Local;
use notify::event::ModifyKind::{self, Name};
use notify::event::RenameMode;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
};

const SERVICE_NAME: &str = "laracli";

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(_arguments: Vec<std::ffi::OsString>) {
    if let Err(e) = run_service() {
        log(&format!("Service failed: {}", e));
    }
}

fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let status_handle =
        service_control_handler::register(
            SERVICE_NAME,
            move |control_event| match control_event {
                ServiceControl::Stop => {
                    log("Received stop event.");
                    running_clone.store(false, Ordering::SeqCst);
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => {
                    log("Service interrogated.");
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            },
        )?;

    let pid = std::process::id();
    log(&format!("Service started with PID: {}", pid));

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::ZERO,
        process_id: Some(pid),
    })?;

    // Load config fresh each time service starts
    let config = load_config();
    log(&format!(
        "Loaded config with {} paths",
        config.watched_paths.len()
    ));

    let mut handles = Vec::new();

    for dir in &config.watched_paths {
        let path = normalize_path(dir);
        log(&format!("Processing path: {} -> {:?}", dir, path));

        if path.exists() {
            let path_clone = path.clone();
            let running_clone = running.clone();
            let handle = thread::spawn(move || {
                if let Err(e) = watch_directory(&path_clone, running_clone) {
                    log(&format!("Error watching {:?}: {}", path_clone, e));
                }
            });
            handles.push(handle);
        } else {
            log(&format!("Configured path does not exist: {}", dir));
        }
    }

    // Keep service alive until stop is requested
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(10));
    }

    log("Service stopping, waiting for threads to finish...");

    // Wait for all watcher threads to finish
    for handle in handles {
        let _ = handle.join();
    }

    // Set service status to stopped
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::ZERO,
        process_id: Some(pid),
    })?;

    log("Service stopped.");
    Ok(())
}

fn watch_directory(
    watch_dir: &Path,
    running: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(tx)?;
    watcher.watch(watch_dir, RecursiveMode::Recursive)?;
    log(&format!("Started watching: {:?}", watch_dir));

    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(event)) => {
                log(&format!("File event received: {:?}", event));

                match event.kind {
                    // Handle folder creation
                    EventKind::Create(_) => {
                        for path in event.paths {
                            if path.is_dir() && path.parent() == Some(watch_dir) {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    log(&format!("Processing new directory: {}", name));
                                    if let Err(e) = add_host_entry(name) {
                                        log(&format!("Failed to add host for {}: {}", name, e));
                                    } else {
                                        log(&format!("Added host for {}.test", name));
                                    }
                                }
                            }
                        }
                    }

                    // Handle folder deletion
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            // Only check parent, not is_dir
                            if path.parent() == Some(watch_dir) {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    log(&format!("Directory removed: {}", name));
                                    if let Err(e) = remove_host_entry(name) {
                                        log(&format!("Failed to remove host for {}: {}", name, e));
                                    } else {
                                        log(&format!("Removed host for {}.test", name));
                                    }
                                }
                            }
                        }
                    }

                    // Handle folder rename
                    EventKind::Modify(ref kind) => match kind {
                        ModifyKind::Name(RenameMode::From) => {
                            if let Some(path) = event.paths.get(0) {
                                if path.parent() == Some(watch_dir) {
                                    if let Some(old_name) =
                                        path.file_name().and_then(|n| n.to_str())
                                    {
                                        log(&format!("Directory renamed from: {}", old_name));
                                        if let Err(e) = remove_host_entry(old_name) {
                                            log(&format!(
                                                "Failed to remove host for {}: {}",
                                                old_name, e
                                            ));
                                        } else {
                                            log(&format!("Removed host for {}.test", old_name));
                                        }
                                    }
                                }
                            }
                        }

                        ModifyKind::Name(RenameMode::To) => {
                            if let Some(path) = event.paths.get(0) {
                                if path.parent() == Some(watch_dir) {
                                    if let Some(new_name) =
                                        path.file_name().and_then(|n| n.to_str())
                                    {
                                        log(&format!("Directory renamed to: {}", new_name));
                                        if let Err(e) = add_host_entry(new_name) {
                                            log(&format!(
                                                "Failed to add host for {}: {}",
                                                new_name, e
                                            ));
                                        } else {
                                            log(&format!("Added host for {}.test", new_name));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    },

                    _ => {}
                }
            }

            Ok(Err(e)) => log(&format!("Watch error: {:?}", e)),
            Err(_) => continue, // Timeout is normal
        }
    }

    log(&format!("Stopped watching: {:?}", watch_dir));
    Ok(())
}

fn add_host_entry(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entry = format!("127.0.0.1\t{}.test #added by laracli", project_name);
    let hosts_path = r"C:\Windows\System32\drivers\etc\hosts";

    // Check if entry already exists
    let file = File::open(hosts_path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.contains(&format!("{}.test", project_name)) {
            log(&format!(
                "Host entry for {}.test already exists",
                project_name
            ));
            return Ok(());
        }
    }

    // Append entry
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(hosts_path)?;
    writeln!(file, "{}", entry)?;
    log(&format!(
        "Successfully added host entry for {}.test",
        project_name
    ));
    Ok(())
}

// Fix path normalization to remove Windows extended-length prefix
fn normalize_path(path_str: &str) -> PathBuf {
    let path = PathBuf::from(path_str);

    // Remove Windows extended-length path prefix if present
    if let Ok(canonical) = path.canonicalize() {
        let canonical_str = canonical.to_string_lossy();
        if canonical_str.starts_with("\\\\?\\") {
            PathBuf::from(&canonical_str[4..])
        } else {
            canonical
        }
    } else {
        path
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub watched_paths: HashSet<String>,
}

fn get_config_path() -> PathBuf {
    // Windows services often don't have access to user home directories
    // Try multiple locations in order of preference

    // First try user home directory
    if let Some(home) = dirs::home_dir() {
        let user_config = home.join(".laracli").join("config.json");
        if user_config.exists() {
            log(&format!("Using user config: {:?}", user_config));
            return user_config;
        }
    }

    // Try common locations where the service can access
    let system_locations = vec![
        PathBuf::from(r"C:\laracli\config.json"),
        PathBuf::from(r"C:\ProgramData\laracli\config.json"),
    ];

    for location in system_locations {
        if location.exists() {
            log(&format!("Using system config: {:?}", location));
            return location;
        }
    }

    // Default to system location
    let default_path = PathBuf::from(r"C:\ProgramData\laracli\config.json");
    log(&format!(
        "Using default config location: {:?}",
        default_path
    ));
    default_path
}

fn load_config() -> Config {
    let path = get_config_path();
    log(&format!("Looking for config at: {:?}", path));

    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                log(&format!("Config file contents length: {}", contents.len()));
                match serde_json::from_str::<Config>(&contents) {
                    Ok(config) => {
                        log(&format!(
                            "Successfully loaded config with {} paths",
                            config.watched_paths.len()
                        ));
                        for path in &config.watched_paths {
                            log(&format!("  - {}", path));
                        }
                        config
                    }
                    Err(e) => {
                        log(&format!("Failed to parse config: {}", e));
                        Config {
                            watched_paths: HashSet::new(),
                        }
                    }
                }
            }
            Err(e) => {
                log(&format!("Failed to read config file: {}", e));
                Config {
                    watched_paths: HashSet::new(),
                }
            }
        }
    } else {
        log(&format!("Config file not found at: {:?}", path));

        // Try to copy from user location if it exists
        if let Some(home) = dirs::home_dir() {
            let user_config = home.join(".laracli").join("config.json");
            if user_config.exists() {
                log(&format!(
                    "Found user config at: {:?}, copying to service location",
                    user_config
                ));
                if let Err(e) = copy_config_to_service_location(&user_config, &path) {
                    log(&format!("Failed to copy config: {}", e));
                } else {
                    return load_config(); // Retry loading
                }
            }
        }

        Config {
            watched_paths: HashSet::new(),
        }
    }
}

fn log(msg: &str) {
    let log_path = r"C:\laracli\laracli.log";
    let _ = std::fs::create_dir_all("C:\\laracli");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);

        // Also print to console in debug mode
        #[cfg(debug_assertions)]
        println!("[{}] {}", timestamp, msg);
    }
}

fn copy_config_to_service_location(
    from: &PathBuf,
    to: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create directory if it doesn't exist
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Copy the config file
    std::fs::copy(from, to)?;
    log(&format!("Copied config from {:?} to {:?}", from, to));
    Ok(())
}

fn main() -> windows_service::Result<()> {
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

fn remove_host_entry(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hosts_path = r"C:\Windows\System32\drivers\etc\hosts";
    let temp_hosts_path = r"C:\laracli\hosts.tmp";

    // Read the current hosts file
    let input_file = File::open(hosts_path)?;
    let reader = BufReader::new(input_file);

    // Write all lines except the one with our project name into a temporary file
    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(temp_hosts_path)?;

    let mut found = false;

    for line in reader.lines() {
        let line = line?;
        if !line.contains(&format!("{}.test", project_name)) {
            writeln!(output_file, "{}", line)?;
        } else {
            found = true;
            log(&format!("Removed host entry for {}.test", project_name));
        }
    }

    // Replace original hosts file with the updated one
    std::fs::copy(temp_hosts_path, hosts_path)?;
    std::fs::remove_file(temp_hosts_path)?; // Clean up temporary file

    if !found {
        log(&format!("No host entry found for {}.test", project_name));
    }

    Ok(())
}
