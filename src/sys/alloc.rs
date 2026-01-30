use core::{alloc::Layout, ptr::NonNull};
use super::prelude::*;

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
/// As a result, block headers are tiny; only two words!
pub struct MDSpecAlloc;

static mut ALLOC_HEAD: FreeBlock = FreeBlock::new(0);

impl MDSpecAlloc {

    #[inline]
    fn head<'cs>(_cs: cs::CriticalSection<'cs>) -> &'cs mut FreeBlock {
        #[allow(static_mut_refs)]
        unsafe { &mut ALLOC_HEAD }
    }

    #[inline]
    pub fn init() {
        with_cs(|cs| {
            Self::init_inner(Self::head(cs));
        });
    }

    #[inline]
    fn init_inner(head: &mut FreeBlock) {
        unsafe {
            Self::add_free_block(head, &raw mut _heap_start, heap_size());
        }
    }

    #[inline(never)]
    unsafe fn add_free_block(head: &mut FreeBlock, ptr: *mut u8, size: usize) {
        let mut current = head;
        while current.next_ref().is_some_and(|next| next.start() < ptr) {
            current = current.next_mut().unwrap_unchecked();
        }

        let block = FreeBlock::new(size);
        let mut block_ptr = NonNull::new_unchecked(ptr.cast::<FreeBlock>());
        block_ptr.write(block);
        current.insert_next(block_ptr);
        block_ptr.as_mut().try_merge_next();
        current.try_merge_next();
    }

    #[inline(never)]
    unsafe fn find_block(head: &mut FreeBlock, layout: Layout) -> Option<(NonNull<FreeBlock>, *mut u8)> {
        let mut current = head;
        while let Some(block) = current.next_mut() {
            if let Some(offset) = Self::alloc_from_block(block, layout) {
                return Some((current.remove_next().unwrap_unchecked(), offset));
            } else {
                current = current.next_mut().unwrap_unchecked();
            }
        }

        None
    }

    #[inline]
    fn align_ptr_up(ptr: *mut u8, align: usize) -> *mut u8 {
        let mask = align - 1;
        ptr.map_addr(|addr| (addr + mask) & !mask)
    }

    #[inline]
    unsafe fn alloc_from_block(block: &FreeBlock, layout: Layout) -> Option<*mut u8> {
        let alloc_start = Self::align_ptr_up(block.start(), layout.align());
        let alloc_end = alloc_start.byte_add(layout.size());

        if alloc_end > block.end() {
            return None;
        }

        let excess = block.end().offset_from_unsigned(alloc_end);
        if excess > 0 && excess < size_of::<FreeBlock>() {
            return None;
        }

        Some(alloc_start)
    }

    #[inline]
    fn adjust_layout(layout: Layout) -> Layout {
        let new_layout = layout
            .align_to(mem::align_of::<FreeBlock>())
            .expect("Could not align layout to align of BlockNode")
            .pad_to_align();
        unsafe { 
            Layout::from_size_align_unchecked(
                new_layout.size().max(size_of::<FreeBlock>()), 
                new_layout.align()
            ) 
        }
    }
    

    #[inline(never)]
    unsafe fn allocate(head: &mut FreeBlock, layout: Layout) -> Option<NonNull<u8>> {
        let layout = Self::adjust_layout(layout);

        if let Some((mut block, alloc_start)) = Self::find_block(head, layout) {
            let alloc_end = alloc_start.byte_add(layout.size());
            let excess = block.as_mut().end().offset_from_unsigned(alloc_end);
            if excess > 0 {
                Self::add_free_block(head, alloc_end, excess);
            }
            Some(NonNull::new_unchecked(alloc_start))
        } else { None }
    }

    #[inline(never)]
    unsafe fn deallocate(head: &mut FreeBlock, ptr: NonNull<u8>, layout: Layout) {
        let layout = Self::adjust_layout(layout);

        Self::add_free_block(head, ptr.as_ptr(), layout.size());
    }
}

unsafe impl core::alloc::GlobalAlloc for MDSpecAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = with_cs(|cs| Self::allocate(Self::head(cs), layout));

        ptr.map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr())
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        with_cs(|cs| Self::deallocate(Self::head(cs), NonNull::new_unchecked(ptr), layout));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_ptr = NonNull::new_unchecked(ptr);
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());

        let new_ptr = with_cs(|cs| {
            let new_ptr = Self::allocate(Self::head(cs), new_layout);

            if let Some(new_ptr) = new_ptr {
                new_ptr.copy_from_nonoverlapping(old_ptr, layout.size().min(new_size));
                Self::deallocate(Self::head(cs), old_ptr, layout);
            }

            new_ptr
        });

        new_ptr.map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = with_cs(|cs| Self::allocate(Self::head(cs), layout));

        if let Some(ptr) = ptr {
            ptr.write_bytes(0, layout.size());
        }

        ptr.map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr())
    }
}

#[repr(C)]
struct FreeBlock {
    pub size: usize,
    pub next: Option<NonNull<FreeBlock>>,
}

impl FreeBlock {
    pub const fn new(size: usize) -> Self {
        Self { size, next: None }
    }

    #[inline]
    pub unsafe fn insert_next(&mut self, mut block: NonNull<FreeBlock>) {
        let old_next = self.next.replace(block);
        block.as_mut().next = old_next
    }

    #[inline]
    pub unsafe fn try_merge_next(&mut self) -> bool {
        if self.size > 0 && self.next_ref().is_some_and(|next| self.end() == next.start()) {
            self.size += self.next_ref().map(|next| next.size).unwrap_unchecked();
            self.remove_next();
            true
        } else { false }
    }

    #[inline(never)]
    pub unsafe fn remove_next(&mut self) -> Option<NonNull<FreeBlock>> {
        let old_next = self.next.take();
        if let Some(mut old_next) = old_next {
            self.next = old_next.as_mut().next.take();
            Some(old_next)
        } else { None }
    }

    #[inline(always)]
    pub fn next_ref(&self) -> Option<&'static FreeBlock> {
        unsafe { self.next.map(|next| next.as_ref()) }
    }

    #[inline(always)]
    pub fn next_mut(&self) -> Option<&'static mut FreeBlock> {
        unsafe { self.next.map(|mut next| next.as_mut()) }
    }

    #[inline(always)]
    pub fn start(&self) -> *mut u8 {
        self as *const Self as *mut u8
    }

    #[inline(always)]
    pub fn end(&self) -> *mut u8 {
        unsafe { self.start().byte_add(self.size) }
    }

    
}