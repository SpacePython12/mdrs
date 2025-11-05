    .global __mulsi3
__mulsi3: 
    link    %a6,#-8
    movem.l %d2-%d3,(%sp)
    movem.l 8(%a6),%d2-%d3 // d2 = a, d3 = b

    move.w  %d2,%d0 
    mulu.w  %d3,%d0 // d0 = al x bl

    swap    %d2 
    move.w  %d2,%d1
    mulu.w  %d3,%d1
    swap    %d1
    clr.w   %d1
    add.l   %d1,%d0 

    swap    %d3
    swap    %d2
    move.w  %d2,%d1
    mulu.w  %d3,%d1
    swap    %d1
    clr.w   %d1
    add.l   %d1,%d0

    movem.l (%sp),%d2-%d3
    unlk    %a6
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

