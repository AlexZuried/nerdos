//! Exo-Layer: Hardware multiplexing layer
//! 
//! Sits between raw hardware and World, providing safe multiplexing.

pub mod exo_keyboard;
pub mod exo_vga;
pub mod exo_serial;

/// Initialize all Exo-layers
pub fn init() {
    log!("Exo-layers initialized");
}
