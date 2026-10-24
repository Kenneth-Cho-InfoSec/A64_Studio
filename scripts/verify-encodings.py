#!/usr/bin/env python3
"""Cross-check every encoder family against LLVM's AArch64 assembler.

Requires the Android NDK. Usage:
  python3 scripts/verify-encodings.py
Exit status is non-zero if any instruction differs or fails to assemble.
"""
import os, subprocess, sys, tempfile, re, shutil

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NDK = os.environ.get("ANDROID_NDK_HOME") or os.path.expanduser("~/Android/Sdk/ndk/28.2.13676358")
BIN = os.path.join(NDK, "toolchains/llvm/prebuilt/linux-x86_64/bin")
CLANG = os.path.join(BIN, "aarch64-linux-android21-clang")
OBJDUMP = os.path.join(BIN, "llvm-objdump")
OURS = os.path.join(ROOT, "core/target/debug/a64asm")

CASES = [
    # moves
    "mov x0, #42", "mov w1, #0x1234", "mov x0, #-1", "mov x0, #0xffffffff",
    "mov x0, #0x100000000", "mov x0, x1", "mov w0, w1", "mov x0, sp", "mov sp, x0",
    "movz x0, #0x1234", "movz x0, #0x1234, lsl #16", "movz w1, #0xabcd, lsl #16",
    "movn x0, #0", "movn w2, #1, lsl #16", "movk x0, #0xabcd, lsl #32", "movk w3, #0xffff",
    # add/sub
    "add x0, x1, x2", "add x0, x1, #1", "add x0, x1, #0x1000", "add sp, sp, #16",
    "add x0, sp, x1", "add x0, x1, w2, uxtw", "add x0, x1, x2, lsl #3", "add x0, x1, x2, lsr #4",
    "adds x0, x1, x2", "adds x0, sp, #1", "sub x0, x1, x2", "sub x0, x1, #1",
    "sub sp, sp, #32", "subs w0, w1, w2, asr #2", "cmp x0, x1", "cmp x0, #10",
    "cmp w0, w1, lsl #2", "cmn x0, #1", "neg x0, x1", "neg x0, x1, lsl #4", "negs w0, w1",
    # logicals
    "and x0, x1, x2", "and x0, x1, #0xff", "and x0, x1, #0xffff0000ffff0000",
    "orr x0, x1, x2", "orr x0, x1, #1", "eor w0, w1, w2, lsl #3", "eor x0, x1, #0xf0f0f0f0f0f0f0f0",
    "ands x0, x1, #1", "bic x0, x1, x2", "orn x0, x1, x2", "eon w0, w1, w2", "bics x0, x1, x2",
    "mvn x0, x1", "mvn w0, w1, lsl #2", "tst x0, #1", "tst w0, w1, lsl #4",
    # shifts / bitfields
    "lsl x0, x1, #4", "lsl w0, w1, #0", "lsr x0, x1, #4", "asr w0, w1, #2", "asr x0, x1, #63",
    "ror x0, x1, #8", "lsl x0, x1, x2", "lsr w0, w1, w2", "asr x0, x1, x2", "ror x0, x1, x2",
    "lslv x0, x1, x2", "lsrv w0, w1, w2", "asrv x0, x1, x2", "rorv w0, w1, w2",
    "ubfm x0, x1, #0, #7", "sbfm w0, w1, #2, #9", "bfm x0, x1, #8, #15",
    "ubfx x0, x1, #8, #8", "sbfx w0, w1, #4, #12", "bfi x0, x1, #8, #8", "bfxil x0, x1, #8, #8",
    "sbfiz x0, x1, #8, #8", "ubfiz w0, w1, #4, #8", "sxtb x0, w1", "sxth w0, w1",
    "sxtw x0, w1", "uxtb w0, w1", "uxth x0, w1", "uxtw x0, w1",
    "rev x0, x1", "rev w0, w1", "rev16 w0, w1", "rev32 x0, x1", "rbit x0, x1", "clz x0, x1", "cls w0, w1",
    # conditional
    "csel x0, x1, x2, eq", "csinc w0, w1, w2, ne", "csinv x0, x1, x2, ge", "csneg x0, x1, x2, lt",
    "cset x0, eq", "cset w1, ne", "csetm x0, ne", "ccmp x0, x1, #0, eq", "ccmp w0, #3, #15, ne",
    "ccmn x0, x1, #4, gt", "ccmn w0, #1, #0, le",
    # multiply
    "mul x0, x1, x2", "mul w0, w1, w2", "madd x0, x1, x2, x3", "msub w0, w1, w2, w3",
    "mneg x0, x1, x2", "smull x0, w1, w2", "umull x0, w1, w2",
    "smaddl x0, w1, w2, x3", "umaddl x0, w1, w2, x3", "smulh x0, x1, x2", "umulh x0, x1, x2",
    "sdiv x0, x1, x2", "udiv w0, w1, w2",
    # memory
    "ldr x0, [x1]", "ldr x0, [x1, #8]", "str x0, [sp, #16]", "ldr w0, [x1, #4]",
    "ldrb w0, [x1]", "strb w0, [x1, #3]", "ldrh w0, [x1]", "strh w0, [x1, #2]",
    "ldrsb x0, [x1]", "ldrsb w0, [x1]", "ldrsh x0, [x1]", "ldrsh w0, [x1]", "ldrsw x0, [x1]",
    "ldur x0, [x1, #-8]", "stur x0, [x1, #255]", "ldur w0, [x1, #-4]",
    "ldr x0, [x1, #-8]", "ldr x0, [x1, #8]!", "str x0, [x1], #8", "ldrb w0, [x1], #1",
    "ldr x0, [x1, x2]", "ldr w0, [x1, w2, uxtw]", "ldr x0, [x1, w2, sxtw]",
    "ldr x0, [x1, x2, lsl #3]", "str x0, [x1, x2, lsl #3]", "ldr q0, [x1]", "str q0, [x1, #16]",
    "ldp x0, x1, [x2]", "stp x0, x1, [x2, #16]", "ldp w0, w1, [x2]", "stp w0, w1, [x2, #-8]",
    "ldp x29, x30, [sp], #16", "stp x29, x30, [sp, #-16]!", "ldp x0, x1, [x2], #16",
    "ldar x0, [x1]", "stlr w0, [x1]", "ldxr x0, [x1]", "stxr w2, x0, [x1]",
    "ldaxr w0, [x1]", "stlxr w2, w0, [x1]", "prfm pldl1keep, [x0]", "prfm pstl2strm, [x0, #64]",
    # system
    "nop", "yield", "wfe", "wfi", "sev", "sevl", "hint #7", "bti", "bti c", "bti j", "bti jc",
    "svc #0", "brk #0", "hlt #0", "dmb ish", "dsb sy", "isb", "isb sy", "clrex",
    "mrs x0, nzcv", "msr nzcv, x0", "mrs x0, fpcr", "msr fpcr, x0", "mrs x0, cntvct_el0",
    "mrs x0, cntfrq_el0", "mrs x0, tpidr_el0", "msr tpidr_el0, x0", "mrs x0, midr_el1",
    "paciasp", "autiasp", "pacibsp", "autibsp", "eret", "drps", "udf #0",
    # SIMD integer
    "add v0.16b, v1.16b, v2.16b", "add v0.4s, v1.4s, v2.4s", "sub v0.8h, v1.8h, v2.8h",
    "mul v0.4s, v1.4s, v2.4s", "mul v0.16b, v1.16b, v2.16b", "and v0.16b, v1.16b, v2.16b",
    "orr v0.8b, v1.8b, v2.8b", "eor v0.16b, v1.16b, v2.16b", "bic v0.16b, v1.16b, v2.16b",
    "not v0.16b, v1.16b", "movi v0.16b, #0", "movi v0.16b, #0x42", "movi v0.4s, #1",
    "movi v0.4s, #1, lsl #8", "movi v0.8h, #0xff", "movi v0.2d, #0",
    "dup v0.4s, w1", "dup v0.16b, w1", "dup v0.2d, x1", "dup v0.4s, v1.s[2]",
    "ins v0.s[1], w1", "umov w0, v1.s[1]",
    "ld1 {v0.16b}, [x1]", "st1 {v0.16b}, [x1]", "ld1 {v0.4s}, [x1]", "ld1 {v0.2d}, [x1]",
    "ld1r {v0.4s}, [x1]", "addv s0, v1.4s", "addp v0.4s, v1.4s, v2.4s",
    # SIMD FP
    "fadd v0.4s, v1.4s, v2.4s", "fadd v0.2d, v1.2d, v2.2d", "fsub v0.4s, v1.4s, v2.4s",
    "fmul v0.4s, v1.4s, v2.4s", "fdiv v0.2d, v1.2d, v2.2d", "fabs v0.4s, v1.4s",
    "fneg v0.2d, v1.2d", "fsqrt v0.4s, v1.4s",
    "fadd s0, s1, s2", "fadd d0, d1, d2", "fsub s0, s1, s2", "fmul d0, d1, d2", "fdiv s0, s1, s2",
    "fabs s0, s1", "fneg d0, d1", "fsqrt s0, s1",
    "fcmp s0, s1", "fcmp d0, d1", "fcmpe s0, s1", "fcmp s0, #0.0",
    "fmov s0, s1", "fmov d0, d1", "fmov w0, s1", "fmov x0, d1", "fmov s0, w1", "fmov d0, x1",
    "scvtf s0, w1", "scvtf d0, x1", "ucvtf s0, x1", "fcvtzs w0, s1", "fcvtzs x0, d1",
    "fcvtzu w0, d1", "fcvt d0, s1", "fcvt s0, d1",
    # crypto
    "aese v0.16b, v1.16b", "aesd v0.16b, v1.16b", "aesmc v0.16b, v1.16b", "aesimc v0.16b, v1.16b",
    "sha256h q0, q1, v2.4s", "sha256h2 q0, q1, v2.4s", "sha256su0 v0.4s, v1.4s",
    "sha256su1 v0.4s, v1.4s, v2.4s", "pmull v0.8h, v1.8b, v2.8b",
    "crc32b w0, w1, w2", "crc32h w0, w1, w2", "crc32w w0, w1, w2", "crc32x w0, w1, x2",
    "crc32cb w0, w1, w2", "crc32ch w0, w1, w2", "crc32cw w0, w1, w2", "crc32cx w0, w1, x2",
]

def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)

def main():
    if not os.path.exists(CLANG):
        print(f"SKIP: NDK clang not found at {CLANG}")
        return 0
    # build our tool
    b = run([os.path.expanduser("~/.cargo/bin/cargo"), "build", "--quiet", "--bin", "a64asm"], cwd=os.path.join(ROOT, "core"))
    if b.returncode != 0:
        print(b.stderr); return 1
    ours = run([OURS], input="\n".join(CASES) + "\n")
    if ours.returncode != 0:
        print(ours.stderr); return 1
    our_lines = [l for l in ours.stdout.splitlines() if l.strip()]
    if len(our_lines) != len(CASES):
        print(f"our tool returned {len(our_lines)} lines for {len(CASES)} cases"); return 1

    with tempfile.TemporaryDirectory() as td:
        s = os.path.join(td, "all.s")
        with open(s, "w") as f:
            f.write(".text\n")
            for c in CASES:
                f.write(c + "\n")
        o = os.path.join(td, "all.o")
        r = run([CLANG, "-march=armv8-a+crc+crypto", "-c", s, "-o", o])
        if r.returncode != 0:
            print("LLVM rejected the test corpus:\n" + r.stderr)
            return 1
        d = run([OBJDUMP, "-d", o])
        if d.returncode != 0:
            print(d.stderr); return 1
        llvm = []
        for line in d.stdout.splitlines():
            m = re.match(r"\s*([0-9a-f]+):\s+([0-9a-f]{8})\s", line)
            if m:
                llvm.append(m.group(2))
    if len(llvm) != len(CASES):
        print(f"LLVM emitted {len(llvm)} words for {len(CASES)} instructions (some cases expanded)")
    bad = 0
    for i, (case, ours_line) in enumerate(zip(CASES, our_lines)):
        if ours_line.startswith("ERR"):
            print(f"FAIL  {case}\n      ours: {ours_line}")
            bad += 1
            continue
        got = ours_line.split("\t", 1)[1].replace(" ", "")
        if i >= len(llvm):
            print(f"FAIL  {case}\n      LLVM produced no word")
            bad += 1
            continue
        want = llvm[i]
        # objdump prints little-endian byte order as a 32-bit word, e.g. d2800540
        want_le = "".join(reversed([want[j:j+2] for j in range(0, 8, 2)]))
        if got != want_le:
            print(f"FAIL  {case}\n      ours: {got}\n      llvm: {want_le}  ({want})")
            bad += 1
    print(f"\n{len(CASES) - bad}/{len(CASES)} encodings match LLVM")
    return 1 if bad else 0

if __name__ == "__main__":
    sys.exit(main())
