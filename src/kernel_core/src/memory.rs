//! # Memory Management
//!
//! NerdOS uses a two-level memory management system:
//!
//! 1. **Physical Memory Manager (PMM)**: Tracks which physical frames are
//!    free or allocated using a bitmap.
//! 2. **Virtual Memory Manager (VMM)**: Manages page tables that map virtual
//!    addresses to physical frames.
//!
//! ## Architecture
//!
//! ```text
//! Virtual Address (48-bit)               Physical Address
//! ┌─────────┬─────────┬─────────┐        ┌─────────────────┐
//! │  PML4   │  PDPT   │   PD    │   ┌───▶│  Physical Frame  │
//! │  Index  │  Index  │  Index  │   │    │    (4 KiB)      │
//! │  (9b)   │  (9b)   │  (9b)   │   │    └─────────────────┘
//! └────┬────┴────┬────┴────┬────┘   │
//!      │         │         │        │
//!      ▼         ▼         ▼        │
//!   ┌─────┐   ┌─────┐   ┌─────┐    │
//!   │PML4 │──▶│PDPT │──▶│ PD  │────┘
//!   └─────┘   └─────┘   └─────┘
//!   CR3 register points to PML4
//! ```
//!
//! ## Frame Allocation
//!
//! The PMM uses a bitmap where each bit represents one 4 KiB physical frame.
//! A `0` means free, a `1` means allocated.

use bitflags::bitflags;
use core::ops::Range;
use multiboot2::{BootInformation, MemoryArea, MemoryAreaType};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Page size on x86_64: 4 KiB.
pub const PAGE_SIZE: usize = 4096;

/// Number of entries in each level of the page table hierarchy.
const PAGE_TABLE_ENTRIES: usize = 512;

/// Virtual address where the kernel is mapped.
/// We use the higher half model: kernel lives at 0xFFFF800000000000+.
pub const KERNEL_VIRT_OFFSET: u64 = 0xFFFF_8000_0000_0000;

// ---------------------------------------------------------------------------
// Physical Address
// ---------------------------------------------------------------------------

/// A physical address (not a pointer - can't be dereferenced directly).
/// Must be converted to a virtual address (via page tables) before use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Create a new physical address.
    ///
    /// # Panics
    /// Panics if the address is not a valid 48-bit physical address.
    pub const fn new(addr: u64) -> Self {
        // x86_64 uses 48-bit physical addresses (with sign extension)
        assert!(addr < (1 << 52), "Physical address exceeds 52 bits");
        PhysAddr(addr)
    }

    /// Get the underlying u64 value.
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Align down to page boundary.
    pub fn align_down(self) -> Self {
        PhysAddr(self.0 & !(PAGE_SIZE as u64 - 1))
    }

    /// Align up to page boundary.
    pub fn align_up(self) -> Self {
        PhysAddr((self.0 + PAGE_SIZE as u64 - 1) & !(PAGE_SIZE as u64 - 1))
    }

    /// Offset within the page.
    pub fn page_offset(self) -> u64 {
        self.0 & (PAGE_SIZE as u64 - 1)
    }

    /// Check if address is page-aligned.
    pub fn is_aligned(self) -> bool {
        self.page_offset() == 0
    }
}

// ---------------------------------------------------------------------------
// Virtual Address
// ---------------------------------------------------------------------------

/// A virtual address that can be used to access memory through page tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Create a new virtual address.
    pub const fn new(addr: u64) -> Self {
        VirtAddr(addr)
    }

    /// Get the underlying u64 value.
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Get the page table indices for this address.
    /// Returns (pml4_index, pdpt_index, pd_index, pt_index).
    pub fn page_table_indices(self) -> (usize, usize, usize, usize) {
        const SIGN_EXTENSION: u64 = 0xFFFF_0000_0000_0000;
        let addr = self.0 | SIGN_EXTENSION; // Sign-extend to 64 bits

        let pml4 = ((addr >> 39) & 0x1FF) as usize;
        let pdpt = ((addr >> 30) & 0x1FF) as usize;
        let pd = ((addr >> 21) & 0x1FF) as usize;
        let pt = ((addr >> 12) & 0x1FF) as usize;

        (pml4, pdpt, pd, pt)
    }

    /// Align down to page boundary.
    pub fn align_down(self) -> Self {
        VirtAddr(self.0 & !(PAGE_SIZE as u64 - 1))
    }

    /// Align up to page boundary.
    pub fn align_up(self) -> Self {
        VirtAddr((self.0 + PAGE_SIZE as u64 - 1) & !(PAGE_SIZE as u64 - 1))
    }

    /// Check if this is a null pointer.
    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Convert to a raw pointer.
    ///
    /// # Safety
    /// The virtual address must be properly mapped.
    pub unsafe fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Convert to a mutable raw pointer.
    ///
    /// # Safety
    /// The virtual address must be properly mapped.
    pub unsafe fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
}

// ---------------------------------------------------------------------------
// Physical Frame
// ---------------------------------------------------------------------------

/// Represents a 4 KiB physical frame.
/// Frames are the unit of physical memory allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame {
    /// Starting physical address of the frame.
    pub start_address: PhysAddr,
}

impl PhysFrame {
    /// Create a PhysFrame from a PhysAddr (must be page-aligned).
    pub fn containing_address(addr: PhysAddr) -> Self {
        PhysFrame {
            start_address: addr.align_down(),
        }
    }

    /// Frame number (address / PAGE_SIZE).
    pub fn number(&self) -> u64 {
        self.start_address.as_u64() / PAGE_SIZE as u64
    }

    /// Convert to a virtual address in the kernel's identity-mapped region.
    ///
    /// # Safety
    /// This assumes identity mapping or a specific mapping scheme.
    pub unsafe fn as_kernel_virt(&self) -> VirtAddr {
        // During early boot, physical addresses are identity-mapped.
        // Later, we use the recursive page table or a fixed mapping.
        VirtAddr::new(self.start_address.as_u64())
    }
}

// ---------------------------------------------------------------------------
// Page Flags
// ---------------------------------------------------------------------------

bitflags! {
    /// Flags for page table entries.
    /// These control access permissions and caching behavior.
    pub struct PageFlags: u64 {
        /// Present: The page is mapped and available.
        const PRESENT = 1 << 0;

        /// Writable: The page can be written to.
        /// Without this, any write causes a page fault.
        const WRITABLE = 1 << 1;

        /// User accessible: Ring 3 code can access this page.
        /// Without this, user code accessing the page causes a GPF.
        const USER = 1 << 2;

        /// Write-through: Use write-through caching instead of write-back.
        const WRITE_THROUGH = 1 << 3;

        /// Cache disable: Do not cache this page.
        const NO_CACHE = 1 << 4;

        /// Accessed: CPU sets this when the page is read/written.
        /// Used by the OS for page replacement decisions.
        const ACCESSED = 1 << 5;

        /// Dirty: CPU sets this when the page is written to.
        /// Used to know if a page needs to be written back to disk.
        const DIRTY = 1 << 6;

        /// Huge page: This entry points to a 2 MiB frame (PD level)
        /// or 1 GiB frame (PDPT level) instead of a page table.
        const HUGE_PAGE = 1 << 7;

        /// Global: TLB entry is not flushed on context switch.
        /// Useful for kernel pages.
        const GLOBAL = 1 << 8;

        /// No execute: Code cannot be executed from this page.
        /// Requires EFER.NXE to be set.
        const NO_EXECUTE = 1 << 63;
    }
}

impl PageFlags {
    /// Default flags for kernel code pages: present, readable (non-writable).
    pub const fn kernel_code() -> Self {
        Self::PRESENT | Self::GLOBAL
    }

    /// Default flags for kernel data pages: present, writable.
    pub const fn kernel_data() -> Self {
        Self::PRESENT | Self::WRITABLE | Self::GLOBAL
    }

    /// Default flags for user code pages: present, user-accessible.
    pub const fn user_code() -> Self {
        Self::PRESENT | Self::USER
    }

    /// Default flags for user data pages: present, writable, user-accessible.
    pub const fn user_data() -> Self {
        Self::PRESENT | Self::WRITABLE | Self::USER
    }

    /// Default flags for MMIO pages: present, writable, uncached.
    pub const fn mmio() -> Self {
        Self::PRESENT | Self::WRITABLE | Self::NO_CACHE | Self::GLOBAL
    }
}

// ---------------------------------------------------------------------------
// Page Table Entry
// ---------------------------------------------------------------------------

/// A single entry in a page table.
/// Contains the physical address of the next-level table or frame,
/// plus permission flags.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create an unused entry.
    pub const fn unused() -> Self {
        PageTableEntry(0)
    }

    /// Create an entry pointing to a frame with the given flags.
    pub fn new(frame: PhysFrame, flags: PageFlags) -> Self {
        PageTableEntry(frame.start_address.as_u64() | flags.bits())
    }

    /// Check if this entry is present (mapped).
    pub fn is_present(&self) -> bool {
        self.0 & PageFlags::PRESENT.bits() != 0
    }

    /// Check if this entry is writable.
    pub fn is_writable(&self) -> bool {
        self.0 & PageFlags::WRITABLE.bits() != 0
    }

    /// Check if this entry is user-accessible.
    pub fn is_user(&self) -> bool {
        self.0 & PageFlags::USER.bits() != 0
    }

    /// Get the flags.
    pub fn flags(&self) -> PageFlags {
        // Mask out the address portion to get just the flags
        PageFlags::from_bits_truncate(self.0)
    }

    /// Get the physical address this entry points to.
    /// Returns None if the entry is not present.
    pub fn addr(&self) -> Option<PhysAddr> {
        if self.is_present() {
            Some(PhysAddr::new(self.0 & 0x000F_FFFF_FFFF_F000))
        } else {
            None
        }
    }

    /// Get the frame this entry points to.
    pub fn frame(&self) -> Option<PhysFrame> {
        self.addr().map(PhysFrame::containing_address)
    }

    /// Set the entry to point to a frame with given flags.
    pub fn set(&mut self, frame: PhysFrame, flags: PageFlags) {
        self.0 = frame.start_address.as_u64() | flags.bits();
    }

    /// Clear the entry (mark as not present).
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

// ---------------------------------------------------------------------------
// Page Table
// ---------------------------------------------------------------------------

/// A page table contains 512 entries, each pointing to the next level
/// in the hierarchy (except for PT entries which point to physical frames).
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// Create an empty page table with all entries unused.
    pub const fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::unused(); PAGE_TABLE_ENTRIES],
        }
    }

    /// Get a reference to an entry.
    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get a mutable reference to an entry.
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Get the physical frame of this page table itself.
    ///
    /// # Safety
    /// The page table must be properly allocated and aligned.
    pub unsafe fn physical_address(&self) -> PhysAddr {
        // We need to convert the virtual address of self to a physical address.
        // During early boot, this is identity-mapped.
        PhysAddr::new(self as *const _ as u64)
    }
}

// ---------------------------------------------------------------------------
// Physical Memory Manager (Bitmap-based)
// ---------------------------------------------------------------------------

/// The Physical Memory Manager tracks which frames are free using a bitmap.
///
/// ## Bitmap Format
/// Each bit in the bitmap represents one 4 KiB frame:
/// - Bit = 0: Frame is FREE
/// - Bit = 1: Frame is ALLOCATED (or reserved)
///
/// ## Frame 0
/// Frame 0 is always marked as used to catch null pointer dereferences
/// (since PhysAddr(0) looks like a null pointer).
pub struct PhysicalMemoryManager {
    /// Bitmap of frame allocations. One bit per frame.
    bitmap: &'static mut [u8],
    /// Total number of physical frames.
    total_frames: usize,
    /// Number of currently free frames.
    free_frames: usize,
    /// Start of allocatable memory (frames below this are reserved).
    frame_start: usize,
}

impl PhysicalMemoryManager {
    /// Create the PMM from the Multiboot2 memory map.
    ///
    /// This parses the EFI memory map or BIOS memory map provided by GRUB2
    /// to determine which regions of physical memory are available.
    ///
    /// # Safety
    /// Must be called exactly once during early boot. The Multiboot info
    /// must be valid and identity-mapped.
    pub unsafe fn from_multiboot(boot_info: &BootInformation) -> Self {
        // Find the largest physical address to know how big our bitmap needs to be.
        let mut max_phys_addr: u64 = 0;

        // Get memory areas from the boot information.
        // The memory map tag contains entries describing available RAM.
        if let Some(memory_map) = boot_info.memory_map() {
            for area in memory_map.entries() {
                let end = area.base() + area.length();
                if end > max_phys_addr {
                    max_phys_addr = end;
                }
            }
        }

        // Calculate bitmap size: 1 bit per 4 KiB frame.
        let total_frames = (max_phys_addr as usize + PAGE_SIZE - 1) / PAGE_SIZE;
        let bitmap_size = (total_frames + 7) / 8; // Round up to bytes

        // Place the bitmap at the end of the kernel image.
        // In a real implementation, we'd use the memory map to find
        // a suitable location. For now, we use a fixed location.
        let bitmap_addr = 0x100000 + 0x100000; // After the first 2 MiB
        let bitmap = core::slice::from_raw_parts_mut(
            bitmap_addr as *mut u8,
            bitmap_size
        );

        // Initialize all frames as allocated (conservative approach).
        for byte in bitmap.iter_mut() {
            *byte = 0xFF; // Mark all as used
        }

        let mut pmm = PhysicalMemoryManager {
            bitmap,
            total_frames,
            free_frames: 0,
            frame_start: 0,
        };

        // Mark available frames from the memory map.
        if let Some(memory_map) = boot_info.memory_map() {
            for area in memory_map.entries() {
                // Only mark conventional RAM as available.
                // Other types (ACPI, reserved, etc.) stay marked as used.
                if area.typ() == MemoryAreaType::Available {
                    let start_frame = (area.base() as usize + PAGE_SIZE - 1) / PAGE_SIZE;
                    let end_frame = (area.base() as usize + area.length() as usize) / PAGE_SIZE;
                    for frame in start_frame..end_frame {
                        pmm.mark_free(frame);
                    }
                }
            }
        }

        // Frame 0 is always reserved to catch null pointer bugs.
        pmm.mark_used(0);

        // Reserve frames used by the kernel image itself.
        // The Multiboot header tells us where the kernel was loaded.
        if let Some(elf_sections) = boot_info.elf_sections() {
            for section in elf_sections.sections() {
                if section.flags().contains(multiboot2::ElfSectionFlags::ALLOCATED) {
                    let start_frame = section.start_address() as usize / PAGE_SIZE;
                    let end_frame = (section.end_address() as usize + PAGE_SIZE - 1) / PAGE_SIZE;
                    for frame in start_frame..end_frame {
                        pmm.mark_used(frame);
                    }
                }
            }
        }

        // Reserve the bitmap region itself.
        let bitmap_start_frame = bitmap_addr / PAGE_SIZE;
        let bitmap_end_frame = (bitmap_addr + bitmap_size + PAGE_SIZE - 1) / PAGE_SIZE;
        for frame in bitmap_start_frame..bitmap_end_frame {
            pmm.mark_used(frame);
        }

        pmm
    }

    /// Mark a frame as free (available for allocation).
    fn mark_free(&mut self, frame: usize) {
        if frame >= self.total_frames {
            return;
        }
        let byte = frame / 8;
        let bit = frame % 8;
        if byte < self.bitmap.len() {
            let was_set = self.bitmap[byte] & (1 << bit) != 0;
            self.bitmap[byte] &= !(1 << bit);
            if was_set {
                self.free_frames += 1;
            }
        }
    }

    /// Mark a frame as used (allocated/reserved).
    fn mark_used(&mut self, frame: usize) {
        if frame >= self.total_frames {
            return;
        }
        let byte = frame / 8;
        let bit = frame % 8;
        if byte < self.bitmap.len() {
            let was_clear = self.bitmap[byte] & (1 << bit) == 0;
            self.bitmap[byte] |= 1 << bit;
            if was_clear {
                self.free_frames -= 1;
            }
        }
    }

    /// Check if a frame is free.
    pub fn is_free(&self, frame: usize) -> bool {
        if frame >= self.total_frames {
            return false;
        }
        let byte = frame / 8;
        let bit = frame % 8;
        byte < self.bitmap.len() && (self.bitmap[byte] & (1 << bit)) == 0
    }

    /// Allocate a single physical frame.
    ///
    /// Returns `Some(frame)` on success, `None` if no frames are available.
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Simple first-fit search. For better performance, we could maintain
        // a free list or use a next-fit pointer.
        for frame in self.frame_start..self.total_frames {
            if self.is_free(frame) {
                self.mark_used(frame);
                return Some(PhysFrame {
                    start_address: PhysAddr::new(frame as u64 * PAGE_SIZE as u64),
                });
            }
        }
        None
    }

    /// Free a previously allocated frame.
    pub fn free_frame(&mut self, frame: PhysFrame) {
        let frame_num = frame.start_address.as_u64() as usize / PAGE_SIZE;
        self.mark_free(frame_num);
    }

    /// Get the total number of frames.
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    /// Get the number of free frames.
    pub fn free_frames(&self) -> usize {
        self.free_frames
    }

    /// Get total physical memory in MiB.
    pub fn total_memory_mib(&self) -> usize {
        self.total_frames * PAGE_SIZE / (1024 * 1024)
    }

    /// Get free memory in MiB.
    pub fn free_memory_mib(&self) -> usize {
        self.free_frames * PAGE_SIZE / (1024 * 1024)
    }
}

// ---------------------------------------------------------------------------
// Global Frame Allocator
// ---------------------------------------------------------------------------

/// Global frame allocator used by the page table code.
/// This is a simple wrapper around the PMM.
///
/// # Safety
/// This uses a raw pointer to the global PMM. It is only safe to use
/// after `PMM` has been initialized in `kernel_main`.
pub struct FrameAllocator;

impl FrameAllocator {
    /// Allocate a frame from the global PMM.
    pub fn allocate() -> Option<PhysFrame> {
        unsafe {
            if let Some(ref mut pmm) = super::PMM {
                pmm.allocate_frame()
            } else {
                None
            }
        }
    }

    /// Free a frame back to the global PMM.
    pub fn free(frame: PhysFrame) {
        unsafe {
            if let Some(ref mut pmm) = super::PMM {
                pmm.free_frame(frame);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Recursive Page Table Mapping
// ---------------------------------------------------------------------------

/// The recursive page table technique maps the page table itself
/// at a fixed virtual address. This allows us to access and modify
/// page tables without needing to know their physical addresses.
///
/// We map PML4 at index 510, so:
/// - 0xFFFF_FFFE_8000_0000: PML4 itself (level 4)
/// - 0xFFFF_FFC0_0000_0000: All PDPTs (level 3)
/// - 0xFFFF_8000_0000_0000: All PDs (level 2)
/// - 0x0000_0000_0000_0000: All PTs (level 1)
///
/// This is the standard "higher half" recursive mapping scheme.
pub mod recursive {
    use super::*;

    /// Virtual address where PML4 is recursively mapped.
    const RECURSIVE_PML4_VIRT: u64 = 0xFFFF_FFFE_8000_0000;

    /// Get a mutable reference to the PML4.
    ///
    /// # Safety
    /// The recursive mapping must be set up correctly.
    pub unsafe fn get_pml4() -> &'static mut PageTable {
        &mut *(RECURSIVE_PML4_VIRT as *mut PageTable)
    }

    /// Map a virtual page to a physical frame.
    ///
    /// This walks the page table hierarchy, creating intermediate
    /// tables as needed using the frame allocator.
    ///
    /// # Safety
    /// The page tables must be properly set up. The virtual address
    /// should not already be mapped (or you'll create a leak).
    pub unsafe fn map_page(
        virt: VirtAddr,
        phys: PhysFrame,
        flags: PageFlags,
    ) -> Result<(), MapError> {
        let (pml4_i, pdpt_i, pd_i, pt_i) = virt.page_table_indices();

        let pml4 = get_pml4();

        // Get or create PDPT
        let pdpt = get_or_create_table(&mut pml4.entries[pml4_i])?;

        // Get or create PD
        let pd = get_or_create_table(&mut pdpt.entries[pdpt_i])?;

        // Get or create PT
        let pt = get_or_create_table(&mut pd.entries[pd_i])?;

        // Set the page table entry
        let entry = &mut pt.entries[pt_i];
        if entry.is_present() {
            return Err(MapError::AlreadyMapped);
        }
        entry.set(phys, flags);

        // Invalidate TLB for this page
        x86_64::instructions::tlb::flush(x86_64::VirtAddr::new(virt.as_u64()));

        Ok(())
    }

    /// Unmap a virtual page.
    ///
    /// Returns the physical frame that was mapped, if any.
    ///
    /// # Safety
    /// The page tables must be properly set up.
    pub unsafe fn unmap_page(virt: VirtAddr) -> Option<PhysFrame> {
        let (pml4_i, pdpt_i, pd_i, pt_i) = virt.page_table_indices();

        let pml4 = get_pml4();
        let pdpt_entry = &pml4.entries[pml4_i];
        if !pdpt_entry.is_present() {
            return None;
        }

        let pdpt = &mut *(pdpt_entry.addr().unwrap().as_u64() as *mut PageTable);
        let pd_entry = &pdpt.entries[pdpt_i];
        if !pd_entry.is_present() {
            return None;
        }

        let pd = &mut *(pd_entry.addr().unwrap().as_u64() as *mut PageTable);
        let pt_entry = &pd.entries[pd_i];
        if !pt_entry.is_present() {
            return None;
        }

        let pt = &mut *(pt_entry.addr().unwrap().as_u64() as *mut PageTable);
        let entry = &mut pt.entries[pt_i];

        if entry.is_present() {
            let frame = entry.frame();
            entry.clear();

            // Invalidate TLB
            x86_64::instructions::tlb::flush(x86_64::VirtAddr::new(virt.as_u64()));

            frame
        } else {
            None
        }
    }

    /// Get or create an intermediate page table.
    ///
    /// If the entry is not present, allocates a frame and creates a new table.
    unsafe fn get_or_create_table(entry: &mut PageTableEntry) -> Result<&'static mut PageTable, MapError> {
        if !entry.is_present() {
            let frame = FrameAllocator::allocate()
                .ok_or(MapError::OutOfMemory)?;

            // Zero out the new page table
            let table = &mut *(frame.start_address.as_u64() as *mut PageTable);
            *table = PageTable::new();

            entry.set(frame, PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::GLOBAL);
        }

        // Convert the physical address in the entry to a virtual address.
        // During early boot, we assume identity mapping.
        let phys_addr = entry.addr().unwrap();
        Ok(&mut *(phys_addr.as_u64() as *mut PageTable))
    }
}

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

/// Errors that can occur during page mapping operations.
#[derive(Debug)]
pub enum MapError {
    /// The virtual page is already mapped to a physical frame.
    AlreadyMapped,
    /// No free physical frames available.
    OutOfMemory,
    /// Invalid page table structure.
    InvalidStructure,
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Initialize paging subsystem.
///
/// This sets up the page tables for the kernel and enables paging.
///
/// # Safety
/// Must be called once during early boot after the PMM is initialized.
pub unsafe fn init_paging(heap_start: usize, heap_size: usize) {
    use recursive::*;

    // Map the heap region.
    // The heap starts as unmapped; we map pages on demand or pre-map here.
    let heap_start_virt = VirtAddr::new(heap_start as u64);
    let heap_pages = heap_size / PAGE_SIZE;

    for i in 0..heap_pages {
        let virt = VirtAddr::new(heap_start_virt.as_u64() + (i * PAGE_SIZE) as u64);
        if let Some(frame) = FrameAllocator::allocate() {
            let _ = map_page(virt, frame, PageFlags::kernel_data());
        }
    }

    // Set up recursive mapping in PML4.
    // This allows us to access page tables as virtual memory.
    let pml4 = recursive::get_pml4();
    let pml4_phys = PhysAddr::new(pml4 as *mut _ as u64);
    let recursive_entry = &mut pml4.entries[510]; // Index 510 is recursive
    recursive_entry.set(
        PhysFrame::containing_address(pml4_phys),
        PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::GLOBAL
    );

    // Enable the NX bit (No Execute) via EFER MSR.
    use x86_64::registers::model_specific::{Efer, EferFlags};
    let mut efer = Efer::read();
    efer.insert(EferFlags::NO_EXECUTE_ENABLE);
    unsafe { Efer::write(efer) };
}

// ---------------------------------------------------------------------------
// Heap Allocator Module
// ---------------------------------------------------------------------------

pub mod allocator {
    //! # Kernel Heap Allocator
    //!
    //! A simple bump allocator for the kernel heap.
    //! This is used for `Box`, `Vec`, `String`, and other `alloc` types.
    //!
    //! ## Design
    //! The bump allocator is the simplest possible allocator:
    //! - Allocation: bump a pointer forward
    //! - Deallocation: does nothing (memory is leaked)
    //!
    //! This is suitable for a kernel where most allocations are permanent.
    //! A more sophisticated allocator (linked list, buddy system) can
    //! be implemented later.

    use core::alloc::{GlobalAlloc, Layout};
    use core::sync::atomic::{AtomicUsize, Ordering};

    /// The kernel heap allocator.
    /// Uses a simple bump allocation strategy.
    pub struct BumpAllocator {
        /// Start of the heap region.
        heap_start: AtomicUsize,
        /// Current allocation offset from heap_start.
        next: AtomicUsize,
        /// Size of the heap region.
        heap_size: AtomicUsize,
    }

    impl BumpAllocator {
        /// Create a new uninitialized allocator.
        pub const fn new() -> Self {
            BumpAllocator {
                heap_start: AtomicUsize::new(0),
                next: AtomicUsize::new(0),
                heap_size: AtomicUsize::new(0),
            }
        }

        /// Initialize the allocator with a memory region.
        ///
        /// # Safety
        /// The memory region must be valid, writable, and not used by anything else.
        pub unsafe fn init(&self, start: usize, size: usize) {
            self.heap_start.store(start, Ordering::Relaxed);
            self.next.store(0, Ordering::Relaxed);
            self.heap_size.store(size, Ordering::Relaxed);
        }
    }

    unsafe impl GlobalAlloc for BumpAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // Simple bump allocation with alignment.
            let current_offset = self.next.load(Ordering::Relaxed);
            let heap_start = self.heap_start.load(Ordering::Relaxed);

            // Align the current offset to the required alignment.
            let aligned_offset = (current_offset + layout.align() - 1) & !(layout.align() - 1);
            let new_offset = aligned_offset + layout.size();

            // Check if we have enough space.
            if new_offset > self.heap_size.load(Ordering::Relaxed) {
                return core::ptr::null_mut(); // Out of memory
            }

            // Update the bump pointer.
            self.next.store(new_offset, Ordering::Relaxed);

            // Return the allocated address.
            (heap_start + aligned_offset) as *mut u8
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
            // Bump allocator doesn't support deallocation.
            // Memory is permanently allocated until reboot.
            // A real allocator would add the block to a free list.
        }
    }

    // ---------------------------------------------------------------------------
    // Global Allocator Instance
    // ---------------------------------------------------------------------------

    /// The global allocator used by the Rust `alloc` crate.
    #[global_allocator]
    static ALLOCATOR: BumpAllocator = BumpAllocator::new();

    /// Initialize the global heap allocator.
    ///
    /// # Safety
    /// Must be called exactly once during boot, after paging is set up
    /// and the heap region is mapped.
    pub unsafe fn init_heap(start: usize, size: usize) {
        ALLOCATOR.init(start, size);
    }
}
