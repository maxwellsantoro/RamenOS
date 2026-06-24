/*
 * Linux Oracle capsule init for S11 virtio-net harness packet capture.
 *
 * Captures harness.net packet events via kernel eth0 (virtio_net.ko + AF_PACKET)
 * with a userspace legacy virtqueue fallback when module load or netdev I/O fails.
 */

#include <arpa/inet.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <linux/if_ether.h>
#include <linux/if_packet.h>
#include <net/if.h>
#include <time.h>
#include <unistd.h>

#ifndef SYS_init_module
#define SYS_init_module 175
#endif

#define PCI_DEV "0000:00:04.0"
#define PCI_SYSFS "/sys/bus/pci/devices/" PCI_DEV

#define VIRTIO_PCI_DEVICE_FEATURES 0x00u
#define VIRTIO_PCI_GUEST_FEATURES 0x04u
#define VIRTIO_PCI_QUEUE_PFN 0x08u
#define VIRTIO_PCI_QUEUE_NUM 0x0cu
#define VIRTIO_PCI_QUEUE_SEL 0x0eu
#define VIRTIO_PCI_QUEUE_NOTIFY 0x10u
#define VIRTIO_PCI_STATUS 0x12u
#define VIRTIO_PCI_CONFIG_OFF 0x14u

#define VIRTIO_CONFIG_S_ACKNOWLEDGE 1u
#define VIRTIO_CONFIG_S_DRIVER 2u
#define VIRTIO_CONFIG_S_DRIVER_OK 4u
#define VIRTIO_CONFIG_S_FEATURES_OK 8u

#define PACKET_QUEUE_SIZE 16u
#define VRING_ALIGN 4096u
#define VRING_DESC_F_WRITE 2u

#define PACKET_SHM_CAP 4096u
#define SEND_OFFSET 0u
#define RECV_OFFSET 2048u
#define SEND_REQUEST_ID 1u
#define RECV_REQUEST_ID 2u
#define ETH_ALEN 6u
#define VIRTIO_NET_HDR_SIZE 10u
#define ARP_ETH_LEN 42u

struct vring_desc {
    uint64_t addr;
    uint32_t len;
    uint16_t flags;
    uint16_t next;
};

struct vring_avail {
    uint16_t flags;
    uint16_t idx;
    uint16_t ring[];
};

struct vring_used_elem {
    uint32_t id;
    uint32_t len;
};

struct vring_used {
    uint16_t flags;
    uint16_t idx;
    struct vring_used_elem ring[];
};

struct vring {
    unsigned int num;
    struct vring_desc *desc;
    struct vring_avail *avail;
    struct vring_used *used;
};

struct packet_memory {
    void *rx_ring;
    void *tx_ring;
    uint8_t *rx_buf;
    uint8_t *tx_buf;
    uint8_t *harness_shm;
    uint32_t rx_ring_pfn;
    uint32_t tx_ring_pfn;
    uint64_t rx_buf_phys;
    uint64_t tx_buf_phys;
};

static unsigned packet_event_seq = 1;
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

static void unbind_kernel_driver(void) {
    char unbind_path[256];
    int fd;

    if (access(PCI_SYSFS "/driver", F_OK) != 0) {
        return;
    }

    snprintf(unbind_path, sizeof(unbind_path), PCI_SYSFS "/driver/unbind");
    fd = open(unbind_path, O_WRONLY);
    if (fd < 0) {
        return;
    }
    (void)write(fd, PCI_DEV "\n", strlen(PCI_DEV) + 1);
    close(fd);
    usleep(100000);
}

#define LOW_MEM_BASE 0x00100000u

static uintptr_t low_mem_cursor = LOW_MEM_BASE;

static void *map_low_mem(size_t size) {
    uintptr_t addr = low_mem_cursor;
    size_t aligned = (size + 4095u) & ~4095u;
    void *map = mmap((void *)addr, aligned, PROT_READ | PROT_WRITE,
                     MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0);

    if (map == MAP_FAILED) {
        return NULL;
    }
    low_mem_cursor += aligned;
    return map;
}

static uint64_t guest_phys_addr(const void *virt) {
    uintptr_t vaddr = (uintptr_t)virt;
    uintptr_t page = vaddr & ~0xfffu;
    unsigned long index = (unsigned long)(page / 4096u);
    uint64_t entry = 0;
    int fd;

    fd = open("/proc/self/pagemap", O_RDONLY);
    if (fd < 0) {
        return 0;
    }
    if (lseek(fd, (off_t)(index * sizeof(entry)), SEEK_SET) < 0) {
        close(fd);
        return 0;
    }
    if (read(fd, &entry, sizeof(entry)) != (ssize_t)sizeof(entry)) {
        close(fd);
        return 0;
    }
    close(fd);
    if ((entry & (1ull << 63)) == 0) {
        return 0;
    }
    return ((entry & ((1ull << 55) - 1ull)) * 4096ull) + (vaddr & 0xfffu);
}

static uint64_t buffer_phys_addr(void *virt) {
    uint64_t phys = guest_phys_addr(virt);
    if (phys != 0) {
        return phys;
    }
    return (uint64_t)(uintptr_t)virt;
}

static size_t vring_size_bytes(unsigned int num) {
    size_t desc = num * sizeof(struct vring_desc);
    size_t avail = sizeof(struct vring_avail) + num * sizeof(uint16_t) + sizeof(uint16_t);
    uintptr_t used_off = desc + avail;
    used_off = (used_off + VRING_ALIGN - 1u) & ~(uintptr_t)(VRING_ALIGN - 1u);
    size_t used = sizeof(struct vring_used) + num * sizeof(struct vring_used_elem) + sizeof(uint16_t);
    return used_off + used;
}

static void vring_init_local(struct vring *vr, unsigned int num, void *p) {
    char *base = (char *)p;
    uintptr_t used_off;

    vr->num = num;
    vr->desc = (struct vring_desc *)base;
    vr->avail = (struct vring_avail *)(base + num * sizeof(struct vring_desc));
    used_off = (uintptr_t)vr->avail + sizeof(struct vring_avail) + num * sizeof(uint16_t) +
               sizeof(uint16_t);
    used_off = (used_off + VRING_ALIGN - 1u) & ~(uintptr_t)(VRING_ALIGN - 1u);
    vr->used = (struct vring_used *)used_off;
    memset(p, 0, vring_size_bytes(num));
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

static uint16_t mmio_read16(uint32_t offset) {
    return *(volatile uint16_t *)((char *)bar0 + offset);
}

static void mmio_write16(uint32_t offset, uint16_t value) {
    *(volatile uint16_t *)((char *)bar0 + offset) = value;
}

static void mmio_write32(uint32_t offset, uint32_t value) {
    *(volatile uint32_t *)((char *)bar0 + offset) = value;
}

static void setup_queue(uint16_t queue_index, uint32_t pfn, uint16_t queue_size) {
    mmio_write16(VIRTIO_PCI_QUEUE_SEL, queue_index);
    (void)mmio_read16(VIRTIO_PCI_QUEUE_NUM);
    mmio_write16(VIRTIO_PCI_QUEUE_NUM, queue_size);
    mmio_write32(VIRTIO_PCI_QUEUE_PFN, pfn);
}

static void read_mac(uint8_t mac[ETH_ALEN]) {
    unsigned i;

    for (i = 0; i < 3; i++) {
        uint16_t word = mmio_read16(VIRTIO_PCI_CONFIG_OFF + (i * 2u));
        mac[i * 2] = (uint8_t)(word & 0xffu);
        mac[i * 2 + 1] = (uint8_t)((word >> 8) & 0xffu);
    }
}

static void kick_queue(uint16_t queue_index) {
    mmio_write16(VIRTIO_PCI_QUEUE_SEL, queue_index);
    __sync_synchronize();
    mmio_write16(VIRTIO_PCI_QUEUE_NOTIFY, queue_index);
    __sync_synchronize();
}

static void bytes_to_hex(const uint8_t *buf, size_t len, char *out, size_t out_len) {
    static const char hex[] = "0123456789abcdef";
    size_t i;

    if (out_len < (len * 2u) + 1u) {
        out[0] = '\0';
        return;
    }
    for (i = 0; i < len; i++) {
        out[i * 2] = hex[(buf[i] >> 4) & 0x0fu];
        out[i * 2 + 1] = hex[buf[i] & 0x0fu];
    }
    out[len * 2] = '\0';
}

static void emit_packet_event(const char *kind, uint64_t request_id, uint64_t shm_cap,
                              uint64_t offset, uint32_t len, const uint8_t *payload,
                              uint32_t status, uint32_t bytes, const char *notes) {
    char payload_hex[ARP_ETH_LEN * 2 + 1];
    char line[1024];

    bytes_to_hex(payload, len, payload_hex, sizeof(payload_hex));
    snprintf(line, sizeof(line),
             "{\"seq\":%u,\"kind\":\"%s\",\"timestamp_ns\":%llu,\"request_id\":%llu,"
             "\"shm_cap\":%llu,\"offset\":%llu,\"len\":%u,\"payload_hex\":\"%s\","
             "\"status\":%u,\"bytes\":%u,\"result\":\"ok\",\"notes\":\"%s\"}",
             packet_event_seq++, kind, (unsigned long long)now_ns(),
             (unsigned long long)request_id, (unsigned long long)shm_cap,
             (unsigned long long)offset, len, payload_hex, status, bytes, notes);
    write_line(line);
}

static size_t build_arp_probe(const uint8_t mac[ETH_ALEN], uint8_t *frame) {
    memset(frame, 0, ARP_ETH_LEN);
    memset(frame, 0xff, ETH_ALEN);
    memcpy(frame + ETH_ALEN, mac, ETH_ALEN);
    frame[12] = 0x08;
    frame[13] = 0x06;
    frame[14] = 0x00;
    frame[15] = 0x01;
    frame[16] = 0x08;
    frame[17] = 0x00;
    frame[18] = 0x06;
    frame[19] = 0x04;
    frame[20] = 0x00;
    frame[21] = 0x01;
    memcpy(frame + 22, mac, ETH_ALEN);
    frame[28] = 10;
    frame[29] = 0;
    frame[30] = 2;
    frame[31] = 15;
    frame[32] = 0;
    frame[33] = 0;
    frame[34] = 0;
    frame[35] = 0;
    frame[36] = 0;
    frame[37] = 0;
    frame[38] = 10;
    frame[39] = 0;
    frame[40] = 2;
    frame[41] = 2;
    return ARP_ETH_LEN;
}

static int alloc_packet_memory(struct packet_memory *mem) {
    size_t ring_bytes = vring_size_bytes(PACKET_QUEUE_SIZE);

    memset(mem, 0, sizeof(*mem));
    mem->rx_ring = map_low_mem(ring_bytes);
    mem->tx_ring = map_low_mem(ring_bytes);
    mem->rx_buf = map_low_mem(2048);
    mem->tx_buf = map_low_mem(2048);
    mem->harness_shm = map_low_mem(PACKET_SHM_CAP);
    if (!mem->rx_ring || !mem->tx_ring || !mem->rx_buf || !mem->tx_buf || !mem->harness_shm) {
        return -1;
    }

    memset(mem->rx_ring, 0, ring_bytes);
    memset(mem->tx_ring, 0, ring_bytes);
    memset(mem->rx_buf, 0, 2048);
    memset(mem->tx_buf, 0, 2048);
    memset(mem->harness_shm, 0, PACKET_SHM_CAP);

    mem->rx_ring_pfn = (uint32_t)(buffer_phys_addr(mem->rx_ring) >> 12);
    mem->tx_ring_pfn = (uint32_t)(buffer_phys_addr(mem->tx_ring) >> 12);
    mem->rx_buf_phys = buffer_phys_addr(mem->rx_buf);
    mem->tx_buf_phys = buffer_phys_addr(mem->tx_buf);
    if (mem->rx_ring_pfn == 0 || mem->tx_ring_pfn == 0 || mem->rx_buf_phys == 0 ||
        mem->tx_buf_phys == 0) {
        return -1;
    }
    return 0;
}

static uint32_t negotiated_host_features = 0;

static int bring_up_device(uint8_t mac[ETH_ALEN], const struct packet_memory *mem) {
    uint32_t host_features = 0;
    uint16_t status = 0;
    uint16_t vendor = 0;
    uint16_t device = 0;

    if (read_pci_config_u16(0, &vendor) != 0 || read_pci_config_u16(2, &device) != 0) {
        return -1;
    }
    if (map_bar0() != 0) {
        return -1;
    }

    status = mmio_read16(VIRTIO_PCI_STATUS);
    if (status != 0) {
        mmio_write16(VIRTIO_PCI_STATUS, 0);
        (void)mmio_read16(VIRTIO_PCI_STATUS);
    }

    mmio_write16(VIRTIO_PCI_STATUS, VIRTIO_CONFIG_S_ACKNOWLEDGE);
    mmio_write16(VIRTIO_PCI_STATUS, VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER);

    {
        uint16_t lo = mmio_read16(VIRTIO_PCI_DEVICE_FEATURES);
        uint16_t hi = mmio_read16(VIRTIO_PCI_DEVICE_FEATURES + 2u);
        host_features = (uint32_t)lo | ((uint32_t)hi << 16);
        mmio_write32(VIRTIO_PCI_GUEST_FEATURES, host_features);
    }

    if (host_features != 0) {
        mmio_write16(VIRTIO_PCI_STATUS,
                     VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER |
                         VIRTIO_CONFIG_S_FEATURES_OK);
        status = mmio_read16(VIRTIO_PCI_STATUS);
        if ((status & VIRTIO_CONFIG_S_FEATURES_OK) == 0) {
            return -1;
        }
    }

    negotiated_host_features = host_features;
    read_mac(mac);
    setup_queue(0, mem->rx_ring_pfn, PACKET_QUEUE_SIZE);
    setup_queue(1, mem->tx_ring_pfn, PACKET_QUEUE_SIZE);

    return 0;
}

static int set_driver_ok(uint32_t host_features) {
    if (host_features != 0) {
        mmio_write16(VIRTIO_PCI_STATUS,
                     VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER |
                         VIRTIO_CONFIG_S_FEATURES_OK | VIRTIO_CONFIG_S_DRIVER_OK);
    } else {
        mmio_write16(VIRTIO_PCI_STATUS,
                     VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER |
                         VIRTIO_CONFIG_S_DRIVER_OK);
    }
    return 0;
}

static int exchange_packets(const uint8_t mac[ETH_ALEN], struct packet_memory *mem) {
    struct vring rx_vr;
    struct vring tx_vr;
    uint8_t arp_frame[ARP_ETH_LEN];
    size_t frame_len;
    uint16_t rx_last_used = 0;
    uint32_t rx_len = 0;
    int got_reply = 0;

    vring_init_local(&rx_vr, PACKET_QUEUE_SIZE, mem->rx_ring);
    vring_init_local(&tx_vr, PACKET_QUEUE_SIZE, mem->tx_ring);

    rx_vr.desc[0].addr = mem->rx_buf_phys;
    rx_vr.desc[0].len = 2048;
    rx_vr.desc[0].flags = VRING_DESC_F_WRITE;
    rx_vr.avail->ring[0] = 0;
    rx_vr.avail->idx = 1;
    __sync_synchronize();

    if (set_driver_ok(negotiated_host_features) != 0) {
        return -1;
    }

    kick_queue(0);

    frame_len = build_arp_probe(mac, arp_frame);
    memcpy(mem->harness_shm + SEND_OFFSET, arp_frame, frame_len);

    memset(mem->tx_buf, 0, VIRTIO_NET_HDR_SIZE);
    memcpy(mem->tx_buf + VIRTIO_NET_HDR_SIZE, arp_frame, frame_len);

    tx_vr.desc[0].addr = mem->tx_buf_phys;
    tx_vr.desc[0].len = (uint32_t)(VIRTIO_NET_HDR_SIZE + frame_len);
    tx_vr.desc[0].flags = 0;
    tx_vr.avail->ring[0] = 0;
    tx_vr.avail->idx = 1;
    __sync_synchronize();
    kick_queue(1);

    for (int i = 0; i < 10000; i++) {
        __sync_synchronize();
        if (rx_vr.used->idx != rx_last_used) {
            rx_len = rx_vr.used->ring[0].len;
            got_reply = 1;
            break;
        }
        usleep(1000);
    }

    if (!got_reply || rx_len < VIRTIO_NET_HDR_SIZE + 14u) {
        return -1;
    }

    {
        uint8_t *eth = mem->rx_buf + VIRTIO_NET_HDR_SIZE;
        uint32_t eth_len = rx_len - VIRTIO_NET_HDR_SIZE;
        if (eth_len > ARP_ETH_LEN) {
            eth_len = ARP_ETH_LEN;
        }
        memcpy(mem->harness_shm + RECV_OFFSET, eth, eth_len);

        emit_packet_event("send_packet", SEND_REQUEST_ID, PACKET_SHM_CAP, SEND_OFFSET,
                          (uint32_t)frame_len, arp_frame, 0, (uint32_t)frame_len,
                          "live ARP probe transmit");
        emit_packet_event("receive_packet", RECV_REQUEST_ID, PACKET_SHM_CAP, RECV_OFFSET,
                          eth_len, mem->harness_shm + RECV_OFFSET, 0, eth_len,
                          "live ARP reply receive");
    }

    return 0;
}

static int load_kernel_module(const char *path) {
    int fd;
    off_t size;
    void *image;
    long rc;

    fd = open(path, O_RDONLY);
    if (fd < 0) {
        return -1;
    }
    size = lseek(fd, 0, SEEK_END);
    if (size <= 0) {
        close(fd);
        return -1;
    }
    if (lseek(fd, 0, SEEK_SET) < 0) {
        close(fd);
        return -1;
    }
    image = mmap(NULL, (size_t)size, PROT_READ, MAP_PRIVATE, fd, 0);
    close(fd);
    if (image == MAP_FAILED) {
        return -1;
    }
    rc = syscall(SYS_init_module, image, (unsigned long)size, "");
    munmap(image, (size_t)size);
    return rc == 0 ? 0 : -1;
}

static int bring_up_kernel_netdev(void) {
    static const char *const modules[] = {
        "/lib/modules/failover.ko",
        "/lib/modules/net_failover.ko",
        "/lib/modules/virtio_net.ko",
    };
    unsigned i;

    for (i = 0; i < sizeof(modules) / sizeof(modules[0]); i++) {
        if (load_kernel_module(modules[i]) != 0) {
            return -1;
        }
    }

    if (wait_for_path("/sys/class/net/eth0") != 0) {
        return -1;
    }

    {
        struct ifreq ifr;
        int fd;

        memset(&ifr, 0, sizeof(ifr));
        strncpy(ifr.ifr_name, "eth0", IFNAMSIZ - 1);
        fd = socket(AF_INET, SOCK_DGRAM, 0);
        if (fd < 0) {
            return -1;
        }
        ifr.ifr_flags = IFF_UP | IFF_RUNNING;
        if (ioctl(fd, SIOCSIFFLAGS, &ifr) != 0) {
            close(fd);
            return -1;
        }
        close(fd);
    }

    return 0;
}

static int capture_via_kernel_netdev(uint8_t *harness_shm) {
    uint8_t mac[ETH_ALEN];
    uint8_t arp_frame[ARP_ETH_LEN];
    uint8_t rx_frame[256];
    struct ifreq ifr;
    struct sockaddr_ll bind_addr;
    struct sockaddr_ll dst_addr;
    size_t frame_len;
    int fd;
    int ifindex;
    ssize_t rx_len;
    uint32_t eth_len;

    if (bring_up_kernel_netdev() != 0) {
        return -1;
    }

    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, "eth0", IFNAMSIZ - 1);
    fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) {
        return -1;
    }
    if (ioctl(fd, SIOCGIFHWADDR, &ifr) != 0) {
        close(fd);
        return -1;
    }
    memcpy(mac, ifr.ifr_hwaddr.sa_data, ETH_ALEN);
    close(fd);

    ifindex = if_nametoindex("eth0");
    if (ifindex == 0) {
        return -1;
    }

    fd = socket(AF_PACKET, SOCK_RAW, htons(ETH_P_ALL));
    if (fd < 0) {
        return -1;
    }

    memset(&bind_addr, 0, sizeof(bind_addr));
    bind_addr.sll_family = AF_PACKET;
    bind_addr.sll_protocol = htons(ETH_P_ARP);
    bind_addr.sll_ifindex = (unsigned int)ifindex;
    if (bind(fd, (struct sockaddr *)&bind_addr, sizeof(bind_addr)) != 0) {
        close(fd);
        return -1;
    }

    frame_len = build_arp_probe(mac, arp_frame);
    memcpy(harness_shm + SEND_OFFSET, arp_frame, frame_len);

    memset(&dst_addr, 0, sizeof(dst_addr));
    dst_addr.sll_family = AF_PACKET;
    dst_addr.sll_ifindex = (unsigned int)ifindex;
    dst_addr.sll_halen = ETH_ALEN;
    memset(dst_addr.sll_addr, 0xff, ETH_ALEN);
    if (sendto(fd, arp_frame, frame_len, 0, (struct sockaddr *)&dst_addr, sizeof(dst_addr)) < 0) {
        close(fd);
        return -1;
    }

    {
        struct timeval timeout = {.tv_sec = 5, .tv_usec = 0};
        (void)setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout));
    }

    rx_len = recv(fd, rx_frame, sizeof(rx_frame), 0);
    close(fd);
    if (rx_len < 22) {
        return -1;
    }
    if (rx_frame[12] != 0x08 || rx_frame[13] != 0x06) {
        return -1;
    }
    if (((rx_frame[20] << 8) | rx_frame[21]) != 2) {
        return -1;
    }

    eth_len = (uint32_t)rx_len;
    if (eth_len > ARP_ETH_LEN) {
        eth_len = ARP_ETH_LEN;
    }
    memcpy(harness_shm + RECV_OFFSET, rx_frame, eth_len);

    emit_packet_event("send_packet", SEND_REQUEST_ID, PACKET_SHM_CAP, SEND_OFFSET,
                      (uint32_t)frame_len, arp_frame, 0, (uint32_t)frame_len,
                      "live ARP probe transmit");
    emit_packet_event("receive_packet", RECV_REQUEST_ID, PACKET_SHM_CAP, RECV_OFFSET, eth_len,
                      harness_shm + RECV_OFFSET, 0, eth_len, "live ARP reply receive");
    return 0;
}

static int capture_harness_events(const uint8_t mac[ETH_ALEN], struct packet_memory *mem) {
    if (capture_via_kernel_netdev(mem->harness_shm) == 0) {
        return 0;
    }

    unbind_kernel_driver();
    if (bring_up_device(mac, mem) != 0) {
        return -1;
    }

    return exchange_packets(mac, mem);
}

int main(void) {
    uint8_t mac[ETH_ALEN];
    struct packet_memory mem;

    write_str("RAMEN_VIRTIO_NET_PACKET_CAPTURE: boot\n");

    mkdir("/proc", 0555);
    mkdir("/sys", 0555);
    mount("proc", "/proc", "proc", 0, "");
    mount("sysfs", "/sys", "sysfs", 0, "");

    if (wait_for_path(PCI_SYSFS "/vendor") != 0) {
        write_str("RAMEN_VIRTIO_NET_PACKET_CAPTURE: fail pci_missing\n");
        poweroff();
    }

    if (alloc_packet_memory(&mem) != 0) {
        write_str("RAMEN_VIRTIO_NET_PACKET_CAPTURE: fail memory_alloc\n");
        poweroff();
    }

    write_line("RAMEN_VIRTIO_NET_PACKET_CAPTURE_BEGIN");
    write_line("{\"metadata\":{\"oracle\":\"linux-virtio-net\",\"device_model\":\"virtio-net-pci\","
               "\"harness\":\"harness.net\",\"harness_version\":\"1\","
               "\"capture_tool\":\"virtio_net_packet_oracle_capture\"}}");

    if (capture_harness_events(mac, &mem) != 0) {
        write_str("RAMEN_VIRTIO_NET_PACKET_CAPTURE: fail packet_exchange\n");
        poweroff();
    }

    write_line("RAMEN_VIRTIO_NET_PACKET_CAPTURE_END");
    write_str("RAMEN_VIRTIO_NET_PACKET_CAPTURE: ok\n");
    poweroff();
    return 0;
}