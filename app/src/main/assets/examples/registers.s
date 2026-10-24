.text
.global asm_main

// Integer register playground.
asm_main:
    mov x0, #10
    mov x1, #20
    add x2, x0, x1      // 30
    sub x3, x1, x0      // 10
    lsl x4, x0, #2      // 40
    orr x5, x2, x3      // 30 | 10 = 30
    mov x0, x2
    ret
