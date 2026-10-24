.text
.global asm_main

// Advanced SIMD: (1 + 2) * 3 = 6 in every byte lane.
asm_main:
    movi v0.16b, #1
    movi v1.16b, #2
    add v0.16b, v0.16b, v1.16b
    movi v2.16b, #3
    mul v0.16b, v0.16b, v2.16b
    umov w0, v0.b[0]
    ret
