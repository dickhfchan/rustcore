/* Minimal PVH stage-0 loader that switches the CPU into 64-bit long mode
 * before tail-calling the Rust kernel entry (`kernel_main`).
 */

.att_syntax prefix

.section .text.boot, "ax"
.code32
.global _start
.extern kernel_main

_start:
    cli

    movw $0xE9, %dx
    movb $'B', %al
    out %al, %dx

    mov $stack_top32, %esp
    mov %esp, %ebp

    lgdt gdt64_ptr

    mov $pml4_table, %eax
    mov %eax, %cr3

    mov %cr4, %eax
    or $0x620, %eax        /* CR4.PAE | OSFXSR | OSXMMEXCPT */
    mov %eax, %cr4

    mov $0xC0000080, %ecx /* IA32_EFER */
    rdmsr
    or $0x00000100, %eax  /* LME */
    wrmsr

    mov %cr0, %eax
    or $0x80000023, %eax  /* PG | PE | MP | NE */
    mov %eax, %cr0

    ljmp $0x08, $long_mode_entry

.code64
long_mode_entry:
    movw $0xE9, %dx
    movb $'L', %al
    out %al, %dx

    mov %cr4, %rax
    or $0x620, %rax
    mov %rax, %cr4

    mov %cr0, %rax
    or $0x22, %rax
    mov %rax, %cr0

    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %ss

    lea stack_top(%rip), %rsp
    xor %rbp, %rbp

    movb $'K', %al
    out %al, %dx

    call kernel_main

1:
    hlt
    jmp 1b

.section .rodata.boot, "a"
.align 8

/* 64-bit GDT: null, code, data */
gdt64:
    .quad 0x0000000000000000
    .quad 0x00AF9A000000FFFF
    .quad 0x00AF92000000FFFF

gdt64_ptr:
    .word gdt64_end - gdt64 - 1
    .quad gdt64

gdt64_end:

.section .data.boot, "aw"
.align 4096

pml4_table:
    .quad pdpt_table + 0x003
    .fill 511, 8, 0

.align 4096
pdpt_table:
    .quad pd_table + 0x003
    .fill 511, 8, 0

.align 4096
pd_table:
    .set idx, 0
    .rept 512
        .quad (idx << 21) + 0x183
        .set idx, idx + 1
    .endr

.section .bss.boot, "aw", @nobits
.align 16
stack_space:
    .skip 8192
stack_top:
stack_top32:
