# Laracli

Laracli is an open-source command-line tool for managing Laravel/PHP projects on Windows. It automates Nginx and MySQL setup, links projects with .test domains, and monitors directories for new projects (beta as of June 10, 2025, 11:59 PM +01). This README covers using the pre-built release, as building from source requires Rust.

## Features

- **Setup Services**: Installs laracli and laracli_config Windows services for automation.
- **Tool Downloads**: Automatically downloads and extracts Nginx 1.23.3 and MySQL 8.4.5 via setup-tools.
- **Link/Unlink Projects**: Configures projects with Nginx and .test domains.
- **Watch Directories**: Monitors directories for new Laravel projects and adds host entries.
- **Service Management**: Start, stop, and reload Nginx and MySQL services.

## Prerequisites

- **Windows OS**: Designed for Windows with service and hosts file support.
- **Administrative Privileges**: Required for service installation, hosts file changes, and downloads.

## Installation

### Download Pre-Built Release

1. **Download the ZIP**:
   - Get the latest release from the [GitHub Releases](https://github.com/soufian212/laracli/releases) page (e.g., laracli-v0.1.0-beta.zip).
   - Extract it to a directory (e.g., C:\laracli).

2. **Initialize Setup**:
   - Open a Command Prompt or PowerShell as Administrator.
   - Navigate to the C:\laracli directory:
     
     cd C:\laracli
     
   - Run the setup command to download tools and configure services:
     
     
     laracli setup
     
     
   This will download Nginx and MySQL, create services, and add laracli to your PATH.

3. **Verify Installation**:
   - Run:
     
     laracli --help
     
   - Ensure no errors occur.

## Usage

### Setup Commands

- **Setup Tools**:
  
  laracli setup
  
  Downloads and extracts Nginx and MySQL to tools/.

- **Setup Services**:
  
  laracli setup
  
  Installs and starts laracli and laracli_config services.

- **Add to PATH**:
  
  laracli add-exe-to-path
  
  Adds laracli to your system PATH for global access.

### Project Management

- **Link a Project**:
  
  laracli link C:\www\myproject
  
  Links myproject with a .test domain and Nginx config.

- **Unlink a Project**:
  
  laracli unlink C:\www\myproject
  
  Removes the link and cleans up configurations.

### Directory Watching

- **Watch a Directory**:
  
  laracli watch C:\www
  
  Monitors C:\www for new Laravel projects and adds .test entries.

- **List Watched Directories**:
  
  laracli list
  
  Shows all watched directories.

- **Unwatch a Directory**:
  
  laracli unwatch C:\www
  
  Stops monitoring the specified directory.

### Service Commands

- **Start Nginx**:
  
  laracli nginx start
  

- **Stop Nginx**:
  
  laracli nginx stop
  

- **Reload Nginx**:
  
  laracli nginx reload
  

- **Start MySQL**:
  
  laracli mysql start
  

- **Stop MySQL**:
  
  laracli mysql stop
  

## Notes

- **Beta Status**: As of June 10, 2025, 11:59 PM +01, watch is in progress. Other commands are functional but may evolve.
- **Logs**: Check C:\laracli\laracli.log and C:\laracli\laracli_config.log for debugging.
- **Config**: Located at C:\ProgramData\laracli\config.json.
- **Permissions**: Run commands in an elevated terminal.
- **License**: Custom Laracli License (no commercial sale allowed, MIT-style use otherwise).
- **Open Source**: Source code is available under the above license. Contribute at [GitHub](https://github.com/soufian212/laracli).

## Troubleshooting

- **Setup Fails**: Verify admin privileges and internet connectivity. Check logs.
- **Service Issues**: Run `sc query laracli` or `sc query laracli_config` and review logs.
- **Download Errors**: Ensure no firewall blocks downloads. Retry setup-tools.

## Contributing
Submit issues or pull requests on the [GitHub repository](https://github.com/soufian212/laracli).
