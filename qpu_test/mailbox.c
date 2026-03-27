// mailbox.c — Mailbox property interface via /dev/vcio ioctl
#include "mailbox.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <sys/mman.h>

#define DEVICE_FILE "/dev/vcio"
#define IOCTL_MBOX_PROPERTY _IOWR(100, 0, char *)

// Send a property request to the mailbox
// Returns 0 on success, -1 on ioctl failure, -2 on firmware error
static int mbox_property(int fd, void *buf) {
    int ret = ioctl(fd, IOCTL_MBOX_PROPERTY, buf);
    if (ret < 0) {
        perror("ioctl MBOX_PROPERTY");
        return -1;
    }
    uint32_t *p = (uint32_t *)buf;
    if (p[1] != 0x80000000) {
        fprintf(stderr, "mailbox firmware error: response 0x%08X\n", p[1]);
        return -2;
    }
    return 0;
}

int mbox_open(void) {
    int fd = open(DEVICE_FILE, O_RDWR);
    if (fd < 0) {
        perror("open /dev/vcio");
    }
    return fd;
}

void mbox_close(int fd) {
    close(fd);
}

int qpu_enable(int fd, int enable) {
    uint32_t buf[32] __attribute__((aligned(16)));
    memset(buf, 0, sizeof(buf));
    int i = 0;
    buf[i++] = 0;             // total size (filled later)
    buf[i++] = 0;             // request code
    buf[i++] = TAG_ENABLE_QPU;
    buf[i++] = 4;             // value buffer size
    buf[i++] = 4;             // request size
    buf[i++] = enable;
    buf[i++] = 0;             // end tag
    buf[0] = i * sizeof(uint32_t);

    if (mbox_property(fd, buf) < 0) return -1;
    return buf[5];
}

int qpu_execute(int fd, int num_qpus, uint32_t control_bus_addr,
                int noflush, int timeout_ms) {
    uint32_t buf[32] __attribute__((aligned(16)));
    memset(buf, 0, sizeof(buf));
    int i = 0;
    buf[i++] = 0;
    buf[i++] = 0;
    buf[i++] = TAG_EXECUTE_QPU;
    buf[i++] = 16;            // value buffer size
    buf[i++] = 16;            // request size
    buf[i++] = num_qpus;
    buf[i++] = control_bus_addr;
    buf[i++] = noflush;
    buf[i++] = timeout_ms;
    buf[i++] = 0;             // end tag
    buf[0] = i * sizeof(uint32_t);

    if (mbox_property(fd, buf) < 0) return -1;
    return buf[5];
}

uint32_t mem_alloc(int fd, uint32_t size, uint32_t align, uint32_t flags) {
    uint32_t buf[32] __attribute__((aligned(16)));
    memset(buf, 0, sizeof(buf));
    int i = 0;
    buf[i++] = 0;
    buf[i++] = 0;
    buf[i++] = TAG_ALLOCATE_MEMORY;
    buf[i++] = 12;            // value buffer size
    buf[i++] = 12;            // request size
    buf[i++] = size;
    buf[i++] = align;
    buf[i++] = flags;
    buf[i++] = 0;             // end tag
    buf[0] = i * sizeof(uint32_t);

    if (mbox_property(fd, buf) < 0) return 0;
    return buf[5];            // handle
}

uint32_t mem_lock(int fd, uint32_t handle) {
    uint32_t buf[32] __attribute__((aligned(16)));
    memset(buf, 0, sizeof(buf));
    int i = 0;
    buf[i++] = 0;
    buf[i++] = 0;
    buf[i++] = TAG_LOCK_MEMORY;
    buf[i++] = 4;
    buf[i++] = 4;
    buf[i++] = handle;
    buf[i++] = 0;
    buf[0] = i * sizeof(uint32_t);

    if (mbox_property(fd, buf) < 0) return 0;
    return buf[5];            // bus address
}

int mem_unlock(int fd, uint32_t handle) {
    uint32_t buf[32] __attribute__((aligned(16)));
    memset(buf, 0, sizeof(buf));
    int i = 0;
    buf[i++] = 0;
    buf[i++] = 0;
    buf[i++] = TAG_UNLOCK_MEMORY;
    buf[i++] = 4;
    buf[i++] = 4;
    buf[i++] = handle;
    buf[i++] = 0;
    buf[0] = i * sizeof(uint32_t);

    if (mbox_property(fd, buf) < 0) return -1;
    return buf[5];
}

int mem_free(int fd, uint32_t handle) {
    uint32_t buf[32] __attribute__((aligned(16)));
    memset(buf, 0, sizeof(buf));
    int i = 0;
    buf[i++] = 0;
    buf[i++] = 0;
    buf[i++] = TAG_RELEASE_MEMORY;
    buf[i++] = 4;
    buf[i++] = 4;
    buf[i++] = handle;
    buf[i++] = 0;
    buf[0] = i * sizeof(uint32_t);

    if (mbox_property(fd, buf) < 0) return -1;
    return buf[5];
}

void *mapmem(uint32_t phys_addr, uint32_t size) {
    int fd = open("/dev/mem", O_RDWR | O_SYNC);
    if (fd < 0) {
        perror("open /dev/mem");
        return NULL;
    }
    uint32_t offset = phys_addr % 4096;
    uint32_t base = phys_addr - offset;
    void *mem = mmap(NULL, size + offset, PROT_READ | PROT_WRITE,
                     MAP_SHARED, fd, base);
    close(fd);
    if (mem == MAP_FAILED) {
        perror("mmap");
        return NULL;
    }
    return (char *)mem + offset;
}

void unmapmem(void *addr, uint32_t size) {
    uint32_t offset = (uintptr_t)addr % 4096;
    munmap((char *)addr - offset, size + offset);
}
