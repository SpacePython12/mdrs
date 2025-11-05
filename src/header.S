    .org 0x000
    .section .text.boot
_vector_table:
    .long _stack_top // Initial Stack Pointer
    .long _start // Initial Program Counter (Entry Point)
    .long _trap // Bus Error
    .long _addr_err // Address Error
    .long _trap // Illegal Instruction
    .long _trap // Zero Division
    .long _trap // CHK Exception
    .long _trap // TRAPV Exception
    .long _trap // Privilege Violation
    .long _trap // TRACE Exception
    .long _trap // Line A Handler
    .long _trap // Line F Handler

    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    .long _trap // Reserved
    
    .long _irq // Spurious Exception
    .long _irq // IRQ level 1
    .long _irq2 // IRQ level 2
    .long _irq // IRQ level 3
    .long _irq4 // IRQ level 4
    .long _irq // IRQ level 5
    .long _irq6 // IRQ level 6
    .long _irq // IRQ level 7
    
    .long _trap, _trap, _trap, _trap, _trap, _trap, _trap, _trap
    .long _trap, _trap, _trap, _trap, _trap, _trap, _trap, _trap
    .long _trap, _trap, _trap, _trap, _trap, _trap, _trap, _trap
    .long _trap, _trap, _trap, _trap, _trap, _trap, _trap, _trap

_rom_header: //          16 |            32 |    40 |    48 |
    .ascii "SEGA GENESIS/MD "
    .ascii "(C)SPPY 2024.APR"
    .ascii "GENESIS TESTER                                  "
    .ascii "GENESIS TESTER                                  "
    .ascii "GM 11111110-00"
    .short 0x0000
    .ascii "J6              "
    .long 0x000000
    .long 0x07FFFF
    .long 0xFF0000
    .long 0xFFFFFF
    .ascii "            "
    .ascii "            "
    .ascii "                                        "
    .ascii "JUE             "

_start:
    move.w  #0x2100,%sr
    move.l  #0x1000000,%sp
    jsr     _init
    jmp     main

    .global _trap
_trap:
    bra     _trap

_irq:
    rte

_irq2:
    movem.l %d0-%d7/%a0-%a6,-(%sp)
    jsr     _extint
    movem.l (%sp)+,%d0-%d7/%a0-%a6
    rte

_irq4: 
    movem.l %d0-%d7/%a0-%a6,-(%sp)
    jsr     _hblank
    movem.l (%sp)+,%d0-%d7/%a0-%a6
    rte

_irq6: 
    movem.l %d0-%d7/%a0-%a6,-(%sp)
    jsr     _vblank
    movem.l (%sp)+,%d0-%d7/%a0-%a6
    rte

// Handle address error because it might be a long branch instruction.
// Yes, this is required because GCC/LLVM/Rust is stupid and keeps outputting long branches even though the 68000 doesn't support them, so they must be implemented in software.
_addr_err:
    move.w  #0x2700,%sr // Disable interrupts for the time being
    movem.l %d0/%a0,-(%sp) // Save the registers we'll be using
    move.l  18(%sp),%d0 // Load the errant PC value from the stack frame
    btst    #0,%d0 // Is the source PC on an odd address?
    beq     _trap // If it isn't, it probably isn't a long branch.
    subq.l  #1,%d0 // Align the value so it doesn't cause another error
    movea.l %d0,%a0 // Move to a0 so we can use it for addressing
    move.w  (%a0)+,%d0 // Load the opcode into d0
    cmpi.w  #0x60FF,%d0 // Is the opcode bra.l?
    bne     _trap // If it isn't, trap because it probably wasn't valid anyways
    move.l  (%a0),%d0 // Load 32-bit offset into d0
    adda.l  %d0,%a0 // Offset a0 with branch offset in d0
    move.l  %a0,18(%sp) // Store the newly offseted PC value so we return to it when we rte
    movem.l (%sp)+,%d0/%a0 // Restore the clobbered registers
    lea     8(%sp),%sp // Set up rte...
    move.w   #0x2100,%sr // Re-enable interrupts
    rte // ...and return back to branch!

    .global _disable_ints
_disable_ints:
    move.w  %sr,%d0
    move.w  %d0,%d1
    ori.w   #0x0700,%d1
    move.w  %d1,%sr
    andi.w  #0x0700,%d0
    rts

    .global _enable_ints
_enable_ints:
    move.w  6(%sp),%d0
    andi.w  #0x0700,%d0
    not.w   %d0
    move.w  %sr,%d1
    and.w   %d0,%d1
    move.w  %d1,%sr
    rts

.global abort
abort:
    illegal