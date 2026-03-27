// qpu_hello.h — Minimal QPU program: write 0xCAFEBABE to shared memory via VPM DMA
//
// Program flow:
//   1. Set up VPM for generic block write (32-bit horizontal)
//   2. Load immediate 0xCAFEBABE into VPM
//   3. Set up VPM DMA store (1 row of 16 elements)
//   4. Read destination bus address from uniform, trigger DMA store
//   5. Wait for DMA store completion
//   6. End thread
//
// Uniform 0: bus address of output buffer (ARM sets this before launch)
//
// CRITICAL: VPM setup and DMA registers are differentiated by regfile A vs B:
//   waddr 49, regfile A = vpmvcd_rd_setup (VPM READ setup)
//   waddr 49, regfile B = vpmvcd_wr_setup (VPM WRITE setup)  <-- we need this
//   waddr 50, regfile A = vpm_ld_addr (DMA LOAD trigger)
//   waddr 50, regfile B = vpm_st_addr (DMA STORE trigger)    <-- we need this
// The ws bit (bit 12 of upper word) selects: ws=0 -> regfile A, ws=1 -> regfile B
//
// QPU ISA encoding reference:
//   Load immediate: sig=0xE (14), lower 32 bits = immediate value
//   ws bit: bit 44 of instruction = bit 12 of upper word
//   Thread end: sig=0x3, followed by 2 mandatory delay-slot nops

#ifndef QPU_HELLO_H
#define QPU_HELLO_H

#include <stdint.h>

// VPM write setup: stride=1, horizontal, 32-bit, addr=0
// py-videocore: stride<<12 | horizontal<<11 | laned<<10 | size<<8 | addr
// = 1<<12 | 1<<11 | 0<<10 | 2<<8 | 0 = 0x1A00
#define VPW_SETUP_H32  0x00001A00

// VPM DMA store setup: nrows=1, ncols=16, horizontal, 32-bit
// py-videocore: 0x80000000 | (nrows&0x7f)<<23 | (ncols&0x7f)<<16 | horiz<<14 | Y<<7 | X<<3 | modew
// = 0x80000000 | 1<<23 | 16<<16 | 1<<14 = 0x80904000
#define VDW_SETUP_H32  0x80904000

static uint32_t qpu_hello_code[] = {
    // Instruction 0: ldi vpmvcd_wr_setup, VPW_SETUP_H32
    // sig=0xE(ldi), ws=1(regfile B), waddr_add=49(vpmvcd_wr_setup on B), waddr_mul=39(NOP)
    // Upper: 0xE0020C67 | (1<<12) = 0xE0021C67
    VPW_SETUP_H32,  0xE0021C67,

    // Instruction 1: ldi vpm, 0xCAFEBABE
    // sig=0xE(ldi), ws=0, waddr_add=48(VPM), waddr_mul=39(NOP)
    // VPM write (48) is same on both regfiles, ws doesn't matter
    0xCAFEBABE,     0xE0020C27,

    // Instruction 2: ldi vpmvcd_wr_setup, VDW_SETUP_H32
    // sig=0xE(ldi), ws=1(regfile B), waddr_add=49(vpmvcd_wr_setup on B), waddr_mul=39(NOP)
    // Second write to vpmvcd_wr_setup = DMA store setup (ping-pong)
    VDW_SETUP_H32,  0xE0021C67,

    // Instruction 3: or vpm_st_addr, unif, unif  (trigger DMA store to uniform address)
    // sig=1, ws=1(regfile B), op_add=OR, raddr_a=32(uniform), waddr_add=50(vpm_st_addr on B)
    // Upper: 0x10020CA7 | (1<<12) = 0x10021CA7
    0x15800D80,     0x10021CA7,

    // Instruction 4: or nop, vpm_st_wait, vpm_st_wait  (stall until DMA store completes)
    // raddr_b=50 = vpm_st_wait (regfile B read), waddr_add=39(NOP)
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
