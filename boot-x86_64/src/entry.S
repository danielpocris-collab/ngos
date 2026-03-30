.section .text.boot,"ax"
.global _start
.type _start, @function

_start:
    cli
    mov dx, 0xe9
    mov al, 'A'
    out dx, al
    lea rsp, [rip + __ngos_boot_stack_top]
    and rsp, -16
    xor rbp, rbp
    mov al, 'B'
    out dx, al
    call x86_64_boot_stage0

.size _start, . - _start
