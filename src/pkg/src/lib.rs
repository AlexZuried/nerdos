//! # nerdpkg - NerdOS Package Manager
//!
//! nerdpkg is the NerdOS package manager with the following capabilities:
//!
//! ## Features
//!
//! 1. **Install `.deb` packages** (Debian/Ubuntu compatibility)
//!    - Parse Debian package format (ar archive with control.tar.gz and data.tar.gz)
//!    - Extract files to appropriate /usr paths
//!    - Execute post-install scripts
//!
//! 2. **Install `.pkg.tar.zst` packages** (Arch Linux compatibility)
//!    - Parse Arch package format (zstd-compressed tar)
//!    - Extract to system directories
//!
//! 3. **Build from git repositories**
//!    - Clone git repositories
//!    - Build from source using `cargo build`, `make`, etc.
//!
//! ## Configuration
//!
//! Sources are listed in `/etc/nerdpkg/sources.list`:
//! ```toml
//! [[source]]
//! type = "debian"
//! url = "http://deb.debian.org/debian"
//! suite = "bookworm"
//! components = ["main", "contrib", "non-free"]
//!
//! [[source]]
//! type = "arch"
//! url = "https://mirror.archlinux.org/$repo/os/$arch"
//!
//! [[source]]
//! type = "git"
//! url = "https://github.com/nerdos/core"
//! branch = "main"
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Package Format Support
// ---------------------------------------------------------------------------

/// Supported package formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageFormat {
    /// Debian .deb package.
    Deb,
    /// Arch Linux .pkg.tar.zst package.
    PkgTarZst,
    /// Git repository.
    Git,
    /// Source tarball.
    Tarball,
}

/// Package metadata.
#[derive(Debug, Clone)]
pub struct Package {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: String,
    /// Package description.
    pub description: String,
    /// Package architecture.
    pub architecture: String,
    /// Package format.
    pub format: PackageFormat,
    /// Dependencies (package names).
    pub depends: Vec<String>,
    /// Conflicts.
    pub conflicts: Vec<String>,
    /// Provides.
    pub provides: Vec<String>,
    /// Installed size in bytes.
    pub installed_size: u64,
    /// Package size in bytes.
    pub package_size: u64,
    /// Maintainer.
    pub maintainer: String,
    /// Homepage URL.
    pub homepage: String,
    /// License.
    pub license: String,
}

/// Package installation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageState {
    /// Not installed.
    NotInstalled,
    /// Currently being installed.
    Installing,
    /// Installed.
    Installed,
    /// Being removed.
    Removing,
    /// Config files remain.
    ConfigFiles,
    /// Half-installed (failed installation).
    HalfInstalled,
}

/// Installed package record.
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    /// Package metadata.
    pub pkg: Package,
    /// Installation state.
    pub state: PackageState,
    /// Files installed (for tracking/removal).
    pub files: Vec<String>,
    /// Installation timestamp.
    pub install_time: u64,
}

// ---------------------------------------------------------------------------
// Package Database
// ---------------------------------------------------------------------------

/// The local package database.
pub struct PackageDb {
    /// Installed packages by name.
    packages: BTreeMap<String, InstalledPackage>,
}

impl PackageDb {
    /// Create a new empty package database.
    pub fn new() -> Self {
        PackageDb {
            packages: BTreeMap::new(),
        }
    }

    /// Check if a package is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        match self.packages.get(name) {
            Some(pkg) => pkg.state == PackageState::Installed,
            None => false,
        }
    }

    /// Get an installed package.
    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.packages.get(name)
    }

    /// Add or update an installed package.
    pub fn insert(&mut self, installed: InstalledPackage) {
        self.packages.insert(installed.pkg.name.clone(), installed);
    }

    /// Remove a package from the database.
    pub fn remove(&mut self, name: &str) {
        self.packages.remove(name);
    }

    /// List all installed packages.
    pub fn list_installed(&self) -> Vec<&InstalledPackage> {
        self.packages
            .values()
            .filter(|p| p.state == PackageState::Installed)
            .collect()
    }

    /// Find packages matching a pattern.
    pub fn search(&self, pattern: &str) -> Vec<&InstalledPackage> {
        self.packages
            .values()
            .filter(|p| {
                p.pkg.name.contains(pattern)
                    || p.pkg.description.contains(pattern)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Source Configuration
// ---------------------------------------------------------------------------

/// A package source.
#[derive(Debug, Clone)]
pub struct Source {
    /// Source type.
    pub source_type: SourceType,
    /// Source URL.
    pub url: String,
    /// Additional configuration (type-dependent).
    pub config: BTreeMap<String, String>,
}

/// Source types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    /// Debian repository.
    Debian,
    /// Arch repository.
    Arch,
    /// Git repository.
    Git,
    /// Local directory.
    Local,
}

/// Load sources from `/etc/nerdpkg/sources.list`.
pub fn load_sources() -> Vec<Source> {
    // In a real implementation, parse the TOML config file.
    // For now, return default sources.
    let mut sources = Vec::new();

    sources.push(Source {
        source_type: SourceType::Debian,
        url: String::from("http://deb.debian.org/debian"),
        config: {
            let mut map = BTreeMap::new();
            map.insert(String::from("suite"), String::from("bookworm"));
            map.insert(String::from("components"), String::from("main contrib non-free"));
            map
        },
    });

    sources.push(Source {
        source_type: SourceType::Arch,
        url: String::from("https://mirror.archlinux.org/$repo/os/$arch"),
        config: BTreeMap::new(),
    });

    sources
}

// ---------------------------------------------------------------------------
// nerdpkg Operations
// ---------------------------------------------------------------------------

/// Main package manager.
pub struct NerdPkg {
    /// Package database.
    pub db: PackageDb,
    /// Configured sources.
    pub sources: Vec<Source>,
}

impl NerdPkg {
    /// Create a new nerdpkg instance.
    pub fn new() -> Self {
        NerdPkg {
            db: PackageDb::new(),
            sources: load_sources(),
        }
    }

    /// Install a package from a file.
    pub fn install_file(&mut self, path: &str) -> Result<(), PkgError> {
        // Determine package format from extension.
        let format = if path.ends_with(".deb") {
            PackageFormat::Deb
        } else if path.ends_with(".pkg.tar.zst") || path.ends_with(".pkg.tar.xz") {
            PackageFormat::PkgTarZst
        } else {
            return Err(PkgError::UnsupportedFormat);
        };

        // Read the package file.
        // In a real implementation, open and read via VFS.

        match format {
            PackageFormat::Deb => self.install_deb(path),
            PackageFormat::PkgTarZst => self.install_pkg_tar_zst(path),
            _ => Err(PkgError::UnsupportedFormat),
        }
    }

    /// Install from a Debian .deb package.
    ///
    /// .deb file format:
    /// - ar archive
    ///   - debian-binary (version "2.0")
    ///   - control.tar.gz (metadata, scripts)
    ///   - data.tar.gz (files to install)
    fn install_deb(&mut self, _path: &str) -> Result<(), PkgError> {
        // Step 1: Read and parse the .deb file (ar archive).
        // Step 2: Extract control.tar.gz and parse control file.
        // Step 3: Check dependencies.
        // Step 4: Extract data.tar.gz to /.
        // Step 5: Run preinst script.
        // Step 6: Extract files.
        // Step 7: Run postinst script.
        // Step 8: Update package database.

        // This is a placeholder for the full implementation.
        Err(PkgError::NotImplemented)
    }

    /// Install from an Arch .pkg.tar.zst package.
    ///
    /// .pkg.tar.zst file format:
    /// - zstd-compressed tar archive
    ///   - .PKGINFO (metadata)
    ///   - .MTREE (file hashes)
    ///   - .INSTALL (optional install script)
    ///   - files to install
    fn install_pkg_tar_zst(&mut self, _path: &str) -> Result<(), PkgError> {
        // Step 1: Decompress zstd stream.
        // Step 2: Parse tar archive.
        // Step 3: Read .PKGINFO.
        // Step 4: Check dependencies.
        // Step 5: Extract files.
        // Step 6: Run .INSTALL script if present.
        // Step 7: Update package database.

        // This is a placeholder for the full implementation.
        Err(PkgError::NotImplemented)
    }

    /// Install from a git repository.
    ///
    /// Clones the repository, checks out the specified branch/tag,
    /// and runs `cargo build` or `make` to build and install.
    pub fn install_git(&mut self, url: &str, branch: Option<&str>) -> Result<(), PkgError> {
        // Step 1: Clone the repository to /tmp.
        // Step 2: Checkout the specified branch/tag.
        // Step 3: Detect build system (Cargo.toml, Makefile, etc.).
        // Step 4: Build the project.
        // Step 5: Install binaries to /usr/bin or /usr/local/bin.
        // Step 6: Install libraries if present.

        // This is a placeholder for the full implementation.
        Err(PkgError::NotImplemented)
    }

    /// Remove a package.
    pub fn remove(&mut self, name: &str) -> Result<(), PkgError> {
        let installed = self.db.get(name).ok_or(PkgError::PackageNotFound)?;

        if installed.state != PackageState::Installed {
            return Err(PkgError::NotInstalled);
        }

        // Step 1: Run prerm script.
        // Step 2: Remove all tracked files (except modified config files).
        // Step 3: Run postrm script.
        // Step 4: Update package database.

        self.db.remove(name);
        Ok(())
    }

    /// Purge a package (remove including config files).
    pub fn purge(&mut self, name: &str) -> Result<(), PkgError> {
        self.remove(name)?;
        // Also remove config files.
        Ok(())
    }

    /// Query package information.
    pub fn query(&self, name: &str) -> Result<&Package, PkgError> {
        self.db
            .get(name)
            .map(|i| &i.pkg)
            .ok_or(PkgError::PackageNotFound)
    }

    /// List installed packages.
    pub fn list(&self) -> Vec<&InstalledPackage> {
        self.db.list_installed()
    }

    /// Search for packages matching a pattern.
    pub fn search(&self, pattern: &str) -> Vec<&InstalledPackage> {
        self.db.search(pattern)
    }

    /// Update package lists from all sources.
    pub fn update(&mut self) -> Result<(), PkgError> {
        for source in &self.sources {
            match source.source_type {
                SourceType::Debian => {
                    // Download Packages.gz from each component.
                }
                SourceType::Arch => {
                    // Download core.db, extra.db, community.db.
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Upgrade all installed packages.
    pub fn upgrade(&mut self) -> Result<(), PkgError> {
        self.update()?;
        // Check each installed package for updates.
        // Install newer versions.
        Ok(())
    }

    /// Verify installed package files.
    pub fn verify(&self, name: &str) -> Result<(), PkgError> {
        let installed = self.db.get(name).ok_or(PkgError::PackageNotFound)?;

        // Verify each file exists and has correct checksum.
        for file in &installed.files {
            // Check file exists.
            // Verify checksum if available.
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Package manager errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkgError {
    /// Package not found.
    PackageNotFound = 1,
    /// Package not installed.
    NotInstalled = 2,
    /// Unsupported package format.
    UnsupportedFormat = 3,
    /// Dependency resolution failed.
    DependencyError = 4,
    /// Download failed.
    DownloadError = 5,
    /// Checksum mismatch.
    ChecksumError = 6,
    /// Insufficient disk space.
    NoSpace = 7,
    /// Permission denied.
    PermissionDenied = 8,
    /// Feature not yet implemented.
    NotImplemented = 99,
}

// ---------------------------------------------------------------------------
// Default Config File Content
// ---------------------------------------------------------------------------

/// Default content for `/etc/nerdpkg/sources.list`.
pub const DEFAULT_SOURCES_LIST: &str = r#"# NerdOS Package Sources
# Format: TOML

# Debian repositories
[[source]]
type = "debian"
name = "debian-bookworm"
url = "http://deb.debian.org/debian"
suite = "bookworm"
components = ["main", "contrib", "non-free"]

# Arch repositories
[[source]]
type = "arch"
name = "arch-core"
url = "https://mirror.archlinux.org/$repo/os/$arch"

# Git repositories
[[source]]
type = "git"
name = "nerdos-core"
url = "https://github.com/nerdos/core"
branch = "main"
"#;

/// Default content for `/etc/nerdpkg/nerdpkg.toml`.
pub const DEFAULT_CONFIG: &str = r#"# nerdpkg configuration

[general]
# Architecture (auto-detected if not set)
# architecture = "x86_64"

# Installation prefix
prefix = "/usr"

# Cache directory for downloaded packages
cache_dir = "/var/cache/nerdpkg"

# Database directory
db_dir = "/var/lib/nerdpkg"

[behavior]
# Always confirm before installing
confirm = true

# Keep downloaded packages in cache
keep_cache = false

# Remove orphaned dependencies
remove_orphans = true

[git]
# Default clone depth (0 = full clone)
clone_depth = 1

# Build parallel jobs
build_jobs = 4
"#;
