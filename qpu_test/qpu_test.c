// qpu_test.c — Standalone QPU execution test for Pi Zero 2W
//
// Usage: sudo ./qpu_test
// Requires: gpu_mem>=64 in config.txt, no vc4 overlay (or unloaded)
//
// Two execution modes:
//   1. Mailbox execute_qpu (firmware-mediated)
//   2. Direct V3D register poke (bypasses firmware)

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <time.h>
#include "mailbox.h"
#include "qpu_hello.h"

// Shared memory layout (all bus-addressable):
//   [0x000 .. code_end)      QPU program binary
//   [0x100]                  Uniforms array (1 entry: output bus addr)
//   [0x200]                  QPU launch message (uniforms_addr, code_addr)
//   [0x300]                  Output buffer (1 x uint32_t)
#define CODE_OFFSET     0x000
#define UNIFORMS_OFFSET 0x100
#define MESSAGE_OFFSET  0x200
#define OUTPUT_OFFSET   0x300
#define ALLOC_SIZE      0x1000  // 4KB total, page-aligned

// BCM2837 V3D registers (physical addresses)
#define V3D_BASE    0x3FC00000
#define V3D_IDENT0  0x000   // V3D identification 0
#define V3D_IDENT1  0x004   // V3D identification 1
#define V3D_SRQPC   0x430   // QPU scheduler: program counter (write to launch)
#define V3D_SRQUA   0x434   // QPU scheduler: uniforms address
#define V3D_SRQUL   0x438   // QPU scheduler: uniforms length
#define V3D_SRQCS   0x43C   // QPU scheduler: control/status
#define V3D_L2CACTL 0x020   // L2 cache control

static volatile uint32_t *v3d_regs = NULL;

static int v3d_init(void) {
    v3d_regs = (volatile uint32_t *)mapmem(V3D_BASE, 0x1000);
    if (!v3d_regs) {
        fprintf(stderr, "FAIL: Cannot map V3D registers\n");
        return -1;
    }
    uint32_t ident0 = v3d_regs[V3D_IDENT0 / 4];
    printf("       V3D_IDENT0 = 0x%08X", ident0);
    if ((ident0 & 0x00FFFFFF) == 0x00443356) {
        printf(" (\"V3D\" — VideoCore IV confirmed)\n");
    } else {
        printf(" (unexpected — not V3D?)\n");
    }
    uint32_t ident1 = v3d_regs[V3D_IDENT1 / 4];
    int nslc = (ident1 >> 4) & 0xF;   // number of slices
    int qpus = (ident1 >> 8) & 0xF;   // QPUs per slice
    printf("       V3D_IDENT1 = 0x%08X (%d slices, %d QPUs/slice = %d QPUs total)\n",
           ident1, nslc, qpus, nslc * qpus);
    return 0;
}

static void v3d_cleanup(void) {
    if (v3d_regs) {
        unmapmem((void *)v3d_regs, 0x1000);
        v3d_regs = NULL;
    }
}

// Direct register poke QPU execution
// Returns 0 on success, -1 on timeout
static int v3d_execute_qpu(uint32_t code_bus, uint32_t uniforms_bus,
                           int timeout_ms) {
    // Read current completion count
    uint32_t srqcs = v3d_regs[V3D_SRQCS / 4];
    uint32_t complete_before = (srqcs >> 16) & 0xFF;
    printf("       V3D_SRQCS before = 0x%08X (complete=%u, error=%u)\n",
           srqcs, complete_before, (srqcs >> 8) & 0xFF);

    // Flush L2 cache
    v3d_regs[V3D_L2CACTL / 4] = (1 << 2);  // L2 cache clear

    // Clear request count: write 1<<16 to clear completed count, 1<<8 for error
    v3d_regs[V3D_SRQCS / 4] = (1 << 7) | (1 << 8) | (1 << 16);

    // Write uniforms address first, then program counter (triggers launch)
    v3d_regs[V3D_SRQUA / 4] = uniforms_bus;
    v3d_regs[V3D_SRQUL / 4] = 1;  // uniforms length = 1
    v3d_regs[V3D_SRQPC / 4] = code_bus;

    printf("       Launched: PC=0x%08X, UA=0x%08X\n", code_bus, uniforms_bus);

    // Poll for completion
    struct timespec start, now;
    clock_gettime(CLOCK_MONOTONIC, &start);

    while (1) {
        srqcs = v3d_regs[V3D_SRQCS / 4];
        uint32_t complete = (srqcs >> 16) & 0xFF;
        if (complete > 0) {
            printf("       V3D_SRQCS after = 0x%08X (complete=%u)\n",
                   srqcs, complete);
            return 0;
        }

        clock_gettime(CLOCK_MONOTONIC, &now);
        long elapsed_ms = (now.tv_sec - start.tv_sec) * 1000 +
                          (now.tv_nsec - start.tv_nsec) / 1000000;
        if (elapsed_ms > timeout_ms) {
            printf("       V3D_SRQCS timeout = 0x%08X (complete=%u, error=%u)\n",
                   srqcs, complete, (srqcs >> 8) & 0xFF);
            return -1;
        }
        usleep(100);
    }
}

int main(int argc, char *argv[]) {
    int use_regpoke = 1;  // default to register poke
    if (argc > 1 && strcmp(argv[1], "--mailbox") == 0) {
        use_regpoke = 0;
    }

    printf("=== QPU Execution Test (%s mode) ===\n",
           use_regpoke ? "register-poke" : "mailbox");

    // Step 1: Open mailbox
    int mbox = mbox_open();
    if (mbox < 0) {
        fprintf(stderr, "FAIL: Cannot open /dev/vcio. Are you root?\n");
        return 1;
    }
    printf("[1/7] Mailbox opened\n");

    // Track acquired resources for cleanup
    uint32_t handle = 0;
    uint32_t bus_addr = 0;
    void *arm_base = NULL;

    // Step 2: Enable QPUs
    int ret = qpu_enable(mbox, 1);
    if (ret != 0) {
        fprintf(stderr, "FAIL: qpu_enable returned %d\n", ret);
        mbox_close(mbox);
        return 1;
    }
    printf("[2/7] QPUs enabled\n");

    // Step 3: Allocate GPU memory (use L1-nonalloc flags like gpu_fft)
    handle = mem_alloc(mbox, ALLOC_SIZE, 4096,
                       MEM_FLAG_DIRECT | MEM_FLAG_COHERENT | MEM_FLAG_ZERO);
    if (handle == 0) {
        fprintf(stderr, "FAIL: GPU memory allocation failed. Is gpu_mem >= 64?\n");
        goto cleanup_qpu;
    }

    bus_addr = mem_lock(mbox, handle);
    if (bus_addr == 0) {
        fprintf(stderr, "FAIL: mem_lock failed\n");
        goto cleanup_mem;
    }
    printf("[3/7] GPU memory allocated: handle=%u, bus=0x%08X, phys=0x%08X\n",
           handle, bus_addr, BUS_TO_PHYS(bus_addr));

    // Step 4: Map to userspace
    arm_base = mapmem(BUS_TO_PHYS(bus_addr), ALLOC_SIZE);
    if (arm_base == NULL) {
        fprintf(stderr, "FAIL: mmap /dev/mem failed. Are you root?\n");
        goto cleanup_lock;
    }
    printf("[4/7] Mapped to ARM address %p\n", arm_base);

    // Map V3D registers for diagnostics (and execution in regpoke mode)
    if (v3d_init() < 0) {
        goto cleanup;
    }

    // Step 5: Copy QPU code, set up uniforms and launch message
    uint8_t *base = (uint8_t *)arm_base;

    // Copy QPU binary
    memcpy(base + CODE_OFFSET, qpu_hello_code, QPU_HELLO_CODE_SIZE);

    // Clear output location with sentinel
    *(volatile uint32_t *)(base + OUTPUT_OFFSET) = 0xDEAD0000;

    // Uniforms: [0] = bus address of output buffer
    uint32_t *uniforms = (uint32_t *)(base + UNIFORMS_OFFSET);
    uniforms[0] = bus_addr + OUTPUT_OFFSET;

    // QPU launch message (for mailbox mode): (uniforms_bus_addr, code_bus_addr) pairs
    uint32_t *message = (uint32_t *)(base + MESSAGE_OFFSET);
    message[0] = bus_addr + UNIFORMS_OFFSET;
    message[1] = bus_addr + CODE_OFFSET;

    printf("[5/7] Code loaded (%lu instructions), uniforms set\n",
           (unsigned long)QPU_HELLO_NUM_INST);

    // Verify code was copied correctly
    uint32_t *code_readback = (uint32_t *)(base + CODE_OFFSET);
    printf("       Code readback: [0]=0x%08X [1]=0x%08X\n",
           code_readback[0], code_readback[1]);

    // Step 6: Execute!
    int pass = 0;

    if (use_regpoke) {
        printf("       Using direct V3D register poke...\n");
        ret = v3d_execute_qpu(bus_addr + CODE_OFFSET,
                              bus_addr + UNIFORMS_OFFSET,
                              5000);
    } else {
        printf("       Using mailbox execute_qpu...\n");
        ret = qpu_execute(mbox,
                          1,
                          bus_addr + MESSAGE_OFFSET,
                          0,
                          5000);
    }

    if (ret != 0) {
        fprintf(stderr, "FAIL: QPU execution returned %d (timeout or error)\n", ret);
        goto cleanup;
    }
    printf("[6/7] QPU execution completed\n");

    // Step 7: Read result
    volatile uint32_t *result = (volatile uint32_t *)(base + OUTPUT_OFFSET);
    uint32_t value = *result;

    printf("[7/7] Result at offset 0x%X: 0x%08X\n", OUTPUT_OFFSET, value);

    if (QPU_HELLO_EXPECTED == 0) {
        printf("\n*** PASS: QPU execution completed without timeout ***\n");
        pass = 1;
    } else if (value == QPU_HELLO_EXPECTED) {
        printf("\n*** PASS: QPU wrote 0x%08X as expected ***\n", value);
        pass = 1;
    } else if (value == 0xDEAD0000) {
        printf("\n*** FAIL: Output unchanged (0xDEAD0000) — QPU did not write ***\n");
        printf("    Possible causes:\n");
        printf("    - QPU instruction encoding error\n");
        printf("    - VPM setup value incorrect\n");
        printf("    - DMA address miscalculation\n");
    } else {
        printf("\n*** FAIL: Expected 0x%08X, got 0x%08X ***\n",
               QPU_HELLO_EXPECTED, value);
    }

cleanup:
    v3d_cleanup();
    unmapmem(arm_base, ALLOC_SIZE);
cleanup_lock:
    mem_unlock(mbox, handle);
cleanup_mem:
    mem_free(mbox, handle);
cleanup_qpu:
    qpu_enable(mbox, 0);
    mbox_close(mbox);

    return pass ? 0 : 1;
}
