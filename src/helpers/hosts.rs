use std::{fs::{self, OpenOptions}, io::BufReader};
use std::io::{Write, BufRead};
use std::fs::File;


pub fn add_host_entry(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {

    // elevate();
    // if !elevate::is_elevated() {
    //     println!("🔒 Elevation required to modify hosts file. Requesting UAC permission...");
    //     elevate::run_as_admin()?;
    //     return Ok(());
    // }

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


pub fn remove_host_entry(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {

    // elevate();
    // if !elevate::is_elevated() {
    //     println!("🔒 Elevation required to modify hosts file. Requesting UAC permission...");
    //     elevate::run_as_admin()?;
    //     return Ok(());
    // }
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
