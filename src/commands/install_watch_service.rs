use windows_service::{
    service::{
        ServiceStartType, ServiceErrorControl, ServiceInfo, ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

pub fn install_service() -> Result<(), Box<dyn std::error::Error>> {
    let service_binary_path = std::env::current_exe()?.with_file_name("laracli-service.exe");

    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
    let service_info = ServiceInfo {
        name: "LaracliWatcher".into(),
        display_name: "Laracli Directory Watcher".into(),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // Runs as LocalSystem
        account_password: None,
    };

    manager.create_service(&service_info, windows_service::service::ServiceAccess::all())?;

    println!("✅ Service installed: LaracliWatcher");
    Ok(())
}
