use colored::Colorize;
mod cli;
mod commands {
    pub mod nginx;
    pub mod mysql;
    pub mod watch;
    pub mod link;
    pub mod setup;
    pub mod php;
}
mod helpers;
mod utils;

const VERSION: &str = "0.3.0-beta";
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
            commands::setup::setup_tools().await.expect("Failed to setup tools");
            commands::setup::setup_services().expect("Failed to setup services");
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

    }
}
