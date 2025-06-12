use argh::FromArgs;

#[derive(FromArgs, Debug)]
/// Simple tool to manage Laravel's or php projects
#[argh(subcommand)]
pub enum Commands {
    Start(Start),
    Stop(Stop),
    Reload(Reload),
    Watch(Watch),
    ListWatched(ListWatched),
    Unwatch(Unwatch),
    Link(Link),
    Unlink(Unlink),
    Setup(Setup),
    StartDev(Run),
    StopDev(StopDev),
    Version(Version),
    Enable(Enable),
    // Disable(Disable),
    PhpExtension(PhpExtension),
}

#[derive(FromArgs, Debug)]
/// Start a service (nginx or mysql)
#[argh(subcommand, name = "start")]
pub struct Start {
    /// service to start: nginx or mysql
    #[argh(subcommand)]
    pub service: Service,
}

#[derive(FromArgs, Debug)]
/// Stop a service (nginx or mysql)
#[argh(subcommand, name = "stop")]
pub struct Stop {
    /// service to stop: nginx or mysql
    #[argh(subcommand)]
    pub service: Service,
}

#[derive(FromArgs, Debug)]
/// Reload nginx service
#[argh(subcommand, name = "reload")]
pub struct Reload {
    /// reload nginx service
    #[argh(subcommand)]
    pub service: NginxReload,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum Service {
    Mysql(Mysql),
    Nginx(Nginx),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "nginx")]
/// Nginx service
pub struct Nginx {}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "nginx")]
/// Nginx reload
pub struct NginxReload {}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "mysql")]
/// MySQL service
pub struct Mysql {}

/// Watch for new Laravel projects and auto-add hosts
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "watch")]
pub struct Watch {
    /// path to watch (e.g., C:\www)
    #[argh(positional)]
    pub path: String,
}

/// List all currently watched directories
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "list-watched")]
pub struct ListWatched {}

/// Remove a directory from the watch list
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "unwatch")]
pub struct Unwatch {
    /// path to stop watching (e.g., C:\www)
    #[argh(positional)]
    pub path: String,
}

/// Link a directory 
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "link")]
pub struct Link {
    /// path to link (e.g., C:\www/laravel)
    #[argh(positional)]
    pub path: String,
}

/// Unlink an existing directory
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "unlink")]
pub struct Unlink {
    /// path to unlink (e.g., C:\www/laravel)
    #[argh(positional)]
    pub path: String,
}

/// Setup and install services with necessary permissions
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "setup")]
pub struct Setup {}

/// start php nginx and mysql services
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "start-dev")]
pub struct Run {}

/// Show version information
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "version")]
pub struct Version {}

/// Enable a feature (e.g., phpmyadmin)
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "enable")]
pub struct Enable {
    /// feature to enable (e.g., phpmyadmin)
    #[argh(subcommand)]
    pub feature: Feature,
}

/// Disable a feature (e.g., phpmyadmin)
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "disable")]
pub struct Disable {
    /// feature to disable (e.g., phpmyadmin)
    #[argh(subcommand)]
    pub feature: Feature,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum Feature {
    PhpMyAdmin(PhpMyAdmin),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "phpmyadmin")]
/// phpMyAdmin feature
pub struct PhpMyAdmin {}

/// Manage PHP extensions
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "php-ext")]
pub struct PhpExtension {
    /// PHP extension subcommand: enable or disable
    #[argh(subcommand)]
    pub action: PhpExtensionAction,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum PhpExtensionAction {
    Enable(EnablePhpExtension),
    Disable(DisablePhpExtension),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "enable")]
/// Enable a PHP extension
pub struct EnablePhpExtension {
    /// name of the PHP extension to enable (e.g., mbstring)
    #[argh(positional)]
    pub extension: String,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "disable")]
/// Disable a PHP extension
pub struct DisablePhpExtension {
    /// name of the PHP extension to disable (e.g., mbstring)
    #[argh(positional)]
    pub extension: String,
}

/// Stop php nginx and mysql services
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "stop-dev")]
pub struct StopDev {}













#[derive(FromArgs, Debug)]
/// laracli
#[argh(description = "Simple tool to manage Laravel's or php projects")]
pub struct Cli {
    #[argh(subcommand)]
    pub command: Commands,
}