use core::{alloc::Layout, ptr::NonNull};


extern "C" {
    static mut _heap_start: u8;
    static mut _heap_end: u8;
}

#[inline]
const fn heap_size() -> usize {
    unsafe { (&raw const _heap_end).offset_from(&raw const _heap_start) as usize }
}

/// A specialized allocator, taking advantage of the fact that RAM is only 64 kB, and can be addressed fully with a u16, rather than a usize.
/// 
/// As a result, block headers are tiny; only a single word!
pub struct MDSpecializeAlloc;

impl MDSpecializeAlloc {
    #[inline]
    const fn root_block(&self) -> NonNull<BlockHeader> {
        unsafe { NonNull::new_unchecked((&raw mut _heap_start).cast()) }
    }

    #[inline]
    unsafe fn get_free_block(&self, layout: Layout) -> Option<NonNull<BlockHeader>> {
        let mut current = Some(self.root_block());
        while let Some(mut curr_ptr) = current {
            let curr_block = curr_ptr.as_mut();
            if curr_block.is_free() {
                // Try combining consecutive free blocks.
                while let Some(next_ptr) = curr_block.next() {
                    // Current block isnt at the end, so start checking the next block.
                    let next_block = next_ptr.as_ref();
                    if next_block.is_free() {
                        // Combine the current block with the next block.
                        curr_block.size += next_block.size & !BlockHeader::FREE_BIT;
                    } else {
                        // Hit a used block, break
                        break;
                    }
                }

                if curr_block.satisfies_layout(layout) {
                    // Current block has a suitable size, so break
                    break;
                } else {
                    current = curr_block.next();
                }
            } else {
                current = curr_block.next();
            }
        }
        current
    }

    #[inline]
    pub const fn new() -> Self {
        Self
    }

    #[inline]
    pub unsafe fn init(&self) {
        // Initialize root block
        *self.root_block().as_mut() = BlockHeader {
            size: BlockHeader::FREE_BIT | ((heap_size() as u16) >> 1),
        };
    }

    #[inline(never)]
    pub unsafe fn allocate(&self, layout: Layout) -> Option<NonNull<u8>> {
        let mut block_ptr = self.get_free_block(layout)?;
        let block = block_ptr.as_mut();

        // Find data pointer and data size
        let data_ptr = block.data_with_layout(layout);
        let data_size = block.data_end().byte_offset_from_unsigned(data_ptr);

        // Initalize new block header
        let mut header_ptr = data_ptr.cast::<BlockHeader>().sub(1);
        *header_ptr.as_mut() = BlockHeader {
            size: (data_size as u16) >> 1, // No free bit
        };

        // Change old block size to reflect new block
        block.size -= block.data_start().byte_offset_from_unsigned(header_ptr) as u16;

        Some(data_ptr)
    }

    #[inline(never)]
    pub unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let mut block_ptr = ptr.cast::<BlockHeader>().sub(1);
        block_ptr.as_mut().size |= BlockHeader::FREE_BIT; // Mark block as free
    }
}

unsafe impl core::alloc::GlobalAlloc for MDSpecializeAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = super::with_cs::<1, 7, _>(|_| self.allocate(layout));

        ptr.map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr())
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        super::with_cs::<1, 7, _>(|_| self.deallocate(NonNull::new_unchecked(ptr), layout));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_ptr = NonNull::new_unchecked(ptr);
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());

        let new_ptr = super::with_cs::<1, 7, _>(|_| {
            let new_ptr = self.allocate(new_layout);

            if let Some(new_ptr) = new_ptr {
                new_ptr.copy_from_nonoverlapping(old_ptr, layout.size().min(new_size));
                self.deallocate(old_ptr, layout);
            }

            new_ptr
        });

        new_ptr.map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = super::with_cs::<1, 7, _>(|_| self.allocate(layout));

        if let Some(ptr) = ptr {
            ptr.write_bytes(0, layout.size());
        }

        ptr.map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr())
    }
}

#[repr(C)]
struct BlockHeader {
    size: u16,
}

impl BlockHeader {
    pub const FREE_BIT: u16 = 0x8000;

    #[inline]
    pub unsafe fn data_with_layout(&self, layout: Layout) -> NonNull<u8> {
        let ptr = self.data_end().byte_sub(layout.size());
        let align_diff = ptr.addr().get() & (layout.align() - 1);
        if align_diff != 0 {
            let align_offset = layout.align() - align_diff;
            ptr.byte_sub(align_offset)
        } else { ptr }
    }

    #[inline]
    pub unsafe fn satisfies_layout(&self, layout: Layout) -> bool {
        unsafe { self.data_start() <= self.data_with_layout(layout) }
    }

    #[inline]
    pub fn is_free(&self) -> bool {
        (self.size as i16) < 0
    }

    #[inline]
    pub fn size(&self) -> usize {
        (self.size << 1) as usize
    }

    #[inline]
    pub fn next(&self) -> Option<NonNull<BlockHeader>> {
        let next_ptr = self.data_end();
        if core::ptr::addr_eq(next_ptr.as_ptr() as *const _, &raw const _heap_end) {
            None
        } else {
            Some(next_ptr.cast())
        }
    }

    #[inline]
    pub fn data_start(&self) -> NonNull<u8> {
        unsafe { NonNull::new_unchecked((&raw const *self).add(1).cast::<u8>() as *mut u8) }
    }

    #[inline]
    pub fn data_end(&self) -> NonNull<u8> {
        unsafe { NonNull::new_unchecked((&raw const *self).add(1).byte_add(self.size()).cast::<u8>() as *mut u8) }
    }
}