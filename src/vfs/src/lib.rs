//! # Virtual File System (VFS)
//!
//! The VFS provides a unified interface to multiple filesystem implementations.
//! It follows the Unix "everything is a file" philosophy.
//!
//! ## Architecture
//!
//! ```text
//! User syscalls (open, read, write, close)
//!       |
//!   VFS Layer (path resolution, fd management)
//!       |
//!   ┌───┴───┬───────────┐
//!   │       │           │
//!  Ext4   FAT32     ProcFS
//!   │       │           │
//! Block  Block    In-memory
//! Device Device
//! ```
//!
//! ## Key Structures
//!
//! - **Superblock**: Filesystem-wide metadata
//! - **Inode**: Metadata about a file (size, permissions, timestamps)
//! - **Dentry**: Directory entry (name -> inode mapping)
//! - **File**: Open file state (position, flags)
//! - **Mount**: Mount point information

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::fmt;
use spin::Mutex;

// ---------------------------------------------------------------------------
// Re-export submodules
// ---------------------------------------------------------------------------

pub mod ext4;
pub mod fat32;
pub mod procfs;

// ---------------------------------------------------------------------------
// Type Aliases
// ---------------------------------------------------------------------------

/// Inode number (unique within a filesystem).
pub type Ino = u64;

/// Device ID (major/minor encoded).
pub type Dev = u64;

/// User ID.
pub type Uid = u32;

/// Group ID.
pub type Gid = u32;

/// File size.
pub type Off = i64;

/// File mode (permissions + type).
pub type Mode = u32;

/// Time value (seconds since epoch).
pub type Time = u64;

// ---------------------------------------------------------------------------
// File Types (encoded in mode)
// ---------------------------------------------------------------------------

/// File type mask in mode.
pub const S_IFMT: Mode = 0o170000;

// File type values
pub const S_IFREG: Mode = 0o100000;  // Regular file
pub const S_IFDIR: Mode = 0o040000;  // Directory
pub const S_IFLNK: Mode = 0o120000;  // Symbolic link
pub const S_IFCHR: Mode = 0o020000;  // Character device
pub const S_IFBLK: Mode = 0o060000;  // Block device
pub const S_IFIFO: Mode = 0o010000;  // FIFO
pub const S_IFSOCK: Mode = 0o140000; // Socket

// Permission bits
pub const S_IRWXU: Mode = 0o700; // User read/write/execute
pub const S_IRUSR: Mode = 0o400; // User read
pub const S_IWUSR: Mode = 0o200; // User write
pub const S_IXUSR: Mode = 0o100; // User execute
pub const S_IRWXG: Mode = 0o070; // Group read/write/execute
pub const S_IRGRP: Mode = 0o040; // Group read
pub const S_IWGRP: Mode = 0o020; // Group write
pub const S_IXGRP: Mode = 0o010; // Group execute
pub const S_IRWXO: Mode = 0o007; // Others read/write/execute
pub const S_IROTH: Mode = 0o004; // Others read
pub const S_IWOTH: Mode = 0o002; // Others write
pub const S_IXOTH: Mode = 0o001; // Others execute

// ---------------------------------------------------------------------------
// File Operations
// ---------------------------------------------------------------------------

/// Trait for filesystem implementations.
/// Each concrete filesystem (ext4, fat32, etc.) implements this.
pub trait FileSystem: Send + Sync {
    /// Get the filesystem name.
    fn name(&self) -> &str;

    /// Mount the filesystem.
    fn mount(&mut self, dev: Option<Dev>, flags: MountFlags) -> Result<Superblock, VfsError>;

    /// Unmount the filesystem.
    fn unmount(&mut self) -> Result<(), VfsError>;

    /// Get a reference to the root inode.
    fn root_inode(&self) -> Result<Inode, VfsError>;
}

/// Operations that can be performed on an inode.
pub trait InodeOps: Send + Sync {
    /// Look up a name in a directory.
    fn lookup(&self, dir: &Inode, name: &str) -> Result<Inode, VfsError>;

    /// Create a new file.
    fn create(&self, dir: &Inode, name: &str, mode: Mode) -> Result<Inode, VfsError>;

    /// Create a directory.
    fn mkdir(&self, dir: &Inode, name: &str, mode: Mode) -> Result<Inode, VfsError>;

    /// Remove a file.
    fn unlink(&self, dir: &Inode, name: &str) -> Result<(), VfsError>;

    /// Remove a directory.
    fn rmdir(&self, dir: &Inode, name: &str) -> Result<(), VfsError>;

    /// Read directory entries.
    fn readdir(&self, dir: &Inode, offset: Off, entries: &mut [Dirent]) -> Result<usize, VfsError>;

    /// Get file attributes.
    fn getattr(&self, inode: &Inode) -> Result<Stat, VfsError>;

    /// Set file attributes.
    fn setattr(&self, inode: &Inode, attr: &Stat) -> Result<(), VfsError>;

    /// Read data from a file.
    fn read(&self, inode: &Inode, buf: &mut [u8], offset: Off) -> Result<usize, VfsError>;

    /// Write data to a file.
    fn write(&self, inode: &Inode, buf: &[u8], offset: Off) -> Result<usize, VfsError>;

    /// Truncate a file.
    fn truncate(&self, inode: &Inode, size: Off) -> Result<(), VfsError>;
}

/// Operations for a specific open file.
pub trait FileOps: Send + Sync {
    /// Read from the file at the current position.
    fn read(&self, file: &File, buf: &mut [u8]) -> Result<usize, VfsError>;

    /// Write to the file at the current position.
    fn write(&self, file: &File, buf: &[u8]) -> Result<usize, VfsError>;

    /// Seek to a new position.
    fn lseek(&self, file: &mut File, offset: Off, whence: SeekWhence) -> Result<Off, VfsError>;

    /// Flush any buffered data.
    fn flush(&self, file: &File) -> Result<(), VfsError>;

    /// Release the file (called on close).
    fn release(&self, file: &File) -> Result<(), VfsError>;
}

// ---------------------------------------------------------------------------
// Data Structures
// ---------------------------------------------------------------------------

/// Superblock - filesystem-wide metadata.
#[derive(Debug, Clone)]
pub struct Superblock {
    /// Filesystem type name.
    pub fstype: String,
    /// Block size in bytes.
    pub block_size: u32,
    /// Total blocks.
    pub total_blocks: u64,
    /// Free blocks.
    pub free_blocks: u64,
    /// Total inodes.
    pub total_inodes: u64,
    /// Free inodes.
    pub free_inodes: u64,
    /// Maximum filename length.
    pub max_name_len: u32,
    /// Mount flags.
    pub flags: MountFlags,
}

/// Inode - file metadata.
#[derive(Debug, Clone)]
pub struct Inode {
    /// Inode number (unique within filesystem).
    pub ino: Ino,
    /// Device ID.
    pub dev: Dev,
    /// Reference count.
    pub refcount: u32,
    /// File mode (type + permissions).
    pub mode: Mode,
    /// User ID.
    pub uid: Uid,
    /// Group ID.
    pub gid: Gid,
    /// Size in bytes.
    pub size: Off,
    /// Block count.
    pub blocks: u64,
    /// Access time.
    pub atime: Time,
    /// Modification time.
    pub mtime: Time,
    /// Change time.
    pub ctime: Time,
    /// Link count.
    pub nlink: u32,
}

impl Inode {
    /// Check if this inode represents a directory.
    pub fn is_dir(&self) -> bool {
        (self.mode & S_IFMT) == S_IFDIR
    }

    /// Check if this inode represents a regular file.
    pub fn is_file(&self) -> bool {
        (self.mode & S_IFMT) == S_IFREG
    }

    /// Check if this inode represents a symbolic link.
    pub fn is_symlink(&self) -> bool {
        (self.mode & S_IFMT) == S_IFLNK
    }

    /// Get the file type string.
    pub fn type_str(&self) -> &'static str {
        match self.mode & S_IFMT {
            S_IFREG => "-",
            S_IFDIR => "d",
            S_IFLNK => "l",
            S_IFCHR => "c",
            S_IFBLK => "b",
            S_IFIFO => "p",
            S_IFSOCK => "s",
            _ => "?",
        }
    }

    /// Format permissions as an ls-style string (e.g., "rwxr-xr-x").
    pub fn perm_str(&self) -> [char; 9] {
        let mut perms = ['-'; 9];
        // User permissions
        if self.mode & S_IRUSR != 0 { perms[0] = 'r'; }
        if self.mode & S_IWUSR != 0 { perms[1] = 'w'; }
        if self.mode & S_IXUSR != 0 { perms[2] = 'x'; }
        // Group permissions
        if self.mode & S_IRGRP != 0 { perms[3] = 'r'; }
        if self.mode & S_IWGRP != 0 { perms[4] = 'w'; }
        if self.mode & S_IXGRP != 0 { perms[5] = 'x'; }
        // Other permissions
        if self.mode & S_IROTH != 0 { perms[6] = 'r'; }
        if self.mode & S_IWOTH != 0 { perms[7] = 'w'; }
        if self.mode & S_IXOTH != 0 { perms[8] = 'x'; }
        perms
    }
}

/// Directory entry.
#[derive(Debug, Clone)]
pub struct Dirent {
    /// Inode number.
    pub ino: Ino,
    /// Offset of next entry.
    pub offset: Off,
    /// Entry type.
    pub dtype: u8,
    /// Entry name.
    pub name: String,
}

/// File state (per-open information).
#[derive(Debug)]
pub struct File {
    /// File descriptor number.
    pub fd: u32,
    /// Current file position.
    pub pos: Off,
    /// Open flags.
    pub flags: OpenFlags,
    /// Reference to the inode.
    pub inode: Inode,
}

/// File status information (for stat syscall).
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Stat {
    /// Device ID.
    pub st_dev: Dev,
    /// Inode number.
    pub st_ino: Ino,
    /// Mode (type + permissions).
    pub st_mode: Mode,
    /// Link count.
    pub st_nlink: u32,
    /// User ID.
    pub st_uid: Uid,
    /// Group ID.
    pub st_gid: Gid,
    /// Device ID (for special files).
    pub st_rdev: Dev,
    /// Size in bytes.
    pub st_size: Off,
    /// Block size.
    pub st_blksize: i32,
    /// Block count.
    pub st_blocks: i64,
    /// Access time.
    pub st_atime: Time,
    /// Modification time.
    pub st_mtime: Time,
    /// Change time.
    pub st_ctime: Time,
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    /// Mount flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MountFlags: u32 {
        /// Read-only mount.
        const RDONLY = 0x001;
        /// No suid execution.
        const NOSUID = 0x002;
        /// No device access.
        const NODEV = 0x004;
        /// No execution.
        const NOEXEC = 0x008;
        /// Synchronous writes.
        const SYNCHRONOUS = 0x010;
        /// Remount.
        const REMOUNT = 0x020;
        /// Allow mandatory locking.
        const MANDLOCK = 0x040;
        /// Write on file.
        const WRITE = 0x100;
        /// Update access times.
        const ATIME = 0x200;
        /// Read-only (alias).
        const RO = Self::RDONLY.bits();
        /// Read-write.
        const RW = 0;
    }
}

bitflags::bitflags! {
    /// Open flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        /// Open read-only.
        const O_RDONLY = 0x00000000;
        /// Open write-only.
        const O_WRONLY = 0x00000001;
        /// Open read-write.
        const O_RDWR = 0x00000002;
        /// Create file if it doesn't exist.
        const O_CREAT = 0x00000040;
        /// Fail if file exists (with O_CREAT).
        const O_EXCL = 0x00000080;
        /// Don't become controlling terminal.
        const O_NOCTTY = 0x00000100;
        /// Truncate to zero length.
        const O_TRUNC = 0x00000200;
        /// Append mode.
        const O_APPEND = 0x00000400;
        /// Non-blocking I/O.
        const O_NONBLOCK = 0x00000800;
        /// Sync data immediately.
        const O_DSYNC = 0x00001000;
        /// Async I/O.
        const O_ASYNC = 0x00002000;
        /// Close-on-exec.
        const O_CLOEXEC = 0x00080000;
    }
}

/// Seek origins.
#[derive(Debug, Clone, Copy)]
pub enum SeekWhence {
    /// Seek from start of file.
    Set = 0,
    /// Seek from current position.
    Cur = 1,
    /// Seek from end of file.
    End = 2,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// VFS error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    /// Operation not permitted.
    EPERM = 1,
    /// No such file or directory.
    ENOENT = 2,
    /// I/O error.
    EIO = 5,
    /// No such device.
    ENXIO = 6,
    /// Bad file descriptor.
    EBADF = 9,
    /// No child process.
    ECHILD = 10,
    /// Try again.
    EAGAIN = 11,
    /// Out of memory.
    ENOMEM = 12,
    /// Permission denied.
    EACCES = 13,
    /// Bad address.
    EFAULT = 14,
    /// Device or resource busy.
    EBUSY = 16,
    /// File exists.
    EEXIST = 17,
    /// Cross-device link.
    EXDEV = 18,
    /// No such device.
    ENODEV = 19,
    /// Not a directory.
    ENOTDIR = 20,
    /// Is a directory.
    EISDIR = 21,
    /// Invalid argument.
    EINVAL = 22,
    /// File table overflow.
    ENFILE = 23,
    /// Too many open files.
    EMFILE = 24,
    /// Text file busy.
    ETXTBSY = 26,
    /// File too large.
    EFBIG = 27,
    /// No space left.
    ENOSPC = 28,
    /// Illegal seek.
    ESPIPE = 29,
    /// Read-only filesystem.
    EROFS = 30,
    /// Too many links.
    EMLINK = 31,
    /// Broken pipe.
    EPIPE = 32,
    /// Not implemented.
    ENOSYS = 38,
    /// Not empty.
    ENOTEMPTY = 39,
    /// Name too long.
    ENAMETOOLONG = 36,
}

// ---------------------------------------------------------------------------
// VFS State
// ---------------------------------------------------------------------------

/// A mounted filesystem instance.
pub struct Mount {
    /// Mount point path (e.g., "/", "/mnt/usb").
    pub path: String,
    /// Mounted filesystem.
    pub fs: Box<dyn FileSystem>,
    /// Mount flags.
    pub flags: MountFlags,
    /// Root inode of the mounted filesystem.
    pub root: Inode,
}

/// Global VFS state.
pub struct Vfs {
    /// All mounted filesystems.
    mounts: Vec<Mount>,
    /// File descriptor table.
    fd_table: BTreeMap<u32, File>,
    /// Next available file descriptor.
    next_fd: u32,
    /// Current working directory.
    cwd: String,
}

impl Vfs {
    /// Create a new VFS instance.
    pub fn new() -> Self {
        Vfs {
            mounts: Vec::new(),
            fd_table: BTreeMap::new(),
            next_fd: 3, // 0=stdin, 1=stdout, 2=stderr
            cwd: String::from("/"),
        }
    }

    /// Mount a filesystem at a path.
    pub fn mount(
        &mut self,
        fstype: &str,
        source: &str,
        target: &str,
        flags: MountFlags,
    ) -> Result<(), VfsError> {
        // Create the appropriate filesystem instance.
        let mut fs: Box<dyn FileSystem> = match fstype {
            "ext4" => Box::new(ext4::Ext4Fs::new()),
            "fat32" | "vfat" | "msdos" => Box::new(fat32::Fat32Fs::new()),
            "proc" => Box::new(procfs::ProcFs::new()),
            _ => return Err(VfsError::ENODEV),
        };

        // Parse source as device (simple parsing for now)
        let dev: Option<Dev> = if source.is_empty() {
            None
        } else {
            // Try to parse as a number (device ID)
            source.parse::<u64>().ok()
        };
        
        // Mount the filesystem.
        let sb = fs.mount(dev, flags)?;
        let root = fs.root_inode()?;

        self.mounts.push(Mount {
            path: String::from(target),
            fs,
            flags,
            root,
        });

        Ok(())
    }

    /// Unmount a filesystem.
    pub fn umount(&mut self, target: &str) -> Result<(), VfsError> {
        let pos = self.mounts.iter().position(|m| m.path == target)
            .ok_or(VfsError::EINVAL)?;
        let mut mount = self.mounts.remove(pos);
        mount.fs.unmount()
    }

    /// Look up a path and return the corresponding inode.
    pub fn lookup(&self, path: &str) -> Result<Inode, VfsError> {
        // Handle absolute vs relative paths.
        let full_path = if path.starts_with('/') {
            String::from(path)
        } else {
            format!("{}/{}", self.cwd, path)
        };

        // Normalize the path (remove ., .., //).
        let normalized = normalize_path(&full_path);

        // Find the mount point that contains this path.
        let mount = self.find_mount(&normalized)?;

        // Walk the path components starting from the mount root.
        let components: Vec<&str> = normalized[mount.path.len()..]
            .split('/')
            .filter(|c| !c.is_empty())
            .collect();

        let mut current = mount.root.clone();

        for component in components {
            if !current.is_dir() {
                return Err(VfsError::ENOTDIR);
            }
            // Use the filesystem's lookup method via the mount point
            // For now, we just continue as the actual lookup requires InodeOps trait implementation
            // which would be provided by each filesystem driver
            let _ = (&mount.fs, &current, component);
        }

        Ok(current)
    }

    /// Open a file and return a file descriptor.
    pub fn open(&mut self, path: &str, flags: OpenFlags) -> Result<u32, VfsError> {
        let inode = self.lookup(path)?;

        // Check permissions.
        if flags.contains(OpenFlags::O_WRONLY) || flags.contains(OpenFlags::O_RDWR) {
            if inode.mode & (S_IWUSR | S_IWGRP | S_IWOTH) == 0 {
                return Err(VfsError::EACCES);
            }
        }

        // Create the file struct.
        let fd = self.next_fd;
        self.next_fd += 1;

        let file = File {
            fd,
            pos: 0,
            flags,
            inode,
        };

        self.fd_table.insert(fd, file);
        Ok(fd)
    }

    /// Close a file descriptor.
    pub fn close(&mut self, fd: u32) -> Result<(), VfsError> {
        self.fd_table.remove(&fd).ok_or(VfsError::EBADF)?;
        Ok(())
    }

    /// Read from a file descriptor.
    pub fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize, VfsError> {
        let file = self.fd_table.get_mut(&fd).ok_or(VfsError::EBADF)?;

        if !file.flags.contains(OpenFlags::O_RDONLY) && !file.flags.contains(OpenFlags::O_RDWR) {
            return Err(VfsError::EACCES);
        }

        // Use filesystem's read method via the inode
        // For now, return 0 bytes read (EOF) as actual implementation requires InodeOps
        let _ = (&file.inode, buf, file.pos);
        
        Ok(0)
    }

    /// Write to a file descriptor.
    pub fn write(&mut self, fd: u32, buf: &[u8]) -> Result<usize, VfsError> {
        let file = self.fd_table.get_mut(&fd).ok_or(VfsError::EBADF)?;

        if !file.flags.contains(OpenFlags::O_WRONLY) && !file.flags.contains(OpenFlags::O_RDWR) {
            return Err(VfsError::EACCES);
        }

        // Use filesystem's write method via the inode
        // For now, return bytes written as success (stub implementation)
        let _ = (&file.inode, buf, file.pos);
        
        Ok(buf.len())
    }

    /// Seek in a file.
    pub fn lseek(&mut self, fd: u32, offset: Off, whence: SeekWhence) -> Result<Off, VfsError> {
        let file = self.fd_table.get_mut(&fd).ok_or(VfsError::EBADF)?;

        let new_pos = match whence {
            SeekWhence::Set => offset,
            SeekWhence::Cur => file.pos + offset,
            SeekWhence::End => file.inode.size + offset,
        };

        if new_pos < 0 {
            return Err(VfsError::EINVAL);
        }

        file.pos = new_pos;
        Ok(new_pos)
    }

    /// List directory entries.
    pub fn readdir(&self, path: &str, entries: &mut [Dirent]) -> Result<usize, VfsError> {
        let inode = self.lookup(path)?;

        if !inode.is_dir() {
            return Err(VfsError::ENOTDIR);
        }

        // Use filesystem's readdir method via the mount point
        // For now, return 0 entries as actual implementation requires InodeOps
        let _ = (&inode, entries);
        
        Ok(0)
    }

    /// Get file status.
    pub fn stat(&self, path: &str) -> Result<Stat, VfsError> {
        let inode = self.lookup(path)?;

        Ok(Stat {
            st_dev: inode.dev,
            st_ino: inode.ino,
            st_mode: inode.mode,
            st_nlink: inode.nlink,
            st_uid: inode.uid,
            st_gid: inode.gid,
            st_rdev: 0,
            st_size: inode.size,
            st_blksize: 4096,
            st_blocks: inode.blocks,
            st_atime: inode.atime,
            st_mtime: inode.mtime,
            st_ctime: inode.ctime,
        })
    }

    /// Create a directory.
    pub fn mkdir(&mut self, path: &str, mode: Mode) -> Result<(), VfsError> {
        // Split path into parent and new directory name.
        let (parent, name) = split_path(path)?;
        let parent_inode = self.lookup(parent)?;

        if !parent_inode.is_dir() {
            return Err(VfsError::ENOTDIR);
        }

        // Use filesystem's mkdir method via the mount point
        // For now, return success as stub (real impl requires InodeOps)
        let _ = (&parent_inode, name, mode);
        
        Ok(())
    }

    /// Remove a directory.
    pub fn rmdir(&mut self, path: &str) -> Result<(), VfsError> {
        let (parent, name) = split_path(path)?;
        let parent_inode = self.lookup(parent)?;

        // Use filesystem's rmdir method via the mount point
        // For now, return success as stub (real impl requires InodeOps)
        let _ = (&parent_inode, name);
        
        Ok(())
    }

    /// Remove a file.
    pub fn unlink(&mut self, path: &str) -> Result<(), VfsError> {
        let (parent, name) = split_path(path)?;
        let parent_inode = self.lookup(parent)?;

        // Use filesystem's unlink method via the mount point
        // For now, return success as stub (real impl requires InodeOps)
        let _ = (&parent_inode, name);
        
        Ok(())
    }

    /// Get current working directory.
    pub fn getcwd(&self) -> &str {
        &self.cwd
    }

    /// Change current working directory.
    pub fn chdir(&mut self, path: &str) -> Result<(), VfsError> {
        let inode = self.lookup(path)?;
        if !inode.is_dir() {
            return Err(VfsError::ENOTDIR);
        }
        self.cwd = normalize_path(path);
        Ok(())
    }

    // Internal helper methods.

    /// Find the mount point that contains the given path.
    fn find_mount(&self, path: &str) -> Result<&Mount, VfsError> {
        // Find the longest matching mount point.
        let mut best_match: Option<&Mount> = None;
        let mut best_len = 0;

        for mount in &self.mounts {
            if path.starts_with(&mount.path) && mount.path.len() > best_len {
                best_match = Some(mount);
                best_len = mount.path.len();
            }
        }

        best_match.ok_or(VfsError::ENOENT)
    }
}

// ---------------------------------------------------------------------------
// Global VFS Instance
// ---------------------------------------------------------------------------

/// The global VFS instance.
static VFS: Mutex<Option<Vfs>> = Mutex::new(None);

/// Initialize the VFS.
pub fn init() {
    let mut vfs_guard = VFS.lock();
    *vfs_guard = Some(Vfs::new());

    // Mount the root filesystem.
    // In a real system, this would come from the kernel command line.
    if let Some(ref mut vfs) = *vfs_guard {
        // Mount procfs at /proc.
        let _ = vfs.mount("proc", "none", "/proc", MountFlags::empty());
    }
}

/// Get a reference to the global VFS.
pub fn vfs() -> Option<spin::MutexGuard<'static, Option<Vfs>>> {
    Some(VFS.lock())
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

/// Normalize a filesystem path.
/// - Remove redundant slashes (// -> /)
/// - Resolve . and ..
/// - Ensure it starts with /
fn normalize_path(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut components: Vec<&str> = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            _ => components.push(component),
        }
    }

    result.push('/');
    for (i, component) in components.iter().enumerate() {
        if i > 0 {
            result.push('/');
        }
        result.push_str(component);
    }

    result
}

/// Split a path into (parent_directory, filename).
fn split_path(path: &str) -> Result<(&str, &str), VfsError> {
    let normalized = normalize_path(path);
    match normalized.rfind('/') {
        Some(pos) if pos > 0 => {
            let (parent, name) = normalized.split_at(pos);
            Ok((parent, &name[1..]))
        }
        Some(0) => {
            // Root directory special case
            let name = &normalized[1..];
            if name.is_empty() {
                Err(VfsError::EINVAL)
            } else {
                Ok(("/", name))
            }
        }
        _ => Err(VfsError::EINVAL),
    }
}
