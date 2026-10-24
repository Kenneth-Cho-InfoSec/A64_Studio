.text
.global asm_main

// Minimal program: return 42 in x0.
asm_main:
    mov x0, #42
    ret
