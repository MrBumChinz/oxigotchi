// qpu_hello.h — Minimal QPU program: write 0xCAFEBABE to shared memory via VPM DMA
//
// Program flow:
//   1. Set up VPM for generic block write (32-bit horizontal)
//   2. Load immediate 0xCAFEBABE into VPM
//   3. Set up VPM DMA store (1 row of 16 elements)
//   4. Read destination bus address from uniform, trigger DMA
//   5. Wait for DMA completion
//   6. End thread
//
// Uniform 0: bus address of output buffer (ARM sets this before launch)
//
// QPU ISA encoding reference:
//   Load immediate: sig=0xE (14), lower 32 bits = immediate value
//   waddr field selects destination register/peripheral
//   Thread end: sig=0x3, followed by 2 mandatory delay-slot nops

#ifndef QPU_HELLO_H
#define QPU_HELLO_H

#include <stdint.h>

// VPM write setup value: horizontal, 32-bit, stride=1, addr=0
#define VPW_SETUP_H32  0x00001A00

// VPM DMA write setup: 1 unit, DEPTH=16, HORIZ=1, 32-bit
#define VDW_SETUP_H32_1ROW  0x80104000

static uint32_t qpu_hello_code[] = {
    // Instruction 0: ldi vpmvcd_wr_setup, VPW_SETUP_H32
    // sig=14(ldi), cond_add=always, waddr_add=49(VPMVCD_SETUP), waddr_mul=39(NOP)
    VPW_SETUP_H32,  0xE0020C67,

    // Instruction 1: ldi vpm, 0xCAFEBABE
    // sig=14(ldi), cond_add=always, waddr_add=48(VPM), waddr_mul=39(NOP)
    0xCAFEBABE,     0xE0020C27,

    // Instruction 2: ldi vpmvcd_wr_setup, VDW_SETUP_H32_1ROW
    // sig=14(ldi), same waddr as instruction 0
    VDW_SETUP_H32_1ROW, 0xE0020C67,

    // Instruction 3: or vpm_wr_addr, unif, unif  (read uniform -> DMA dest addr)
    // sig=1(normal), op_add=21(OR), raddr_a=32(uniform), add_a/b=6(regA)
    // waddr_add=50(VPM_ADDR), waddr_mul=39(NOP)
    0x15800D80,     0x10020CA7,

    // Instruction 4: or nop, rb50, rb50  (wait for DMA store completion)
    // sig=1(normal), op_add=21(OR), raddr_b=50(VPM_ST_WAIT), add_a/b=7(regB)
    // waddr_add=39(NOP), waddr_mul=39(NOP)
    0x15032FC0,     0x100209E7,

    // Instruction 5: thrend (signal thread end)
    0x009E7000,     0x300009E7,

    // Instruction 6: nop (mandatory delay slot 1)
    0x009E7000,     0x100009E7,

    // Instruction 7: nop (mandatory delay slot 2)
    0x009E7000,     0x100009E7,
};

#define QPU_HELLO_CODE_SIZE sizeof(qpu_hello_code)
#define QPU_HELLO_NUM_INST  (QPU_HELLO_CODE_SIZE / 8)

// Expected output value
#define QPU_HELLO_EXPECTED  0xCAFEBABE

#endif // QPU_HELLO_H
