//! Six-Layer Membrane Gate
//! 
//! Every access to a World object must pass through this gate.

use crate::security::unforgeable::{CapabilityHandle, RightsSet};
use crate::world::object::ObjectID;

/// Action tag identifying what operation is being requested
#[derive(Debug, Clone, Copy)]
pub struct ActionTag(pub u32);

/// Membrane check result
#[derive(Debug)]
pub enum MembraneError {
    NoCapability { target: ObjectID, action: ActionTag },
    InsufficientRights { held: RightsSet, required: RightsSet },
    OutOfBounds { bounds: (u64, u64), requested: u64 },
    ThreadExpired { expired_at: u64, current: u64 },
    InvariantViolated { invariant_name: &'static str, detail: &'static str },
    CovenantForbids { article_id: usize, article_text: &'static str },
}

/// Successful membrane passage
#[derive(Debug)]
pub struct MembranePass {
    pub log_token: u64,
    pub action: ActionTag,
    pub budget_remaining: usize,
}

/// World context for membrane checking
pub struct WorldContext {
    pub current_time: u64,
    pub active_covenant: Option<usize>,
}

/// The Membrane Gate - six-layer access check
pub fn check(
    caller_cap: CapabilityHandle,
    target: ObjectID,
    action: ActionTag,
    context: &WorldContext,
) -> Result<MembranePass, MembraneError> {
    // LAYER 1: CAPABILITY VALIDITY
    // TODO: Check if capability exists and is not revoked
    
    // LAYER 2: RIGHTS CHECK
    // TODO: Verify capability has required rights
    
    // LAYER 3: BOUNDS CHECK
    // TODO: Check memory bounds if applicable
    
    // LAYER 4: EXPIRY CHECK
    // TODO: Check if capability has expired
    
    // LAYER 5: INVARIANT CHECK
    // TODO: Run invariant checks on target object
    
    // LAYER 6: COVENANT CHECK
    // TODO: Verify covenant allows this action
    
    // For now, allow everything (stub implementation)
    Ok(MembranePass {
        log_token: 0,
        action,
        budget_remaining: 100,
    })
}

/// Initialize membrane gate
pub fn init() {
    log!("Membrane gate initialized");
}
