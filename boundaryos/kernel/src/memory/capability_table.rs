//! Kernel-side capability table
//! 
//! Stores Unforgeable Threads in a kernel-resident table.
//! User-space holds opaque handles that reference this table.

use spin::Mutex;
use crate::security::unforgeable::UnforgeableThread;

/// Maximum number of capabilities in the system
const MAX_CAPABILITIES: usize = 65536;

/// Global capability table
static CAPABILITY_TABLE: Mutex<CapabilityTable> = Mutex::new(CapabilityTable {
    entries: [None; MAX_CAPABILITIES],
    next_id: 0,
});

/// Capability table storing all Unforgeable Threads
pub struct CapabilityTable {
    entries: [Option<UnforgeableThread>; MAX_CAPABILITIES],
    next_id: u64,
}

impl CapabilityTable {
    /// Allocate a new capability ID
    fn allocate_id(&mut self) -> Option<u64> {
        if self.next_id >= MAX_CAPABILITIES as u64 {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        Some(id)
    }
    
    /// Insert a capability into the table
    pub fn insert(&mut self, cap: UnforgeableThread) -> Option<u64> {
        let id = self.allocate_id()?;
        self.entries[id as usize] = Some(cap);
        Some(id)
    }
    
    /// Get a capability by ID
    pub fn get(&self, id: u64) -> Option<&UnforgeableThread> {
        if id >= MAX_CAPABILITIES as u64 {
            return None;
        }
        self.entries[id as usize].as_ref()
    }
    
    /// Revoke a capability by ID
    pub fn revoke(&mut self, id: u64) -> bool {
        if id >= MAX_CAPABILITIES as u64 {
            return false;
        }
        if let Some(ref mut cap) = self.entries[id as usize] {
            cap.revoked = true;
            true
        } else {
            false
        }
    }
}

/// Initialize the capability table
pub fn init() {
    log!("Capability table initialized with {} slots", MAX_CAPABILITIES);
    
    // Create root capability for kernel
    // TODO: Create actual root capability
}
