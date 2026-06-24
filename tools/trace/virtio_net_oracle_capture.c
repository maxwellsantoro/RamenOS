/*
 * Linux Oracle capsule init for S11 virtio-net PCI/MMIO capture.
 *
 * Performs legacy virtio-net-pci bring-up (features, queues, MAC) against live
 * QEMU hardware and emits driver_protocol_trace_v0-compatible JSONL on ttyS0.
 */

#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define PCI_DEV "0000:00:04.0"
#define PCI_SYSFS "/sys/bus/pci/devices/" PCI_DEV

#define VIRTIO_PCI_DEVICE_FEATURES 0x00u
#define VIRTIO_PCI_GUEST_FEATURES 0x04u
#define VIRTIO_PCI_QUEUE_PFN 0x08u
#define VIRTIO_PCI_QUEUE_NUM 0x0cu
#define VIRTIO_PCI_QUEUE_SEL 0x0eu
#define VIRTIO_PCI_STATUS 0x12u
#define VIRTIO_PCI_CONFIG_OFF 0x14u

#define VIRTIO_CONFIG_S_ACKNOWLEDGE 1u
#define VIRTIO_CONFIG_S_DRIVER 2u
#define VIRTIO_CONFIG_S_DRIVER_OK 4u
#define VIRTIO_CONFIG_S_FEATURES_OK 8u

#define RX_QUEUE_PFN 0x100000u
#define TX_QUEUE_PFN 0x101000u
#define DEFAULT_QUEUE_SIZE 256u

static unsigned event_seq = 1;
static void *bar0 = NULL;
static size_t bar_size = 0;

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
    for (int i = 0; i < 100; i++) {
        if (access(path, F_OK) == 0) {
            return 0;
        }
        usleep(100000);
    }
    return -1;
}

static int read_pci_config_u16(unsigned offset, uint16_t *out) {
    char path[256];
    uint8_t buf[2];
    ssize_t nread;
    int fd;

    snprintf(path, sizeof(path), PCI_SYSFS "/config");
    fd = open(path, O_RDONLY);
    if (fd < 0) {
        return -1;
    }
    if (lseek(fd, (off_t)offset, SEEK_SET) < 0) {
        close(fd);
        return -1;
    }
    nread = read(fd, buf, sizeof(buf));
    close(fd);
    if (nread != (ssize_t)sizeof(buf)) {
        return -1;
    }
    *out = (uint16_t)buf[0] | ((uint16_t)buf[1] << 8);
    return 0;
}

static int map_bar0(void) {
    char path[256];
    char size_path[256];
    char size_buf[32];
    ssize_t nread;
    size_t map_size = 4096;
    void *map;
    int fd;
    int size_fd;
    unsigned bar;

    for (bar = 0; bar < 6; bar++) {
        snprintf(path, sizeof(path), PCI_SYSFS "/resource%lu", (unsigned long)bar);
        if (access(path, F_OK) != 0) {
            continue;
        }

        snprintf(size_path, sizeof(size_path), PCI_SYSFS "/resource%lu_size", (unsigned long)bar);
        map_size = 4096;
        size_fd = open(size_path, O_RDONLY);
        if (size_fd >= 0) {
            memset(size_buf, 0, sizeof(size_buf));
            nread = read(size_fd, size_buf, sizeof(size_buf) - 1);
            close(size_fd);
            if (nread > 0) {
                unsigned long long parsed = strtoull(size_buf, NULL, 0);
                if (parsed > 0 && parsed < (1ull << 30)) {
                    map_size = (size_t)parsed;
                }
            }
        }

        fd = open(path, O_RDWR | O_SYNC);
        if (fd < 0) {
            continue;
        }

        map = mmap(NULL, map_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        close(fd);
        if (map == MAP_FAILED) {
            continue;
        }

        bar0 = map;
        bar_size = map_size;
        return 0;
    }

    return -1;
}

static void emit_event(const char *kind, unsigned bar, uint64_t offset, unsigned width,
                       uint64_t value) {
    char line[512];

    snprintf(line, sizeof(line),
             "{\"seq\":%u,\"timestamp_ns\":%llu,\"kind\":\"%s\",\"bar\":%u,"
             "\"offset\":%llu,\"width\":%u,\"value\":%llu,\"result\":\"ok\"}",
             event_seq++, (unsigned long long)now_ns(), kind, bar, (unsigned long long)offset, width,
             (unsigned long long)value);
    write_line(line);
}

static uint32_t mmio_read32(uint32_t offset) {
    return *(volatile uint32_t *)((char *)bar0 + offset);
}

static uint16_t mmio_read16(uint32_t offset) {
    return *(volatile uint16_t *)((char *)bar0 + offset);
}

static void mmio_write32(uint32_t offset, uint32_t value) {
    *(volatile uint32_t *)((char *)bar0 + offset) = value;
}

static void mmio_write16(uint32_t offset, uint16_t value) {
    *(volatile uint16_t *)((char *)bar0 + offset) = value;
}

static uint32_t record_mmio_read32(uint32_t offset) {
    uint32_t value = mmio_read32(offset);
    emit_event("mmio_read", 0, offset, 4, value);
    return value;
}

static uint16_t record_mmio_read16(uint32_t offset) {
    uint16_t value = mmio_read16(offset);
    emit_event("mmio_read", 0, offset, 2, value);
    return value;
}

static void record_mmio_write32(uint32_t offset, uint32_t value) {
    mmio_write32(offset, value);
    emit_event("mmio_write", 0, offset, 4, value);
}

static void record_mmio_write16(uint32_t offset, uint16_t value) {
    mmio_write16(offset, value);
    emit_event("mmio_write", 0, offset, 2, value);
}

static void setup_queue(uint16_t queue_index, uint32_t pfn, uint16_t queue_size) {
    record_mmio_write16(VIRTIO_PCI_QUEUE_SEL, queue_index);
    (void)record_mmio_read16(VIRTIO_PCI_QUEUE_NUM);
    record_mmio_write16(VIRTIO_PCI_QUEUE_NUM, queue_size);
    record_mmio_write32(VIRTIO_PCI_QUEUE_PFN, pfn);
}

static void record_mac_bytes(void) {
    uint32_t offset;
    unsigned i;

    for (i = 0; i < 3; i++) {
        offset = VIRTIO_PCI_CONFIG_OFF + (i * 2u);
        (void)record_mmio_read16(offset);
    }
}

int main(void) {
    uint16_t vendor = 0;
    uint16_t device = 0;
    uint32_t host_features = 0;
    uint16_t status = 0;

    write_str("RAMEN_VIRTIO_NET_CAPTURE: boot\n");

    mkdir("/proc", 0555);
    mkdir("/sys", 0555);
    mount("proc", "/proc", "proc", 0, "");
    mount("sysfs", "/sys", "sysfs", 0, "");

    if (wait_for_path(PCI_SYSFS "/vendor") != 0) {
        write_str("RAMEN_VIRTIO_NET_CAPTURE: fail pci_device_missing\n");
        poweroff();
    }

    write_line("RAMEN_VIRTIO_NET_CAPTURE_BEGIN");
    write_line("{\"metadata\":{\"oracle\":\"linux-virtio-net\",\"device_model\":\"virtio-net-pci\","
               "\"pci_vendor_id\":6900,\"pci_device_id\":4096,"
               "\"pci_bdf\":\"0000:00:04.0\",\"capture_tool\":\"virtio_net_oracle_capture\"}}");

    if (read_pci_config_u16(0, &vendor) != 0) {
        write_str("RAMEN_VIRTIO_NET_CAPTURE: fail vendor_read\n");
        poweroff();
    }
    emit_event("pci_config_read", 255, 0, 2, vendor);

    if (read_pci_config_u16(2, &device) != 0) {
        write_str("RAMEN_VIRTIO_NET_CAPTURE: fail device_read\n");
        poweroff();
    }
    emit_event("pci_config_read", 255, 2, 2, device);

    if (map_bar0() != 0) {
        write_str("RAMEN_VIRTIO_NET_CAPTURE: fail bar0_map\n");
        poweroff();
    }

    status = record_mmio_read16(VIRTIO_PCI_STATUS);
    if (status != 0) {
        record_mmio_write16(VIRTIO_PCI_STATUS, 0);
        status = record_mmio_read16(VIRTIO_PCI_STATUS);
    }

    record_mmio_write16(VIRTIO_PCI_STATUS, VIRTIO_CONFIG_S_ACKNOWLEDGE);
    record_mmio_write16(VIRTIO_PCI_STATUS, VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER);

    {
        uint16_t lo = record_mmio_read16(VIRTIO_PCI_DEVICE_FEATURES);
        uint16_t hi = record_mmio_read16(VIRTIO_PCI_DEVICE_FEATURES + 2u);
        host_features = (uint32_t)lo | ((uint32_t)hi << 16);
        record_mmio_write32(VIRTIO_PCI_GUEST_FEATURES, host_features);
    }

    if (host_features != 0) {
        record_mmio_write16(VIRTIO_PCI_STATUS,
                            VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER |
                                VIRTIO_CONFIG_S_FEATURES_OK);
        status = record_mmio_read16(VIRTIO_PCI_STATUS);
        if ((status & VIRTIO_CONFIG_S_FEATURES_OK) == 0) {
            write_str("RAMEN_VIRTIO_NET_CAPTURE: fail features_ok_rejected\n");
            poweroff();
        }
    }

    record_mac_bytes();
    setup_queue(0, RX_QUEUE_PFN, DEFAULT_QUEUE_SIZE);
    setup_queue(1, TX_QUEUE_PFN, DEFAULT_QUEUE_SIZE);

    if (host_features != 0) {
        record_mmio_write16(VIRTIO_PCI_STATUS,
                            VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER |
                                VIRTIO_CONFIG_S_FEATURES_OK | VIRTIO_CONFIG_S_DRIVER_OK);
    } else {
        record_mmio_write16(VIRTIO_PCI_STATUS,
                            VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER |
                                VIRTIO_CONFIG_S_DRIVER_OK);
    }

    munmap(bar0, bar_size);
    write_line("RAMEN_VIRTIO_NET_CAPTURE_END");
    write_str("RAMEN_VIRTIO_NET_CAPTURE: ok\n");
    poweroff();
    return 0;
}