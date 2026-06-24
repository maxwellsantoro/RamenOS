#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/stat.h>
#include <sys/syscall.h>

static void write_str(const char *msg) {
    write(STDOUT_FILENO, msg, strlen(msg));
}

static void do_reboot(void) {
    syscall(SYS_reboot, 0xfee1dead, 0x28121969, 0x4321fedc, 0);
}

static void ensure_dir(const char *path) {
    mkdir(path, 0755);
}

static int wait_for_device(const char *path) {
    for (int i = 0; i < 50; i++) {
        if (access(path, F_OK) == 0) {
            return 0;
        }
        usleep(100000);
    }
    return -1;
}

int main(void) {
    write_str("COMPAT_S2: hello\n");

    ensure_dir("/dev");
    ensure_dir("/artifact");
    mount("devtmpfs", "/dev", "devtmpfs", 0, "");

    if (wait_for_device("/dev/vda") != 0) {
        write_str("COMPAT_S2: artifact device missing\n");
        sync();
        do_reboot();
        for (;;) {
            pause();
        }
    }

    if (mount("/dev/vda", "/artifact", "ext4", MS_RDONLY, "") != 0) {
        write_str("COMPAT_S2: mount artifact failed\n");
        sync();
        do_reboot();
        for (;;) {
            pause();
        }
    }

    int fd = open("/artifact/artifact.txt", O_RDONLY);
    if (fd >= 0) {
        char buf[4] = {0};
        if (read(fd, buf, 3) == 3 && buf[0] == 'o' && buf[1] == 'k' && buf[2] == '\n') {
            write_str("COMPAT_S2: read artifact ok\n");
        } else {
            write_str("COMPAT_S2: read artifact bad\n");
        }
        close(fd);
    } else {
        write_str("COMPAT_S2: read artifact missing\n");
    }

    int wfd = open("/artifact/should_fail.txt", O_WRONLY | O_CREAT, 0644);
    if (wfd >= 0) {
        write(wfd, "no\n", 3);
        close(wfd);
        write_str("COMPAT_S2: write blocked fail\n");
    } else {
        write_str("COMPAT_S2: write blocked ok\n");
    }

    sync();
    do_reboot();
    for (;;) {
        pause();
    }
    return 0;
}
