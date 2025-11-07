    .global __mulsi3
__mulsi3: 
    move.w  4(%sp),%d0  // Move high word of x into d0
    mulu.w  10(%sp),%d0 // Multiply high word of x with low word of y
    move.w  8(%sp),%d1  // Move high word of y into d1
    move.w  6(%sp),%d1  // Multiply high word of y with low word of x
    add.w   %d1,%d0
    swap    %d0

    move.w  6(%sp),%d1  // Move low word of x into d1
    mulu.w  10(%sp),%d1 // Multiply low word of x with low word of y
    add.l   %d1,%d0

    rts

    .global __modsi3
__modsi3:
    move.l  8(%sp),%d1  // Load divisor into d1
    move.l  4(%sp),%d0  // Load dividend into d0

    move.l  %d1,-(%sp)  // Push divisor to argument 1
    move.l  %d0,-(%sp)  // Push dividend to argument 0
    bsr.s   __divsi3    // Jump to signed division routine
    addq.l  #8,%sp      // Deallocate arguments

    move.l  8(%sp),%d1  // Load divisor into d1
    move.l  %d1,-(%sp)  // Push divisor to argument 1
    move.l  %d0,-(%sp)  // Push dividend to argument 0
    bsr.s   __mulsi3    // Jump to multiplication routine
    addq.l  #8,%sp      // Deallocate arguments

    move.l  4(%sp),%d1  // Load dividend into d1
    sub.l   %d0,%d1     // d1 = dividend - quotient x divisor
    move.l  %d1,%d0

    rts

    .global __umodsi3
__umodsi3:
    move.l  8(%sp),%d1  // Load divisor into d1
    move.l  4(%sp),%d0  // Load dividend into d0

    move.l  %d1,-(%sp)  // Push divisor to argument 1
    move.l  %d0,-(%sp)  // Push dividend to argument 0
    bsr.s   __udivsi3   // Jump to unsigned division routine
    addq.l  #8,%sp      // Deallocate arguments

    move.l  8(%sp),%d1  // Load divisor into d1
    move.l  %d1,-(%sp)  // Push divisor to argument 1
    move.l  %d0,-(%sp)  // Push dividend to argument 0
    bsr.s   __mulsi3    // Jump to multiplication routine
    addq.l  #8,%sp      // Deallocate arguments

    move.l  4(%sp),%d1  // Load dividend into d1
    sub.l   %d0,%d1     // d1 = dividend - quotient x divisor
    move.l  %d1,%d0

    rts

    .global __divsi3
__divsi3:
    move.l  %d2,-(%sp)  // Save d2
    
    moveq   #1,%d2      // Load sign of quotient into d2
    move.l  12(%sp),%d1 // Load divisor into d1
    bpl.s   1f          // If divisor is positive, skip ahead
    neg.l   %d1         // Negate the divisor
    neg.b   %d2         // Flip the sign

1:  move.l  8(%sp),%d0  // Load dividend into d0
    bpl.s   1f          // If dividend is positive, skip ahead
    neg.l   %d0         // Negate the dividend
    neg.b   %d2         // Flip the sign

1:  move.l  %d1,-(%sp)  // Push divisor to argument 1
    move.l  %d0,-(%sp)  // Push dividend to argument 0
    bsr     __udivsi3   // Jump to unsigned division routine
    addq.l  #8,%sp      // Deallocate arguments

    tst.b   %d2         // Check if sign is negative
    bpl.s   1f          // If sign is positive, skip ahead
    neg.l   %d0         // Negate the quotient

1:  move.l  (%sp)+,%d2  // Restore d2
    rts

    .global __udivsi3
__udivsi3:
	move.l  %d2,-(%sp)  // Save d2
    move.l  8(%sp),%d0  // Load dividend into d0
    move.l  12(%sp),%d1 // Load divisor into d1

    cmpi.l  #0x10000,%d1 // Is divisor greater than 2^16?
    bcc.s   1f          // If it is, try the next algorithm
    move.l  %d0,%d2
    clr.w   %d2
    swap    %d2
    divu.w  %d1,%d2     // Upper quotient in lower word
    move.w  %d2,%d0     // Save upper quotient
    swap    %d0
    move.w  10(%sp),%d2 // Lower dividend + upper rest
    divu.w  %d1,%d2     // Lower quotient
    move.w  %d2,%d0
    bra.s   2f

1:  move.l  %d1,%d2     // Use d2 as divisor backup
1:  lsr.l   #1,%d1      // Shift divisor down
    lsr.l   #1,%d0      // Shift dividend down
    cmpi.l  #0x10000,%d1 // Is divisor still greater than 2^16?
    bcc.s   1b          // If it is, do it again!
    divu.w  %d1,%d0     // Now we have 16 bit divisor
    andi.l  #0xFFFF,%d0 // Mask out divisor

/* Multiply the 16 bit tentative quotient with the 32 bit divisor.  Because of
   the operand ranges, this might give a 33 bit product.  If this product is
   greater than the dividend, the tentative quotient was too large. */

    move.l  %d2,%d1
    mulu.w  %d0,%d1     // Low part, 32 bits
    swap    %d2
    mulu.w  %d0,%d2     // High part, no more than 17 bits
    swap    %d2         // Align high part with low part
    tst.w   %d2         // Is the high part 17 bits?
    bne.s   1f          // If it is, the quotient is too large
    add.l   %d2,%d1     // Add parts together
    bcs.s   1f          // Quotient was too large and overflowed
    cmp.l   8(%sp),%d1  // Is the sum greater than the divident?
    bls.w   2f          // If it is, the quotient is too large
1:  subq.l  #1,%d0      // Adjust quotient

2:  move.l  (%sp)+,%d2  // Restore d2
    rts

// for copy pseudofunctions: a0 = dst, a1 = src, d1 = len, d0 = scratch (when going backward src and dst are the END of the array)

    .global memcpy
memcpy:
    movea.l 4(%sp),%a0 // Load dst into a0
    movea.l 8(%sp),%a1 // Load src into a1
    move.l  12(%sp),%d1 // Load len into d1
    bsr.s   copy_forward
    movea.l 4(%sp),%a0 // Load dst into a0 (because memcpy returns dst)
    rts

copy_forward:
    cmpi.l  #2,%d1 // Is len less than 2?
    bcs.s   3f // If it is, skip to last byte copy
    move.l  %a0,%d0 // Move dst to scratch so we can check if its aligned
    btst    #0,%d0 // Is dst aligned?
    beq.s   1f // If it is, skip
    move.b  (%a1)+,(%a0)+ // Copy byte (now dst is aligned)
1:  move.l  %a1,%d0 // Move src to scratch so we can check if its aligned
    btst    #0,%d0 // Is src aligned?
    bne.s   4f // If not, do byte copy instead

    // Aligned word copy start
    cmpi.l  #4,%d1 // Is len less than 4?
    bcs.s   2f // If it is skip to final word copy
1:  move.l  (%a1)+,(%a0)+ // Copy long
    subq.l  #4,%d1 // Decrement length
    cmpi.l  #4,%d1 // Is len less than 4?
    bcc.s   1b // If not, keep going
2:  cmpi.l  #2,%d1 // Is len less than 2?
    bcs.s   5f // If it is return
    move.w  (%a1)+,(%a0)+ // Copy word
    subq.l  #2,%d1 // Decrement length
    // Aligned word copy end

3:  tst.l   %d1 // Is length zero?
    beq.s   5f // If it is, return
    move.b  (%a1)+,(%a0)+ // Copy byte
    subq.l  #1,%d1 // Decrement length
    bra.s   5f // Return

    // Byte copy start
4:  move.b  (%a1)+,(%a0)+ // Copy byte
    subq.l  #1,%d1 // Decrement length
    bne.s   4b // If length is not zero, keep going
    // Byte copy end

5:  rts

.global memmove
memmove:
    movea.l 4(%sp),%a0 // Load dst into a0
    movea.l 8(%sp),%a1 // Load src into a1
    move.l  12(%sp),%d1 // Load len into d1
    move.l  %a0,%d0
    sub.l   %a1,%d0 // Get distance between dst and src
    cmp.l   %d1,%d0 // Is distance greater than length?
    bcs.s   1f // If not, copy backwards
    bsr.s   copy_forward
    bra.s   2f
1:  adda.l  %d1,%a0 // Make a0 point to the end of dst
    adda.l  %d1,%a1 // Make a1 point to the end of src
    bsr.s   copy_backward
2:  movea.l 4(%sp),%a0 // Load dst into a0 (because memmove returns dst)
    rts

copy_backward:
    cmpi.l  #2,%d1 // Is len less than 2?
    bcs.s   3f // If it is, skip to last byte copy
    move.l  %a0,%d0 // Move dst to scratch so we can check if its aligned
    btst    #0,%d0 // Is dst aligned?
    beq.s   1f // If it is, skip
    move.b  -(%a1),-(%a0) // Copy byte (now dst is aligned)
1:  move.l  %a1,%d0 // Move src to scratch so we can check if its aligned
    btst    #0,%d0 // Is src aligned?
    bne.s   4f // If not, do byte copy instead

    // Aligned word copy start
    cmpi.l  #4,%d1 // Is len less than 4?
    bcs.s   2f // If it is skip to final word copy
1:  move.l  -(%a1),-(%a0) // Copy long
    subq.l  #4,%d1 // Decrement length
    cmpi.l  #4,%d1 // Is len less than 4?
    bcc.s   1b // If not, keep going
2:  cmpi.l  #2,%d1 // Is len less than 2?
    bcs.s   5f // If it is return
    move.w  -(%a1),-(%a0) // Copy word
    subq.l  #2,%d1 // Decrement length
    // Aligned word copy end

3:  tst.l   %d1 // Is length zero?
    beq.s   5f // If it is, return
    move.b  -(%a1),-(%a0) // Copy byte
    subq.l  #1,%d1 // Decrement length
    bra.s   5f // Return

    // Byte copy start
4:  move.b  -(%a1),-(%a0) // Copy byte
    subq.l  #1,%d1 // Decrement length
    bne.s   4b // If length is not zero, keep going
    // Byte copy end

5:  rts

    .global memset
memset:
    movea.l 4(%sp),%a0 // Load dst into a0
    move.b  11(%sp),%d0 // Load val into d0
    move.l  12(%sp),%d1 // Load len into d1
    move.l  %d2,-(%sp) // Save d2
    
    // Load val into all 4 bytes of d2
    move.b  %d0,%d2
    lsl.l   #8,%d2
    move.b  %d0,%d2
    move.w  %d2,%d0
    swap    %d2
    move.w  %d0,%d2
    
    cmpi.l  #2,%d1 // Is len less than 2?
    bcs.s   3f // If it is, skip to last byte copy
    move.l  %a0,%d0 // Move dst to scratch so we can check if its aligned
    btst    #0,%d0 // Is dst aligned?
    beq.s   1f // If it is, skip
    move.b  %d2,(%a0)+ // Copy byte (now dst is aligned)
1:  move.l  %a1,%d0 // Move src to scratch so we can check if its aligned
    btst    #0,%d0 // Is src aligned?
    bne.s   4f // If not, do byte copy instead

    // Aligned word copy start
    cmpi.l  #4,%d1 // Is len less than 4?
    bcs.s   2f // If it is skip to final word copy
1:  move.l  %d2,(%a0)+ // Copy long
    subq.l  #4,%d1 // Decrement length
    cmpi.l  #4,%d1 // Is len less than 4?
    bcc.s   1b // If not, keep going
2:  cmpi.l  #2,%d1 // Is len less than 2?
    bcs.s   5f // If it is return
    move.w  %d2,(%a0)+ // Copy word
    subq.l  #2,%d1 // Decrement length
    // Aligned word copy end

3:  tst.l   %d1 // Is length zero?
    beq.s   5f // If it is, return
    move.b  %d2,(%a0)+ // Copy byte
    subq.l  #1,%d1 // Decrement length
    bra.s   5f // Return

    // Byte copy start
4:  move.b  %d2,(%a0)+ // Copy byte
    subq.l  #1,%d1 // Decrement length
    bne.s     4b // If length is not zero, keep going
    // Byte copy end

5:  move.l  (%sp)+,%d2 // Restore d2
    movea.l 4(%sp),%a0 // Load dst into a0 (because memset returns dst)
    rts

    .global memcmp
memcmp:
    .global bcmp
bcmp:
    movea.l 4(%sp),%a0 // Load srca into a0
    movea.l 8(%sp),%a1 // Load srcb into a1
    move.l  12(%sp),%d1 // Load len into d1

1:  move.b  (%a0)+,%d0 // Load byte from srca
    sub.b   (%a1)+,%d0 // Subtract byte from srcb
    bne.s   1f // If values not the same, break
    subq.l  #1,%d1 // Decrement length
    bne.s   1b // If length is not zero, keep going

1:  ext.w   %d0 // Extend return value from byte to word
    ext.l   %d0 // Extend return value from word to long

    rts

