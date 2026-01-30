#define VALUE %d0
#define REPEAT %d1
#define BITBUF %d2
#define BITCNT %d3
#define TEMP0 %d4
#define TEMP1 %d5

#define BUFFER_PTR %a0
#define FRAME_PTR %a1
#define PLANE_DATA %a2
#define MASK_TABLE %a3
#define BUFFER_END %a4

BAVideo_MaskTable:
    .short 0x0000
    .short 0x0001
    .short 0x0003
    .short 0x0007
    .short 0x000F
    .short 0x001F
    .short 0x003F
    .short 0x007F
    .short 0x00FF
    .short 0x01FF
    .short 0x03FF
    .short 0x07FF
    .short 0x0FFF
    .short 0x1FFF
    .short 0x3FFF
    .short 0x7FFF
    .short 0xFFFF

    .global BAVideo_DecodePlane
BAVideo_DecodePlane:
    movea.l 4(%sp),BUFFER_PTR
    movea.l 8(%sp),FRAME_PTR
    move.l 12(%sp),VALUE // tile offset
    movem.l BITBUF/BITCNT/TEMP0/TEMP1/PLANE_DATA/MASK_TABLE/BUFFER_END,-(%sp)
    link %a6,#-8

    moveq #0,REPEAT
    move.w 10(FRAME_PTR),REPEAT
    lea 14(FRAME_PTR,REPEAT),PLANE_DATA
    lea (BAVideo_MaskTable).l,MASK_TABLE
    lea 0x1000(BUFFER_PTR),BUFFER_END

    // Set up stack variables
    move.w #0x271B,-2(%a6) // 0x271B in hex (upper byte 39, lower byte 27, used for row/col counters)
    move.w (PLANE_DATA)+,-4(%a6)
    move.w VALUE,-6(%a6)
    move.w 0(FRAME_PTR),-8(%a6)

    moveq #0,REPEAT
    moveq #0,VALUE
    moveq #0,BITBUF
    moveq #0,BITCNT

2:
    subq.b #1,REPEAT // Decrement repeat
    bcc 3f // If repeat underflows, we need to read a new tile and repeat
    
    // Start of read_bit copy/paste code
    cmpi.b #0,BITCNT
    bne 4f // If there are enough bits in the buffer, continue
    // Start of refill_buffer copy/paste code
    move.w (PLANE_DATA)+,BITBUF // Move next bits value into buffer
    moveq #16,BITCNT // Num new bits - 1 (so that it works when decrementing first)
    // End of refill_buffer copy/paste code
4:
    subq.b #1,BITCNT // Decrement bit count
    lsr.w #1,BITBUF // bit in CARRY FLAG MAKE SURE NOT TO MESS IT UP
    // End of read_bit copy/paste code
    bcs 5f // If bit was set, we will check for another bit

    // Read solid tile here
    // We need to read 8 bits here.

    // Start of read_bits copy/paste code (optimized since bit count is constant here)
    move.b #8,REPEAT // Move the index bit count to repeat (repeat is just -1 anyway)
    move.w REPEAT,TEMP0 // Save the len (and undefined upper byte!) to stack as a backup
    move.w BITBUF,VALUE // Move current bits value to value (we will use it whether we have enough bits or not)
    cmp.b REPEAT,BITCNT // Check if we need to read another value
    bcc 6f // If we don't need to, skip this part
    // Start of refill_buffer copy/paste code
    move.w (PLANE_DATA)+,BITBUF // Move next bits value into buffer
    // (THIS IS NOT PART OF COPY/PASTE)
    move.w BITBUF,TEMP1 // Save bit buffer on stack 
    lsl.w BITCNT,BITBUF // Shift bit buffer to fill remaining bits in value
    or.w BITBUF,VALUE // OR bit buffer with value to put remaining bits in (note that any extra bits need to be ANDed out now)
    move.w TEMP1,BITBUF // Restore bit buffer
    sub.b BITCNT,REPEAT // Subtract old bit count from len
    // (back to copy/paste)
    moveq #16,BITCNT // Num new bits - 1 (so that it works when decrementing first)
    // End of refill_buffer copy/paste code
6:
    lsr.w REPEAT,BITBUF // Shift bit buffer down
    sub.b REPEAT,BITCNT // Subtract len from bit buffer len
    move.w TEMP0,REPEAT // Restore len in case it was modified
    and.l #0xF,REPEAT // AND out undefined upper byte of len so it doesn't cause problems
    add.b REPEAT,REPEAT // Double len so its a word index
    and.w (0,MASK_TABLE,REPEAT),VALUE // AND out excess bits (using precomputed masks)
    // End of read_bits copy/paste code (optimized since bit count is constant here)

    move.b VALUE,REPEAT // Copy value to repeat
    lsr.b #4,REPEAT // Shift out solid tile index
    and.w #0xF,VALUE // Mask out solid tile index

    bra 3f // Exit if statement
5:
    // Start of read_bit copy/paste code
    cmpi.b #0,BITCNT
    bne 4f // If there are enough bits in the buffer, continue
    // Start of refill_buffer copy/paste code
    move.w (PLANE_DATA)+,BITBUF // Move next bits value into buffer
    moveq #16,BITCNT // Num new bits - 1 (so that it works when decrementing first)
    // End of refill_buffer copy/paste code
4:
    subq.b #1,BITCNT // Decrement bit count
    lsr.w #1,BITBUF // bit in CARRY FLAG MAKE SURE NOT TO MESS IT UP
    // End of read_bit copy/paste code
    bcs 5f // If bit was set, we know it was a unique tile

    // Read shared tile here

    // Start of read_bits copy/paste code
    move.b -4(%a6),REPEAT // Move the index bit count to repeat (repeat is just -1 anyway)
    addq.b #4,REPEAT // Add 4 to that count for repeat and flip
    move.w REPEAT,TEMP0 // Save the len (and undefined upper byte!) to stack as a backup
    move.w BITBUF,VALUE // Move current bits value to value (we will use it whether we have enough bits or not)
    cmp.b REPEAT,BITCNT // Check if we need to read another value
    bcc 6f // If we don't need to, skip this part
    // Start of refill_buffer copy/paste code
    move.w (PLANE_DATA)+,BITBUF // Move next bits value into buffer
    // (THIS IS NOT PART OF COPY/PASTE)
    move.w BITBUF,TEMP1 // Save bit buffer on stack 
    lsl.w BITCNT,BITBUF // Shift bit buffer to fill remaining bits in value
    or.w BITBUF,VALUE // OR bit buffer with value to put remaining bits in (note that any extra bits need to be ANDed out now)
    move.w TEMP1,BITBUF // Restore bit buffer
    sub.b BITCNT,REPEAT // Subtract old bit count from len
    // (back to copy/paste)
    moveq #16,BITCNT // Num new bits - 1 (so that it works when decrementing first)
    // End of refill_buffer copy/paste code
6:
    lsr.w REPEAT,BITBUF // Shift bit buffer down
    sub.b REPEAT,BITCNT // Subtract len from bit buffer len
    move.w TEMP0,REPEAT // Restore len in case it was modified (note that it wasn't actually popped from the stack)
    and.l #0xFF,REPEAT // AND out undefined upper byte of len so it doesn't cause problems
    add.b REPEAT,REPEAT // Double len so its a word index
    and.w (0,MASK_TABLE,REPEAT),VALUE // AND out excess bits (using precomputed masks)
    // End of read_bits copy/paste code

    move.b -4(%a6),REPEAT // Load shared index bits to repeat
    move.w VALUE,TEMP0 // Save value to stack
    lsr.w REPEAT,VALUE // Shift value down to get repeat and flip
    move.b VALUE,REPEAT // Get repeat and flip into repeat
    move.w TEMP0,VALUE // Restore value
    move.w REPEAT,TEMP0 // Save repeat 
    move.b -4(%a6),REPEAT // Load shared index bits to repeat (again...)
    add.b REPEAT,REPEAT // Double len so its a word index
    and.w (0,MASK_TABLE,REPEAT),VALUE // AND out everything except for tile index in value
    move.w TEMP0,REPEAT // Restore (but not pop) repeat + flip
    and.b #0x3,REPEAT // Mask in only flip
    lsl.w #6,REPEAT // Shift up flip halfway
    lsl.w #5,REPEAT // ...and again.
    or.w REPEAT,VALUE // OR with value to set tile flip flags
    move.w TEMP0,REPEAT // Restore repeat for real this time
    lsr.b #2,REPEAT // Shift out flip

    add.w -6(%a6),VALUE // Add tile offset

    // ori.w #0x2000,VALUE

    // And at long last, we're done. No wonder decompressing this takes so long.

    bra 3f // Exit if statement
5:

    // Read unique tile here

    // Start of read_bits copy/paste code
    move.b -3(%a6),REPEAT // Move the index bit count to repeat (repeat is just -1 anyway)
    addq.b #3,REPEAT // Add 3 to that count for repeat and flip
    move.w REPEAT,TEMP0 // Save the len (and undefined upper byte!) to stack as a backup
    move.w BITBUF,VALUE // Move current bits value to value (we will use it whether we have enough bits or not)
    cmp.b REPEAT,BITCNT // Check if we need to read another value
    bcc 6f // If we don't need to, skip this part
    // Start of refill_buffer copy/paste code
    move.w (PLANE_DATA)+,BITBUF // Move next bits value into buffer
    // (THIS IS NOT PART OF COPY/PASTE)
    move.w BITBUF,TEMP1 // Save bit buffer on stack 
    lsl.w BITCNT,BITBUF // Shift bit buffer to fill remaining bits in value
    or.w BITBUF,VALUE // OR bit buffer with value to put remaining bits in (note that any extra bits need to be ANDed out now)
    move.w TEMP1,BITBUF // Restore bit buffer
    sub.b BITCNT,REPEAT // Subtract old bit count from len
    // (back to copy/paste)
    moveq #16,BITCNT // Num new bits - 1 (so that it works when decrementing first)
    // End of refill_buffer copy/paste code
6:
    lsr.w REPEAT,BITBUF // Shift bit buffer down
    sub.b REPEAT,BITCNT // Subtract len from bit buffer len
    move.w TEMP0,REPEAT // Restore len in case it was modified (note that it wasn't actually popped from the stack)
    and.l #0xFF,REPEAT // AND out undefined upper byte of len so it doesn't cause problems
    add.b REPEAT,REPEAT // Double len so its a word index
    and.w (0,MASK_TABLE,REPEAT),VALUE // AND out excess bits (using precomputed masks)
    // End of read_bits copy/paste code

    move.b -3(%a6),REPEAT // Load shared index bits to repeat
    move.w VALUE,TEMP0 // Save value to stack
    lsr.w REPEAT,VALUE // Shift value down to get repeat and flip
    move.b VALUE,REPEAT // Get repeat and flip into repeat
    move.w TEMP0,VALUE // Restore value
    move.w REPEAT,TEMP0 // Save repeat 
    move.b -3(%a6),REPEAT // Load shared index bits to repeat (again...)
    add.b REPEAT,REPEAT // Double len so its a word index
    and.w (0,MASK_TABLE,REPEAT),VALUE // AND out everything except for tile index in value
    move.w TEMP0,REPEAT // Restore (but not pop) repeat + flip
    and.b #0x3,REPEAT // Mask in only flip
    lsl.w #6,REPEAT // Shift up flip halfway
    lsl.w #5,REPEAT // ...and again.
    or.w REPEAT,VALUE // OR with value to set tile flip flags
    move.w TEMP0,REPEAT // Restore repeat for real this time
    lsr.b #2,REPEAT // Shift out flip

    add.w -6(%a6),VALUE // Add tile offset
    add.w -8(%a6),VALUE // Add shared tile offset

    // ori.w #0x2000,VALUE

    // And at long last, we're done. No wonder decompressing this takes so long.

3:
    move.w VALUE,(BUFFER_PTR)+
    cmpa.l BUFFER_END,BUFFER_PTR // If we've somehow reached the end of the buffer, break early.
    bcc 3f
    subq.b #1,-2(%a6) // Decrement column
    bcc 2b // If column is not zero keep going
    adda.l #48,BUFFER_PTR // Skip to start of next row
    move.b #39,-2(%a6) // Set column
    subq.b #1,-1(%a6) // Decrement row
    bcc 2b // If row is not zero keep going
3:

    unlk %a6
    movem.l (%sp)+,BITBUF/BITCNT/TEMP0/TEMP1/PLANE_DATA/MASK_TABLE/BUFFER_END
    rts

// %a0 = buffer pointer
// %a1 = frame pointer / used tiles pointer
// %a2 = shared tiles pointer
// %a3 = tile pointer
// %d0 = tile count
// %d1 = tile index
    .global BAVideo_DecodeSharedTiles
BAVideo_DecodeSharedTiles:
    movea.l 4(%sp),%a0
    movea.l 8(%sp),%a1
    move.l 12(%sp),%d0 // video pointer
    movem.l %a2-%a3,-(%sp) // Save extra variables

    movea.l %d0,%a2
    lea 8(%a2),%a2 // %a2 now points to shared tiles
    move.w 0(%a1),%d0 // %d0 now contains tile count
    lea 14(%a1),%a1 // %a1 now points to used tiles

    tst.w %d0
    beq 3f
2:
    moveq #0,%d1
    
    move.w (%a1)+,%d1 // Load next tile index
    lsl.l #4,%d1 // index * 32 so its a byte address
    lea (0,%a2,%d1),%a3 // Get address of tile
    
    // Copy longs of tile (vectorized)
    move.l (%a3),(%a0)+
    move.l (%a3)+,(%a0)+
    move.l (%a3),(%a0)+
    move.l (%a3)+,(%a0)+
    move.l (%a3),(%a0)+
    move.l (%a3)+,(%a0)+
    move.l (%a3),(%a0)+
    move.l (%a3)+,(%a0)+
    
    subq.w #1,%d0 // Did tile count underflow?
    bne 2b
3:
    movem.l (%sp)+,%a2-%a3 // Restore extra variables
    rts

// %a0 = buffer pointer
// %a1 = frame pointer / unique tiles pointer
// %a2 = buffer end pointer
// %d0 = tile count
// %d1 = line
// %d2 = nybble
// %d3 = repeat
// %d4 = index
    .global BAVideo_DecodeUniqueTiles
BAVideo_DecodeUniqueTiles:
    movea.l 4(%sp),%a0
    movea.l 8(%sp),%a1
    movem.l %d2-%d4/%a2,-(%sp) // Save extra variables

    moveq #0,%d0
    move.w 2(%a1),%d0 // %d0 now contains tile count

    tst.w %d0 // Is tile count zero?
    beq 4f // If it is, skip

    lsr.l #5,%d0 // %d0 = tiles byte size
    lea (0,%a0,%d0),%a2 // %a2 = buffer end

    move.w 2(%a1),%d0 // %d0 now contains tile count
    moveq #0,%d1 // Clear upper word of %d1
    move.w 8(%a1),%d1 // %d1 now contains unique tile offset
    lea 14(%a1,%d1),%a1 // %a1 now points to unique tiles

    moveq #0,%d1 // line = 0
    moveq #0,%d3 // repeat = 0
    moveq #7,%d4 // index = 7

    lsl.w #3,%d0 // tile_count = number of lines
2:
    subq.b #1,%d3
    bcc 3f // if repeat underflows:
    move.b (%a1)+,%d2
    move.b %d2,%d3
    lsr.b #4,%d3
    andi.b #15,%d2
3:
    lsl.l #4,%d1
    or.b %d2,%d1
    subq.b #1,%d4
    bcc 2b // if index underflows:
    move.l %d1,(%a0)+
    // cmpa.l %a2,%a0 // If we've somehow reached the end of the buffer, break early.
    // bcc 4f
    move.l %d1,(%a0)+
    // cmpa.l %a2,%a0 // If we've somehow reached the end of the buffer, break early.
    // bcc 4f
    moveq #7,%d4
    subq.w #2,%d0
    bcc 2b // If count didn't underflow, go again
4:
    movem.l (%sp)+,%d2-%d4/%a2 // Restore extra variables
    rts