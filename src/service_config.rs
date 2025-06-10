use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

use chrono::Local;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use serde::{Deserialize, Serialize};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
};
use std::collections::HashSet;


const SERVICE_NAME: &str = "laracli_config";
const CONFIG_PATH: &str = "C:\\laracli\\config.json";
const LOG_PATH: &str = "C:\\laracli\\laracli_config.log";

define_windows_service!(ffi_service_config_main, config_service_main);

fn config_service_main(_arguments: Vec<std::ffi::OsString>) {
    if let Err(e) = run_service() {
        log(&format!("Config service failed: {}", e));
    }
}

fn run_service() -> windows_service::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let status_handle =
        service_control_handler::register(
            SERVICE_NAME,
            move |control_event| match control_event {
                ServiceControl::Stop => {
                    log("Received stop event for config service.");
                    running_clone.store(false, Ordering::SeqCst);
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            },
        )?;

    let pid = std::process::id();
    log(&format!("Config service started with PID: {}", pid));

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::ZERO,
        process_id: Some(pid),
    })?;

    let mut previous_paths = load_linked_paths(CONFIG_PATH);

    let config_dir = Path::new(CONFIG_PATH).parent().unwrap();
    let config_path = PathBuf::from(CONFIG_PATH);

    let watcher_running = running.clone();
    fn to_set(paths: &[String]) -> HashSet<String> {
        paths.iter().cloned().collect()
    }

    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    // Debounce small bursts of writes
                    thread::sleep(Duration::from_millis(100));

                    if let Some(current_paths) = load_linked_paths(CONFIG_PATH) {
                        let current_set = to_set(&current_paths);
                        let prev_set = previous_paths.as_ref().map(|v| to_set(v.as_slice())).unwrap_or_default();

                        let added: Vec<_> = current_set.difference(&prev_set).cloned().collect();
                        let removed: Vec<_> = prev_set.difference(&current_set).cloned().collect();

                        if !added.is_empty() || !removed.is_empty() {
                            if !added.is_empty() {
                                for path in &added {
                                    log(&format!("linked_paths: added \"{}\"", path));
                                    let file_name = Path::new(path).file_name().unwrap().to_str().unwrap().to_string();
                                    match add_host_entry(&file_name) {
                                        Ok(_) => {
                                            log(&format!("linked_paths: added \"{}\"", path));
                                            
                                        }
                                        Err(e) => {
                                            log(&format!("linked_paths: failed to add \"{}\"", path));
                                            log(&format!("linked_paths: {}", e));
                                        }
                                    }

                                    match reload_nginx() {
                                        Ok(_) => {
                                            log(&format!("linked_paths: reloaded Nginx"));
                                        }
                                        Err(e) => {
                                            log(&format!("linked_paths: failed to reload Nginx"));
                                            log(&format!("linked_paths: {}", e));
                                        }
                                    }
                                    
                                    
                                }
                            }

                            if !removed.is_empty() {
                                for path in &removed {
                                    log(&format!("linked_paths: removed \"{}\"", path));
                                    let file_name = Path::new(path).file_name().unwrap().to_str().unwrap().to_string();
                                    match remove_host_entry(&file_name) {
                                        Ok(_) => {
                                            log(&format!("linked_paths: removed \"{}\"", path));
                                        }
                                        Err(e) => {
                                            log(&format!("linked_paths: failed to remove \"{}\"", path));
                                            log(&format!("linked_paths: {}", e));
                                        }
                                    }
                                    match reload_nginx() {
                                        Ok(_) => {
                                            log(&format!("linked_paths: reloaded Nginx"));
                                        }
                                        Err(e) => {
                                            log(&format!("linked_paths: failed to reload Nginx"));
                                            log(&format!("linked_paths: {}", e));
                                        }
                                    }
                                }
                            }

                            previous_paths = Some(current_paths);
                        }
                    } else {
                        log("Failed to load config.json.");
                    }
                }
            }
        })
        .expect("Failed to create watcher");

    watcher.watch(config_dir, RecursiveMode::NonRecursive);

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(5));
    }

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::ZERO,
        process_id: Some(pid),
    })?;

    log("Config service stopped.");
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
struct Config {
    linked_paths: Vec<String>,
    // Other config fields can be added here
}

fn load_linked_paths<P: AsRef<Path>>(path: P) -> Option<Vec<String>> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader).ok()?;
    Some(config.linked_paths)
}

fn log(msg: &str) {
    let _ = std::fs::create_dir_all("C:\\laracli");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG_PATH) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
        #[cfg(debug_assertions)]
        println!("[{}] {}", timestamp, msg);
    }
}

fn main() -> windows_service::Result<()> {
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_config_main)?;
    Ok(())
}


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
            println!("🔒 Removed host entry for {}.test", project_name);
        }
    }

    // Replace original hosts file with the updated one
    std::fs::copy(temp_hosts_path, hosts_path)?;
    std::fs::remove_file(temp_hosts_path)?; // Clean up temporary file

    if !found {
        println!("Host entry for {}.test not found.", project_name);
    }

    Ok(())
}

fn reload_nginx() -> Result<(), Box<dyn std::error::Error>> {
    let nginx_path = laracli::helpers::path::get_nginx_path().unwrap();
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
    if !output.success() {
        return Err("Failed to reload Nginx service".to_string().into());
    } 

    Ok(())
}