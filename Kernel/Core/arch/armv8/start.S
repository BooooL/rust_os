//
//
//
#define KERNEL_BASE	0xFFFF800000000000

#define ENTRY(v)	.globl v; .type v,"function"; v:
#define GLOBAL(v)	.globl v; v:

#define PUSH(_t1,_t2)	stp _t1,_t2, [sp, #-16]!
#define POP(_t1,_t2)	ldp _t1,_t2, [sp], #16

.section VECTORS
vector_cur_sp0_sync:
	b .
	.rept 64/4-1
		b .
	.endr
vector_cur_sp0_irq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_cur_sp0_fiq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_cur_sp0_serror:
	b .
	.rept 64/4-1
		b .
	.endr
vector_cur_sync:
	b .
	.rept 64/4-1
		b .
	.endr
vector_cur_irq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_cur_fiq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_cur_serror:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower64_sync:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower64_irq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower64_fiq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower64_serror:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower32_sync:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower32_irq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower32_fiq:
	b .
	.rept 64/4-1
		b .
	.endr
vector_lower32_serror:
	b .
	.rept 64/4-1
		b .
	.endr

//.section .inittext
.section .text

.extern hexdump
.extern kmain
.globl start
start:
	ldr w0, =0x1badb002
	cmp w0, w13
	beq 1f
	// TODO: What to do if we weren't loaded by our loader
	// - For now, we return
	ret
1:
	// R9: UART Address
	// R10: FDT base address
	// R11: Symbol information base
	// R12: End of used RAM
	// R13: Magic
	
	// 0. Print a '\n' to the serial port
	mov w1, #'T' ; str w1, [x9]
	mov w1, #'i' ; str w1, [x9]
	mov w1, #'f' ; str w1, [x9]
	mov w1, #'f' ; str w1, [x9]
	mov w1, #'l' ; str w1, [x9]
	mov w1, #'i' ; str w1, [x9]
	mov w1, #'n' ; str w1, [x9]
	mov w1, #'\n'; str w1, [x9]
	
	// To get RAM start: subtract linked address of current instruction from real address
	ldr x0, =(1f-KERNEL_BASE)
	bl 1f
1:
	sub x8, x30, x0
	// Save bootloader information in various globals
	ldr x0, =(kernel_phys_start - KERNEL_BASE)
	add x0,x0, x8
	str x8, [x0]
	ldr x0, =(dt_phys_base - KERNEL_BASE)
	add x0,x0, x8
	str x10, [x0]
	ldr x0, =(symbol_info_phys - KERNEL_BASE)
	add x0,x0, x8
	str x11, [x0]
	ldr x0, =(ram_first_free - KERNEL_BASE)
	add x0,x0, x8
	str x12, [x0]
	
	mov x12, x8
	
prep_page_tables:
	ldr x0, =(kernel_root-KERNEL_BASE)
	add x0, x0, x12

	// - Mutate all populated fields in the tables - offset by RAM base
	mov x4, x0
	ldr x5, =kernel_maps_len
1:
	ldr x3, [x4], #8
	cmp x3, #0
	beq 2f
	add x3, x3, x12
	str x3, [x4, #-8]
2:
	subs x5, x5, #8
	bne 1b
	
	// Create an identity mapping before enabling paging
	orr x1, x12, #0x001	// R1 = R12 | (1<<10) | 1 (Valid, AF, Block, Kernel RWX)
	orr x1, x1, #0x400
	lsr x2, x12, #25 	// R2 = (R12 / 32MB)
	// - Split x2 into two values, indexing Lv1 and Lvl2
	lsr x6, x2, #11
	and x6, x6, #0x7FF
	and x2, x2, #0x7FF
	// - Set entries in the top and next level of the low mappings
	ldr x3, =(user0_root-KERNEL_BASE) ; add x3, x3, x12
	ldr x4, =(user0_tab2-KERNEL_BASE) ; add x4, x4, x12
	add x5, x4, #3	// Valid, Table, Kernel RWX
	str x5, [x3,x6,LSL 3]
	str x1, [x4,x2,LSL 3]
	
	// X0: Physical address of kernel_root
	// X3: Physical address of user0_root
vmsa_setup:
	msr TTBR0_EL1, x3
	msr TTBR1_EL1, x0
	// Translation Control Register
	// 34:32 - IPS = 5 (48 bits)
	// 31:30 - TG1 = 1 (16KB)
	// 29:28 - SH1 = 0 (Non-sharable)
	// 27:26 - ORGN1 = 0
	// 25:24 - IRGN1 = 0
	// 23    - EPD1 = 0 (enabled)
	// 22    - A1 = 0 (TTBR0 gives ASID)
	// 21:16 - T1SZ = 17 (47 bits)
	// 15:14 - TG0 = 2 (16KB)
	// 13:12 - SH0 = 0 (non-sharable)
	// 11:10 - ORGN0 = 0
	//  9: 8 - IRGN0 = 0
	//  7    - EPD0 = 0 (enabled)
	//  5: 0 - T0SZ = 17 (47 bits)
	ldr x1, =0x540118011
	msr TCR_EL1, x1
	isb

	// Populate the first HWMapping address with the UART's base
	orr x0, x9, #0x3
	orr x0, x0, #0x400
	ldr x1, =(kernel_hwmap_level3+0 - KERNEL_BASE)
	add x1, x1, x12
	str x0, [x1]
	
	ldr x0, =init_stack
	mov sp, x0
	ldr x0, =kmain

	//  4 - SA0 = 1 (SP alignment check)
	//  3 - SA = 1 (SP alignment check)
	//  2 - C = 0
	//  1 - A = 1 (Alignment check on)
	//  0 - M = 1 (MMU on)
	ldr x1, =0x1B
	msr SCTLR_EL1, x1
	isb
	ldr x1, =(vector_cur_sp0_sync)
	msr VBAR_EL1, x1
	mov x29, #0	// Clear FP so kernel backtrace code knows to terminate cleanly
	br x0


.section .text
ENTRY(thread_trampoline)
	//.fnstart
	//.cantunwind
	POP(x1, x0)	// X1: "thread_root" (generic over closure type), X0: Pop pointer to the closure
	br x1
	//.fnend
// pub fn task_switch(old_stack: &mut usize, new_stack: usize, new_ttbr0: usize, new_tp: usize);
// R0: Old stack save location
// R1: New stack
// R2: New TTBR0
// R3: New Thread pointer
ENTRY(task_switch)
	//.fnstart
	//.cantunwind
	// Save callee-save state (19-30)
	PUSH(x19, x20)
	PUSH(x21, x22)
	PUSH(x23, x24)
	PUSH(x25, x26)
	PUSH(x27, x28)
	PUSH(x29, x30)
	// Save user state (User SP and ELR)
	mrs x5, SP_EL0
	mrs x6, TPIDR_EL0
	PUSH(x5, x6)
	// Save ELR (exception return)
	mrs x5, ELR_EL1
	PUSH(x5, x6)
	
	// Save SP to provided location
	mov x4, sp
	str x4, [x0]

	// Update VMM root
	msr TTBR0_EL1, x2
	// Set new thread pointer
	msr TPIDR_EL1, x3

	// Set new SP
	mov sp, x1

	// Restore ELR
	POP(x5,x6)
	msr ELR_EL1, x5
	// Restore user
	POP(x5,x6)
	msr SP_EL0, x5
	msr TPIDR_EL0, x6

	// Restore callee state
	POP(x29,x30)
	POP(x27,x28)
	POP(x25,x26)
	POP(x23,x24)
	POP(x21,x22)
	POP(x19,x20)
	ret
	//.fnend

//@ pub fn drop_to_user(entry: usize, stack: usize, cmdline_len: usize) -> !;
//@ R0: entry
//@ R1: stack
//@ R2: cmdline_len
ENTRY(drop_to_user)
	//.fnstart
	//.cantunwind
	msr SPSel, #0
	mov sp, x1
	msr SPSel, #1
	msr ELR_EL1, x0
	mov x0, x2	// Set R0 = commandline length
	eret
	//.fnend

#include "../../../../Usermode/rustrt0/armv8-helpers.S"

ENTRY(__aeabi_unwind_cpp_pr0)
ENTRY(__aeabi_unwind_cpp_pr1)
	b .

.section .rodata
data_abort_message:	.ascii "Data Abort: "
data_abort_message_end:
data_abort_message2:	.ascii "\n"
data_abort_message2_end:



.section .data
GLOBAL(dt_phys_base)	.quad	0 	// (Firmware) Device Tree base location
GLOBAL(kernel_phys_start).quad	0	// Start of kernel in RAM
GLOBAL(ram_first_free)	.quad	0
GLOBAL(symbol_info_phys).quad	0

.section .bss
init_stack_base:
	.space 0x20000, 0
init_stack:
	.space 0x1000, 0
abort_stack:
.section .pabss, "aw", @nobits
user0_root:
	.space 0x4000, 0
user0_tab2:
	.space 0x4000, 0


// Page Aligned data
.section .padata
// - Top level table (lvl1), 16KB (one page), covering 2048 entries of 64GB each
// > 47 bits total
.globl kernel_maps_start
kernel_maps_start:
.globl kernel_root
kernel_root:
	.quad (kernel_image_level2-KERNEL_BASE)+0x403
	.rept 2048-1-4
		.quad 0
	.endr
	.quad (kernel_hwmap_level2-KERNEL_BASE)+0x403	// 0xFFFF_FFC0_0000_0000
	.quad 0                                    	// 0xFFFF_FFD0_0000_0000
	.quad (kernel_root-KERNEL_BASE)+0x403    	// 0xFFFF_FFE0_0000_0000
	.quad 0
// - Level 2 table for kernel image, 32MB per entry
kernel_image_level2:
	.quad 0+0x401	// Kernel image "identity" map, Priv RW only
	.rept 2048-1
		.quad 0
	.endr
// - Level 2 table for hardware mappings
kernel_hwmap_level2:
	.quad (kernel_hwmap_level3-KERNEL_BASE)+0x403
	.rept 2048-1
		.quad 0
	.endr
.globl kernel_hwmap_level3
kernel_hwmap_level3:
	.rept 2048
		.quad 0
	.endr
.globl kernel_maps_end
kernel_maps_end:

// vim: ft=armasm
