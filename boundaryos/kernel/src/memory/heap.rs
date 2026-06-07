//! Kernel heap allocator
//! 
//! Provides dynamic memory allocation for the kernel using linked_list_allocator.

use spin::Mutex;
use linked_list_allocator::LockedHeap;

/// Kernel heap - 8MB starting at __kernel_end
#[cfg_attr(not(test), global_allocator)]
static HEAP: LockedHeap = LockedHeap::empty();

/// Heap start address (set by linker)
extern "C" {
    static __kernel_end: u8;
}

/// Heap size (8MB)
const HEAP_SIZE: usize = 8 * 1024 * 1024;

/// Initialize the kernel heap
/// 
/// # Safety
/// This function must only be called once during boot after paging is set up.
pub unsafe fn init() {
    let heap_start = &__kernel_end as *const u8 as usize;
    // Align to page boundary
    let heap_start = (heap_start + 0xFFF) & !0xFFF;
    
    HEAP.lock().init(heap_start, HEAP_SIZE);
    
    log!("Kernel heap initialized: {} bytes at {:x}", HEAP_SIZE, heap_start);
}

/// Allocate memory from the kernel heap
pub fn allocate(size: usize, align: usize) -> Result<usize, &'static str> {
    let layout = core::alloc::Layout::from_size_align(size, align)
        .map_err(|_| "Invalid layout")?;
    
    let ptr = HEAP.lock().allocate_first_fit(layout)
        .map_err(|_| "Out of memory")?;
    
    Ok(ptr.as_ptr() as usize)
}

/// Deallocate memory from the kernel heap
/// 
/// # Safety
/// Caller must ensure the pointer was allocated by this allocator.
pub unsafe fn deallocate(ptr: usize, size: usize, align: usize) {
    let layout = core::alloc::Layout::from_size_align(size, align).unwrap();
    HEAP.lock().deallocate(core::ptr::NonNull::new_unchecked(ptr as *mut u8), layout);
}
