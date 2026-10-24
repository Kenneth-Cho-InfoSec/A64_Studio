.text
.global asm_main

// Iterative Fibonacci: returns F(10) = 55 in x0.
asm_main:
    mov x0, #0
    mov x1, #1
    mov x2, #10
loop:
    add x3, x0, x1
    mov x0, x1
    mov x1, x3
    subs x2, x2, #1
    b.ne loop
    ret
