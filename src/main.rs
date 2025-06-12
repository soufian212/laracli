use colored::Colorize;
mod cli;
mod commands {
    pub mod link;
    pub mod mysql;
    pub mod nginx;
    pub mod php;
    pub mod phpmyadmin;
    pub mod setup;
    pub mod watch;
}
mod helpers;
mod utils;

const VERSION: &str = "0.4.3-beta";
const NAME: &str = "laracli";
const BUILD_DATE: &str = env!("BUILD_DATE");
const GIT_HASH: &str = env!("GIT_HASH");

fn print_version() {
    println!("{} v{}", NAME.bright_cyan(), VERSION.bright_green());
    println!("Build: {} ({})", BUILD_DATE.dimmed(), GIT_HASH.dimmed());
    println!("Platform: {}", std::env::consts::OS);
}

#[tokio::main]
async fn main() {
    let cli: cli::Cli = argh::from_env();

    match cli.command {
        cli::Commands::Reload(_) => {
            commands::nginx::reload().expect("Failed to reload Nginx");
        }
        cli::Commands::Start(start) => match start.service {
            cli::Service::Nginx(_) => {
                println!("Starting Nginx...");
                commands::nginx::start().expect("Failed to start Nginx");
            }
            cli::Service::Mysql(_) => {
                println!("Starting MySQL...");
                commands::mysql::start().expect("Failed to start MySQL");
            }
        },
        cli::Commands::Stop(stop) => match stop.service {
            cli::Service::Nginx(_) => {
                commands::nginx::stop().expect("Failed to stop Nginx");
            }
            cli::Service::Mysql(_) => {
                println!("Stopping MySQL...");
                commands::mysql::stop().expect("Failed to stop MySQL");
            }
        },
        cli::Commands::Watch(watch) => {
            println!("Watching directory: {}", watch.path);
            commands::watch::watch_directory(&watch.path).unwrap();
        }
        cli::Commands::ListWatched(_) => {
            commands::watch::list_watched_directories()
                .expect("Failed to list watched directories");
        }
        cli::Commands::Unwatch(unwatch_cmd) => {
            commands::watch::unwatch_directory(&unwatch_cmd.path)
                .expect("Failed to unwatch directory");
        }
        cli::Commands::Link(link) => {
            commands::link::link(&link.path).unwrap();
        }
        cli::Commands::Unlink(unlink) => {
            commands::link::unlink(&unlink.path).unwrap();
        }
        cli::Commands::Setup(_) => {
            println!("{}", "Setting up services...".yellow());
            commands::setup::setup_tools()
                .await
                .expect("Failed to setup tools");
            commands::setup::setup_services().expect("Failed to setup services");
            commands::setup::setup_permissions().expect("Failed to setup permissions");
            helpers::config::create_config_file();
            commands::setup::add_exe_to_path().expect("Failed to add exe to path");
        }
        cli::Commands::StartDev(_) => {
            commands::php::start_php_cgi().expect("Failed to start PHP CGI");
            commands::nginx::start().expect("Failed to start Nginx");
            commands::mysql::start().expect("Failed to start MySQL");
        }
        cli::Commands::Version(_) => {
            print_version();
        }
        cli::Commands::Enable(enable) => match enable.feature {
            cli::Feature::PhpMyAdmin(_) => {
                commands::phpmyadmin::enable_phpmyadmin().await.expect("Failed to enable phpMyAdmin");
            }
        },
        cli::Commands::PhpExtension(ext) => match ext.action {
            cli::PhpExtensionAction::Enable(ext_cmd) => {
                commands::php::enable_php_extension(&ext_cmd.extension)
                    .expect("Failed to enable PHP extension");
            }
            cli::PhpExtensionAction::Disable(ext_cmd) => {
                commands::php::disable_php_extension(&ext_cmd.extension)
                    .expect("Failed to disable PHP extension");
            }
        },
        cli::Commands::StopDev(_) => {
            commands::php::stop_php_cgi().expect("Failed to start PHP CGI");
            commands::nginx::stop().expect("Failed to start Nginx");
            commands::mysql::stop().expect("Failed to start MySQL");
        }
    }
}