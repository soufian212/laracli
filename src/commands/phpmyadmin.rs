use crate::helpers;
use colored::Colorize;
use std::fs;

pub async fn enable_phpmyadmin() -> Result<(), Box<dyn std::error::Error>> {
    let tools_dir = helpers::path::get_tools_path().unwrap();
    //check if phpmyadmin is already installed
    if !tools_dir.join("phpMyAdmin-5.2.2-all-languages").exists() {
        println!("{}", "Installing PhpMyAdmin".yellow());
        //download  phpmyadmin
        let phpmyadmin_url =
            "https://files.phpmyadmin.net/phpMyAdmin/5.2.2/phpMyAdmin-5.2.2-all-languages.zip";
        let phpmyadmin_zip = "phpMyAdmin-5.2.2-all-languages.zip";
        println!("{}", "Downloading PhpMyAdmin".yellow());
        helpers::download::download_with_progress_async(
            phpmyadmin_url,
            phpmyadmin_zip,
            "PhpMyAdmin",
            3,
        )
        .await
        .expect("Failed to download PhpMyAdmin");

        //unzip phpmyadmin
        println!("{}", "Extracting PhpMyAdmin".yellow());
        let phpmyadmin_file =
            fs::File::open(phpmyadmin_zip).expect("Failed to open phpmyadmin zip file");
        let mut zip =
            zip::ZipArchive::new(phpmyadmin_file).expect("Failed to open phpmyadmin zip file");
        zip.extract(&tools_dir)?;
        fs::remove_file(phpmyadmin_zip).expect("Failed to remove phpmyadmin zip file");
        println!("{}", "PhpMyAdmin installed successfully".green());
    }

    println!("{}", "Creating conf file for phpmyadmin".yellow());
    helpers::nginx::create_nginx_config(
        &tools_dir
            .join("phpMyAdmin-5.2.2-all-languages")
            .to_str()
            .unwrap(),
        Some("phpmyadmin"),
    )
    .expect("failed to create nginx config");
    println!("{}", "✅ config file created".yellow());
    println!("{}", "Linking directory".yellow());
    helpers::config::add_to_linked_paths(tools_dir.join("phpmyadmin").to_str().unwrap());
    println!("✅ Updated config with linked path: {}", tools_dir.join("phpmyadmin").to_str().unwrap());
    println!("{}", "Adding config.php...");

    //rename config.sample.inc.php to config.inc.php
    let config_path = tools_dir.join("phpMyAdmin-5.2.2-all-languages").join("config.sample.inc.php");
    let config_path_new = tools_dir.join("phpMyAdmin-5.2.2-all-languages").join("config.inc.php");
    fs::rename(config_path, &config_path_new).expect("Failed to rename config.sample.inc.php to config.inc.php");
    // $cfg['Servers'][$i]['AllowNoPassword'] = false; to true 
    let config_file = fs::read_to_string(&config_path_new).expect("Failed to read config.php");
    let config_file = config_file.replace("$cfg['Servers'][$i]['AllowNoPassword'] = false;", "$cfg['Servers'][$i]['AllowNoPassword'] = true;");
    let config_file = config_file.replace("$cfg['Servers'][$i]['user'] = 'root';", "$cfg['Servers'][$i]['user'] = 'root';");
    fs::write(&config_path_new, config_file).expect("Failed to write config.php");

    crate::commands::nginx::reload().expect("Failed to reload nginx");
    println!("✅ ready to go visit http://phpmyadmin.test");


    Ok(())
}
