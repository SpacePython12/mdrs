
// Assumes that size is in d0 and align is in d1. Clobbers a0, a1, a2, d2, d3.
get_free_block:
    lea (_heap_start).l,a0  // Load heap start pointer into a0
    clr.l   d2          // Clear upper half of d2

    // Begin outer loop
    move.w  (a0)+,d2    // Read size from block header
    lsl.w   #1,d2       // Get actual size of block, free flag will be in carry/extend flags
    movea.l a0,a1       // a0 is data start, a1 will be data end
    adda.l  d2,a1       // Add block size to make data end
