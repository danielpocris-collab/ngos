.section .text.exceptions,"ax"

.macro PUSH_GPRS
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rbp
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax
.endm

.macro POP_GPRS
    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop rbp
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
.endm

.macro ISR_NOERR vector
.global __ngos_x86_64_isr\vector
.type __ngos_x86_64_isr\vector, @function
__ngos_x86_64_isr\vector:
    push 0
    push \vector
    jmp __ngos_x86_64_exception_common
.size __ngos_x86_64_isr\vector, . - __ngos_x86_64_isr\vector
.endm

.macro ISR_ERR vector
.global __ngos_x86_64_isr\vector
.type __ngos_x86_64_isr\vector, @function
__ngos_x86_64_isr\vector:
    push \vector
    jmp __ngos_x86_64_exception_common
.size __ngos_x86_64_isr\vector, . - __ngos_x86_64_isr\vector
.endm

.equ SYSCALL_FRAME_NUMBER, 0
.equ SYSCALL_FRAME_ARG0, 8
.equ SYSCALL_FRAME_ARG1, 16
.equ SYSCALL_FRAME_ARG2, 24
.equ SYSCALL_FRAME_ARG3, 32
.equ SYSCALL_FRAME_ARG4, 40
.equ SYSCALL_FRAME_ARG5, 48

.equ SYSCALL_RESULT_RAW, 56
.equ SYSCALL_RESULT_DISPOSITION, 64

.equ SAVE_R15, 72
.equ SAVE_R14, 80
.equ SAVE_R13, 88
.equ SAVE_R12, 96
.equ SAVE_RBP, 104
.equ SAVE_RBX, 112
.equ SAVE_R10, 120
.equ SAVE_R9, 128
.equ SAVE_R8, 136
.equ SAVE_RDI, 144
.equ SAVE_RSI, 152
.equ SAVE_RDX, 160
.equ SAVE_RAX, 168
.equ SYSCALL_STACK_FRAME_SIZE, 176

.extern __ngos_x86_64_syscall_stack_top
.extern __ngos_x86_64_saved_user_rsp
.extern __ngos_x86_64_saved_user_rip
.extern __ngos_x86_64_saved_user_rflags
.extern x86_64_syscall_dispatch

.global __ngos_x86_64_exception_common
.type __ngos_x86_64_exception_common, @function
__ngos_x86_64_exception_common:
    cld
    PUSH_GPRS
    mov rdi, rsp
    mov r12, rsp
    and rsp, -16
    call x86_64_exception_dispatch
    mov rsp, r12
    test rax, rax
    jnz __ngos_x86_64_exception_halt
    POP_GPRS
    add rsp, 16
    iretq

__ngos_x86_64_exception_halt:
    cli
1:
    hlt
    jmp 1b
.size __ngos_x86_64_exception_common, . - __ngos_x86_64_exception_common

.section .text.syscall,"ax"
.global __ngos_x86_64_syscall_entry
.type __ngos_x86_64_syscall_entry, @function
__ngos_x86_64_syscall_entry:
    cld
    mov qword ptr [rip + __ngos_x86_64_saved_user_rsp], rsp
    mov qword ptr [rip + __ngos_x86_64_saved_user_rip], rcx
    mov qword ptr [rip + __ngos_x86_64_saved_user_rflags], r11
    mov rsp, qword ptr [rip + __ngos_x86_64_syscall_stack_top]
    sub rsp, SYSCALL_STACK_FRAME_SIZE

    mov qword ptr [rsp + SYSCALL_FRAME_NUMBER], rax
    mov qword ptr [rsp + SYSCALL_FRAME_ARG0], rdi
    mov qword ptr [rsp + SYSCALL_FRAME_ARG1], rsi
    mov qword ptr [rsp + SYSCALL_FRAME_ARG2], rdx
    mov qword ptr [rsp + SYSCALL_FRAME_ARG3], r10
    mov qword ptr [rsp + SYSCALL_FRAME_ARG4], r8
    mov qword ptr [rsp + SYSCALL_FRAME_ARG5], r9

    mov qword ptr [rsp + SAVE_R15], r15
    mov qword ptr [rsp + SAVE_R14], r14
    mov qword ptr [rsp + SAVE_R13], r13
    mov qword ptr [rsp + SAVE_R12], r12
    mov qword ptr [rsp + SAVE_RBP], rbp
    mov qword ptr [rsp + SAVE_RBX], rbx
    mov qword ptr [rsp + SAVE_R10], r10
    mov qword ptr [rsp + SAVE_R9], r9
    mov qword ptr [rsp + SAVE_R8], r8
    mov qword ptr [rsp + SAVE_RDI], rdi
    mov qword ptr [rsp + SAVE_RSI], rsi
    mov qword ptr [rsp + SAVE_RDX], rdx
    mov qword ptr [rsp + SAVE_RAX], rax

    lea rdi, [rsp + SYSCALL_FRAME_NUMBER]
    mov rsi, qword ptr [rip + __ngos_x86_64_saved_user_rip]
    mov rdx, qword ptr [rip + __ngos_x86_64_saved_user_rsp]
    mov rcx, qword ptr [rip + __ngos_x86_64_saved_user_rflags]
    lea r8, [rsp + SYSCALL_RESULT_RAW]
    call x86_64_syscall_dispatch

    cmp qword ptr [rsp + SYSCALL_RESULT_DISPOSITION], 0
    jne __ngos_x86_64_syscall_halt

    mov r15, qword ptr [rsp + SAVE_R15]
    mov r14, qword ptr [rsp + SAVE_R14]
    mov r13, qword ptr [rsp + SAVE_R13]
    mov r12, qword ptr [rsp + SAVE_R12]
    mov rbp, qword ptr [rsp + SAVE_RBP]
    mov rbx, qword ptr [rsp + SAVE_RBX]
    mov r10, qword ptr [rsp + SAVE_R10]
    mov r9, qword ptr [rsp + SAVE_R9]
    mov r8, qword ptr [rsp + SAVE_R8]
    mov rdi, qword ptr [rsp + SAVE_RDI]
    mov rsi, qword ptr [rsp + SAVE_RSI]
    mov rdx, qword ptr [rsp + SAVE_RDX]
    mov rax, qword ptr [rsp + SYSCALL_RESULT_RAW]
    mov rcx, qword ptr [rip + __ngos_x86_64_saved_user_rip]
    mov r11, qword ptr [rip + __ngos_x86_64_saved_user_rflags]
    mov rsp, qword ptr [rip + __ngos_x86_64_saved_user_rsp]
    sysretq

__ngos_x86_64_syscall_halt:
    cli
2:
    hlt
    jmp 2b
.size __ngos_x86_64_syscall_entry, . - __ngos_x86_64_syscall_entry

ISR_NOERR 0
ISR_NOERR 1
ISR_NOERR 2
ISR_NOERR 3
ISR_NOERR 4
ISR_NOERR 5
ISR_NOERR 6
ISR_NOERR 7
ISR_ERR 8
ISR_NOERR 9
ISR_ERR 10
ISR_ERR 11
ISR_ERR 12
ISR_ERR 13
ISR_ERR 14
ISR_NOERR 15
ISR_NOERR 16
ISR_ERR 17
ISR_NOERR 18
ISR_NOERR 19
ISR_NOERR 20
ISR_ERR 21
ISR_NOERR 22
ISR_NOERR 23
ISR_NOERR 24
ISR_NOERR 25
ISR_NOERR 26
ISR_NOERR 27
ISR_NOERR 28
ISR_NOERR 29
ISR_NOERR 30
ISR_NOERR 31
ISR_NOERR 32
ISR_NOERR 33
ISR_NOERR 34
ISR_NOERR 35
ISR_NOERR 36
ISR_NOERR 37
ISR_NOERR 38
ISR_NOERR 39
ISR_NOERR 40
ISR_NOERR 41
ISR_NOERR 42
ISR_NOERR 43
ISR_NOERR 44
ISR_NOERR 45
ISR_NOERR 46
ISR_NOERR 47
