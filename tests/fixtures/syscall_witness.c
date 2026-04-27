/*
 * Minimal x86_64 program that issues a fixed sequence of syscalls
 * without any libc startup or runtime. Compile with:
 *
 *   cc -nostdlib -static -no-pie -o syscall_witness syscall_witness.c
 *
 * and invoke as the target of `sandprint profile run --`.
 *
 * Issued syscalls, in order:
 *
 *   write(2, "witness:start\n", 14)
 *   openat(AT_FDCWD, "/dev/null", O_RDONLY)
 *   close(<fd>)             (only when openat succeeds, which it always does)
 *   write(2, "witness:done\n", 13)
 *   exit_group(0)
 *
 * Tests that drive the binary should expect at least these syscalls
 * (write, openat, close, exit_group) in the trace.
 */

#if !defined(__x86_64__)
# error "syscall_witness.c is x86_64-only; gate the test that builds it"
#endif

#define SYS_write       1
#define SYS_close       3
#define SYS_openat      257
#define SYS_exit_group  231

#define AT_FDCWD        (-100)
#define O_RDONLY        0

static long sys3(long n, long a1, long a2, long a3) {
    long ret;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "0"(n), "D"(a1), "S"(a2), "d"(a3)
        : "rcx", "r11", "memory"
    );
    return ret;
}

void _start(void) {
    static const char start_msg[] = "witness:start\n";
    static const char done_msg[]  = "witness:done\n";
    static const char devnull[]   = "/dev/null";

    sys3(SYS_write, 2, (long)start_msg, sizeof(start_msg) - 1);
    long fd = sys3(SYS_openat, AT_FDCWD, (long)devnull, O_RDONLY);
    if (fd >= 0) {
        sys3(SYS_close, fd, 0, 0);
    }
    sys3(SYS_write, 2, (long)done_msg, sizeof(done_msg) - 1);
    sys3(SYS_exit_group, 0, 0, 0);

    /* Unreachable; keep the linker happy if -fno-asynchronous-unwind-tables
       is not in effect. */
    for (;;) { }
}
