# BoundaryOS Construction Phases

A 100-phase roadmap to build BoundaryOS, introducing ~1,000 lines of code per phase.
Total target: ~100,000 lines of production-ready, no_std Rust code.

---

## PHASES 1-10: Foundation & Boot

### Phase 1: Project Skeleton (1k lines)
**Files:** `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `linker.ld`, `kernel/src/main.rs`, `kernel/src/panic.rs`
**Goals:**
- Establish no_std bare-metal toolchain
- Custom linker script with kernel sections (.boot, .text, .rodata, .data, .bss, .stack)
- Multiboot2 header in assembly
- Basic panic handler with serial output
- Kernel entry point stub
**Laws:** Nothing is Hidden (explicit linker layout), Nothing is Magic (clear boot flow)

### Phase 2: Assembly Boot Code (1k lines)
**Files:** `boot/boot.S`, `boot/trampoline32.S`, `boot/grub.cfg`
**Goals:**
- Multiboot2 header with all required tags
- 32-bit entry point, stack setup
- Trampoline for 32→64 bit mode switch
- GRUB configuration with serial console
- Early identity mapping setup
**Laws:** Nothing is Hidden (every register manipulation commented), Nothing is Magic (explicit mode transitions)

### Phase 3: GDT & TSS (1k lines)
**Files:** `kernel/src/arch/x86_64/gdt.rs`, `kernel/src/arch/x86_64/mod.rs`
**Goals:**
- Global Descriptor Table with kernel/user segments
- Task State Segment for interrupt stacks
- GDT loading code with proper selectors
- TLS descriptor stub
- Safety documentation for every unsafe block
**Laws:** Nothing is Hidden (GDT entries inspectable), Nothing is Safe by Obscurity (dangerous segment ops named)

### Phase 4: IDT & Interrupts (1k lines)
**Files:** `kernel/src/arch/x86_64/idt.rs`, `kernel/src/arch/x86_64/interrupts.rs`
**Goals:**
- Interrupt Descriptor Table with all 256 entries
- CPU exception handlers (divide-by-zero, GPF, page fault, etc.)
- IRQ handlers for PIC/APIC
- Interrupt context saving/restoring
- Serial logging in interrupt context
**Laws:** Nothing is Hidden (IDT entries visible in anatomy), Nothing is Safe by Obscurity (interrupt handlers logged)

### Phase 5: Paging & Memory Maps (1k lines)
**Files:** `kernel/src/arch/x86_64/paging.rs`, `kernel/src/memory/physical.rs`
**Goals:**
- Parse Multiboot2 memory map
- Buddy allocator for physical frames
- 4-level page table implementation
- Identity mapping for kernel
- Virtual address space layout
**Laws:** Nothing is Hidden (page tables inspectable), Nothing is Owned by System (memory map exposed as WorldObject)

### Phase 6: Kernel Heap (1k lines)
**Files:** `kernel/src/memory/virtual.rs`, `kernel/src/memory/heap.rs`
**Goals:**
- Virtual memory mapper with allocation tracking
- Linked list allocator for kernel heap
- Heap initialization after paging
- Stack guard pages
- Allocation failure handling
**Laws:** Nothing is Hidden (heap state visible), Nothing is Irrevocable (allocations logged to fossil heap later)

### Phase 7: Serial Driver (1k lines)
**Files:** `kernel/src/drivers/exo/exo_serial.rs`, `kernel/src/drivers/myth/myth_serial.rs`
**Goals:**
- COM1 UART driver (16550A)
- Exo-layer for raw serial I/O
- Myth layer "Wire Whisperer" with fields: voice, hearing, speed, parity, buffer
- AnatomyTable for serial port
- Blocking and non-blocking write/read
**Laws:** Nothing is Hidden (register values visible), Nothing is Magic (baud rate calculation explicit)

### Phase 8: VGA Text Buffer (1k lines)
**Files:** `kernel/src/drivers/exo/exo_vga.rs`, `kernel/src/drivers/myth/myth_vga.rs`, `kernel/src/world/renderer.rs`
**Goals:**
- VGA text mode driver (80x25)
- Double-buffered screen updates
- Myth layer "Green Glass" with fields: cells, palette, cursor, pulse, memory
- Color attribute handling
- Cursor positioning and scrolling
**Laws:** Nothing is Hidden (screen buffer inspectable), Nothing is Owned by System (user owns their display)

### Phase 9: PS/2 Keyboard (1k lines)
**Files:** `kernel/src/drivers/exo/exo_keyboard.rs`, `kernel/src/drivers/myth/myth_keyboard.rs`
**Goals:**
- PS/2 keyboard controller driver
- Scan code set 2 translation
- Myth layer "Keyboard Beast" with fields: teeth, breath, memory, mood, nerves
- Interrupt-driven key event queue
- LED control (CapsLock, NumLock, ScrollLock)
**Laws:** Nothing is Hidden (key states visible), Nothing is Safe by Obscurity (keyboard interrupts logged)

### Phase 10: PIT & TSC Timing (1k lines)
**Files:** `kernel/src/arch/x86_64/pit.rs`, `kernel/src/arch/x86_64/tsc.rs`
**Goals:**
- Programmable Interval Timer initialization
- TSC calibration against PIT
- High-resolution time source
- Monotonic WorldTime counter
- Timer interrupt handler for scheduler tick
**Laws:** Nothing is Hidden (timer state visible), Nothing is Magic (calibration algorithm documented)

---

## PHASES 11-20: Memory & Security Core

### Phase 11: Fossil Pages (COW Snapshots) (1k lines)
**Files:** `kernel/src/memory/fossil_pages.rs`
**Goals:**
- Copy-on-write page snapshot mechanism
- Page reference counting
- Snapshot creation on write
- Page reclamation on snapshot drop
- Integration with buddy allocator
**Laws:** Nothing is Irrevocable (every page mutation snapshotted), Nothing is Hidden (snapshot chain visible)

### Phase 12: Capability Table (1k lines)
**Files:** `kernel/src/memory/capability_table.rs`, `kernel/src/security/unforgeable.rs`
**Goals:**
- Kernel-side UnforgeableThread storage
- CapabilityID generation (monotonic, unique)
- RightsSet bitfield implementation
- Delegation depth tracking
- Parent chain for revocation traversal
**Laws:** Nothing is Safe by Obscurity (capabilities named and logged), Nothing is Owned by System (user holds handles)

### Phase 13: Membrane Gate (1k lines)
**Files:** `kernel/src/security/membrane_gate.rs`
**Goals:**
- Six-layer membrane check implementation
- Layer 1: Capability validity
- Layer 2: Rights check
- Layer 3: Bounds check
- Layer 4: Expiry check
- Layer 5: Invariant check
- Layer 6: Covenant check
- Error types with detailed diagnostics
**Laws:** Nothing is Safe by Obscurity (all checks explicit), Nothing is Magic (error messages name the membrane)

### Phase 14: Invariants System (1k lines)
**Files:** `kernel/src/security/invariant_judge.rs`, `kernel/src/world/object.rs` (partial)
**Goals:**
- Invariant expression AST (typed constraints)
- Soft/Hard/RitualUnlockable modes
- Runtime invariant evaluator
- Violation counting and logging
- Integration with WorldObject structure
**Laws:** Nothing is Hidden (invariants inspectable), Nothing is Magic (constraint evaluation transparent)

### Phase 15: Covenant Engine (1k lines)
**Files:** `kernel/src/security/covenant.rs`
**Goals:**
- Covenant data structure (versioned, signed)
- CovenantArticle parsing and storage
- Rule evaluation engine
- Violation policies (Log/Notify/Block/BlockAndLog)
- Default boot covenant with sensible defaults
**Laws:** Nothing is Owned by System (covenant is user-editable contract), Nothing is Magic (rules are human-readable)

### Phase 16: Naked Mode Ceremony (1k lines)
**Files:** `kernel/src/security/naked_mode.rs`
**Goals:**
- Multi-level NakedModeState (0-3)
- OathObject tracking
- Scoped naked mode (per-object)
- Duration management (UntilExit/Timed/UntilRitual)
- Fossil logging of entry/exit events
**Laws:** Nothing is Safe by Obscurity (danger is visible ceremony), Nothing is Magic (oaths are explicit text)

### Phase 17: Audit Logger (1k lines)
**Files:** `kernel/src/security/audit.rs`
**Goals:**
- Append-only audit fossil writer
- Circular buffer for recent events
- Serial output of critical violations
- Integration with membrane gate
- FossilRef generation for audit trail
**Laws:** Nothing is Hidden (audit trail navigable), Nothing is Irrevocable (audit entries fossilized)

### Phase 18: WorldObject Core (1k lines)
**Files:** `kernel/src/world/object.rs`
**Goals:**
- Complete WorldObject structure
- ObjectID and WorldTime implementation
- BoundaryStr (interned string system)
- FieldMap (typed key-value store)
- MemoryClass enum with decay rules
- describe() method for inspection
**Laws:** Nothing is Hidden (every field inspectable), Nothing is Owned by System (user owns all objects)

### Phase 19: Type System (Gradual Structural) (1k lines)
**Files:** `kernel/src/world/type_system.rs`
**Goals:**
- TypeTag enumeration (primitives, structs, arrays)
- Structural type compatibility checking
- Gradual typing (dynamic + static)
- Type coercion nodes for behavior graphs
- Type error reporting with suggestions
**Laws:** Nothing is Magic (type errors explain the mismatch), Nothing is Hidden (type graph navigable)

### Phase 20: Behavior Graph IR (1k lines)
**Files:** `kernel/src/world/behavior_graph.rs`
**Goals:**
- BehaviorNode kinds (Source/Sink/Transform/etc.)
- BehaviorEdge with type checking
- Graph validation (cycles, type mismatches)
- Lowering to BoundaryIR bytecode
- NativeCodeBlob stub for JIT hot paths
**Laws:** Nothing is Magic (graph is the program), Nothing is Hidden (graph structure inspectable)

---

## PHASES 21-30: Temporal Store & World Runtime

### Phase 21: Fossil Heap Journal (1k lines)
**Files:** `kernel/src/world/fossil_heap.rs`, `kernel/src/image/journal.rs`
**Goals:**
- Append-only fossil journal
- FossilEntry structure with all MutationKind variants
- Content-addressed deduplication
- COW page integration for large objects
- Journal replay on boot
**Laws:** Nothing is Irrevocable (past always reachable), Nothing is Hidden (fossil trail navigable)

### Phase 22: Haunting & Temporal Queries (1k lines)
**Files:** `kernel/src/world/fossil_heap.rs` (extensions)
**Goals:**
- haunt() function for past state retrieval
- diff() for snapshot comparison
- Time-based queries (at WorldTime, before, after)
- Ghost overlay rendering support
- Address syntax parser (@object.now, @object.5_seconds_ago)
**Laws:** Nothing is Irrevocable (past touchable), Nothing is Magic (temporal queries explicit)

### Phase 23: Forgetting & Decay (1k lines)
**Files:** `kernel/src/world/forgetting.rs`
**Goals:**
- MemoryClass decay personalities
- Pressure-based forgetting algorithms
- Sand/Dream/Secret class handling
- Ritual-based preservation
- GC integration with fossil compaction
**Laws:** Nothing is Irrevocable by Accident (decay is named ritual), Nothing is Owned by System (user controls decay)

### Phase 24: Atlas Navigator (1k lines)
**Files:** `kernel/src/world/atlas.rs`
**Goals:**
- Navigable object map
- Hierarchical path resolution (/beasts/keyboard_beast)
- Object discovery and enumeration
- Relationship tracking (edges between objects)
- Search and filtering
**Laws:** Nothing is Hidden (entire world navigable), Nothing is Owned by System (atlas is user-owned view)

### Phase 25: Pulse Loom Scheduler (1k lines)
**Files:** `kernel/src/runtime/pulse_loom.rs`
**Goals:**
- Pulse scheduling (Critical/Timed/Event/Background/Manual/Resonance)
- EffectBudget tracking
- Deadline enforcement
- Priority-based preemption
- Integration with APIC timer
**Laws:** Nothing is Hidden (pulse state visible), Nothing is Magic (scheduling decisions logged)

### Phase 26: Event Loop Dispatcher (1k lines)
**Files:** `kernel/src/runtime/event_loop.rs`
**Goals:**
- World event distribution
- IRQ → WorldEvent translation
- Manual pulse triggering
- Resonance event stub
- Event queue management
**Laws:** Nothing is Hidden (event stream visible), Nothing is Magic (event routing explicit)

### Phase 27: Graph Interpreter (1k lines)
**Files:** `kernel/src/runtime/graph_interpreter.rs`
**Goals:**
- BehaviorGraph execution engine
- Node evaluation (Source/Sink/Transform/etc.)
- Edge traversal with type checking
- Buffer node implementation (circular buffers)
- Threshold branching logic
**Laws:** Nothing is Magic (graph execution transparent), Nothing is Hidden (interpreter state inspectable)

### Phase 28: Native Lowering Stub (1k lines)
**Files:** `kernel/src/runtime/native_lowering.rs`
**Goals:**
- BoundaryIR → x86_64 lowering stub
- Hot path detection
- JIT compilation framework (placeholder)
- NativeCodeBlob lifecycle
- Fallback to interpreter
**Laws:** Nothing is Magic (lowering decisions logged), Nothing is Hidden (IR visible alongside native)

### Phase 29: Resonance Protocol Stub (1k lines)
**Files:** `kernel/src/runtime/resonance.rs`
**Goals:**
- Inter-World communication stub
- WorldAddr addressing
- Message serialization format
- Incoming resonance event handling
- Future network integration points
**Laws:** Nothing is Hidden (resonance events logged), Nothing is Owned by System (remote worlds are peers)

### Phase 30: Boot World Initialization (1k lines)
**Files:** `kernel/src/image/boot_world.rs`
**Goals:**
- Seed World creation on first boot
- Kernel objects as WorldObjects
- Hardware mythic objects registration
- Default capabilities for boot pulse
- Initial covenant installation
**Laws:** Nothing is Owned by System (even kernel is inspectable), Nothing is Hidden (boot world navigable)

---

## PHASES 31-40: PCI & Hardware Drivers (Exo-Layer)

### Phase 31: PCI Bus Scanner (1k lines)
**Files:** `kernel/src/drivers/pci.rs`
**Goals:**
- PCI configuration space access (port-mapped)
- Bus/device/function enumeration
- BAR (Base Address Register) detection
- Device class identification
- PCI device list as WorldObject
**Laws:** Nothing is Hidden (PCI tree inspectable), Nothing is Magic (BAR calculation explicit)

### Phase 32: Exo-Layer Framework (1k lines)
**Files:** `kernel/src/drivers/exo/mod.rs`, `kernel/src/drivers/mod.rs`
**Goals:**
- RawDevice trait definition
- ExoLayer<D> generic multiplexer
- ResourcePartition allocation
- Observer registration for IRQ fan-out
- IOMMU mapping stub
**Laws:** Nothing is Hidden (partitions visible), Nothing is Safe by Obscurity (multiplexing explicit)

### Phase 33: AHCI Exo-Layer (1k lines)
**Files:** `kernel/src/drivers/exo/exo_ahci.rs`
**Goals:**
- AHCI controller initialization
- Port discovery and configuration
- Command list and FIS area setup
- DMA buffer management
- partition_rx_ring equivalent for storage
**Laws:** Nothing is Hidden (AHCI registers visible), Nothing is Magic (command slot allocation explicit)

### Phase 34: e1000 Exo-Layer (1k lines)
**Files:** `kernel/src/drivers/exo/exo_e1000.rs`
**Goals:**
- Intel e1000 NIC initialization
- RX/TX ring descriptor setup
- MAC address reading
- Link state detection
- partition_rx_ring() for network partitions
- register_irq_observer() for packet events
**Laws:** Nothing is Hidden (ring descriptors visible), Nothing is Safe by Obscurity (DMA regions logged)

### Phase 35: AHCI Myth Layer (1k lines)
**Files:** `kernel/src/drivers/myth/myth_ahci.rs`
**Goals:**
- "Disk Octopus" mythic object
- Fields: arms (ports), signature, command_list, fis_area, dma_buffers
- AnatomyTable with metal fields (ABAR, PI, VS, etc.)
- Live register reads
- Safe/unsafe action definitions
**Laws:** Nothing is Hidden (octopus anatomy inspectable), Nothing is Magic (register names from spec)

### Phase 36: e1000 Myth Layer (1k lines)
**Files:** `kernel/src/drivers/myth/myth_e1000.rs`
**Goals:**
- "Packet Goat" mythic object
- Fields: rx_ring, tx_ring, mac_address, link_state, interrupt_cause
- AnatomyTable with metal fields (CTRL, STATUS, RDBAL, TDBAL, etc.)
- Bitfield descriptions for key registers
- Interrupt cause decoding
**Laws:** Nothing is Hidden (goat anatomy inspectable), Nothing is Safe by Obscurity (interrupt causes named)

### Phase 37: APIC & Local Interrupts (1k lines)
**Files:** `kernel/src/arch/x86_64/apic.rs`
**Goals:**
- Local APIC detection and mapping
- APIC timer configuration
- Spurious interrupt handler
- IPI (Inter-Processor Interrupt) stub
- APIC registers as Myth fields
**Laws:** Nothing is Hidden (APIC state visible), Nothing is Magic (timer calibration documented)

### Phase 38: IOMMU/VT-d Stub (1k lines)
**Files:** `kernel/src/arch/x86_64/iommu.rs`
**Goals:**
- DMAR table parsing
- IOMMU detection
- DMA remapping stub
- Logging when IOMMU unavailable
- Safety boundaries for DMA
**Laws:** Nothing is Safe by Obscurity (DMA dangers named), Nothing is Hidden (IOMMU status visible)

### Phase 39: Syscall Surface (Minimal) (1k lines)
**Files:** `kernel/src/arch/x86_64/syscall.rs`
**Goals:**
- syscall/sysret instruction setup
- Minimal syscall number space
- User-mode entry/exit
- Capability handle passing convention
- Error return conventions
**Laws:** Nothing is Magic (syscall ABI explicit), Nothing is Hidden (syscall table inspectable)

### Phase 40: Driver Integration Tests (1k lines)
**Files:** `kernel/tests/driver_tests.rs`, `kernel/tests/capability_tests.rs`
**Goals:**
- Unit tests for Exo-Layer partitioning
- Integration tests for Myth layer anatomy
- Capability creation/delegation tests
- Membrane gate test suite
- Serial output of test results
**Laws:** Nothing is Hidden (test results visible), Nothing is Safe by Obscurity (failure modes explicit)

---

## PHASES 41-50: Interaction Surface (Word Mode)

### Phase 41: Word Surface Parser (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (new)
**Goals:**
- Command line parser for VGA text mode
- Tokenization with object references
- Ritual name recognition
- Capability reference parsing
- Fossil address syntax (@object.time)
**Laws:** Nothing is Magic (parse errors explain the issue), Nothing is Hidden (parse tree visible)

### Phase 42: Object Inspection Cards (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `<name>` command: open object card
- describe() output formatting
- Field value rendering
- Type tag display
- Navigation hints
**Laws:** Nothing is Hidden (full object visible), Nothing is Owned by System (user inspects anything)

### Phase 43: Anatomy View Command (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `anatomy <name>` command
- Three-panel layout (Myth/Metal/Security)
- Live myth field updates
- Register hex dumps
- Capability list for object
**Laws:** Nothing is Hidden (myth+metal side-by-side), Nothing is Magic (register names from anatomy table)

### Phase 44: Haunt Command (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `haunt <name>` command
- Ghost overlay of past versions
- `haunt <name> at <WorldTime>` specific query
- Diff visualization
- Temporal navigation controls
**Laws:** Nothing is Irrevocable (past reachable), Nothing is Hidden (ghost layers visible)

### Phase 45: Memory Class Rituals (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `bless <name>`: promote to Immortal
- `forget <name>`: begin decay ritual
- Confirmation prompts
- Fossil logging of class changes
- Visual feedback on success
**Laws:** Nothing is Irrevocable by Accident (rituals are explicit), Nothing is Magic (decay rules visible)

### Phase 46: Object Fork & Merge (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `fork <name>`: create derived copy
- `merge <a> <b>`: fold ghost into present
- Field-level merge conflict resolution
- New object ID assignment
- Fossil entries for fork/merge events
**Laws:** Nothing is Irrevocable (fork preserves original), Nothing is Hidden (merge decisions logged)

### Phase 47: Naked Mode Commands (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `naked level=<N>` command
- Oath prompting for levels 2-3
- Visual border color changes
- Scope declaration
- `clothe` command to exit
**Laws:** Nothing is Safe by Obscurity (ceremony is visible), Nothing is Magic (oaths are explicit text)

### Phase 48: Behavior Binding (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `bind <a>.<field> -> <b>.<field>` command
- Edge creation in behavior graph
- Type compatibility checking
- Live binding activation
- Unbind command
**Laws:** Nothing is Magic (binding is explicit transformation), Nothing is Hidden (edge visible in atlas)

### Phase 49: Status Commands (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `pulses`: show active Pulse Loom state
- `caps`: show current capabilities
- `covenant`: open Covenant inspector
- `atlas`: open Atlas navigator
- `fossils`: open Fossil Heap browser
**Laws:** Nothing is Hidden (all state visible), Nothing is Owned by System (user owns the view)

### Phase 50: Help & Discovery (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `help` command with full command list
- Context-sensitive suggestions
- Object discovery hints
- Error message improvements
- Tab completion stub
**Laws:** Nothing is Magic (help explains the path forward), Nothing is Hidden (all commands discoverable)

---

## PHASES 51-60: Graph & Anatomy Surfaces

### Phase 51: Framebuffer Mode Switch (1k lines)
**Files:** `kernel/src/drivers/exo/exo_vga.rs` (extensions), `kernel/src/image/interaction_graph.rs`
**Goals:**
- VGA Mode 13h (320x200, 256 colors) setup
- Linear framebuffer access
- Double buffering for smooth updates
- Mode switching from text to graphics
- Palette configuration
**Laws:** Nothing is Hidden (framebuffer memory visible), Nothing is Magic (mode registers documented)

### Phase 52: Graph Renderer (1k lines)
**Files:** `kernel/src/image/interaction_graph.rs`
**Goals:**
- Atlas graph rendering
- Node positioning (force-directed or grid)
- Edge drawing with curvature
- Color coding by type/rights
- Zoom and pan controls
**Laws:** Nothing is Hidden (entire world as graph), Nothing is Magic (layout algorithm explicit)

### Phase 53: Graph Navigation (1k lines)
**Files:** `kernel/src/image/interaction_graph.rs` (extensions)
**Goals:**
- Arrow key navigation
- Node selection and highlighting
- Enter to inspect (switch to card view)
- 'e' to create edge
- Right-click context menu stub
**Laws:** Nothing is Hidden (navigation state visible), Nothing is Magic (controls documented on-screen)

### Phase 54: Anatomy Surface Layout (1k lines)
**Files:** `kernel/src/image/interaction_anatomy.rs`
**Goals:**
- Three-panel anatomy layout
- Myth panel: live field values with animations
- Metal panel: register hex dumps with bitfields
- Security panel: capabilities, invariants, pulses
- Synchronized scrolling
**Laws:** Nothing is Hidden (all three layers simultaneous), Nothing is Magic (panel boundaries explicit)

### Phase 55: Live Myth Updates (1k lines)
**Files:** `kernel/src/image/interaction_anatomy.rs` (extensions)
**Goals:**
- Real-time myth field refresh
- Animation for changing values
- Color coding for danger levels
- Interactive field editing (with membrane check)
- Refresh rate control
**Laws:** Nothing is Hidden (live hardware state), Nothing is Safe by Obscurity (danger visible in real-time)

### Phase 56: Metal Register Inspection (1k lines)
**Files:** `kernel/src/image/interaction_anatomy.rs` (extensions)
**Goals:**
- Live MMIO/port I/O reads
- Bitfield expansion on hover/select
- Register write capability (in Naked Mode)
- Hex/decimal/binary toggle
- Historical value trail
**Laws:** Nothing is Hidden (raw hardware visible), Nothing is Magic (register meanings documented)

### Phase 57: Security Panel (1k lines)
**Files:** `kernel/src/image/interaction_anatomy.rs` (extensions)
**Goals:**
- Capability list for selected object
- Invariant status (pass/fail/violations)
- Active pulses affecting object
- Membrane definition preview
- Audit trail snippets
**Laws:** Nothing is Safe by Obscurity (security posture visible), Nothing is Hidden (all checks inspectable)

### Phase 58: Mode Switching Commands (1k lines)
**Files:** `kernel/src/image/mod.rs`
**Goals:**
- `mode word` / `mode graph` / `mode anatomy`
- State preservation across modes
- Mode-specific help
- Default mode configuration
- Mode transition animations
**Laws:** Nothing is Magic (mode switches are explicit), Nothing is Hidden (mode state visible)

### Phase 59: Proof Objects (1k lines)
**Files:** `kernel/src/security/proof.rs` (new)
**Goals:**
- ProofObject structure for verified properties
- Invariant satisfaction proofs
- Capability chain proofs
- Temporal consistency proofs
- `proofs` command to display
**Laws:** Nothing is Hidden (proofs are inspectable), Nothing is Magic (proof construction explicit)

### Phase 60: Resonance Status (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `resonance` command implementation
- Remote World connection status
- Incoming/outgoing message queue
- WorldAddr directory
- Future network hooks
**Laws:** Nothing is Hidden (resonance state visible), Nothing is Owned by System (remote worlds are peers)

---

## PHASES 61-70: Image Persistence & GC

### Phase 61: World Snapshot Format (1k lines)
**Files:** `kernel/src/image/snapshot.rs`
**Goals:**
- Complete World image serialization
- Object graph with edges
- Capability table snapshot
- Fossil heap checkpoint
- Version tagging
**Laws:** Nothing is Irrevocable (snapshots are fossils), Nothing is Hidden (snapshot format documented)

### Phase 62: Boot Image Replay (1k lines)
**Files:** `kernel/src/image/boot_world.rs` (extensions)
**Goals:**
- Detect existing image on storage
- Deserialize snapshot
- Rehydrate capabilities
- Restore behavior graphs
- Continue WorldTime from checkpoint
**Laws:** Nothing is Irrevocable (world survives reboot), Nothing is Magic (replay is explicit deserialization)

### Phase 63: Fossil Compaction GC (1k lines)
**Files:** `kernel/src/image/compaction.rs`
**Goals:**
- Mark-and-sweep for fossil heap
- Honor MemoryClass decay rules
- Content-addressed deduplication
- Tombstone handling for oblivion
- Compaction progress reporting
**Laws:** Nothing is Irrevocable by Accident (GC respects memory classes), Nothing is Hidden (compaction visible)

### Phase 64: Oblivion Ritual (1k lines)
**Files:** `kernel/src/world/fossil_heap.rs` (extensions)
**Goals:**
- oblivion() function implementation
- Level 3 Naked Mode requirement
- Oath confirmation
- Second confirmation step
- Tombstone creation (cannot fully erase)
**Laws:** Nothing is Irrevocable by Accident (oblivion is explicit ritual), Nothing is Safe by Obscurity (danger is ceremony)

### Phase 65: Storage Backend Stub (1k lines)
**Files:** `kernel/src/drivers/storage_backend.rs` (new)
**Goals:**
- Abstract storage interface
- AHCI-backed block device stub
- In-memory fallback for testing
- Read/write sector operations
- Error handling for media failures
**Laws:** Nothing is Hidden (storage state visible), Nothing is Magic (block operations explicit)

### Phase 66: Image Save Command (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `save` command to persist world
- Named snapshots
- Incremental save (only changed fossils)
- Save progress indicator
- Verification after save
**Laws:** Nothing is Irrevocable (save creates fossil), Nothing is Hidden (save process visible)

### Phase 67: Auto-Save Policy (1k lines)
**Files:** `kernel/src/image/autosave.rs` (new)
**Goals:**
- Periodic auto-save pulse
- Configurable interval
- Save on significant mutations
- Recovery from incomplete saves
- User notification
**Laws:** Nothing is Irrevocable by Accident (auto-save prevents loss), Nothing is Magic (policy is covenant article)

### Phase 68: Fossil Browser UI (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `fossils` command full implementation
- Chronological fossil listing
- Filter by object/mutation type
- Jump to specific WorldTime
- Preview fossil content
**Laws:** Nothing is Hidden (entail history navigable), Nothing is Irrevocable (fossils always reachable)

### Phase 69: Memory Pressure Monitor (1k lines)
**Files:** `kernel/src/memory/pressure.rs` (new)
**Goals:**
- Physical memory usage tracking
- Fossil heap size monitoring
- Pressure thresholds
- Automatic decay triggering
- User warnings before aggressive GC
**Laws:** Nothing is Hidden (pressure visible), Nothing is Irrevocable by Accident (decay is warned)

### Phase 70: Covenant Update Ritual (1k lines)
**Files:** `kernel/src/security/covenant.rs` (extensions)
**Goals:**
- Covenant versioning
- Update proposal mechanism
- User confirmation ritual
- Fossil logging of changes
- Rollback to previous version
**Laws:** Nothing is Owned by System (user controls covenant), Nothing is Irrevocable (history preserved)

---

## PHASES 71-80: Advanced Security & Effects

### Phase 71: Effect Budget Enforcement (1k lines)
**Files:** `kernel/src/runtime/pulse_loom.rs` (extensions)
**Goals:**
- Live effect counter during pulse execution
- max_memory_mutations enforcement
- max_fossil_writes limiting
- max_membrane_crossings tracking
- Pulse termination on budget exhaustion
**Laws:** Nothing is Safe by Obscurity (effects are budgeted), Nothing is Magic (budget exhaustion logged)

### Phase 72: Deadline Scheduler (1k lines)
**Files:** `kernel/src/runtime/pulse_loom.rs` (extensions)
**Goals:**
- Deadline tracking for timed pulses
- Earliest-deadline-first scheduling
- Missed deadline handling
- Deadline extension ritual
- Statistical tracking of deadline adherence
**Laws:** Nothing is Hidden (deadline state visible), Nothing is Magic (scheduling decisions logged)

### Phase 73: Revocation System (1k lines)
**Files:** `kernel/src/security/unforgeable.rs` (extensions)
**Goals:**
- Capability revocation by parent
- Cascade revocation through delegation chain
- Immediate effect on membrane gate
- Revocation fossil logging
- Handle invalidation
**Laws:** Nothing is Safe by Obscurity (revocation is explicit), Nothing is Hidden (revocation chain visible)

### Phase 74: Delegation Depth Control (1k lines)
**Files:** `kernel/src/security/unforgeable.rs` (extensions)
**Goals:**
- DelegationDepth enforcement
- Depth decrement on delegate
- Zero-depth rejection
- Depth inspection in capability cards
- Depth-based trust metrics
**Laws:** Nothing is Safe by Obscurity (delegation limits explicit), Nothing is Hidden (depth visible)

### Phase 75: Bounds Checking Enhancement (1k lines)
**Files:** `kernel/src/security/membrane_gate.rs` (extensions)
**Goals:**
- MemBounds implementation (start/length)
- Address validation on MAP rights
- Range intersection checking
- Bounds violation logging
- Dynamic bounds adjustment ritual
**Laws:** Nothing is Safe by Obscurity (bounds are enforced), Nothing is Hidden (bounds visible in capability)

### Phase 76: Expiry System (1k lines)
**Files:** `kernel/src/security/unforgeable.rs` (extensions)
**Goals:**
- WorldTime-based expiry
- Expiry check in membrane gate
- Expiry extension ritual
- Expired capability visualization
- Auto-cleanup of expired threads
**Laws:** Nothing is Safe by Obscurity (expiry is explicit), Nothing is Hidden (expiry visible)

### Phase 77: Invariant Expression Language (1k lines)
**Files:** `kernel/src/security/invariant_judge.rs` (extensions)
**Goals:**
- Richer InvariantExpr AST
- Arithmetic comparisons
- Logical operators (AND/OR/NOT)
- Field path references
- Quantifiers (forall/exists stub)
**Laws:** Nothing is Magic (expressions are typed), Nothing is Hidden (expression tree inspectable)

### Phase 78: Ritual Unlockable Invariants (1k lines)
**Files:** `kernel/src/security/invariant_judge.rs` (extensions)
**Goals:**
- RitualID tracking for invariants
- Unlock ritual execution
- Temporary invariant suspension
- Re-locking after ritual
- Audit trail of unlocks
**Laws:** Nothing is Irrevocable by Accident (unlock is ritual), Nothing is Safe by Obscurity (danger is ceremony)

### Phase 79: Covenant Rule Engine (1k lines)
**Files:** `kernel/src/security/covenant.rs` (extensions)
**Goals:**
- All CovenantRule variants implemented
- CapabilityRequired enforcement
- NakedLevelRequired checking
- RitualRequired verification
- DecayAfter policy application
- ForbidCompletely hard blocks
**Laws:** Nothing is Owned by System (covenant rules bind system too), Nothing is Magic (rule evaluation explicit)

### Phase 80: Audit Trail Queries (1k lines)
**Files:** `kernel/src/security/audit.rs` (extensions)
**Goals:**
- Audit fossil search
- Filter by actor/object/action
- Timeline reconstruction
- Export audit trail (serial)
- Integrity verification stub
**Laws:** Nothing is Hidden (audit is queryable), Nothing is Irrevocable (audit persists)

---

## PHASES 81-90: Polish & Completeness

### Phase 81: Error Message Overhaul (1k lines)
**Files:** All modules (error type enhancements)
**Goals:**
- Human-readable error messages
- Membrane error naming (which layer failed)
- Suggested remediation steps
- Covenant article citation
- Fossil reference in errors
**Laws:** Nothing is Magic (errors explain the path forward), Nothing is Hidden (error context complete)

### Phase 82: Boot Banner Polish (1k lines)
**Files:** `kernel/src/main.rs` (extensions)
**Goals:**
- ASCII art banner with counts
- Object/invariant/capability/fossil counts
- Quick start suggestions
- Serial boot log completeness
- Boot timing statistics
**Laws:** Nothing is Hidden (system state on boot), Nothing is Magic (counts are accurate)

### Phase 83: Anatomy Table Completion (1k lines)
**Files:** All driver myth modules
**Goals:**
- Complete AnatomyTable for all 5 devices
- All myth_fields with live_value functions
- All metal_fields with offsets/bitfields
- safe_actions/unsafe_actions populated
- interrupt_sources/dma_regions filled
**Laws:** Nothing is Hidden (full anatomy for all devices), Nothing is Safe by Obscurity (actions classified)

### Phase 84: Documentation Pass (1k lines of docs)
**Files:** All modules (/// comments)
**Goals:**
- Rustdoc for all public items
- DESIGN NOTE comments for trade-offs
- SAFETY comments verified
- MODULE SIZE comments updated
- Cross-references between modules
**Laws:** Nothing is Hidden (documentation complete), Nothing is Magic (design decisions explained)

### Phase 85: Size Audit (1k lines of analysis)
**Files:** `SIZE_AUDIT.md` (new)
**Goals:**
- Line count per module
- Budget vs actual tracking
- Optimization opportunities
- Dead code elimination
- Target: <100k lines verification
**Laws:** Nothing is Hidden (size is tracked), Nothing is Magic (budget discipline explicit)

### Phase 86: Performance Profiling Stub (1k lines)
**Files:** `kernel/src/runtime/profiler.rs` (new)
**Goals:**
- Pulse execution timing
- Fossil write latency tracking
- Membrane gate overhead measurement
- GC pause time recording
- Profile data export
**Laws:** Nothing is Hidden (performance visible), Nothing is Magic (profiling is explicit)

### Phase 87: Debug Commands (1k lines)
**Files:** `kernel/src/image/interaction_word.rs` (extensions)
**Goals:**
- `debug gdt` / `debug idt` / `debug pages`
- Register dumps
- Memory map visualization
- Interrupt statistics
- Heap fragmentation report
**Laws:** Nothing is Hidden (debug info available), Nothing is Safe by Obscurity (debug requires naked mode)

### Phase 88: Test Suite Expansion (1k lines)
**Files:** `kernel/tests/*.rs` (multiple files)
**Goals:**
- Invariant tests
- Fossil heap tests
- Membrane gate tests
- Behavior graph tests
- Integration tests for all commands
**Laws:** Nothing is Hidden (test results visible), Nothing is Magic (test failures explain why)

### Phase 89: Recovery Mechanisms (1k lines)
**Files:** `kernel/src/image/recovery.rs` (new)
**Goals:**
- Corrupt snapshot detection
- Partial recovery from damaged journal
- Fallback to minimal boot world
- User notification of recovery
- Recovery fossil logging
**Laws:** Nothing is Irrevocable by Accident (recovery is possible), Nothing is Hidden (recovery process visible)

### Phase 90: Internationalization Stub (1k lines)
**Files:** `kernel/src/i18n.rs` (new)
**Goals:**
- UTF-8 support in BoundaryStr
- Unicode rendering in VGA
- Locale-aware number formatting
- Translation framework stub
- English default with hooks
**Laws:** Nothing is Hidden (encoding explicit), Nothing is Magic (translation is typed)

---

## PHASES 91-100: Advanced Features & Future-Proofing

### Phase 91: Multi-Monitor Stub (1k lines)
**Files:** `kernel/src/drivers/exo/exo_vga.rs` (extensions)
**Goals:**
- Multiple VGA adapter detection
- Secondary monitor initialization
- Extended desktop mode
- Per-monitor WorldObject
- Spanning/cloning options
**Laws:** Nothing is Hidden (monitor topology visible), Nothing is Magic (multi-head setup explicit)

### Phase 92: USB HID Skeleton (1k lines)
**Files:** `kernel/src/drivers/exo/exo_usb.rs` (new)
**Goals:**
- UHCI/OHCI controller stub
- USB device enumeration
- HID class detection
- Keyboard/mouse placeholder
- Future expansion points
**Laws:** Nothing is Hidden (USB tree visible), Nothing is Magic (enumeration explicit)

### Phase 93: Network Stack Skeleton (1k lines)
**Files:** `kernel/src/network/mod.rs` (new)
**Goals:**
- Packet buffer management
- Ethernet frame parsing stub
- IP header parsing stub
- UDP/TCP placeholders
- Resonance protocol future hooks
**Laws:** Nothing is Hidden (network state visible), Nothing is Magic (packet handling explicit)

### Phase 94: Cryptography Primitives (1k lines)
**Files:** `kernel/src/crypto/mod.rs` (new)
**Goals:**
- SHA-256 implementation
- HMAC-SHA256
- Ed25519 signature verification stub
- Covenant signing support
- Secure random number generator (RDRAND)
**Laws:** Nothing is Safe by Obscurity (crypto is explicit), Nothing is Hidden (algorithm choices documented)

### Phase 95: Secure Boot Stub (1k lines)
**Files:** `kernel/src/security/secure_boot.rs` (new)
**Goals:**
- Covenant signature verification
- Boot image hash checking
- Key enrollment ritual
- Secure boot status WorldObject
- Fallback to insecure boot with warning
**Laws:** Nothing is Safe by Obscurity (boot integrity checked), Nothing is Owned by System (user controls keys)

### Phase 96: Power Management (ACPI Skeleton) (1k lines)
**Files:** `kernel/src/arch/x86_64/acpi.rs` (new)
**Goals:**
- RSDP/XSDT parsing
- FADT interpretation
- Sleep state stubs (S1-S5)
- Reboot/shutdown commands
- Battery status (if available)
**Laws:** Nothing is Hidden (ACPI tables visible), Nothing is Magic (power states documented)

### Phase 97: SMP Bootstrap (1k lines)
**Files:** `kernel/src/arch/x86_64/smp.rs` (new)
**Goals:**
- MADT parsing for APIC IDs
- AP startup via INIT/SIPI
- Per-CPU GDT/IDT/TSS
- Spinlock for early synchronization
- CPU online WorldObject per core
**Laws:** Nothing is Hidden (CPU topology visible), Nothing is Magic (AP startup explicit)

### Phase 98: Per-CPU Pulse Loom (1k lines)
**Files:** `kernel/src/runtime/pulse_loom.rs` (extensions)
**Goals:**
- Multi-core pulse scheduling
- Load balancing across CPUs
- Affinity setting for pulses
- Cross-CPU IPI for wake-up
- Per-CPU effect budgets
**Laws:** Nothing is Hidden (scheduler state per-CPU), Nothing is Magic (load balancing explicit)

### Phase 99: Resonance Network Prototype (1k lines)
**Files:** `kernel/src/runtime/resonance.rs` (extensions)
**Goals:**
- UDP-based resonance messaging
- World discovery broadcast
- Remote object reference proxy stub
- Message authentication
- Latency measurement
**Laws:** Nothing is Hidden (network activity visible), Nothing is Owned by System (remote worlds are peers)

### Phase 100: Final Integration & Launch (1k lines)
**Files:** All modules (final polish)
**Goals:**
- Full system integration test
- Boot to interaction surface verification
- All 30+ commands functional
- Anatomy tables complete for all devices
- Boot banner shows accurate counts
- Size audit confirms <100k lines
- Documentation complete
- Ready for real-world implications
**Laws:** All Five Laws fully realized

---

## EXECUTION STRATEGY

Each phase introduces ~1,000 lines through:
1. **Shell-driven file creation**: `mkdir -p`, `cat > file.rs << 'EOF'`
2. **Incremental compilation**: `cargo build --target x86_64-unknown-none` after each phase
3. **Verification**: QEMU boot test, serial output check
4. **Documentation**: Update SIZE_AUDIT.md with cumulative line count

### Shell Execution Pattern per Phase:
```bash
# Phase N execution
cd /workspace/boundaryos
# Create necessary directories
mkdir -p kernel/src/new_module
# Create/update files with heredocs or str_replace
# Build and verify
cargo build --target x86_64-unknown-none
# Test in QEMU
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/boundaryos -serial stdio
# Record progress
echo "Phase N complete: $(find kernel/src -name '*.rs' | xargs wc -l)" >> SIZE_AUDIT.md
```

### Cumulative Milestones:
- **Phase 10**: Boot to serial, basic drivers
- **Phase 20**: Security core functional
- **Phase 30**: Temporal store operational
- **Phase 40**: All hardware drivers with anatomy
- **Phase 50**: Word surface complete (30+ commands)
- **Phase 60**: All three interaction surfaces working
- **Phase 70**: Persistence and GC complete
- **Phase 80**: Advanced security features done
- **Phase 90**: Polish and documentation complete
- **Phase 100**: Production-ready BoundaryOS

---

## REAL-WORLD READINESS CHECKLIST

After Phase 100, BoundaryOS will have:
- ✓ Complete bare-metal x86_64 kernel in Rust
- ✓ Capability-based security with six-layer membrane
- ✓ Temporal object store with haunting capability
- ✓ Five mythic hardware drivers with full anatomy
- ✓ Three interaction surfaces (Word/Graph/Anatomy)
- ✓ Covenant-based configuration system
- ✓ Naked Mode ceremony for unsafe operations
- ✓ Fossil heap with GC and oblivion rituals
- ✓ Pulse Loom scheduler with effect budgets
- ✓ Behavior graph programming model
- ✓ Full audit trail and proof objects
- ✓ <100k lines of well-documented code
- ✓ All five philosophical laws upheld

**The machine is alive. The user is inside.**
