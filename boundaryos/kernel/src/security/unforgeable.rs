//! Unforgeable Thread (Capability) implementation
//! 
//! Core capability type for BoundaryOS security model.

use bitflags::bitflags;
use crate::world::object::ObjectID;
use crate::world::fossil_heap::FossilRef;
use spin::Mutex;

bitflags! {
    /// Rights granted by a capability
    pub struct RightsSet: u32 {
        const READ     = 1 << 0;
        const WRITE    = 1 << 1;
        const PULSE    = 1 << 2;
        const DELEGATE = 1 << 3;
        const REVOKE   = 1 << 4;
        const OBSERVE  = 1 << 5;
        const FORGET   = 1 << 6;
        const MAP      = 1 << 7;
    }
}

/// Unique capability identifier
pub type CapabilityID = u64;

/// Delegation depth limit
#[derive(Debug, Clone, Copy)]
pub enum DelegationDepth {
    None,      // Cannot be delegated
    Limited(u8), // Can be delegated N more times
    Unlimited, // Can be delegated forever
}

/// Memory bounds restriction
#[derive(Debug, Clone)]
pub struct MemBounds {
    pub start: u64,
    pub end: u64,
}

/// An Unforgeable Thread - the core capability token
#[derive(Debug, Clone)]
pub struct UnforgeableThread {
    pub id: CapabilityID,
    pub target: ObjectID,
    pub rights: RightsSet,
    pub bounds: Option<MemBounds>,
    pub expiry: Option<u64>, // WorldTime
    pub depth: DelegationDepth,
    pub parent: Option<CapabilityID>,
    pub audit_ref: FossilRef,
    pub revoked: bool,
}

impl UnforgeableThread {
    /// Create a new capability
    pub fn new(id: CapabilityID, target: ObjectID, rights: RightsSet, audit_ref: FossilRef) -> Self {
        Self {
            id,
            target,
            rights,
            bounds: None,
            expiry: None,
            depth: DelegationDepth::None,
            parent: None,
            audit_ref,
            revoked: false,
        }
    }
    
    /// Check if this capability is valid
    pub fn is_valid(&self, current_time: u64) -> bool {
        !self.revoked && self.expiry.map_or(true, |exp| exp > current_time)
    }
    
    /// Check if this capability has a specific right
    pub fn has_right(&self, right: RightsSet) -> bool {
        self.rights.contains(right)
    }
}

/// Opaque handle to a capability (user-space representation)
#[derive(Debug, Clone, Copy)]
pub struct CapabilityHandle(pub u64);

static NEXT_CAP_ID: Mutex<u64> = Mutex::new(0);

/// Allocate a new capability ID
fn allocate_cap_id() -> u64 {
    let mut next = NEXT_CAP_ID.lock();
    let id = *next;
    *next += 1;
    id
}

/// Initialize capability system
pub fn init() {
    log!("Unforgeable Thread system initialized");
    // Root capability created in main.rs
}
