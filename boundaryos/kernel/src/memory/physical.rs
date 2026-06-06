//! Physical memory manager (buddy allocator)
//! 
//! Manages physical page frames using a buddy system allocator.

use spin::Mutex;
use crate::arch::MemoryRegion;

/// Page size in bytes (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Maximum number of pages we can track
const MAX_PAGES: usize = 1024 * 1024; // 4GB of RAM

/// Physical memory manager state
static PHYS_MANAGER: Mutex<PhysicalMemoryManager> = Mutex::new(PhysicalMemoryManager {
    total_pages: 0,
    free_pages: 0,
    used_pages: 0,
});

/// Physical memory manager
pub struct PhysicalMemoryManager {
    pub total_pages: usize,
    pub free_pages: usize,
    pub used_pages: usize,
}

impl PhysicalMemoryManager {
    /// Initialize the physical memory manager
    pub fn init(regions: &[MemoryRegion]) {
        let mut manager = PHYS_MANAGER.lock();
        
        // Count available pages from memory map
        for region in regions {
            if region.region_type == crate::arch::MemoryRegionType::Available {
                let start_page = region.base as usize / PAGE_SIZE;
                let page_count = region.size as usize / PAGE_SIZE;
                manager.total_pages += page_count;
                manager.free_pages += page_count;
            }
        }
        
        log!("Physical memory: {} pages total, {} free", manager.total_pages, manager.free_pages);
    }
    
    /// Allocate a physical page frame
    pub fn allocate() -> Option<usize> {
        let mut manager = PHYS_MANAGER.lock();
        if manager.free_pages == 0 {
            return None;
        }
        manager.free_pages -= 1;
        manager.used_pages += 1;
        // TODO: Implement actual buddy allocation
        Some(0) // Dummy implementation
    }
    
    /// Free a physical page frame
    pub fn free(page_frame: usize) {
        let mut manager = PHYS_MANAGER.lock();
        manager.free_pages += 1;
        manager.used_pages -= 1;
        // TODO: Implement actual buddy deallocation
    }
}

/// Initialize physical memory management
pub fn init(regions: &[MemoryRegion]) {
    PhysicalMemoryManager::init(regions);
}
