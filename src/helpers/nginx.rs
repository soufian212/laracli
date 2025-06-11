use std::fs::OpenOptions;
use std::io::Write;
use crate::helpers::{path};


pub fn create_global_nginx_config() -> Result<(), Box<dyn std::error::Error>> {
    let nginx_path = path::get_nginx_path().unwrap();
    let global_config_path = std::path::Path::new(&nginx_path).join("conf/nginx.conf");
    let config_content = generate_nginx_global_config(std::path::Path::new(&nginx_path).join("sites-enabled").to_str().unwrap());
    let mut file = OpenOptions::new()
    .create(true)
    .write(true)
    .open(&global_config_path)?;
    file.write_all(config_content.as_bytes())?;
    Ok(())
}



pub fn create_nginx_config(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let  nginx_path = path::get_nginx_path().unwrap();
    let global_config_path = std::path::Path::new(&nginx_path).join("conf/nginx.conf");

    //check global nginx.conf exists
    if !global_config_path.exists() {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&global_config_path)?;

    let config_content = generate_nginx_global_config(std::path::Path::new(&nginx_path).join("sites-enabled").to_str().unwrap());


        file.write_all(config_content.as_bytes())?;

        return Ok(());
    }

    //check if sites-enabled exists
    if !std::path::Path::new(&nginx_path).join("sites-enabled").exists() {
        std::fs::create_dir(std::path::Path::new(&nginx_path).join("sites-enabled"))?;
    }


    let config_name = std::path::Path::new(&path).file_name().unwrap().to_str().unwrap();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(std::path::Path::new(&path::get_nginx_path().unwrap()).join("sites-enabled").join(format!("{}.conf", config_name)))?;

    let config_content = generate_nginx_site_config(path, config_name);

    file.write_all(config_content.as_bytes())?;
    Ok(())
}


fn generate_nginx_global_config(include_path: &str) -> String {
    format!(
        r#"worker_processes  1;

events {{
    worker_connections  1024;
}}

http {{
    include       mime.types;
    default_type  application/octet-stream;

    log_format  main  '$remote_addr - $remote_user [$time_local] "$request" '
                      '$status $body_bytes_sent "$http_referer" '
                      '"$http_user_agent" "$http_x_forwarded_for"';

    access_log  logs/access.log  main;

    sendfile        on;
    #tcp_nopush     on;

    keepalive_timeout  65;

    #gzip  on;

    include "{}/*.conf";

    server {{
        listen       80;
        server_name  localhost;

        location / {{
            root   html;
            index  index.html index.htm;
        }}

        error_page   500 502 503 504  /50x.html;
        location = /50x.html {{
            root   html;
        }}
    }}
}}
pid        logs/nginx.pid;
"#,
        include_path.replace('\\', "/")
    )
}

fn generate_nginx_site_config(path: &str, server_name: &str) -> String {
    let root_path = {
        let path = std::path::Path::new(path);
        if path.join("index.php").exists() || path.join("index.html").exists() || path.join("index.htm").exists() {
            path.to_string_lossy().into_owned()
        } else if path.join("public").join("index.php").exists() || path.join("public").join("index.html").exists() || path.join("public").join("index.htm").exists() {
            path.join("public").to_string_lossy().into_owned()
        } else {
            path.to_string_lossy().into_owned()
        }
    };

    format!(
        r#"
       

server {{
    listen       80;
    server_name  {}.test;

    location / {{
        root   "{}";
        index  index.php index.html index.htm;
        try_files $uri $uri/ /index.php?$query_string;
    }}

    location ~ \.php$ {{
        root           "{}";
        fastcgi_pass   127.0.0.1:9000;
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME $document_root$fastcgi_script_name;
        include        fastcgi_params;
    }}

    location ~ /\.ht {{
        deny all;
    }}
}}

"#,
        server_name,
        root_path.replace('\\', "/"),
        root_path.replace('\\', "/")
    )
}
pub fn delete_nginx_config(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_name = std::path::Path::new(&path).file_name().unwrap().to_str().unwrap();
    let nginx_path = path::get_nginx_path().unwrap();
    std::fs::remove_file(std::path::Path::new(&nginx_path).join("sites-enabled").join(format!("{}.conf", config_name)))?; 
    Ok(())
}