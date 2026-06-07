//! Memory management subsystem
//! 
//! Handles physical memory allocation, virtual memory mapping,
//! kernel heap, and fossil pages.
//!
//! MODULE SIZE: ~0.1k lines | budget: 15k lines of 100k total

pub mod physical;
pub mod virtual;
pub mod heap;
pub mod fossil_pages;
pub mod capability_table;

pub use physical::init as physical_mm_init;
pub use heap::init as heap_init;
