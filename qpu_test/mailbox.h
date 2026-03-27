// mailbox.h — VideoCore mailbox interface via /dev/vcio
// Based on Raspberry Pi Foundation gpu_fft mailbox protocol
#ifndef MAILBOX_H
#define MAILBOX_H

#include <stdint.h>

// Mailbox property tags
#define TAG_ALLOCATE_MEMORY   0x3000C
#define TAG_LOCK_MEMORY       0x3000D
#define TAG_UNLOCK_MEMORY     0x3000E
#define TAG_RELEASE_MEMORY    0x3000F
#define TAG_EXECUTE_QPU       0x30011
#define TAG_ENABLE_QPU        0x30012

// Memory allocation flags
#define MEM_FLAG_DIRECT       (1 << 2)
#define MEM_FLAG_COHERENT     (1 << 3)  // L2-coherent, alias for L1_NONALLOC
#define MEM_FLAG_L1_NONALLOC  (1 << 3)  // Same bit: do not allocate in L1 cache
#define MEM_FLAG_ZERO         (1 << 4)
#define MEM_FLAG_HINT_PERMALOCK (1 << 6)

// Bus address to physical address conversion (BCM2837)
#define BUS_TO_PHYS(addr) ((addr) & ~0xC0000000)

// Open/close mailbox
int mbox_open(void);
void mbox_close(int fd);

// QPU control
int qpu_enable(int fd, int enable);
int qpu_execute(int fd, int num_qpus, uint32_t control_bus_addr,
                int noflush,       // 1 = skip L1/L2 cache flush before exec
                int timeout_ms);

// GPU memory management
uint32_t mem_alloc(int fd, uint32_t size, uint32_t align, uint32_t flags);
uint32_t mem_lock(int fd, uint32_t handle);
int mem_unlock(int fd, uint32_t handle);
int mem_free(int fd, uint32_t handle);

// Map physical address to userspace
void *mapmem(uint32_t phys_addr, uint32_t size);
void unmapmem(void *addr, uint32_t size);

#endif // MAILBOX_H
