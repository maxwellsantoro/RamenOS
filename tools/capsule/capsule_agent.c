/* capsule_agent.c — minimal guest agent for virtio-serial relay */
#include <fcntl.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include "capsule_control_v0.h"

/* Protocol constants (must match kernel_api and capsule_relay) */
#define PROTOCOL_CAPSULE_CONTROL 0x200
#define MSG_HELLO 1
#define MSG_HELLO_REPLY 2
#define MSG_HEALTH 3
#define MSG_HEALTH_REPLY 4
#define MSG_SHUTDOWN 5
#define MSG_SHUTDOWN_REPLY 6

#define PROTOCOL_ECHO 0x210
#define MSG_ECHO_REQUEST 1
#define MSG_ECHO_REPLY 2

#define STATUS_OK 0
#define STATUS_ERR 1
#define STATUS_INVALID 2

#define CAPSULE_ID 0xC0A50000000001ULL

/*
 * Envelope struct — must match kernel_api::ipc::Envelope exactly.
 * #[repr(C)] in Rust means System V ABI: natural alignment, trailing padding.
 * Fields: protocol(4) + msg_type(4) + handle(8) + payload_len(4) + payload(64) = 84
 * Alignment of handle (u64) forces struct alignment = 8, so sizeof = 88.
 *
 * Wire format: **little-endian** for cross-arch determinism (traces as spec).
 * x86_64 and aarch64 are both little-endian, so native memcpy works.
 * If targeting big-endian guests, use htole32/le32toh macros.
 */
typedef struct {
    uint32_t protocol;
    uint32_t msg_type;
    uint64_t handle;
    uint32_t payload_len;
    uint8_t payload[64];
} Envelope;

_Static_assert(sizeof(Envelope) == 88, "Envelope size must be 88 bytes");

_Static_assert(sizeof(Hello) == 16, "Hello size mismatch");

_Static_assert(sizeof(HelloReply) == 16, "HelloReply size mismatch");

_Static_assert(sizeof(Health) == 8, "Health size mismatch");

_Static_assert(sizeof(HealthReply) == 16, "HealthReply size mismatch");

_Static_assert(sizeof(Shutdown) == 16, "Shutdown size mismatch");

_Static_assert(sizeof(ShutdownReply) == 16, "ShutdownReply size mismatch");

typedef struct {
    uint64_t request_id;
    uint32_t payload_len;
    uint32_t reserved;
} EchoRequest;
_Static_assert(sizeof(EchoRequest) == 16, "EchoRequest size mismatch");

typedef struct {
    uint64_t request_id;
    uint32_t payload_len;
    uint32_t status;
} EchoReply;
_Static_assert(sizeof(EchoReply) == 16, "EchoReply size mismatch");

/* Agent state */
typedef struct {
    uint64_t session_id;
    uint32_t error_count;
} AgentState;

static void write_str(const char *msg) {
    write(STDOUT_FILENO, msg, strlen(msg));
}

static int read_all(int fd, void *buf, size_t n) {
    uint8_t *p = buf;
    while (n > 0) {
        ssize_t r = read(fd, p, n);
        if (r <= 0) return -1;
        p += r;
        n -= (size_t)r;
    }
    return 0;
}

static int write_all(int fd, const void *buf, size_t n) {
    const uint8_t *p = buf;
    while (n > 0) {
        ssize_t w = write(fd, p, n);
        if (w <= 0) return -1;
        p += w;
        n -= (size_t)w;
    }
    return 0;
}

static void handle_hello(const Envelope *req, Envelope *reply, AgentState *state) {
    Hello h;
    memcpy(&h, req->payload, sizeof(Hello));

    uint64_t session_id = CAPSULE_ID ^ h.capsule_id ^ 0x5A5A;
    state->session_id = session_id;

    HelloReply hr = {
        .session_id = session_id,
        .status = STATUS_OK,
        .reserved = 0,
    };

    reply->protocol = PROTOCOL_CAPSULE_CONTROL;
    reply->msg_type = MSG_HELLO_REPLY;
    reply->handle = 0;
    reply->payload_len = sizeof(HelloReply);
    memcpy(reply->payload, &hr, sizeof(HelloReply));
}

static void handle_health(const Envelope *req, Envelope *reply, AgentState *state) {
    Health h;
    memcpy(&h, req->payload, sizeof(Health));

    uint32_t status = (h.session_id == state->session_id) ? STATUS_OK : STATUS_INVALID;
    if (status == STATUS_INVALID) {
        state->error_count++;
    }

    HealthReply hr = {
        .session_id = h.session_id,
        .status = status,
        .error_count = state->error_count,
    };

    reply->protocol = PROTOCOL_CAPSULE_CONTROL;
    reply->msg_type = MSG_HEALTH_REPLY;
    reply->handle = 0;
    reply->payload_len = sizeof(HealthReply);
    memcpy(reply->payload, &hr, sizeof(HealthReply));
}

static void handle_shutdown(const Envelope *req, Envelope *reply, AgentState *state) {
    Shutdown s;
    memcpy(&s, req->payload, sizeof(Shutdown));

    uint32_t status = (s.session_id == state->session_id) ? STATUS_OK : STATUS_INVALID;
    if (status == STATUS_INVALID) {
        state->error_count++;
    }

    ShutdownReply sr = {
        .session_id = s.session_id,
        .status = status,
        .reserved = 0,
    };

    reply->protocol = PROTOCOL_CAPSULE_CONTROL;
    reply->msg_type = MSG_SHUTDOWN_REPLY;
    reply->handle = 0;
    reply->payload_len = sizeof(ShutdownReply);
    memcpy(reply->payload, &sr, sizeof(ShutdownReply));
}

static void handle_echo(const Envelope *req, Envelope *reply, AgentState *state) {
    EchoRequest er;
    memcpy(&er, req->payload, sizeof(EchoRequest));

    uint32_t status = (er.payload_len > 0) ? STATUS_OK : STATUS_ERR;
    if (status == STATUS_ERR) {
        state->error_count++;
    }

    EchoReply erep = {
        .request_id = er.request_id,
        .payload_len = er.payload_len,
        .status = status,
    };

    reply->protocol = PROTOCOL_ECHO;
    reply->msg_type = MSG_ECHO_REPLY;
    reply->handle = 0;
    reply->payload_len = sizeof(EchoReply);
    memcpy(reply->payload, &erep, sizeof(EchoReply));
}

int main(void) {
    write_str("CAPSULE_AGENT: starting\n");

    /* Mount devtmpfs so /dev/vport0p1 appears */
    mkdir("/dev", 0755);
    mount("devtmpfs", "/dev", "devtmpfs", 0, NULL);

    /* Wait for virtio-serial port to appear */
    int fd = -1;
    for (int i = 0; i < 50 && fd < 0; i++) {
        fd = open("/dev/vport0p1", O_RDWR);
        if (fd < 0) usleep(100000); /* 100ms */
    }
    if (fd < 0) {
        write_str("CAPSULE_AGENT: failed to open /dev/vport0p1\n");
        return 1;
    }
    write_str("CAPSULE_AGENT: opened virtio port\n");

    AgentState state = {.session_id = 0, .error_count = 0};
    Envelope req, reply;

    while (1) {
        if (read_all(fd, &req, sizeof(Envelope)) != 0) {
            write_str("CAPSULE_AGENT: read failed\n");
            break;
        }

        memset(&reply, 0, sizeof(Envelope));
        int do_shutdown = 0;

        if (req.protocol == PROTOCOL_CAPSULE_CONTROL) {
            switch (req.msg_type) {
            case MSG_HELLO:
                handle_hello(&req, &reply, &state);
                break;
            case MSG_HEALTH:
                handle_health(&req, &reply, &state);
                break;
            case MSG_SHUTDOWN:
                handle_shutdown(&req, &reply, &state);
                do_shutdown = 1;
                break;
            default:
                write_str("CAPSULE_AGENT: unknown control message\n");
                /* Send error reply to avoid host-side timeout */
                reply.protocol = PROTOCOL_CAPSULE_CONTROL;
                reply.msg_type = 0xFF; /* error indicator */
                reply.handle = 0;
                reply.payload_len = 0;
                break;
            }
        } else if (req.protocol == PROTOCOL_ECHO) {
            if (req.msg_type == MSG_ECHO_REQUEST) {
                handle_echo(&req, &reply, &state);
            } else {
                write_str("CAPSULE_AGENT: unknown echo message\n");
                reply.protocol = PROTOCOL_ECHO;
                reply.msg_type = 0xFF;
                reply.handle = 0;
                reply.payload_len = 0;
            }
        } else {
            write_str("CAPSULE_AGENT: unknown protocol\n");
            reply.protocol = req.protocol;
            reply.msg_type = 0xFF;
            reply.handle = 0;
            reply.payload_len = 0;
        }

        if (write_all(fd, &reply, sizeof(Envelope)) != 0) {
            write_str("CAPSULE_AGENT: write failed\n");
            break;
        }

        if (do_shutdown) {
            break;
        }
    }

    close(fd);
    write_str("CAPSULE_AGENT: shutdown\n");
    sync();
    return 0;
}
