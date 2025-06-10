use std::io::Write;
use crate::helpers::path;

pub fn create_my_ini_file() {
    //create my.ini file
    let my_ini_dir_path = path::get_mysql_path().unwrap();
    let my_ini_path = std::path::Path::new(&my_ini_dir_path).join("my.ini");

    let mut file = std::fs::File::create(&my_ini_path).unwrap();
    file.write_all(generate_ini_file(&my_ini_path.to_str().unwrap(), &my_ini_dir_path).as_bytes()).unwrap();

}

fn generate_ini_file(path: &str, my_ini_dir_path: &str) -> String {
    format!(
        r#"
[mysqld]
basedir={}
datadir={}/data
lc-messages-dir={}/share

[client]
port=3306
socket=mysql.sock
log-error=./mysql-error.log
pid-file=./mysql.pid
lc-messages-dir=./share
        "#,
        path.replace("\\", "/"),
        my_ini_dir_path.replace("\\", "/"),
        my_ini_dir_path.replace("\\", "/")
    )
}