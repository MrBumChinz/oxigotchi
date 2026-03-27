// qpu_test.c — Standalone QPU execution test for Pi Zero 2W
//
// Usage: sudo ./qpu_test
// Requires: dtoverlay=vc4-fkms-v3d, gpu_mem>=64 in config.txt

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
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

int main(void) {
    printf("=== QPU Execution Test ===\n");

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
        fprintf(stderr, "FAIL: qpu_enable returned %d. Is dtoverlay=vc4-fkms-v3d set?\n", ret);
        mbox_close(mbox);
        return 1;
    }
    printf("[2/7] QPUs enabled\n");

    // Step 3: Allocate GPU memory
    handle = mem_alloc(mbox, ALLOC_SIZE, 4096,
                       MEM_FLAG_COHERENT | MEM_FLAG_ZERO);
    if (handle == 0) {
        fprintf(stderr, "FAIL: GPU memory allocation failed. Is gpu_mem >= 64?\n");
        goto cleanup_qpu;
    }

    bus_addr = mem_lock(mbox, handle);
    if (bus_addr == 0) {
        fprintf(stderr, "FAIL: mem_lock failed\n");
        goto cleanup_mem;
    }
    printf("[3/7] GPU memory allocated: handle=%u, bus=0x%08X\n", handle, bus_addr);

    // Step 4: Map to userspace
    arm_base = mapmem(BUS_TO_PHYS(bus_addr), ALLOC_SIZE);
    if (arm_base == NULL) {
        fprintf(stderr, "FAIL: mmap /dev/mem failed. Are you root?\n");
        goto cleanup_lock;
    }
    printf("[4/7] Mapped to ARM address %p\n", arm_base);

    // Step 5: Copy QPU code, set up uniforms and launch message
    uint8_t *base = (uint8_t *)arm_base;

    // Copy QPU binary
    memcpy(base + CODE_OFFSET, qpu_hello_code, QPU_HELLO_CODE_SIZE);

    // Clear output location
    *(volatile uint32_t *)(base + OUTPUT_OFFSET) = 0xDEAD0000;

    // Uniforms: [0] = bus address of output buffer
    uint32_t *uniforms = (uint32_t *)(base + UNIFORMS_OFFSET);
    uniforms[0] = bus_addr + OUTPUT_OFFSET;

    // QPU launch message: array of (uniforms_bus_addr, code_bus_addr) pairs
    uint32_t *message = (uint32_t *)(base + MESSAGE_OFFSET);
    message[0] = bus_addr + UNIFORMS_OFFSET;  // uniforms pointer
    message[1] = bus_addr + CODE_OFFSET;      // code pointer

    printf("[5/7] Code loaded (%u instructions), uniforms set\n",
           QPU_HELLO_NUM_INST);

    // Step 6: Execute!
    printf("       Executing QPU...\n");
    int pass = 0;
    ret = qpu_execute(mbox,
                      1,                          // num_qpus
                      bus_addr + MESSAGE_OFFSET,   // control message bus addr
                      0,                          // noflush = 0
                      5000);                      // timeout = 5 seconds

    if (ret != 0) {
        fprintf(stderr, "FAIL: qpu_execute returned %d (timeout or error)\n", ret);
        goto cleanup;
    }
    printf("[6/7] QPU execution completed\n");

    // Step 7: Read result
    volatile uint32_t *result = (volatile uint32_t *)(base + OUTPUT_OFFSET);
    uint32_t value = *result;

    printf("[7/7] Result at offset 0x%X: 0x%08X\n", OUTPUT_OFFSET, value);

    if (value == QPU_HELLO_EXPECTED) {
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
