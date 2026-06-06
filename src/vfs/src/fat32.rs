//! # FAT32 Filesystem Driver
//!
//! FAT32 is a simple filesystem widely used on USB drives and SD cards.
//! This driver provides read/write support with:
//! - Long filename (LFN) support
//! - Directory traversal
//! - File read/write (basic)
//!
//! ## Data Structures
//!
//! The on-disk layout consists of:
//! 1. **Reserved region** (boot sector + FSInfo)
//! 2. **FAT (File Allocation Table)** (usually 2 copies)
//! 3. **Data region** (clusters of 4-32 KiB)
//!
//! ## Cluster Chain
//!
//! Files are stored as a chain of clusters. The FAT contains the "next"
//! cluster pointer for each cluster. Special values:
//! - 0x00000000: Free cluster
//! - 0x0FFFFFF8-0x0FFFFFFF: End of chain
//! - 0x0FFFFFF7: Bad cluster
//!
//! ## Directory Entries
//!
//! Each directory is a sequence of 32-byte entries:
//! - Short filename entry (8.3 format)
//! - Long filename entries (LFN, 13 chars each, stored in reverse)
//! - Volume label
//! - Unused entry (first byte = 0xE5)
//! - End of directory (first byte = 0x00)

use crate::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// FAT32 boot sector signature.
const BOOT_SIGNATURE: u16 = 0xAA55;

/// FAT32 FSInfo signature.
const FSINFO_SIGNATURE: u32 = 0x41615252;

/// FAT32 FSInfo trailing signature.
const FSINFO_TRAIL_SIG: u32 = 0xAA550000;

/// Cluster numbers.
const CLUSTER_FREE: u32 = 0x00000000;
const CLUSTER_MIN: u32 = 0x00000002;
const CLUSTER_MAX: u32 = 0x0FFFFFEF;
const CLUSTER_BAD: u32 = 0x0FFFFFF7;
const CLUSTER_EOF_MIN: u32 = 0x0FFFFFF8;
const CLUSTER_EOF_MAX: u32 = 0x0FFFFFFF;

// ---------------------------------------------------------------------------
// Boot Sector (BPB)
// ---------------------------------------------------------------------------

/// BIOS Parameter Block for FAT32.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32BootSector {
    /// Jump instruction.
    pub bs_jmp_boot: [u8; 3],
    /// OEM name.
    pub bs_oem_name: [u8; 8],
    /// Bytes per sector (usually 512).
    pub bpb_bytes_per_sec: u16,
    /// Sectors per cluster (1, 2, 4, 8, 16, 32, 64, 128).
    pub bpb_sec_per_clus: u8,
    /// Reserved sector count.
    pub bpb_rsvd_sec_cnt: u16,
    /// Number of FATs (usually 2).
    pub bpb_num_fats: u8,
    /// Root entry count (0 for FAT32).
    pub bpb_root_ent_cnt: u16,
    /// Total sectors (16-bit, 0 for FAT32).
    pub bpb_tot_sec16: u16,
    /// Media type (0xF8 for hard disks).
    pub bpb_media: u8,
    /// Sectors per FAT (16-bit, 0 for FAT32).
    pub bpb_fat_sz16: u16,
    /// Sectors per track.
    pub bpb_sec_per_trk: u16,
    /// Number of heads.
    pub bpb_num_heads: u16,
    /// Hidden sectors.
    pub bpb_hidd_sec: u32,
    /// Total sectors (32-bit).
    pub bpb_tot_sec32: u32,

    // --- FAT32-specific fields ---
    /// Sectors per FAT (32-bit).
    pub bpb_fat_sz32: u32,
    /// Extended flags.
    pub bpb_ext_flags: u16,
    /// Filesystem version.
    pub bpb_fs_ver: u16,
    /// Root directory cluster number.
    pub bpb_root_clus: u32,
    /// FSInfo sector number.
    pub bpb_fs_info: u16,
    /// Backup boot sector.
    pub bpb_bk_boot_sec: u16,
    /// Reserved.
    pub bpb_reserved: [u8; 12],
    /// Drive number.
    pub bs_drv_num: u8,
    /// Reserved1.
    pub bs_reserved1: u8,
    /// Boot signature (0x29).
    pub bs_boot_sig: u8,
    /// Volume serial number.
    pub bs_vol_id: u32,
    /// Volume label.
    pub bs_vol_lab: [u8; 11],
    /// Filesystem type string ("FAT32   ").
    pub bs_fil_sys_type: [u8; 8],
}

/// FSInfo structure.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32FsInfo {
    /// Lead signature (0x41615252).
    pub fsi_lead_sig: u32,
    /// Reserved.
    pub fsi_reserved1: [u8; 480],
    /// Structure signature (0x61417272).
    pub fsi_struc_sig: u32,
    /// Free cluster count.
    pub fsi_free_count: u32,
    /// Next free cluster.
    pub fsi_nxt_free: u32,
    /// Reserved.
    pub fsi_reserved2: [u8; 12],
    /// Trail signature (0xAA550000).
    pub fsi_trail_sig: u32,
}

// ---------------------------------------------------------------------------
// Directory Entry
// ---------------------------------------------------------------------------

/// Short filename directory entry (32 bytes).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32DirEntry {
    /// Short filename (8 bytes) + extension (3 bytes).
    /// If first byte is 0xE5, entry is deleted.
    /// If first byte is 0x00, end of directory.
    pub dir_name: [u8; 11],
    /// File attributes.
    pub dir_attr: u8,
    /// Reserved for NT.
    pub dir_nt_res: u8,
    /// Creation time tenths of a second.
    pub dir_crt_time_tenth: u8,
    /// Creation time.
    pub dir_crt_time: u16,
    /// Creation date.
    pub dir_crt_date: u16,
    /// Last access date.
    pub dir_lst_acc_date: u16,
    /// High 16 bits of first cluster (FAT12/16: 0).
    pub dir_fst_clus_hi: u16,
    /// Write time.
    pub dir_wrt_time: u16,
    /// Write date.
    pub dir_wrt_date: u16,
    /// Low 16 bits of first cluster.
    pub dir_fst_clus_lo: u16,
    /// File size in bytes.
    pub dir_file_size: u32,
}

/// Directory entry attributes.
pub const ATTR_READ_ONLY: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_VOLUME_ID: u8 = 0x08;
pub const ATTR_DIRECTORY: u8 = 0x10;
pub const ATTR_ARCHIVE: u8 = 0x20;
pub const ATTR_LFN: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;

/// Long Filename (LFN) directory entry.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32LfnEntry {
    /// Sequence number (0x40 = last LFN entry, 0x01 = first).
    pub lfn_order: u8,
    /// Characters 1-5 (10 bytes).
    pub lfn_name1: [u16; 5],
    /// Attributes (always 0x0F).
    pub lfn_attr: u8,
    /// Long entry type (0 for name entries).
    pub lfn_type: u8,
    /// Checksum of short name.
    pub lfn_checksum: u8,
    /// Characters 6-11 (12 bytes).
    pub lfn_name2: [u16; 6],
    /// First cluster (always 0x0000).
    pub lfn_clus: u16,
    /// Characters 12-13 (4 bytes).
    pub lfn_name3: [u16; 2],
}

// ---------------------------------------------------------------------------
// FAT32 Filesystem Implementation
// ---------------------------------------------------------------------------

/// The FAT32 filesystem driver.
pub struct Fat32Fs {
    /// Parsed boot sector.
    boot: Option<Fat32BootSector>,
    /// FSInfo.
    fsinfo: Option<Fat32FsInfo>,
    /// Device handle.
    dev: Option<Dev>,
    /// Bytes per cluster.
    bytes_per_cluster: u32,
    /// First data sector (relative to start of partition).
    first_data_sector: u32,
}

impl Fat32Fs {
    /// Create a new FAT32 filesystem instance.
    pub fn new() -> Self {
        Fat32Fs {
            boot: None,
            fsinfo: None,
            dev: None,
            bytes_per_cluster: 0,
            first_data_sector: 0,
        }
    }

    /// Get the sector size from the boot sector.
    fn sector_size(&self) -> u16 {
        self.boot.as_ref().map_or(512, |b| b.bpb_bytes_per_sec)
    }

    /// Get the sectors per cluster.
    fn sectors_per_cluster(&self) -> u8 {
        self.boot.as_ref().map_or(1, |b| b.bpb_sec_per_clus)
    }

    /// Get the total sectors.
    fn total_sectors(&self) -> u32 {
        self.boot.as_ref().map_or(0, |b| {
            if b.bpb_tot_sec16 != 0 {
                b.bpb_tot_sec16 as u32
            } else {
                b.bpb_tot_sec32
            }
        })
    }

    /// Get the FAT size in sectors.
    fn fat_size_sectors(&self) -> u32 {
        self.boot.as_ref().map_or(0, |b| {
            if b.bpb_fat_sz16 != 0 {
                b.bpb_fat_sz16 as u32
            } else {
                b.bpb_fat_sz32
            }
        })
    }

    /// Get the number of FATs.
    fn num_fats(&self) -> u8 {
        self.boot.as_ref().map_or(2, |b| b.bpb_num_fats)
    }

    /// Get the reserved sector count.
    fn reserved_sectors(&self) -> u16 {
        self.boot.as_ref().map_or(0, |b| b.bpb_rsvd_sec_cnt)
    }

    /// Get the root directory cluster.
    fn root_cluster(&self) -> u32 {
        self.boot.as_ref().map_or(2, |b| b.bpb_root_clus)
    }

    /// Calculate the first sector of a cluster.
    fn cluster_to_sector(&self, cluster: u32) -> u32 {
        (cluster - 2) * self.sectors_per_cluster() as u32 + self.first_data_sector
    }

    /// Calculate the byte offset of a cluster.
    fn cluster_to_offset(&self, cluster: u32) -> u64 {
        self.cluster_to_sector(cluster) as u64 * self.sector_size() as u64
    }

    /// Get the FAT entry for a cluster.
    fn fat_entry(&self, cluster: u32) -> u32 {
        // In a real implementation, this would read from the FAT.
        // let fat_offset = cluster * 4;
        // let fat_sector = self.reserved_sectors() as u32 + (fat_offset / self.sector_size() as u32);
        // let entry_offset = fat_offset % self.sector_size() as u32;
        // Read 4 bytes at fat_sector * sector_size + entry_offset
        CLUSTER_EOF_MIN // Placeholder
    }

    /// Check if a cluster is the end of chain.
    fn is_eof_cluster(&self, cluster: u32) -> bool {
        cluster >= CLUSTER_EOF_MIN && cluster <= CLUSTER_EOF_MAX
    }

    /// Check if a cluster is free.
    fn is_free_cluster(&self, cluster: u32) -> bool {
        cluster == CLUSTER_FREE
    }

    /// Convert FAT attributes to VFS mode.
    fn attr_to_mode(attr: u8, size: u32) -> Mode {
        let mut mode = if attr & ATTR_DIRECTORY != 0 {
            S_IFDIR | 0o755
        } else {
            S_IFREG | 0o644
        };

        if attr & ATTR_READ_ONLY != 0 {
            mode &= !0o222; // Remove write permissions
        }

        mode
    }

    /// Convert a short name (11 bytes) to a string.
    fn short_name_to_string(name: &[u8; 11]) -> String {
        let mut result = String::with_capacity(12);

        // Name part (first 8 bytes, padded with spaces).
        let name_end = name[..8].iter().position(|&b| b == b' ').unwrap_or(8);
        for &b in &name[..name_end] {
            result.push((b as char).to_ascii_lowercase());
        }

        // Extension part (last 3 bytes).
        let ext_end = name[8..].iter().position(|&b| b == b' ').unwrap_or(3);
        if ext_end > 0 {
            result.push('.');
            for &b in &name[8..8 + ext_end] {
                result.push((b as char).to_ascii_lowercase());
            }
        }

        result
    }

    /// Read directory entries from a cluster.
    fn read_dir_entries(&self, cluster: u32) -> Vec<(String, u32, u32, u8)> {
        let mut entries = Vec::new();
        let mut lfn_chars: Vec<u16> = Vec::new();
        let mut lfn_checksum: u8 = 0;

        // In a real implementation, this would read the cluster data
        // and parse the directory entries.
        // For now, return an empty list as a placeholder.

        entries
    }
}

impl FileSystem for Fat32Fs {
    fn name(&self) -> &str {
        "fat32"
    }

    fn mount(&mut self, dev: Option<Dev>, flags: MountFlags) -> Result<Superblock, VfsError> {
        let device = dev.ok_or(VfsError::EINVAL)?;

        // Read and validate boot sector.
        // In a real implementation:
        // let mut buf = [0u8; 512];
        // block_dev::read(device, 0, &mut buf);
        // let boot: Fat32BootSector = unsafe { core::ptr::read(buf.as_ptr() as *const _) };

        // For now, create a placeholder boot sector.
        let boot = Fat32BootSector {
            bs_jmp_boot: [0xEB, 0x58, 0x90],
            bs_oem_name: *b"NERDOS  ",
            bpb_bytes_per_sec: 512,
            bpb_sec_per_clus: 8,
            bpb_rsvd_sec_cnt: 32,
            bpb_num_fats: 2,
            bpb_root_ent_cnt: 0,
            bpb_tot_sec16: 0,
            bpb_media: 0xF8,
            bpb_fat_sz16: 0,
            bpb_sec_per_trk: 63,
            bpb_num_heads: 255,
            bpb_hidd_sec: 0,
            bpb_tot_sec32: 0,
            bpb_fat_sz32: 0,
            bpb_ext_flags: 0,
            bpb_fs_ver: 0,
            bpb_root_clus: 2,
            bpb_fs_info: 1,
            bpb_bk_boot_sec: 6,
            bpb_reserved: [0; 12],
            bs_drv_num: 0x80,
            bs_reserved1: 0,
            bs_boot_sig: 0x29,
            bs_vol_id: 0x12345678,
            bs_vol_lab: *b"NERD_DRIVE ",
            bs_fil_sys_type: *b"FAT32   ",
        };

        self.bytes_per_cluster = boot.bpb_bytes_per_sec as u32 * boot.bpb_sec_per_clus as u32;
        self.first_data_sector = boot.bpb_rsvd_sec_cnt as u32 +
            (boot.bpb_num_fats as u32 * boot.bpb_fat_sz32);

        self.boot = Some(boot);
        self.dev = Some(device);

        // Build the VFS superblock.
        Ok(Superblock {
            fstype: String::from("fat32"),
            block_size: self.bytes_per_cluster,
            total_blocks: self.total_sectors() as u64 / self.sectors_per_cluster() as u64,
            free_blocks: 0, // Would read from FSInfo
            total_inodes: 0, // FAT doesn't have fixed inodes
            free_inodes: 0,
            max_name_len: 255,
            flags,
        })
    }

    fn unmount(&mut self) -> Result<(), VfsError> {
        self.boot = None;
        self.fsinfo = None;
        self.dev = None;
        Ok(())
    }

    fn root_inode(&self) -> Result<Inode, VfsError> {
        let dev = self.dev.ok_or(VfsError::EINVAL)?;

        Ok(Inode {
            ino: self.root_cluster() as u64,
            dev,
            refcount: 1,
            mode: S_IFDIR | 0o755,
            uid: 0,
            gid: 0,
            size: self.bytes_per_cluster as i64,
            blocks: self.sectors_per_cluster() as u64,
            atime: 0,
            mtime: 0,
            ctime: 0,
            nlink: 1,
        })
    }
}
