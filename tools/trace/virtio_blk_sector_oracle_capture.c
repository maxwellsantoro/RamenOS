/*
 * Linux Oracle capsule for S13 virtio-blk harness.block sector capture.
 *
 * Performs one live sector read and one sector write against the QEMU virtio-blk
 * disk and emits block_sector_trace_v0-compatible JSONL on ttyS0.
 */

#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define SECTOR_SIZE 512u
#define BLOCK_SHM_CAP 4096u
#define READ_OFFSET 0u
#define WRITE_OFFSET 512u
#define READ_REQUEST_ID 1u
#define WRITE_REQUEST_ID 2u
#define READ_LBA 0ull
#define WRITE_LBA 1ull
#define BLOCK_COUNT 1u

static unsigned event_seq = 1;
static uint8_t harness_shm[BLOCK_SHM_CAP];

static void write_str(const char *msg) {
    write(STDOUT_FILENO, msg, strlen(msg));
}

static void write_line(const char *line) {
    write_str(line);
    write_str("\n");
}

static void poweroff(void) {
    sync();
    syscall(SYS_reboot, 0xfee1dead, 0x28121969, 0x4321fedc, 0);
    for (;;) {
        pause();
    }
}

static uint64_t now_ns(void) {
    struct timespec ts;

    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
        return 0;
    }
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

static int wait_for_path(const char *path) {
    for (int i = 0; i < 200; i++) {
        if (access(path, F_OK) == 0) {
            return 0;
        }
        usleep(100000);
    }
    return -1;
}

static void fill_sector(uint8_t *buf, uint8_t mul) {
    for (unsigned i = 0; i < SECTOR_SIZE; i++) {
        buf[i] = (uint8_t)((i * mul) & 0xffu);
    }
}

static int hex_encode(const uint8_t *data, unsigned len, char *out, size_t out_cap) {
    static const char *hex = "0123456789abcdef";
    if (out_cap < (size_t)len * 2u + 1u) {
        return -1;
    }
    for (unsigned i = 0; i < len; i++) {
        out[i * 2] = hex[(data[i] >> 4) & 0x0f];
        out[i * 2 + 1] = hex[data[i] & 0x0f];
    }
    out[len * 2] = '\0';
    return 0;
}

static void emit_sector_event(const char *kind, unsigned request_id, uint64_t lba,
                              const uint8_t *payload) {
    char hex[(SECTOR_SIZE * 2u) + 1];
    char line[4096];

    if (hex_encode(payload, SECTOR_SIZE, hex, sizeof(hex)) != 0) {
        write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: fail hex_encode\n");
        poweroff();
    }

    snprintf(line, sizeof(line),
             "{\"seq\":%u,\"timestamp_ns\":%llu,\"kind\":\"%s\","
             "\"request_id\":%u,\"lba\":%llu,\"block_count\":%u,\"block_size\":%u,"
             "\"shm_cap\":%u,\"offset\":%u,\"len\":%u,\"payload_hex\":\"%s\","
             "\"status\":0,\"bytes\":%u,\"result\":\"ok\"}",
             event_seq++, (unsigned long long)now_ns(), kind, request_id,
             (unsigned long long)lba, BLOCK_COUNT, SECTOR_SIZE, BLOCK_SHM_CAP,
             (unsigned)(strcmp(kind, "read_blocks") == 0 ? READ_OFFSET : WRITE_OFFSET),
             SECTOR_SIZE, hex, SECTOR_SIZE);
    write_line(line);
}

int main(void) {
    uint8_t read_pattern[SECTOR_SIZE];
    uint8_t write_pattern[SECTOR_SIZE];
    int fd;

    write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: boot\n");

    mkdir("/proc", 0555);
    mkdir("/sys", 0555);
    mkdir("/dev", 0555);
    mount("proc", "/proc", "proc", 0, "");
    mount("sysfs", "/sys", "sysfs", 0, "");
    mount("devtmpfs", "/dev", "devtmpfs", 0, "");

    if (wait_for_path("/dev/vda") != 0) {
        write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: fail block_device_missing\n");
        poweroff();
    }

    fill_sector(read_pattern, 0x13u);
    fill_sector(write_pattern, 0x37u);

    fd = open("/dev/vda", O_RDWR | O_SYNC);
    if (fd < 0) {
        write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: fail block_open\n");
        poweroff();
    }

    if (pwrite(fd, read_pattern, SECTOR_SIZE, (off_t)(READ_LBA * SECTOR_SIZE)) !=
        (ssize_t)SECTOR_SIZE) {
        write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: fail seed_read_sector\n");
        poweroff();
    }

    write_line("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE_BEGIN");
    write_line("{\"metadata\":{\"oracle\":\"linux-virtio-blk\",\"device_model\":\"virtio-blk-pci\","
               "\"harness\":\"harness.block\",\"harness_version\":\"1\","
               "\"capture_tool\":\"virtio_blk_sector_oracle_capture\"}}");

    if (pread(fd, harness_shm + READ_OFFSET, SECTOR_SIZE, (off_t)(READ_LBA * SECTOR_SIZE)) !=
        (ssize_t)SECTOR_SIZE) {
        write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: fail live_read_sector\n");
        poweroff();
    }
    emit_sector_event("read_blocks", READ_REQUEST_ID, READ_LBA, harness_shm + READ_OFFSET);

    memcpy(harness_shm + WRITE_OFFSET, write_pattern, SECTOR_SIZE);
    if (pwrite(fd, harness_shm + WRITE_OFFSET, SECTOR_SIZE, (off_t)(WRITE_LBA * SECTOR_SIZE)) !=
        (ssize_t)SECTOR_SIZE) {
        write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: fail live_write_sector\n");
        poweroff();
    }
    emit_sector_event("write_blocks", WRITE_REQUEST_ID, WRITE_LBA, harness_shm + WRITE_OFFSET);

    close(fd);
    write_line("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE_END");
    write_str("RAMEN_VIRTIO_BLK_SECTOR_CAPTURE: ok\n");
    poweroff();
    return 0;
}