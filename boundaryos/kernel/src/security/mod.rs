//! Security subsystem
//! 
//! Implements capability-based security, membrane gates, and covenant enforcement.
//!
//! MODULE SIZE: ~0.1k lines | budget: 15k lines of 100k total

pub mod unforgeable;
pub mod membrane_gate;
pub mod invariant_judge;
pub mod covenant;
pub mod naked_mode;
pub mod audit;

pub use unforgeable::init as capability_table_init;
pub use covenant::load as covenant_load;
pub use invariant_judge::init as invariant_judge_init;
pub use membrane_gate::init as membrane_gate_init;

/// Initialize the entire security subsystem
pub fn security_init() {
    capability_table_init();
    covenant_load();
    invariant_judge_init();
    membrane_gate_init();
}
