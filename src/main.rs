use argh::FromArgs;
use colored::Colorize;
mod cli;
mod commands {
    pub mod nginx;
    pub mod mysql;
    pub mod watch;
    pub mod link;
    pub mod setup;
}
mod helpers;
mod utils;

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
            commands::setup::setup_tools().await.expect("Failed to setup tools");
            commands::setup::setup_services().expect("Failed to setup services");
            helpers::config::create_config_file();
            commands::setup::add_exe_to_path().expect("Failed to add exe to path");
        }
        cli::Commands::Run(_) => {
            commands::nginx::start().expect("Failed to start Nginx");
            commands::mysql::start().expect("Failed to start MySQL");
        }
    }
}
