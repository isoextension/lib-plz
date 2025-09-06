use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub maintainer: String,
    pub homepage: Option<String>,
    pub depends: Vec<String>,
    pub optdepends: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
    pub architecture: String,
    pub download_url: Option<String>,
    pub package_file: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    pub path: PathBuf,
    packages: HashMap<String, Package>,
}

impl Repository {
    pub fn new(name: &str) -> Self {
        let path = PathBuf::from("/var/lib/plz/repos").join(format!("{}.db", name));
        Repository {
            name: name.to_string(),
            path,
            packages: HashMap::new(),
        }
    }

    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.path.exists() {
            let content = fs::read_to_string(&self.path)?;
            self.packages = serde_yaml::from_str(&content)?;
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(self.path.parent().unwrap())?;
        let content = serde_yaml::to_string(&self.packages)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.packages.get(name)
    }

    pub fn add_package(&mut self, package: Package) {
        self.packages.insert(package.name.clone(), package);
    }

    pub fn list_packages(&self) -> Vec<&Package> {
        self.packages.values().collect()
    }
}

#[derive(Debug)]
pub struct PlzManager {
    repositories: Vec<Repository>,
    installed_packages_path: PathBuf,
}

impl PlzManager {
    pub fn new() -> Self {
        PlzManager {
            repositories: Vec::new(),
            installed_packages_path: PathBuf::from("/var/lib/plz/pkg/info"),
        }
    }

    pub fn add_repository(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut repo = Repository::new(name);
        repo.load()?;
        self.repositories.push(repo);
        Ok(())
    }

    pub fn find_package(&self, name: &str) -> Option<&Package> {
        for repo in &self.repositories {
            if let Some(package) = repo.get_package(name) {
                return Some(package);
            }
        }
        None
    }

    pub fn list_all_packages(&self) -> Vec<&Package> {
        let mut packages = Vec::new();
        for repo in &self.repositories {
            packages.extend(repo.list_packages());
        }
        packages
    }

    pub fn get_installed_packages(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut installed = Vec::new();
        
        if !self.installed_packages_path.exists() {
            return Ok(installed);
        }

        for entry in fs::read_dir(&self.installed_packages_path)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("yaml") {
                let content = fs::read_to_string(entry.path())?;
                let mut package: Package = serde_yaml::from_str(&content)?;
                package.installed = true;
                installed.push(package);
            }
        }
        
        Ok(installed)
    }

    pub fn install_package(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(package) = self.find_package(name) {
            println!("Installing package: {}", package.name);
            
            // Create directories
            let pkg_root = PathBuf::from("/var/lib/plz/pkg");
            let cache_dir = PathBuf::from("/var/cache/plz");
            fs::create_dir_all(&pkg_root)?;
            fs::create_dir_all(&cache_dir)?;
            fs::create_dir_all(&self.installed_packages_path)?;

            // Download package if URL provided
            if let Some(url) = &package.download_url {
                let package_file = cache_dir.join(format!("{}-{}.tar.xz", package.name, package.version));
                
                println!("Downloading from: {}", url);
                let status = Command::new("wget")
                    .args(&["-O", package_file.to_str().unwrap(), url])
                    .status()?;
                
                if !status.success() {
                    return Err("Failed to download package".into());
                }

                // Extract package
                println!("Extracting package...");
                let status = Command::new("tar")
                    .args(&["-xf", package_file.to_str().unwrap(), "-C", pkg_root.to_str().unwrap()])
                    .status()?;
                
                if !status.success() {
                    return Err("Failed to extract package".into());
                }
            } else {
                // If no download URL, try to get from system pacman cache or build from source
                println!("No download URL provided, attempting to get from system...");
                
                // Try copying from pacman cache first
                let mut found_package = false;
                
                // Look for package in pacman cache
                if let Ok(output) = Command::new("find")
                    .args(&["/var/cache/pacman/pkg", "-name", &format!("{}-{}*.pkg.tar.*", package.name, package.version)])
                    .output() 
                {
                    let files = String::from_utf8_lossy(&output.stdout);
                    if let Some(package_file) = files.lines().next() {
                        println!("Found package in pacman cache: {}", package_file);
                        
                        // Extract from pacman package
                        let status = Command::new("tar")
                            .args(&["-xf", package_file, "-C", pkg_root.to_str().unwrap(), "--exclude=.PKGINFO", "--exclude=.BUILDINFO", "--exclude=.MTREE"])
                            .status()?;
                        
                        if status.success() {
                            found_package = true;
                        }
                    }
                }
                
                if !found_package {
                    return Err(format!("Could not find or download package: {}", package.name).into());
                }
            }
            
            // Save package metadata to installed packages
            let package_info_path = self.installed_packages_path.join(format!("{}.yaml", name));
            let mut installed_package = package.clone();
            installed_package.installed = true;
            let content = serde_yaml::to_string(&installed_package)?;
            fs::write(package_info_path, content)?;
            
            println!("Package '{}' installed successfully!", name);
            println!("Files installed to: /var/lib/plz/pkg/");
            Ok(())
        } else {
            Err(format!("Package '{}' not found in any repository", name).into())
        }
    }
}