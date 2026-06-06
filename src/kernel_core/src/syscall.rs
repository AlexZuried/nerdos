//! # System Call Interface
//!
//! NerdOS provides a Linux-compatible syscall interface using both:
//! - `syscall`/`sysret` instructions (fast, preferred)
//! - `int 0x80` (legacy compatibility)
//!
//! ## Calling Convention (x86_64)
//!
//! | Register | Purpose |
//! |----------|---------|
//! | RAX | Syscall number (also return value) |
//! | RDI | Argument 1 |
//! | RSI | Argument 2 |
//! | RDX | Argument 3 |
//! | R10 | Argument 4 |
//! | R8  | Argument 5 |
//! | R9  | Argument 6 |
//!
//! ## Syscall Numbers
//!
//! We use the same numbers as Linux where possible for compatibility.

use x86_64::registers::model_specific::{LStar, Star, SFMask};
use x86_64::registers::rflags::RFlags;

// ---------------------------------------------------------------------------
// Syscall Numbers (Linux-compatible)
// ---------------------------------------------------------------------------

pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_OPEN: u64 = 2;
pub const SYS_CLOSE: u64 = 3;
pub const SYS_STAT: u64 = 4;
pub const SYS_FSTAT: u64 = 5;
pub const SYS_LSTAT: u64 = 6;
pub const SYS_POLL: u64 = 7;
pub const SYS_LSEEK: u64 = 8;
pub const SYS_MMAP: u64 = 9;
pub const SYS_MPROTECT: u64 = 10;
pub const SYS_MUNMAP: u64 = 11;
pub const SYS_BRK: u64 = 12;
pub const SYS_RT_SIGACTION: u64 = 13;
pub const SYS_RT_SIGPROCMASK: u64 = 14;
pub const SYS_RT_SIGRETURN: u64 = 15;
pub const SYS_IOCTL: u64 = 16;
pub const SYS_PREAD64: u64 = 17;
pub const SYS_PWRITE64: u64 = 18;
pub const SYS_READV: u64 = 19;
pub const SYS_WRITEV: u64 = 20;
pub const SYS_ACCESS: u64 = 21;
pub const SYS_PIPE: u64 = 22;
pub const SYS_SELECT: u64 = 23;
pub const SYS_SCHED_YIELD: u64 = 24;
pub const SYS_MREMAP: u64 = 25;
pub const SYS_MSYNC: u64 = 26;
pub const SYS_MINCORE: u64 = 27;
pub const SYS_MADVISE: u64 = 28;
pub const SYS_SHMGET: u64 = 29;
pub const SYS_SHMAT: u64 = 30;
pub const SYS_SHMCTL: u64 = 31;
pub const SYS_DUP: u64 = 32;
pub const SYS_DUP2: u64 = 33;
pub const SYS_PAUSE: u64 = 34;
pub const SYS_NANOSLEEP: u64 = 35;
pub const SYS_GETITIMER: u64 = 36;
pub const SYS_ALARM: u64 = 37;
pub const SYS_SETITIMER: u64 = 38;
pub const SYS_GETPID: u64 = 39;
pub const SYS_SENDFILE: u64 = 40;
pub const SYS_SOCKET: u64 = 41;
pub const SYS_CONNECT: u64 = 42;
pub const SYS_ACCEPT: u64 = 43;
pub const SYS_SENDTO: u64 = 44;
pub const SYS_RECVFROM: u64 = 45;
pub const SYS_SENDMSG: u64 = 46;
pub const SYS_RECVMSG: u64 = 47;
pub const SYS_SHUTDOWN: u64 = 48;
pub const SYS_BIND: u64 = 49;
pub const SYS_LISTEN: u64 = 50;
pub const SYS_GETSOCKNAME: u64 = 51;
pub const SYS_GETPEERNAME: u64 = 52;
pub const SYS_SOCKETPAIR: u64 = 53;
pub const SYS_SETSOCKOPT: u64 = 54;
pub const SYS_GETSOCKOPT: u64 = 55;
pub const SYS_CLONE: u64 = 56;
pub const SYS_FORK: u64 = 57;
pub const SYS_VFORK: u64 = 58;
pub const SYS_EXECVE: u64 = 59;
pub const SYS_EXIT: u64 = 60;
pub const SYS_WAIT4: u64 = 61;
pub const SYS_KILL: u64 = 62;
pub const SYS_UNAME: u64 = 63;
pub const SYS_SEMGET: u64 = 64;
pub const SYS_SEMOP: u64 = 65;
pub const SYS_SEMCTL: u64 = 66;
pub const SYS_SHMDT: u64 = 67;
pub const SYS_MSGGET: u64 = 68;
pub const SYS_MSGSND: u64 = 69;
pub const SYS_MSGRCV: u64 = 70;
pub const SYS_MSGCTL: u64 = 71;
pub const SYS_FCNTL: u64 = 72;
pub const SYS_FLOCK: u64 = 73;
pub const SYS_FSYNC: u64 = 74;
pub const SYS_FDATASYNC: u64 = 75;
pub const SYS_TRUNCATE: u64 = 76;
pub const SYS_FTRUNCATE: u64 = 77;
pub const SYS_GETDENTS: u64 = 78;
pub const SYS_GETCWD: u64 = 79;
pub const SYS_CHDIR: u64 = 80;
pub const SYS_FCHDIR: u64 = 81;
pub const SYS_RENAME: u64 = 82;
pub const SYS_MKDIR: u64 = 83;
pub const SYS_RMDIR: u64 = 84;
pub const SYS_CREAT: u64 = 85;
pub const SYS_LINK: u64 = 86;
pub const SYS_UNLINK: u64 = 87;
pub const SYS_SYMLINK: u64 = 88;
pub const SYS_READLINK: u64 = 89;
pub const SYS_CHMOD: u64 = 90;
pub const SYS_FCHMOD: u64 = 91;
pub const SYS_CHOWN: u64 = 92;
pub const SYS_FCHOWN: u64 = 93;
pub const SYS_LCHOWN: u64 = 94;
pub const SYS_UMASK: u64 = 95;
pub const SYS_GETTIMEOFDAY: u64 = 96;
pub const SYS_GETRLIMIT: u64 = 97;
pub const SYS_GETRUSAGE: u64 = 98;
pub const SYS_SYSINFO: u64 = 99;
pub const SYS_TIMES: u64 = 100;
pub const SYS_PTRACE: u64 = 101;
pub const SYS_GETUID: u64 = 102;
pub const SYS_SYSLOG: u64 = 103;
pub const SYS_GETGID: u64 = 104;
pub const SYS_SETUID: u64 = 105;
pub const SYS_SETGID: u64 = 106;
pub const SYS_GETEUID: u64 = 107;
pub const SYS_GETEGID: u64 = 108;
pub const SYS_SETPGID: u64 = 109;
pub const SYS_GETPPID: u64 = 110;
pub const SYS_GETPGRP: u64 = 111;
pub const SYS_SETSID: u64 = 112;
pub const SYS_SETREUID: u64 = 113;
pub const SYS_SETREGID: u64 = 114;
pub const SYS_GETGROUPS: u64 = 115;
pub const SYS_SETGROUPS: u64 = 116;
pub const SYS_SETRESUID: u64 = 117;
pub const SYS_GETRESUID: u64 = 118;
pub const SYS_SETRESGID: u64 = 119;
pub const SYS_GETRESGID: u64 = 120;
pub const SYS_GETPGID: u64 = 121;
pub const SYS_SETFSUID: u64 = 122;
pub const SYS_SETFSGID: u64 = 123;
pub const SYS_GETSID: u64 = 124;
pub const SYS_CAPGET: u64 = 125;
pub const SYS_CAPSET: u64 = 126;
pub const SYS_RT_SIGPENDING: u64 = 127;
pub const SYS_RT_SIGTIMEDWAIT: u64 = 128;
pub const SYS_RT_SIGQUEUEINFO: u64 = 129;
pub const SYS_RT_SIGSUSPEND: u64 = 130;
pub const SYS_SIGALTSTACK: u64 = 131;
pub const SYS_UTIME: u64 = 132;
pub const SYS_MKNOD: u64 = 133;
pub const SYS_USELIB: u64 = 134;
pub const SYS_PERSONALITY: u64 = 135;
pub const SYS_USTAT: u64 = 136;
pub const SYS_STATFS: u64 = 137;
pub const SYS_FSTATFS: u64 = 138;
pub const SYS_SYSFS: u64 = 139;
pub const SYS_GETPRIORITY: u64 = 140;
pub const SYS_SETPRIORITY: u64 = 141;
pub const SYS_SCHED_SETPARAM: u64 = 142;
pub const SYS_SCHED_GETPARAM: u64 = 143;
pub const SYS_SCHED_SETSCHEDULER: u64 = 144;
pub const SYS_SCHED_GETSCHEDULER: u64 = 145;
pub const SYS_SCHED_GET_PRIORITY_MAX: u64 = 146;
pub const SYS_SCHED_GET_PRIORITY_MIN: u64 = 147;
pub const SYS_SCHED_RR_GET_INTERVAL: u64 = 148;
pub const SYS_MLOCK: u64 = 149;
pub const SYS_MUNLOCK: u64 = 150;
pub const SYS_MLOCKALL: u64 = 151;
pub const SYS_MUNLOCKALL: u64 = 152;
pub const SYS_VHANGUP: u64 = 153;
pub const SYS_MODIFY_LDT: u64 = 154;
pub const SYS_PIVOT_ROOT: u64 = 155;
pub const SYS__SYSCTL: u64 = 156;
pub const SYS_PRCTL: u64 = 157;
pub const SYS_ARCH_PRCTL: u64 = 158;
pub const SYS_ADJTIMEX: u64 = 159;
pub const SYS_SETRLIMIT: u64 = 160;
pub const SYS_CHROOT: u64 = 161;
pub const SYS_SYNC: u64 = 162;
pub const SYS_ACCT: u64 = 163;
pub const SYS_SETTIMEOFDAY: u64 = 164;
pub const SYS_MOUNT: u64 = 165;
pub const SYS_UMOUNT2: u64 = 166;
pub const SYS_SWAPON: u64 = 167;
pub const SYS_SWAPOFF: u64 = 168;
pub const SYS_REBOOT: u64 = 169;
pub const SYS_SETHOSTNAME: u64 = 170;
pub const SYS_SETDOMAINNAME: u64 = 171;
pub const SYS_IOPL: u64 = 172;
pub const SYS_IOPERM: u64 = 173;
pub const SYS_CREATE_MODULE: u64 = 174;
pub const SYS_INIT_MODULE: u64 = 175;
pub const SYS_DELETE_MODULE: u64 = 176;
pub const SYS_GET_KERNEL_SYMS: u64 = 177;
pub const SYS_QUERY_MODULE: u64 = 178;
pub const SYS_QUOTACTL: u64 = 179;
pub const SYS_NFSSERVCTL: u64 = 180;
pub const SYS_GETPMSG: u64 = 181;
pub const SYS_PUTPMSG: u64 = 182;
pub const SYS_AFS_SYSCALL: u64 = 183;
pub const SYS_TUXCALL: u64 = 184;
pub const SYS_SECURITY: u64 = 185;
pub const SYS_GETTID: u64 = 186;
pub const SYS_READAHEAD: u64 = 187;
pub const SYS_SETXATTR: u64 = 188;
pub const SYS_LSETXATTR: u64 = 189;
pub const SYS_FSETXATTR: u64 = 190;
pub const SYS_GETXATTR: u64 = 191;
pub const SYS_LGETXATTR: u64 = 192;
pub const SYS_FGETXATTR: u64 = 193;
pub const SYS_LISTXATTR: u64 = 194;
pub const SYS_LLISTXATTR: u64 = 195;
pub const SYS_FLISTXATTR: u64 = 196;
pub const SYS_REMOVEXATTR: u64 = 197;
pub const SYS_LREMOVEXATTR: u64 = 198;
pub const SYS_FREMOVEXATTR: u64 = 199;
pub const SYS_TKILL: u64 = 200;
pub const SYS_TIME: u64 = 201;
pub const SYS_FUTEX: u64 = 202;
pub const SYS_SCHED_SETAFFINITY: u64 = 203;
pub const SYS_SCHED_GETAFFINITY: u64 = 204;
pub const SYS_SET_THREAD_AREA: u64 = 205;
pub const SYS_IO_SETUP: u64 = 206;
pub const SYS_IO_DESTROY: u64 = 207;
pub const SYS_IO_GETEVENTS: u64 = 208;
pub const SYS_IO_SUBMIT: u64 = 209;
pub const SYS_IO_CANCEL: u64 = 210;
pub const SYS_GET_THREAD_AREA: u64 = 211;
pub const SYS_LOOKUP_DCOOKIE: u64 = 212;
pub const SYS_EPOLL_CREATE: u64 = 213;
pub const SYS_EPOLL_CTL_OLD: u64 = 214;
pub const SYS_EPOLL_WAIT_OLD: u64 = 215;
pub const SYS_REMAP_FILE_PAGES: u64 = 216;
pub const SYS_GETDENTS64: u64 = 217;
pub const SYS_SET_TID_ADDRESS: u64 = 218;
pub const SYS_RESTART_SYSCALL: u64 = 219;
pub const SYS_SEMTIMEDOP: u64 = 220;
pub const SYS_FADVISE64: u64 = 221;
pub const SYS_TIMER_CREATE: u64 = 222;
pub const SYS_TIMER_SETTIME: u64 = 223;
pub const SYS_TIMER_GETTIME: u64 = 224;
pub const SYS_TIMER_GETOVERRUN: u64 = 225;
pub const SYS_TIMER_DELETE: u64 = 226;
pub const SYS_CLOCK_SETTIME: u64 = 227;
pub const SYS_CLOCK_GETTIME: u64 = 228;
pub const SYS_CLOCK_GETRES: u64 = 229;
pub const SYS_CLOCK_NANOSLEEP: u64 = 230;
pub const SYS_EXIT_GROUP: u64 = 231;
pub const SYS_EPOLL_WAIT: u64 = 232;
pub const SYS_EPOLL_CTL: u64 = 233;
pub const SYS_TGKILL: u64 = 234;
pub const SYS_UTGA: u64 = 235;
pub const SYS_VSERVER: u64 = 236;
pub const SYS_MBIND: u64 = 237;
pub const SYS_SET_MEMPOLICY: u64 = 238;
pub const SYS_GET_MEMPOLICY: u64 = 239;
pub const SYS_MQ_OPEN: u64 = 240;
pub const SYS_MQ_UNLINK: u64 = 241;
pub const SYS_MQ_TIMEDSEND: u64 = 242;
pub const SYS_MQ_TIMEDRECEIVE: u64 = 243;
pub const SYS_MQ_NOTIFY: u64 = 244;
pub const SYS_MQ_GETSETATTR: u64 = 245;
pub const SYS_KEXEC_LOAD: u64 = 246;
pub const SYS_WAITID: u64 = 247;
pub const SYS_ADD_KEY: u64 = 248;
pub const SYS_REQUEST_KEY: u64 = 249;
pub const SYS_KEYCTL: u64 = 250;
pub const SYS_IOPRIO_SET: u64 = 251;
pub const SYS_IOPRIO_GET: u64 = 252;
pub const SYS_INOTIFY_INIT: u64 = 253;
pub const SYS_INOTIFY_ADD_WATCH: u64 = 254;
pub const SYS_INOTIFY_RM_WATCH: u64 = 255;
pub const SYS_MIGRATE_PAGES: u64 = 256;
pub const SYS_OPENAT: u64 = 257;
pub const SYS_MKDIRAT: u64 = 258;
pub const SYS_MKNODAT: u64 = 259;
pub const SYS_FCHOWNAT: u64 = 260;
pub const SYS_FUTIMESAT: u64 = 261;
pub const SYS_NEWFSTATAT: u64 = 262;
pub const SYS_UNLINKAT: u64 = 263;
pub const SYS_RENAMEAT: u64 = 264;
pub const SYS_LINKAT: u64 = 265;
pub const SYS_SYMLINKAT: u64 = 266;
pub const SYS_READLINKAT: u64 = 267;
pub const SYS_FCHMODAT: u64 = 268;
pub const SYS_FACCESSAT: u64 = 269;
pub const SYS_PSELECT6: u64 = 270;
pub const SYS_PPOLL: u64 = 271;
pub const SYS_UNSHARE: u64 = 272;
pub const SYS_SET_ROBUST_LIST: u64 = 273;
pub const SYS_GET_ROBUST_LIST: u64 = 274;
pub const SYS_SPLICE: u64 = 275;
pub const SYS_TEE: u64 = 276;
pub const SYS_SYNC_FILE_RANGE: u64 = 277;
pub const SYS_VMSPLICE: u64 = 278;
pub const SYS_MOVE_PAGES: u64 = 279;
pub const SYS_UTIMENSAT: u64 = 280;
pub const SYS_EPOLL_PWAIT: u64 = 281;
pub const SYS_SIGNALFD: u64 = 282;
pub const SYS_TIMERFD_CREATE: u64 = 283;
pub const SYS_EVENTFD: u64 = 284;
pub const SYS_FALLOCATE: u64 = 285;
pub const SYS_TIMERFD_SETTIME: u64 = 286;
pub const SYS_TIMERFD_GETTIME: u64 = 287;
pub const SYS_ACCEPT4: u64 = 288;
pub const SYS_SIGNALFD4: u64 = 289;
pub const SYS_EVENTFD2: u64 = 290;
pub const SYS_EPOLL_CREATE1: u64 = 291;
pub const SYS_DUP3: u64 = 292;
pub const SYS_PIPE2: u64 = 293;
pub const SYS_INOTIFY_INIT1: u64 = 294;
pub const SYS_PREADV: u64 = 295;
pub const SYS_PWRITEV: u64 = 296;
pub const SYS_RT_TGSIGQUEUEINFO: u64 = 297;
pub const SYS_PERF_EVENT_OPEN: u64 = 298;
pub const SYS_RECVMMSG: u64 = 299;
pub const SYS_FANOTIFY_INIT: u64 = 300;
pub const SYS_FANOTIFY_MARK: u64 = 301;
pub const SYS_PRLIMIT64: u64 = 302;
pub const SYS_NAME_TO_HANDLE_AT: u64 = 303;
pub const SYS_OPEN_BY_HANDLE_AT: u64 = 304;
pub const SYS_CLOCK_ADJTIME: u64 = 305;
pub const SYS_SYNCFS: u64 = 306;
pub const SYS_SENDMMSG: u64 = 307;
pub const SYS_SETNS: u64 = 308;
pub const SYS_GETCPU: u64 = 309;
pub const SYS_PROCESS_VM_READV: u64 = 310;
pub const SYS_PROCESS_VM_WRITEV: u64 = 311;
pub const SYS_KCMP: u64 = 312;
pub const SYS_FINIT_MODULE: u64 = 313;
pub const SYS_SCHED_SETATTR: u64 = 314;
pub const SYS_SCHED_GETATTR: u64 = 315;
pub const SYS_RENAMEAT2: u64 = 316;
pub const SYS_SECCOMP: u64 = 317;
pub const SYS_GETRANDOM: u64 = 318;
pub const SYS_MEMFD_CREATE: u64 = 319;
pub const SYS_KEXEC_FILE_LOAD: u64 = 320;
pub const SYS_BPF: u64 = 321;
pub const SYS_STUB_EXECVEAT: u64 = 322;
pub const SYS_USERFAULTFD: u64 = 323;
pub const SYS_MEMBARRIER: u64 = 324;
pub const SYS_MLOCK2: u64 = 325;
pub const SYS_COPY_FILE_RANGE: u64 = 326;
pub const SYS_PREADV2: u64 = 327;
pub const SYS_PWRITEV2: u64 = 328;
pub const SYS_PKEY_MPROTECT: u64 = 329;
pub const SYS_PKEY_ALLOC: u64 = 330;
pub const SYS_PKEY_FREE: u64 = 331;
pub const SYS_STATX: u64 = 332;
pub const SYS_IO_PGETEVENTS: u64 = 333;
pub const SYS_RSEQ: u64 = 334;
pub const SYS_PIDFD_SEND_SIGNAL: u64 = 424;
pub const SYS_IO_URING_SETUP: u64 = 425;
pub const SYS_IO_URING_ENTER: u64 = 426;
pub const SYS_IO_URING_REGISTER: u64 = 427;
pub const SYS_OPEN_TREE: u64 = 428;
pub const SYS_MOVE_MOUNT: u64 = 429;
pub const SYS_FSOPEN: u64 = 430;
pub const SYS_FSCONFIG: u64 = 431;
pub const SYS_FSMOUNT: u64 = 432;
pub const SYS_FSPICK: u64 = 433;
pub const SYS_PIDFD_OPEN: u64 = 434;
pub const SYS_CLONE3: u64 = 435;
pub const SYS_OPENAT2: u64 = 437;
pub const SYS_PIDFD_GETFD: u64 = 438;
pub const SYS_FACCESSAT2: u64 = 439;
pub const SYS_PROCESS_MADVISE: u64 = 440;
pub const SYS_EPOLL_PWAIT2: u64 = 441;
pub const SYS_MOUNT_SETATTR: u64 = 442;
pub const SYS_QUOTACTL_FD: u64 = 443;
pub const SYS_LANDLOCK_CREATE_OR_RULESET: u64 = 444;
pub const SYS_LANDLOCK_ADD_RULE: u64 = 445;
pub const SYS_LANDLOCK_RESTRICT_SET_SELF: u64 = 446;

// ---------------------------------------------------------------------------
// Syscall Return Values
// ---------------------------------------------------------------------------

/// Error codes returned by syscalls (negative values).
pub const EPERM: i64 = -1;       // Operation not permitted
pub const ENOENT: i64 = -2;      // No such file or directory
pub const ESRCH: i64 = -3;       // No such process
pub const EINTR: i64 = -4;       // Interrupted system call
pub const EIO: i64 = -5;         // I/O error
pub const ENXIO: i64 = -6;       // No such device or address
pub const E2BIG: i64 = -7;       // Argument list too long
pub const ENOEXEC: i64 = -8;     // Exec format error
pub const EBADF: i64 = -9;       // Bad file number
pub const ECHILD: i64 = -10;     // No child processes
pub const EAGAIN: i64 = -11;     // Try again
pub const ENOMEM: i64 = -12;     // Out of memory
pub const EACCES: i64 = -13;     // Permission denied
pub const EFAULT: i64 = -14;     // Bad address
pub const ENOTBLK: i64 = -15;    // Block device required
pub const EBUSY: i64 = -16;      // Device or resource busy
pub const EEXIST: i64 = -17;     // File exists
pub const EXDEV: i64 = -18;      // Cross-device link
pub const ENODEV: i64 = -19;     // No such device
pub const ENOTDIR: i64 = -20;    // Not a directory
pub const EISDIR: i64 = -21;     // Is a directory
pub const EINVAL: i64 = -22;     // Invalid argument
pub const ENFILE: i64 = -23;     // File table overflow
pub const EMFILE: i64 = -24;     // Too many open files
pub const ENOTTY: i64 = -25;     // Not a typewriter
pub const ETXTBSY: i64 = -26;    // Text file busy
pub const EFBIG: i64 = -27;      // File too large
pub const ENOSPC: i64 = -28;     // No space left on device
pub const ESPIPE: i64 = -29;     // Illegal seek
pub const EROFS: i64 = -30;      // Read-only file system
pub const EMLINK: i64 = -31;     // Too many links
pub const EPIPE: i64 = -32;      // Broken pipe
pub const EDOM: i64 = -33;       // Math argument out of domain
pub const ERANGE: i64 = -34;     // Math result not representable

// ---------------------------------------------------------------------------
// Syscall Handler
// ---------------------------------------------------------------------------

/// Initialize the syscall interface.
/// Sets up the `syscall`/`sysret` MSRs.
pub fn init() {
    use crate::gdt;

    // STAR MSR: Segments for syscall/sysret.
    // Bits 32-47: Kernel CS (also sets kernel DS = CS + 8)
    // Bits 48-63: User CS (also sets user DS = CS + 8)
    let selectors = &gdt::GDT.1;
    let star_value =
        ((selectors.user_code_selector.0 as u64) << 48) |
        ((selectors.code_selector.0 as u64) << 32);

    unsafe {
        Star::write(
            x86_64::PrivilegeLevel::Ring3,
            x86_64::PrivilegeLevel::Ring0,
            selectors.user_code_selector,
            selectors.code_selector,
        );
    }

    // LSTAR MSR: Address of the syscall handler entry point.
    unsafe {
        LStar::write(x86_64::VirtAddr::new(syscall_entry as u64));
    }

    // SFMASK MSR: RFLAGS mask.
    // Bits cleared in RFLAGS when syscall is executed.
    // We clear IF (interrupt flag) and other sensitive flags.
    unsafe {
        SFMask::write(
            RFlags::INTERRUPT_FLAG |
            RFlags::TRAP_FLAG |
            RFlags::DIRECTION_FLAG |
            RFlags::IOPL_LOW |
            RFlags::IOPL_HIGH |
            RFlags::NESTED_TASK |
            RFlags::RESUME_FLAG
        );
    }

    // Enable the syscall instruction via EFER MSR.
    use x86_64::registers::model_specific::{Efer, EferFlags};
    unsafe {
        let mut efer = Efer::read();
        efer.insert(EferFlags::SYSTEM_CALL_EXTENSIONS);
        Efer::write(efer);
    }
}

/// The syscall entry point called by the `syscall` instruction.
///
/// When `syscall` executes:
/// - RCX = saved RIP (return address)
/// - R11 = saved RFLAGS
/// - CS = kernel code segment
/// - SS = kernel data segment
/// - RIP = LSTAR MSR value (this function)
///
/// Register state on entry:
/// - RAX = syscall number
/// - RDI = arg1, RSI = arg2, RDX = arg3
/// - R10 = arg4, R8 = arg5, R9 = arg6
#[no_mangle]
pub extern "C" fn syscall_entry() {
    // This is called from assembly. We need to save all registers,
    // then dispatch to the appropriate handler.
    //
    // In a complete implementation, this would be a naked function
    // with inline assembly. For now, we outline the structure:

    unsafe {
        core::arch::asm!(
            // Save user registers on the kernel stack.
            "push rcx",           // Saved RIP
            "push r11",           // Saved RFLAGS
            "push rax",
            "push rdi",
            "push rsi",
            "push rdx",
            "push r10",
            "push r8",
            "push r9",
            "push rbx",

            // Load arguments for dispatch.
            "mov rdi, rax",       // syscall number
            "mov rsi, [rsp + 24]", // arg1 (saved rdi)
            "mov rdx, [rsp + 32]", // arg2 (saved rsi)
            "mov rcx, [rsp + 40]", // arg3 (saved rdx)

            // Call the Rust dispatcher.
            "call {}",

            // Restore registers.
            "pop rbx",
            "pop r9",
            "pop r8",
            "pop r10",
            "pop rdx",
            "pop rsi",
            "pop rdi",
            "add rsp, 8",         // Skip saved rax (return value is in rax)
            "pop r11",            // Restore RFLAGS
            "pop rcx",            // Restore RIP

            // Return to user space.
            "sysretq",
            sym Syscall::dispatch,
            options(noreturn)
        );
    }
}

// ---------------------------------------------------------------------------
// Syscall Dispatcher
// ---------------------------------------------------------------------------

/// The syscall dispatcher.
/// Routes syscalls to their implementations based on the syscall number.
pub struct Syscall;

impl Syscall {
    /// Dispatch a syscall to its handler.
    ///
    /// # Arguments
    /// * `num` - Syscall number (from RAX)
    /// * `a1` - Argument 1 (from RDI)
    /// * `a2` - Argument 2 (from RSI)
    /// * `a3` - Argument 3 (from RDX)
    ///
    /// # Returns
    /// The syscall return value (placed back in RAX).
    pub fn dispatch(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {
        let result = match num {
            SYS_READ => Self::sys_read(a1, a2, a3),
            SYS_WRITE => Self::sys_write(a1, a2, a3),
            SYS_OPEN => Self::sys_open(a1, a2, a3),
            SYS_CLOSE => Self::sys_close(a1),
            SYS_EXIT => Self::sys_exit(a1 as i32),
            SYS_GETPID => Self::sys_getpid(),
            SYS_GETPPID => Self::sys_getppid(),
            SYS_KILL => Self::sys_kill(a1, a2 as i32),
            SYS_FORK => Self::sys_fork(),
            SYS_EXECVE => Self::sys_execve(a1, a2, a3),
            SYS_BRK => Self::sys_brk(a1),
            SYS_MMAP => Self::sys_mmap(a1, a2, a3),
            SYS_MUNMAP => Self::sys_munmap(a1, a2),
            SYS_GETDENTS => Self::sys_getdents(a1, a2, a3),
            SYS_CHDIR => Self::sys_chdir(a1),
            SYS_GETCWD => Self::sys_getcwd(a1, a2),
            SYS_MKDIR => Self::sys_mkdir(a1, a2),
            SYS_RMDIR => Self::sys_rmdir(a1),
            SYS_UNLINK => Self::sys_unlink(a1),
            SYS_RENAME => Self::sys_rename(a1, a2),
            SYS_STAT => Self::sys_stat(a1, a2),
            SYS_FSTAT => Self::sys_fstat(a1, a2),
            SYS_MOUNT => Self::sys_mount(a1, a2, a3),
            SYS_UMOUNT2 => Self::sys_umount2(a1, a2),
            SYS_NANOSLEEP => Self::sys_nanosleep(a1, a2),
            SYS_UNAME => Self::sys_uname(a1),
            SYS_SYSINFO => Self::sys_sysinfo(a1),
            SYS_GETUID => Self::sys_getuid(),
            SYS_GETGID => Self::sys_getgid(),
            SYS_SETUID => Self::sys_setuid(a1),
            SYS_SETGID => Self::sys_setgid(a1),
            SYS_GETEUID => Self::sys_geteuid(),
            SYS_GETEGID => Self::sys_getegid(),
            SYS_IOCTL => Self::sys_ioctl(a1, a2, a3),
            SYS_SCHED_YIELD => Self::sys_sched_yield(),
            SYS_DUP => Self::sys_dup(a1),
            SYS_DUP2 => Self::sys_dup2(a1, a2),
            SYS_PIPE => Self::sys_pipe(a1),
            SYS_SYNC => Self::sys_sync(),
            SYS_REBOOT => Self::sys_reboot(a1),
            _ => {
                serial_println!("[SYSCALL] Unknown syscall: {}", num);
                ENOSYS as u64
            }
        };

        // Return value: if negative, it's an error code.
        if (result as i64) < 0 {
            // Return as unsigned negative (two's complement)
            result as u64
        } else {
            result as u64
        }
    }

    // -----------------------------------------------------------------------
    // Syscall Implementations
    // -----------------------------------------------------------------------

    /// Read from a file descriptor.
    /// `read(fd, buf, count)` -> bytes read
    fn sys_read(fd: u64, buf: u64, count: u64) -> u64 {
        match fd {
            0 => {
                // STDIN - read from keyboard buffer
                // In a real implementation, this would block until input is available
                0 // EOF for now
            }
            _ => {
                // Read from file descriptor table
                // Would look up the file and call vfs::read()
                EBADF as u64
            }
        }
    }

    /// Write to a file descriptor.
    /// `write(fd, buf, count)` -> bytes written
    fn sys_write(fd: u64, buf: u64, count: u64) -> u64 {
        match fd {
            1 => {
                // STDOUT - write to VGA/serial
                let slice = unsafe {
                    core::slice::from_raw_parts(buf as *const u8, count as usize)
                };
                if let Ok(s) = core::str::from_utf8(slice) {
                    print!("{}", s);
                }
                count
            }
            2 => {
                // STDERR - write to serial only
                let slice = unsafe {
                    core::slice::from_raw_parts(buf as *const u8, count as usize)
                };
                if let Ok(s) = core::str::from_utf8(slice) {
                    serial_print!("{}", s);
                }
                count
            }
            _ => {
                // Write to file descriptor table
                // Would look up the file and call vfs::write()
                EBADF as u64
            }
        }
    }

    /// Open a file.
    /// `open(pathname, flags, mode)` -> fd
    fn sys_open(pathname: u64, flags: u64, mode: u64) -> u64 {
        // Would convert pathname to string, parse flags, and call vfs::open()
        // For now, return a placeholder
        serial_println!("[SYSCALL] open({:#x}, flags={:#x}, mode={:#x})", pathname, flags, mode);
        0 // Return stdin as placeholder
    }

    /// Close a file descriptor.
    fn sys_close(fd: u64) -> u64 {
        // Would remove from file descriptor table
        0
    }

    /// Exit the current process.
    /// `exit(status)` -> never returns
    fn sys_exit(status: i32) -> u64 {
        // In a real implementation:
        // 1. Set process state to Zombie
        // 2. Store exit code
        // 3. Notify parent
        // 4. Yield CPU
        serial_println!("[SYSCALL] exit({})", status);
        0
    }

    /// Get process ID.
    fn sys_getpid() -> u64 {
        // Return current process's PID
        unsafe {
            if let Some(ref sched) = super::SCHEDULER {
                if let Some(proc) = sched.current_process() {
                    return proc.pid;
                }
            }
        }
        0
    }

    /// Get parent process ID.
    fn sys_getppid() -> u64 {
        unsafe {
            if let Some(ref sched) = super::SCHEDULER {
                if let Some(proc) = sched.current_process() {
                    return proc.ppid;
                }
            }
        }
        0
    }

    /// Send signal to a process.
    /// `kill(pid, sig)` -> 0 on success
    fn sys_kill(pid: u64, sig: i32) -> u64 {
        serial_println!("[SYSCALL] kill(pid={}, sig={})", pid, sig);
        unsafe {
            if let Some(ref mut sched) = super::SCHEDULER {
                if sched.kill(pid) {
                    return 0;
                }
            }
        }
        ESRCH as u64
    }

    /// Fork the current process.
    fn sys_fork() -> u64 {
        // Would duplicate the current process
        // 1. Allocate new PID
        // 2. Copy page tables (COW)
        // 3. Copy file descriptors
        // 4. Add to ready queue
        // Return child PID to parent, 0 to child
        ENOSYS as u64
    }

    /// Execute a program.
    fn sys_execve(pathname: u64, argv: u64, envp: u64) -> u64 {
        // Would:
        // 1. Read executable from VFS
        // 2. Parse ELF headers
        // 3. Set up new address space
        // 4. Transfer control to entry point
        ENOSYS as u64
    }

    /// Change data segment size (heap).
    fn sys_brk(new_brk: u64) -> u64 {
        // Would adjust the process's heap limit
        // Return new program break on success
        new_brk
    }

    /// Map memory pages.
    fn sys_mmap(addr: u64, length: u64, prot: u64) -> u64 {
        // Would allocate virtual memory region
        ENOSYS as u64
    }

    /// Unmap memory pages.
    fn sys_munmap(addr: u64, length: u64) -> u64 {
        // Would free virtual memory region
        0
    }

    /// Read directory entries.
    fn sys_getdents(fd: u64, dirp: u64, count: u64) -> u64 {
        // Would read directory entries from VFS
        ENOSYS as u64
    }

    /// Change working directory.
    fn sys_chdir(path: u64) -> u64 {
        // Would update process's cwd in the VFS
        0
    }

    /// Get current working directory.
    fn sys_getcwd(buf: u64, size: u64) -> u64 {
        // Would copy cwd string to user buffer
        0
    }

    /// Create a directory.
    fn sys_mkdir(pathname: u64, mode: u64) -> u64 {
        // Would call vfs::mkdir()
        0
    }

    /// Remove a directory.
    fn sys_rmdir(pathname: u64) -> u64 {
        // Would call vfs::rmdir()
        0
    }

    /// Delete a file.
    fn sys_unlink(pathname: u64) -> u64 {
        // Would call vfs::unlink()
        0
    }

    /// Rename a file.
    fn sys_rename(oldpath: u64, newpath: u64) -> u64 {
        // Would call vfs::rename()
        0
    }

    /// Get file status.
    fn sys_stat(pathname: u64, statbuf: u64) -> u64 {
        // Would call vfs::stat()
        0
    }

    /// Get file status by fd.
    fn sys_fstat(fd: u64, statbuf: u64) -> u64 {
        // Would look up fd and call vfs::stat()
        0
    }

    /// Mount a filesystem.
    fn sys_mount(src: u64, target: u64, fstype: u64) -> u64 {
        // Would call vfs::mount()
        0
    }

    /// Unmount a filesystem.
    fn sys_umount2(target: u64, flags: u64) -> u64 {
        // Would call vfs::umount()
        0
    }

    /// Sleep for a specified time.
    fn sys_nanosleep(req: u64, rem: u64) -> u64 {
        // Would block the process and set a timer
        0
    }

    /// Get system information.
    fn sys_uname(buf: u64) -> u64 {
        // Would fill in a utsname structure
        0
    }

    /// Get system statistics.
    fn sys_sysinfo(buf: u64) -> u64 {
        // Would fill in a sysinfo structure
        0
    }

    /// Get user ID.
    fn sys_getuid() -> u64 {
        0 // Root
    }

    /// Get group ID.
    fn sys_getgid() -> u64 {
        0 // Root
    }

    /// Set user ID.
    fn sys_setuid(uid: u64) -> u64 {
        0
    }

    /// Set group ID.
    fn sys_setgid(gid: u64) -> u64 {
        0
    }

    /// Get effective user ID.
    fn sys_geteuid() -> u64 {
        0 // Root
    }

    /// Get effective group ID.
    fn sys_getegid() -> u64 {
        0 // Root
    }

    /// Device control.
    fn sys_ioctl(fd: u64, request: u64, arg: u64) -> u64 {
        // Would dispatch to device-specific ioctl handler
        EINVAL as u64
    }

    /// Yield the CPU.
    fn sys_sched_yield() -> u64 {
        // Would move current process to end of ready queue and switch
        0
    }

    /// Duplicate a file descriptor.
    fn sys_dup(fd: u64) -> u64 {
        // Would duplicate fd in the file descriptor table
        fd
    }

    /// Duplicate a file descriptor to a specific number.
    fn sys_dup2(oldfd: u64, newfd: u64) -> u64 {
        // Would duplicate oldfd to newfd
        newfd
    }

    /// Create a pipe.
    fn sys_pipe(pipefd: u64) -> u64 {
        // Would create a pipe and write fds to the array
        0
    }

    /// Sync filesystems.
    fn sys_sync() -> u64 {
        // Would flush all dirty buffers to disk
        0
    }

    /// Reboot or halt the system.
    fn sys_reboot(cmd: u64) -> u64 {
        match cmd {
            0x01234567 => {
                // LINUX_REBOOT_CMD_RESTART
                println!("Restarting system...");
                unsafe {
                    x86_64::instructions::port::Port::new(0x64).write(0xFE as u8);
                }
            }
            0x4321FEDC => {
                // LINUX_REBOOT_CMD_HALT
                println!("Halting system...");
                loop {
                    unsafe { x86_64::instructions::hlt(); }
                }
            }
            _ => {}
        }
        0
    }
}

const ENOSYS: i64 = -38; // Function not implemented
