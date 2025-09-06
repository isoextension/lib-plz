use clap::{Arg, Command};
use colored::*;
use lib_plz::{PlzManager, Package};
use nix::unistd::Uid;

fn has_root() -> bool {
    Uid::effective().is_root()
}

fn assert_root() {
    if !has_root() {
        eprintln!("{}: This command requires root privileges (run with sudo)", "Error".red().bold());
        std::process::exit(1);
    }
}

fn list_packages(packages: &[&Package]) {
    println!("{}", "Available Packages:".bold().blue());
    println!();
    
    for package in packages {
        let status = if package.installed {
            "[installed]".green()
        } else {
            "[available]".yellow()
        };
        
        println!(
            "{} {} {} - {}",
            package.name.bold(),
            package.version.dimmed(),
            status,
            package.description
        );
    }
}

fn show_package_info(manager: &PlzManager, name: &str) {
    if let Some(package) = manager.find_package(name) {
        println!("{}: {}", "Package".bold(), package.name);
        println!("{}: {}", "Version".bold(), package.version);
        println!("{}: {}", "Architecture".bold(), package.architecture);
        println!("{}: {}", "Maintainer".bold(), package.maintainer);
        if let Some(homepage) = &package.homepage {
            println!("{}: {}", "Homepage".bold(), homepage);
        }
        println!("{}: {}", "Description".bold(), package.description);
        
        if package.installed {
            println!("{}: {}", "Status".bold(), "Installed".green());
        } else {
            println!("{}: {}", "Status".bold(), "Not installed".yellow());
        }
        
        if !package.depends.is_empty() {
            println!("{}: {}", "Depends".bold(), package.depends.join(", "));
        }
        
        if !package.optdepends.is_empty() {
            println!("{}: {}", "Optional Depends".bold(), package.optdepends.join(", "));
        }
        
        if !package.conflicts.is_empty() {
            println!("{}: {}", "Conflicts".bold(), package.conflicts.join(", "));
        }
        }
        Ok(None) => {
            println!("{}: Package '{}' not found", "Error".red().bold(), name);
        }
        Err(e) => {
            eprintln!("{}: Failed to get package info: {}", "Error".red().bold(), e);
        }
}

fn main() {
    let matches = Command::new("plz")
        .about("Please - A polite package manager")
        .version("0.1.0")
        .subcommand(
            Command::new("list")
                .about("List available packages")
        )
        .subcommand(
            Command::new("show")
                .about("Show detailed information about a package")
                .arg(
                    Arg::new("package")
                        .help("Package name")
                        .required(true)
                        .index(1)
                )
        )
        .subcommand(
            Command::new("install")
                .about("Install a package")
                .arg(
                    Arg::new("package")
                        .help("Package name")
                        .required(true)
                        .index(1)
                )
        )
        .get_matches();

    // Initialize the plz manager
    let mut manager = PlzManager::new();
    
    // Add default repositories
    if let Err(e) = manager.add_repository("core") {
        eprintln!("{}: Failed to load core repository: {}", "Warning".yellow().bold(), e);
    }
    if let Err(e) = manager.add_repository("extra") {
        eprintln!("{}: Failed to load extra repository: {}", "Warning".yellow().bold(), e);
    }
    if let Err(e) = manager.add_repository("community") {
        eprintln!("{}: Failed to load community repository: {}", "Warning".yellow().bold(), e);
    }

    match matches.subcommand() {
        Some(("list", _)) => {
            let packages = manager.list_all_packages();
            list_packages(&packages);
        }
        Some(("show", sub_matches)) => {
            let package_name = sub_matches.get_one::<String>("package").unwrap();
            show_package_info(&manager, package_name);
        }
        Some(("install", sub_matches)) => {
            assert_root();
            let package_name = sub_matches.get_one::<String>("package").unwrap();
            
            println!("{} {}: {}", "plz".green().bold(), "installing".blue(), package_name.bold());
            
            match manager.install_package(package_name) {
                Ok(()) => {
                    println!("{}: Package '{}' installed successfully!", "Success".green().bold(), package_name);
                }
                Err(e) => {
                    eprintln!("{}: Failed to install '{}': {}", "Error".red().bold(), package_name, e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            println!("{}", "Please specify a command. Use --help for usage information.".yellow());
        }
    }
}