use std::{env, path::PathBuf};


pub fn get_current_exe_dir() -> Result<String, Box<dyn std::error::Error>> {
    let current_exe_path = env::current_exe()?;
    let current_exe_dir = current_exe_path.parent().ok_or("Failed to get parent directory of the executable")?;
    let current_exe_dir_str = current_exe_dir.to_str().ok_or("Failed to convert current exe directory to string")?;
    Ok(current_exe_dir_str.to_string())
}
pub fn get_nginx_path() -> Result<String, Box<dyn std::error::Error>> {
    let current_exe_dir = get_current_exe_dir()?;
    let nginx_path = std::path::Path::new(&current_exe_dir).join("tools/nginx-1.23.3");
    let nginx_path_str = nginx_path.to_str().ok_or("Failed to convert nginx path to string")?;
    Ok(nginx_path_str.to_string())
}

pub fn get_mysql_path() -> Result<String, Box<dyn std::error::Error>> {
    let current_exe_dir = get_current_exe_dir()?;
    let mysql_path = std::path::Path::new(&current_exe_dir).join("tools/mysql-8.4.5-winx64");
    let mysql_path_str = mysql_path.to_str().ok_or("Failed to convert mysql path to string")?;
    Ok(mysql_path_str.to_string())
}

pub fn get_php_path() -> Result< PathBuf, Box<dyn std::error::Error>> {
    let current_exe_dir = get_current_exe_dir()?;
    let php_path = std::path::Path::new(&current_exe_dir).join("tools/php-8.4.8-nts-Win32-vs17-x64");
    Ok(php_path)
}