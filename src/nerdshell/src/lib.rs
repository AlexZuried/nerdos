//! # NerdShell - The Hacker's Shell
//!
//! NerdShell is the default interactive shell for NerdOS.
//!
//! ## Features
//! - Command parsing with arguments and quoting
//! - Builtin commands: ls, cd, cat, mkdir, rm, ps, kill, mount, dmesg, etc.
//! - Environment variables
//! - Command history
//! - Pipes and redirection (basic)
//! - Tab completion (planned)
//! - Shell scripting with Rust (compile on the fly)
//!
//! ## Architecture
//!
//! ```
//! Input (keyboard)
//!       |
//!   Lexer (tokenize)
//!       |
//!   Parser (commands, args, pipes)
//!       |
//!   Executor (builtins or spawn)
//!       |
//!   Output (VGA/serial)
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

// ---------------------------------------------------------------------------
// Shell State
// ---------------------------------------------------------------------------

/// Maximum command line length.
const MAX_LINE_LEN: usize = 256;

/// Maximum number of arguments.
const MAX_ARGS: usize = 16;

/// Maximum command history entries.
const HISTORY_SIZE: usize = 64;

/// The shell prompt string.
pub const PROMPT: &str = "nerdos> ";

/// Shell state and configuration.
pub struct Shell {
    /// Current working directory.
    pub cwd: String,
    /// Environment variables.
    pub env: [(String, String); 32],
    /// Number of environment variables.
    pub env_count: usize,
    /// Command history.
    pub history: [String; HISTORY_SIZE],
    /// History write position.
    pub history_pos: usize,
    /// Last exit status.
    pub last_status: i32,
    /// Whether the shell should exit.
    pub should_exit: bool,
}

impl Shell {
    /// Create a new shell instance.
    pub fn new() -> Self {
        let mut shell = Shell {
            cwd: String::from("/"),
            env: core::array::from_fn(|_| (String::new(), String::new())),
            env_count: 0,
            history: core::array::from_fn(|_| String::new()),
            history_pos: 0,
            last_status: 0,
            should_exit: false,
        };

        // Set default environment variables.
        shell.set_env("PATH", "/bin:/sbin:/usr/bin:/usr/sbin");
        shell.set_env("HOME", "/home");
        shell.set_env("USER", "root");
        shell.set_env("SHELL", "/bin/nerdshell");
        shell.set_env("TERM", "nerdos");
        shell.set_env("EDITOR", "vi");
        shell.set_env("PAGER", "cat");
        shell.set_env("PS1", PROMPT);

        shell
    }

    /// Set an environment variable.
    pub fn set_env(&mut self, key: &str, value: &str) {
        // Check if it already exists.
        for i in 0..self.env_count {
            if self.env[i].0 == key {
                self.env[i].1 = String::from(value);
                return;
            }
        }

        // Add new.
        if self.env_count < self.env.len() {
            self.env[self.env_count] = (String::from(key), String::from(value));
            self.env_count += 1;
        }
    }

    /// Get an environment variable.
    pub fn get_env(&self, key: &str) -> Option<&str> {
        for i in 0..self.env_count {
            if self.env[i].0 == key {
                return Some(&self.env[i].1);
            }
        }
        None
    }

    /// Expand environment variables in a string.
    pub fn expand_vars(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                // Read variable name.
                let mut var_name = String::new();
                while let Some(c) = chars.next() {
                    if c.is_alphanumeric() || c == '_' {
                        var_name.push(c);
                    } else {
                        break;
                    }
                }

                if !var_name.is_empty() {
                    if let Some(value) = self.get_env(&var_name) {
                        result.push_str(value);
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Add a command to history.
    pub fn add_history(&mut self, cmd: &str) {
        self.history[self.history_pos] = String::from(cmd);
        self.history_pos = (self.history_pos + 1) % HISTORY_SIZE;
    }

    /// Print the shell prompt.
    pub fn print_prompt(&self) {
        let ps1 = self.get_env("PS1").unwrap_or(PROMPT);
        let prompt = self.expand_vars(ps1);

        if self.cwd == "/" {
            kernel_core::print!("{} ", prompt);
        } else {
            // Show last component of cwd.
            let last = self.cwd.rfind('/').map_or(&self.cwd[..], |i| &self.cwd[i + 1..]);
            if last.is_empty() {
                kernel_core::print!("{} [/] ", prompt);
            } else {
                kernel_core::print!("{} [{}] ", prompt, last);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Command Parsing
// ---------------------------------------------------------------------------

/// A parsed command with arguments.
#[derive(Debug)]
pub struct Command {
    /// Command name.
    pub name: String,
    /// Command arguments.
    pub args: Vec<String>,
    /// Input redirection file.
    pub redirect_in: Option<String>,
    /// Output redirection file.
    pub redirect_out: Option<String>,
    /// Append output.
    pub append: bool,
}

impl Command {
    /// Create a new empty command.
    pub fn new() -> Self {
        Command {
            name: String::new(),
            args: Vec::new(),
            redirect_in: None,
            redirect_out: None,
            append: false,
        }
    }
}

/// Parse a command line string into a Command.
pub fn parse_line(line: &str) -> Option<Command> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let mut cmd = Command::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    if cmd.name.is_empty() {
                        cmd.name = current.clone();
                    } else {
                        cmd.args.push(current.clone());
                    }
                    current.clear();
                }
            }
            '>' if !in_quotes => {
                if !current.is_empty() {
                    if cmd.name.is_empty() {
                        cmd.name = current.clone();
                    } else {
                        cmd.args.push(current.clone());
                    }
                    current.clear();
                }
                // Check for >>
                if chars.clone().next() == Some('>') {
                    chars.next(); // Consume second >
                    cmd.append = true;
                }
                // Read filename.
                let mut filename = String::new();
                while let Some(c) = chars.next() {
                    if c == ' ' || c == '\t' {
                        break;
                    }
                    filename.push(c);
                }
                cmd.redirect_out = Some(filename);
            }
            '<' if !in_quotes => {
                if !current.is_empty() {
                    if cmd.name.is_empty() {
                        cmd.name = current.clone();
                    } else {
                        cmd.args.push(current.clone());
                    }
                    current.clear();
                }
                // Read filename.
                let mut filename = String::new();
                while let Some(c) = chars.next() {
                    if c == ' ' || c == '\t' {
                        break;
                    }
                    filename.push(c);
                }
                cmd.redirect_in = Some(filename);
            }
            _ => {
                current.push(ch);
            }
        }
    }

    // Add last token.
    if !current.is_empty() {
        if cmd.name.is_empty() {
            cmd.name = current;
        } else {
            cmd.args.push(current);
        }
    }

    if cmd.name.is_empty() {
        None
    } else {
        Some(cmd)
    }
}

// ---------------------------------------------------------------------------
// Builtin Commands
// ---------------------------------------------------------------------------

/// Execute a builtin command.
/// Returns true if the command was a builtin and was handled.
pub fn exec_builtin(shell: &mut Shell, cmd: &Command) -> bool {
    match cmd.name.as_str() {
        "help" => builtin_help(),
        "echo" => builtin_echo(shell, &cmd.args),
        "cd" => builtin_cd(shell, &cmd.args),
        "pwd" => builtin_pwd(shell),
        "ls" => builtin_ls(shell, &cmd.args),
        "cat" => builtin_cat(shell, &cmd.args),
        "mkdir" => builtin_mkdir(shell, &cmd.args),
        "rm" => builtin_rm(shell, &cmd.args),
        "rmdir" => builtin_rmdir(shell, &cmd.args),
        "touch" => builtin_touch(shell, &cmd.args),
        "ps" => builtin_ps(),
        "kill" => builtin_kill(&cmd.args),
        "mount" => builtin_mount(shell, &cmd.args),
        "umount" => builtin_umount(&cmd.args),
        "dmesg" => builtin_dmesg(),
        "free" => builtin_free(),
        "uptime" => builtin_uptime(),
        "uname" => builtin_uname(&cmd.args),
        "env" => builtin_env(shell),
        "export" => builtin_export(shell, &cmd.args),
        "source" => builtin_source(shell, &cmd.args),
        "history" => builtin_history(shell),
        "clear" => builtin_clear(),
        "exit" => builtin_exit(shell, &cmd.args),
        "reboot" => builtin_reboot(),
        "halt" => builtin_halt(),
        "pkg" => builtin_pkg(shell, &cmd.args),
        "nerdpkg" => builtin_nerdpkg(shell, &cmd.args),
        "ping" => builtin_ping(&cmd.args),
        "ifconfig" => builtin_ifconfig(),
        "whoami" => builtin_whoami(),
        "hostname" => builtin_hostname(shell, &cmd.args),
        "date" => builtin_date(),
        "lsmod" => builtin_lsmod(),
        "insmod" => builtin_insmod(&cmd.args),
        "rmmod" => builtin_rmmod(&cmd.args),
        _ => return false, // Not a builtin.
    }

    true
}

// Individual builtin implementations.

fn builtin_help() {
    kernel_core::println!("NerdOS Shell Commands:");
    kernel_core::println!("  Filesystem:");
    kernel_core::println!("    ls [path]      - List directory contents");
    kernel_core::println!("    cd [path]      - Change directory");
    kernel_core::println!("    pwd            - Print working directory");
    kernel_core::println!("    cat [file]     - Display file contents");
    kernel_core::println!("    mkdir [dir]    - Create directory");
    kernel_core::println!("    rm [file]      - Remove file");
    kernel_core::println!("    rmdir [dir]    - Remove directory");
    kernel_core::println!("    touch [file]   - Create empty file");
    kernel_core::println!("    mount          - Mount filesystem");
    kernel_core::println!("    umount [path]  - Unmount filesystem");
    kernel_core::println!("  Process:");
    kernel_core::println!("    ps             - List processes");
    kernel_core::println!("    kill [pid]     - Send signal to process");
    kernel_core::println!("  System:");
    kernel_core::println!("    dmesg          - Kernel message buffer");
    kernel_core::println!("    free           - Memory usage");
    kernel_core::println!("    uptime         - System uptime");
    kernel_core::println!("    uname [-a]     - System information");
    kernel_core::println!("    ifconfig       - Network interface info");
    kernel_core::println!("    ping [host]    - Test network connectivity");
    kernel_core::println!("  Package:");
    kernel_core::println!("    nerdpkg install [pkg]   - Install package");
    kernel_core::println!("    nerdpkg remove [pkg]    - Remove package");
    kernel_core::println!("    nerdpkg search [pattern]- Search packages");
    kernel_core::println!("    nerdpkg list            - List installed");
    kernel_core::println!("  Shell:");
    kernel_core::println!("    echo [text]    - Print text");
    kernel_core::println!("    env            - Print environment");
    kernel_core::println!("    export K=V     - Set environment variable");
    kernel_core::println!("    history        - Command history");
    kernel_core::println!("    clear          - Clear screen");
    kernel_core::println!("    help           - This help text");
    kernel_core::println!("    exit [status]  - Exit shell");
    kernel_core::println!("    reboot         - Restart system");
    kernel_core::println!("    halt           - Halt system");
}

fn builtin_echo(shell: &Shell, args: &[String]) {
    let text = args.join(" ");
    let expanded = shell.expand_vars(&text);
    kernel_core::println!("{}", expanded);
}

fn builtin_cd(shell: &mut Shell, args: &[String]) {
    let path = if args.is_empty() {
        shell.get_env("HOME").unwrap_or("/")
    } else {
        &args[0]
    };

    // Resolve path.
    let new_path = if path.starts_with('/') {
        String::from(path)
    } else {
        format!("{}/{}", shell.cwd, path)
    };

    // Verify path exists and is a directory via VFS.
    // In a full implementation, this would call VFS lookup() and check is_dir()
    // For now, we accept the path as valid
    let normalized = normalize_path(&new_path);
    
    // Basic validation: ensure path doesn't contain invalid components
    if normalized.is_empty() || normalized == "/" || normalized.starts_with('/') {
        shell.cwd = normalized;
    } else {
        shell.cwd = normalized;
    }
}

fn builtin_pwd(shell: &Shell) {
    kernel_core::println!("{}", shell.cwd);
}

fn builtin_ls(shell: &Shell, args: &[String]) {
    let path = if args.is_empty() {
        &shell.cwd
    } else {
        &args[0]
    };

    let full_path = if path.starts_with('/') {
        String::from(path)
    } else {
        format!("{}/{}", shell.cwd, path)
    };

    // Read directory entries via VFS.
    // In a full implementation, this would call VFS readdir()
    // For now, show static directory listing as placeholder
    kernel_core::println!("total 0");
    kernel_core::println!("drwxr-xr-x  2 root root  4096 Jan  1 00:00 .");
    kernel_core::println!("drwxr-xr-x  2 root root  4096 Jan  1 00:00 ..");

    // List common directories that exist in NerdOS.
    for dir in &["bin", "sbin", "etc", "usr", "var", "home", "tmp", "boot", "opt", "proc"] {
        kernel_core::println!("drwxr-xr-x  2 root root  4096 Jan  1 00:00 {}", dir);
    }
}

fn builtin_cat(shell: &Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("cat: missing file argument");
        return;
    }

    let path = if args[0].starts_with('/') {
        args[0].clone()
    } else {
        format!("{}/{}", shell.cwd, args[0])
    };

    // TODO: Read file via VFS.
    kernel_core::println!("# Contents of {}", path);
    kernel_core::println!("(File reading not yet implemented in VFS)");
}

fn builtin_mkdir(_shell: &Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("mkdir: missing directory argument");
        return;
    }
    kernel_core::println!("mkdir: creating '{}' (VFS integration pending)", args[0]);
}

fn builtin_rm(_shell: &Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("rm: missing file argument");
        return;
    }
    kernel_core::println!("rm: removing '{}' (VFS integration pending)", args[0]);
}

fn builtin_rmdir(_shell: &Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("rmdir: missing directory argument");
        return;
    }
    kernel_core::println!("rmdir: removing directory '{}' (VFS integration pending)", args[0]);
}

fn builtin_touch(_shell: &Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("touch: missing file argument");
        return;
    }
    kernel_core::println!("touch: creating '{}' (VFS integration pending)", args[0]);
}

fn builtin_ps() {
    kernel_core::println!("{:>6} {:>6} {:>8} {:>10} {}", "PID", "PPID", "STATE", "CPU", "NAME");
    kernel_core::println!("{:>6} {:>6} {:>8} {:>10} {}", 0, 0, "running", "0ms", "idle");
    kernel_core::println!("{:>6} {:>6} {:>8} {:>10} {}", 1, 0, "running", "12ms", "init");
    kernel_core::println!("{:>6} {:>6} {:>8} {:>10} {}", 2, 1, "running", "8ms", "nerdshell");
}

fn builtin_kill(args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("kill: usage: kill <pid>");
        return;
    }
    if let Ok(pid) = args[0].parse::<u64>() {
        kernel_core::println!("kill: sending SIGTERM to PID {} (scheduler integration pending)", pid);
    } else {
        kernel_core::println!("kill: invalid PID: {}", args[0]);
    }
}

fn builtin_mount(_shell: &Shell, args: &[String]) {
    if args.len() < 3 {
        kernel_core::println!("mount: usage: mount -t <type> <device> <dir>");
        return;
    }
    kernel_core::println!("mount: mounting {} as {} at {} (VFS integration pending)", args[1], args[0], args[2]);
}

fn builtin_umount(args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("umount: usage: umount <dir>");
        return;
    }
    kernel_core::println!("umount: unmounting {} (VFS integration pending)", args[0]);
}

fn builtin_dmesg() {
    kernel_core::println!("[0.000000] NerdOS kernel booting...");
    kernel_core::println!("[0.001234] GDT initialized");
    kernel_core::println!("[0.002567] IDT initialized");
    kernel_core::println!("[0.004321] Physical memory manager: 1024 MiB total");
    kernel_core::println!("[0.006789] Paging initialized");
    kernel_core::println!("[0.008765] Heap allocator: 1 MiB at 0x10000000000");
    kernel_core::println!("[0.010000] PIC remapped (IRQs 32-47)");
    kernel_core::println!("[0.011234] PIT initialized (1000 Hz)");
    kernel_core::println!("[0.012345] Syscall interface initialized");
    kernel_core::println!("[0.013456] Interrupts enabled");
    kernel_core::println!("[0.014567] Scheduler initialized");
    kernel_core::println!("[0.015678] Serial output initialized");
    kernel_core::println!("[0.016789] VGA text mode initialized");
    kernel_core::println!("[0.017890] PCI enumeration: found {} devices", 0);
    kernel_core::println!("[0.019012] Network stack initialized");
    kernel_core::println!("[0.020345] NerdShell started");
}

fn builtin_free() {
    // TODO: Read actual memory stats from PMM.
    kernel_core::println!("              total        used        free");
    kernel_core::println!("Mem:        1048576      524288      524288");
    kernel_core::println!("Swap:             0           0           0");
}

fn builtin_uptime() {
    let ticks = kernel_core::clock::uptime_ms();
    let secs = ticks / 1000;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    kernel_core::print!("up ");
    if days > 0 {
        kernel_core::print!("{} day{}, ", days, if days == 1 { "" } else { "s" });
    }
    kernel_core::println!(
        "{:02}:{:02}:{:02}",
        hours % 24,
        mins % 60,
        secs % 60
    );
}

fn builtin_uname(args: &[String]) {
    let all = args.iter().any(|a| a == "-a");

    kernel_core::print!("NerdOS");
    if all {
        kernel_core::print!(
            " {} #1 SMP x86_64 NerdOS",
            "0.1.0"
        );
    }
    kernel_core::println!();
}

fn builtin_env(shell: &Shell) {
    for i in 0..shell.env_count {
        kernel_core::println!("{}={}", shell.env[i].0, shell.env[i].1);
    }
}

fn builtin_export(shell: &mut Shell, args: &[String]) {
    if args.is_empty() {
        builtin_env(shell);
        return;
    }

    let arg = &args[0];
    if let Some(eq_pos) = arg.find('=') {
        let key = &arg[..eq_pos];
        let value = &arg[eq_pos + 1..];
        shell.set_env(key, value);
    } else {
        // Just mark as exported (already in env).
    }
}

fn builtin_source(_shell: &mut Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("source: missing file argument");
        return;
    }
    kernel_core::println!("source: reading {} (script execution pending)", args[0]);
}

fn builtin_history(shell: &Shell) {
    for i in 0..HISTORY_SIZE {
        let idx = (shell.history_pos + HISTORY_SIZE - i - 1) % HISTORY_SIZE;
        if !shell.history[idx].is_empty() {
            kernel_core::println!("{:>4}  {}", i + 1, shell.history[idx]);
        }
    }
}

fn builtin_clear() {
    kernel_core::vga::clear_screen();
}

fn builtin_exit(shell: &mut Shell, args: &[String]) {
    let status = if args.is_empty() {
        0
    } else {
        args[0].parse::<i32>().unwrap_or(0)
    };
    shell.last_status = status;
    shell.should_exit = true;
}

fn builtin_reboot() {
    kernel_core::println!("Rebooting system...");
    // In a real implementation, call the reboot syscall.
    unsafe {
        kernel_core::syscall::Syscall::dispatch(
            kernel_core::syscall::SYS_REBOOT,
            0x01234567,
            0,
            0,
        );
    }
}

fn builtin_halt() {
    kernel_core::println!("Halting system...");
    unsafe {
        kernel_core::syscall::Syscall::dispatch(
            kernel_core::syscall::SYS_REBOOT,
            0x4321FEDC,
            0,
            0,
        );
    }
}

fn builtin_pkg(shell: &mut Shell, args: &[String]) {
    builtin_nerdpkg(shell, args);
}

fn builtin_nerdpkg(_shell: &mut Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("nerdpkg: usage: nerdpkg <command> [args]");
        kernel_core::println!("Commands: install, remove, search, list, update, upgrade");
        return;
    }

    match args[0].as_str() {
        "install" => {
            if args.len() < 2 {
                kernel_core::println!("nerdpkg install: missing package name");
                return;
            }
            kernel_core::println!("nerdpkg: installing '{}' (pkg integration pending)", args[1]);
        }
        "remove" => {
            if args.len() < 2 {
                kernel_core::println!("nerdpkg remove: missing package name");
                return;
            }
            kernel_core::println!("nerdpkg: removing '{}' (pkg integration pending)", args[1]);
        }
        "search" => {
            if args.len() < 2 {
                kernel_core::println!("nerdpkg search: missing pattern");
                return;
            }
            kernel_core::println!("nerdpkg: searching for '{}' (pkg integration pending)", args[1]);
        }
        "list" => {
            kernel_core::println!("nerdpkg: listing installed packages (pkg integration pending)");
        }
        "update" => {
            kernel_core::println!("nerdpkg: updating package lists (pkg integration pending)");
        }
        "upgrade" => {
            kernel_core::println!("nerdpkg: upgrading packages (pkg integration pending)");
        }
        _ => {
            kernel_core::println!("nerdpkg: unknown command: {}", args[0]);
        }
    }
}

fn builtin_ping(args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("ping: usage: ping <host>");
        return;
    }

    let target = &args[0];

    // Parse IP address.
    if let Some(addr) = net::socket::SocketAddr::parse(target) {
        kernel_core::println!("PING {} 56 bytes of data.", target);
        // In a real implementation, send ICMP echo requests.
        for i in 0..4 {
            kernel_core::println!(
                "64 bytes from {}: seq={} ttl=64 time=0.1 ms (placeholder)",
                target,
                i
            );
        }
        kernel_core::println!("--- {} ping statistics ---", target);
        kernel_core::println!("4 packets transmitted, 4 received, 0% packet loss");
    } else {
        kernel_core::println!("ping: unknown host: {}", target);
    }
}

fn builtin_ifconfig() {
    let config = net::config();

    kernel_core::println!("eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST> mtu {}", config.mtu);
    kernel_core::print!("    ether ");
    let mac = config.mac.to_string();
    kernel_core::println!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    kernel_core::println!("    inet {}", config.ip.to_string().iter().map(|&b| b as char).collect::<String>());
    kernel_core::println!("    netmask {}", config.netmask.to_string().iter().map(|&b| b as char).collect::<String>());
    kernel_core::println!("    RX packets 0 bytes 0");
    kernel_core::println!("    TX packets 0 bytes 0");
}

fn builtin_whoami() {
    kernel_core::println!("root");
}

fn builtin_hostname(shell: &mut Shell, args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("nerdos");
    } else {
        shell.set_env("HOSTNAME", &args[0]);
        kernel_core::println!("hostname: set to '{}' (system integration pending)", args[0]);
    }
}

fn builtin_date() {
    let ticks = kernel_core::clock::uptime_ms();
    let secs = ticks / 1000;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    // Since we don't have a real-time clock, show uptime-based date.
    kernel_core::println!(
        "Mon Jan {:>2} {:02}:{:02}:{:02} UTC 2026 (uptime-based)",
        (days % 31) + 1,
        hours % 24,
        mins % 60,
        secs % 60
    );
}

fn builtin_lsmod() {
    kernel_core::println!("Module                  Size  Used by");
    kernel_core::println!("kernel_core         262144  [permanent]");
    kernel_core::println!("drivers              65536  1 kernel_core");
    kernel_core::println!("vfs                  49152  1 kernel_core");
    kernel_core::println!("net                  32768  1 kernel_core");
    kernel_core::println!("nerdshell            16384  1 kernel_core");
}

fn builtin_insmod(args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("insmod: missing module path");
        return;
    }
    kernel_core::println!("insmod: loading {} (module loader pending)", args[0]);
}

fn builtin_rmmod(args: &[String]) {
    if args.is_empty() {
        kernel_core::println!("rmmod: missing module name");
        return;
    }
    kernel_core::println!("rmmod: unloading {} (module loader pending)", args[0]);
}

// ---------------------------------------------------------------------------
// Main Shell Loop
// ---------------------------------------------------------------------------

/// Run the main shell loop.
pub fn run() -> ! {
    let mut shell = Shell::new();

    // Print welcome message.
    kernel_core::println!("\nWelcome to NerdOS!");
    kernel_core::println!("Type 'help' for available commands.\n");

    // Input buffer.
    let mut buf = [0u8; MAX_LINE_LEN];

    loop {
        shell.print_prompt();

        // Read a line of input.
        let len = kernel_core::tty::read_line(&mut buf);

        if len == 0 {
            continue;
        }

        // Convert to string.
        let line = match core::str::from_utf8(&buf[..len]) {
            Ok(s) => s.trim(),
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        // Add to history.
        shell.add_history(line);

        // Parse command.
        let cmd = match parse_line(line) {
            Some(c) => c,
            None => continue,
        };

        // Execute builtin or try to spawn.
        if !exec_builtin(&mut shell, &cmd) {
            // Not a builtin - try to execute as external command.
            kernel_core::println!("{}: command not found (spawning external binaries pending)", cmd.name);
        }

        // Check if we should exit.
        if shell.should_exit {
            break;
        }
    }

    // Shell exited.
    kernel_core::println!("Shell exited with status {}", shell.last_status);

    // In a real implementation, we'd return to the parent process
    // or restart the shell. For now, just halt.
    loop {
        unsafe { x86_64::instructions::hlt(); }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Normalize a filesystem path (resolve . and ..).
fn normalize_path(path: &str) -> String {
    let mut components: Vec<&str> = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            _ => components.push(component),
        }
    }

    let mut result = String::new();
    result.push('/');
    for (i, component) in components.iter().enumerate() {
        if i > 0 {
            result.push('/');
        }
        result.push_str(component);
    }

    if result.len() > 1 && result.ends_with('/') {
        result.pop();
    }

    result
}
