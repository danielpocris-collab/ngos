.section .text.smp_trampoline,"ax"

.global __ngos_x86_64_ap_trampoline_start
.global __ngos_x86_64_ap_trampoline_end
.global __ngos_x86_64_ap_entry_stub
.global __ngos_x86_64_ap_pm_entry_ptr
.global __ngos_x86_64_ap_pm_stack_ptr
.global __ngos_x86_64_ap_rendezvous_cr3_ptr
.global __ngos_x86_64_ap_gdt16_base_ptr
.global __ngos_x86_64_ap_gdt64_linear_ptr
.global __ngos_x86_64_ap_gdt64_base_ptr
.global __ngos_x86_64_ap_lm_entry_ptr
.global __ngos_x86_64_ap_lm_stack_ptr
.global __ngos_x86_64_ap_rendezvous_ptr
.global __ngos_x86_64_ap_rust_entry_ptr

.equ AP_RENDEZVOUS_OFFSET, 0x400
.equ AP_TEMP_STACK_OFFSET, 0xff0

.equ AP_PROTECTED_MODE_OFFSET, 0x40
.equ AP_LONG_MODE_OFFSET, 0x100
.equ AP_GDT16_PTR_OFFSET, 0x2a8
.equ AP_GDT64_PTR_OFFSET, 0x2b0
.equ AP_RUST_ENTRY_PTR_OFFSET, 0x2c0

.equ RD_BOOTSTRAP_CR3, 16

.equ IA32_EFER, 0xc0000080
.equ CR0_PE, 0x00000001
.equ CR0_MP, 0x00000002
.equ CR0_EM, 0x00000004
.equ CR0_PG, 0x80000000
.equ CR4_PAE, 0x00000020
.equ CR4_OSFXSR, 0x00000200
.equ CR4_OSXMMEXCPT, 0x00000400
.equ EFER_LME, 0x00000100

.code16
__ngos_x86_64_ap_trampoline_start:
__ngos_x86_64_ap_entry_stub:
    cli
    cld
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    lgdt cs:[AP_GDT16_PTR_OFFSET]
    mov eax, cr0
    or eax, CR0_PE
    mov cr0, eax
    .byte 0x66, 0xea
__ngos_x86_64_ap_pm_entry_ptr:
    .long AP_PROTECTED_MODE_OFFSET
    .word 0x08

.org 0x40
.code32
ap_protected_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    .byte 0xbc
__ngos_x86_64_ap_pm_stack_ptr:
    .long AP_TEMP_STACK_OFFSET

    .byte 0xa1
__ngos_x86_64_ap_rendezvous_cr3_ptr:
    .long AP_RENDEZVOUS_OFFSET + RD_BOOTSTRAP_CR3
    mov cr3, eax

    mov eax, cr4
    or eax, CR4_PAE
    mov cr4, eax

    mov ecx, IA32_EFER
    rdmsr
    or eax, EFER_LME
    wrmsr

    .byte 0x0f, 0x01, 0x15
__ngos_x86_64_ap_gdt64_linear_ptr:
    .long AP_GDT64_PTR_OFFSET

    mov eax, cr0
    and eax, ~CR0_EM
    or eax, CR0_MP
    or eax, CR0_PG
    mov cr0, eax
    .byte 0xea
__ngos_x86_64_ap_lm_entry_ptr:
    .long AP_LONG_MODE_OFFSET
    .word 0x18

.org 0x100
.code64
ap_long_mode_entry:
    mov ax, 0x20
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    .byte 0x48, 0xbc
__ngos_x86_64_ap_lm_stack_ptr:
    .quad 0

    mov rax, cr4
    or rax, CR4_OSFXSR | CR4_OSXMMEXCPT
    mov cr4, rax
    fninit

    .byte 0x48, 0xbf
__ngos_x86_64_ap_rendezvous_ptr:
    .quad 0
    mov rax, qword ptr [AP_RUST_ENTRY_PTR_OFFSET]
    call rax

1:
    cli
    hlt
    jmp 1b

.org 0x280
ap_gdt:
    .quad 0x0000000000000000
    .quad 0x00cf9a000000ffff
    .quad 0x00cf92000000ffff
    .quad 0x00af9a000000ffff
    .quad 0x00af92000000ffff

.org 0x2a8
ap_gdt16_ptr:
    .word (5 * 8) - 1
__ngos_x86_64_ap_gdt16_base_ptr:
    .long 0x280

.org 0x2b0
ap_gdt64_ptr:
    .word (5 * 8) - 1
__ngos_x86_64_ap_gdt64_base_ptr:
    .quad 0x280

.org 0x2c0
__ngos_x86_64_ap_rust_entry_ptr:
    .quad 0

__ngos_x86_64_ap_trampoline_end:
