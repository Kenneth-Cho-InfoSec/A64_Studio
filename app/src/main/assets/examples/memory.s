.text
.global asm_main

// Store and load through .data. The image maps code RX and data RW.
asm_main:
    adr x1, buffer
    mov x2, #0x1234
    str x2, [x1]
    ldr x3, [x1]
    add x0, x3, #1
    ret

.data
.align 3
buffer:
    .space 16, 0
