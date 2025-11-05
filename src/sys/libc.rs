

// /// This code is shamelessly copied from compiler-builtins 
// mod impls {
//     const WORD_SIZE: usize = core::mem::size_of::<usize>();
//     const WORD_MASK: usize = WORD_SIZE - 1;
//     const WORD_COPY_THRESHOLD: usize = 2 * WORD_SIZE;

//     /// Loads a `T`-sized chunk from `src` into `dst` at offset `offset`, if that does not exceed
//     /// `load_sz`. The offset pointers must both be `T`-aligned. Returns the new offset, advanced by the
//     /// chunk size if a load happened.
//     #[inline(always)]
//     unsafe fn load_chunk_aligned<T: Copy>(
//         src: *const usize,
//         dst: *mut usize,
//         load_sz: usize,
//         offset: usize,
//     ) -> usize {
//         let chunk_sz = core::mem::size_of::<T>();
//         if (load_sz & chunk_sz) != 0 {
//             *dst.wrapping_byte_add(offset).cast::<T>() = *src.wrapping_byte_add(offset).cast::<T>();
//             offset | chunk_sz
//         } else {
//             offset
//         }
//     }

//     /// Load `load_sz` many bytes from `src`, which must be usize-aligned. Acts as if we did a `usize`
//     /// read with the out-of-bounds part filled with 0s.
//     /// `load_sz` be strictly less than `WORD_SIZE`.
//     #[inline(always)]
//     unsafe fn load_aligned_partial(src: *const usize, load_sz: usize) -> usize {
//         debug_assert!(load_sz < WORD_SIZE);
//         // We can read up to 7 bytes here, which is enough for WORD_SIZE of 8
//         // (since `load_sz < WORD_SIZE`).
//         const { assert!(WORD_SIZE <= 8) };

//         let mut i = 0;
//         let mut out = 0usize;
//         // We load in decreasing order, so the pointers remain sufficiently aligned for the next step.
//         i = load_chunk_aligned::<u32>(src, &raw mut out, load_sz, i);
//         i = load_chunk_aligned::<u16>(src, &raw mut out, load_sz, i);
//         i = load_chunk_aligned::<u8>(src, &raw mut out, load_sz, i);
//         debug_assert!(i == load_sz);
//         out
//     }

//     /// Load `load_sz` many bytes from `src.wrapping_byte_add(WORD_SIZE - load_sz)`. `src` must be
//     /// `usize`-aligned. The bytes are returned as the *last* bytes of the return value, i.e., this acts
//     /// as if we had done a `usize` read from `src`, with the out-of-bounds part filled with 0s.
//     /// `load_sz` be strictly less than `WORD_SIZE`.
//     #[inline(always)]
//     unsafe fn load_aligned_end_partial(src: *const usize, load_sz: usize) -> usize {
//         debug_assert!(load_sz < WORD_SIZE);
//         // We can read up to 7 bytes here, which is enough for WORD_SIZE of 8
//         // (since `load_sz < WORD_SIZE`).
//         const { assert!(WORD_SIZE <= 8) };

//         let mut i = 0;
//         let mut out = 0usize;
//         // Obtain pointers pointing to the beginning of the range we want to load.
//         let src_shifted = src.wrapping_byte_add(WORD_SIZE - load_sz);
//         let out_shifted = (&raw mut out).wrapping_byte_add(WORD_SIZE - load_sz);
//         // We load in increasing order, so by the time we reach `u16` things are 2-aligned etc.
//         i = load_chunk_aligned::<u8>(src_shifted, out_shifted, load_sz, i);
//         i = load_chunk_aligned::<u16>(src_shifted, out_shifted, load_sz, i);
//         i = load_chunk_aligned::<u32>(src_shifted, out_shifted, load_sz, i);
//         debug_assert!(i == load_sz);
//         out
//     }

//     #[inline(always)]
//     pub unsafe fn copy_forward(mut dest: *mut u8, mut src: *const u8, mut n: usize) {
//         #[inline(always)]
//         unsafe fn copy_forward_bytes(mut dest: *mut u8, mut src: *const u8, n: usize) {
//             let dest_end = dest.wrapping_add(n);
//             while dest < dest_end {
//                 *dest = *src;
//                 dest = dest.wrapping_add(1);
//                 src = src.wrapping_add(1);
//             }
//         }

//         #[inline(always)]
//         unsafe fn copy_forward_aligned_words(dest: *mut u8, src: *const u8, n: usize) {
//             let mut dest_usize = dest as *mut usize;
//             let mut src_usize = src as *mut usize;
//             let dest_end = dest.wrapping_add(n) as *mut usize;

//             while dest_usize < dest_end {
//                 *dest_usize = *src_usize;
//                 dest_usize = dest_usize.wrapping_add(1);
//                 src_usize = src_usize.wrapping_add(1);
//             }
//         }

//         /// `n` is in units of bytes, but must be a multiple of the word size and must not be 0.
//         /// `src` *must not* be `usize`-aligned.
//         #[inline(always)]
//         unsafe fn copy_forward_misaligned_words(dest: *mut u8, src: *const u8, n: usize) {
//             debug_assert!(n > 0 && n % WORD_SIZE == 0);
//             debug_assert!(src.addr() % WORD_SIZE != 0);

//             let mut dest_usize = dest as *mut usize;
//             let dest_end = dest.wrapping_add(n) as *mut usize;

//             // Calculate the misalignment offset and shift needed to reassemble value.
//             // Since `src` is definitely not aligned, `offset` is in the range 1..WORD_SIZE.
//             let offset = src as usize & WORD_MASK;
//             let shift = offset * 8;

//             // Realign src
//             let mut src_aligned = src.wrapping_byte_sub(offset) as *mut usize;
//             let mut prev_word = load_aligned_end_partial(src_aligned, WORD_SIZE - offset);

//             while dest_usize.wrapping_add(1) < dest_end {
//                 src_aligned = src_aligned.wrapping_add(1);
//                 let cur_word = *src_aligned;
//                 let reassembled = if cfg!(target_endian = "little") {
//                     prev_word >> shift | cur_word << (WORD_SIZE * 8 - shift)
//                 } else {
//                     prev_word << shift | cur_word >> (WORD_SIZE * 8 - shift)
//                 };
//                 prev_word = cur_word;

//                 *dest_usize = reassembled;
//                 dest_usize = dest_usize.wrapping_add(1);
//             }

//             // There's one more element left to go, and we can't use the loop for that as on the `src` side,
//             // it is partially out-of-bounds.
//             src_aligned = src_aligned.wrapping_add(1);
//             let cur_word = load_aligned_partial(src_aligned, offset);
//             let reassembled = if cfg!(target_endian = "little") {
//                 prev_word >> shift | cur_word << (WORD_SIZE * 8 - shift)
//             } else {
//                 prev_word << shift | cur_word >> (WORD_SIZE * 8 - shift)
//             };
//             // prev_word does not matter any more

//             *dest_usize = reassembled;
//             // dest_usize does not matter any more
//         }

//         if n >= WORD_COPY_THRESHOLD {
//             // Align dest
//             // Because of n >= 2 * WORD_SIZE, dst_misalignment < n
//             let dest_misalignment = (dest as usize).wrapping_neg() & WORD_MASK;
//             copy_forward_bytes(dest, src, dest_misalignment);
//             dest = dest.wrapping_add(dest_misalignment);
//             src = src.wrapping_add(dest_misalignment);
//             n -= dest_misalignment;

//             let n_words = n & !WORD_MASK;
//             let src_misalignment = src as usize & WORD_MASK;
//             if core::hint::likely(src_misalignment == 0) {
//                 copy_forward_aligned_words(dest, src, n_words);
//             } else {
//                 copy_forward_misaligned_words(dest, src, n_words);
//             }
//             dest = dest.wrapping_add(n_words);
//             src = src.wrapping_add(n_words);
//             n -= n_words;
//         }
//         copy_forward_bytes(dest, src, n);
//     }

//     #[inline(always)]
//     pub unsafe fn copy_backward(dest: *mut u8, src: *const u8, mut n: usize) {
//         // The following backward copy helper functions uses the pointers past the end
//         // as their inputs instead of pointers to the start!
//         #[inline(always)]
//         unsafe fn copy_backward_bytes(mut dest: *mut u8, mut src: *const u8, n: usize) {
//             let dest_start = dest.wrapping_sub(n);
//             while dest_start < dest {
//                 dest = dest.wrapping_sub(1);
//                 src = src.wrapping_sub(1);
//                 *dest = *src;
//             }
//         }

//         #[inline(always)]
//         unsafe fn copy_backward_aligned_words(dest: *mut u8, src: *const u8, n: usize) {
//             let mut dest_usize = dest as *mut usize;
//             let mut src_usize = src as *mut usize;
//             let dest_start = dest.wrapping_sub(n) as *mut usize;

//             while dest_start < dest_usize {
//                 dest_usize = dest_usize.wrapping_sub(1);
//                 src_usize = src_usize.wrapping_sub(1);
//                 *dest_usize = *src_usize;
//             }
//         }

//         /// `n` is in units of bytes, but must be a multiple of the word size and must not be 0.
//         /// `src` *must not* be `usize`-aligned.
//         #[inline(always)]
//         unsafe fn copy_backward_misaligned_words(dest: *mut u8, src: *const u8, n: usize) {
//             debug_assert!(n > 0 && n % WORD_SIZE == 0);
//             debug_assert!(src.addr() % WORD_SIZE != 0);

//             let mut dest_usize = dest as *mut usize;
//             let dest_start = dest.wrapping_sub(n) as *mut usize; // we're moving towards the start

//             // Calculate the misalignment offset and shift needed to reassemble value.
//             // Since `src` is definitely not aligned, `offset` is in the range 1..WORD_SIZE.
//             let offset = src as usize & WORD_MASK;
//             let shift = offset * 8;

//             // Realign src
//             let mut src_aligned = src.wrapping_byte_sub(offset) as *mut usize;
//             let mut prev_word = load_aligned_partial(src_aligned, offset);

//             while dest_start.wrapping_add(1) < dest_usize {
//                 src_aligned = src_aligned.wrapping_sub(1);
//                 let cur_word = *src_aligned;
//                 let reassembled = if cfg!(target_endian = "little") {
//                     prev_word << (WORD_SIZE * 8 - shift) | cur_word >> shift
//                 } else {
//                     prev_word >> (WORD_SIZE * 8 - shift) | cur_word << shift
//                 };
//                 prev_word = cur_word;

//                 dest_usize = dest_usize.wrapping_sub(1);
//                 *dest_usize = reassembled;
//             }

//             // There's one more element left to go, and we can't use the loop for that as on the `src` side,
//             // it is partially out-of-bounds.
//             src_aligned = src_aligned.wrapping_sub(1);
//             let cur_word = load_aligned_end_partial(src_aligned, WORD_SIZE - offset);
//             let reassembled = if cfg!(target_endian = "little") {
//                 prev_word << (WORD_SIZE * 8 - shift) | cur_word >> shift
//             } else {
//                 prev_word >> (WORD_SIZE * 8 - shift) | cur_word << shift
//             };
//             // prev_word does not matter any more

//             dest_usize = dest_usize.wrapping_sub(1);
//             *dest_usize = reassembled;
//         }

//         let mut dest = dest.wrapping_add(n);
//         let mut src = src.wrapping_add(n);

//         if n >= WORD_COPY_THRESHOLD {
//             // Align dest
//             // Because of n >= 2 * WORD_SIZE, dst_misalignment < n
//             let dest_misalignment = dest as usize & WORD_MASK;
//             copy_backward_bytes(dest, src, dest_misalignment);
//             dest = dest.wrapping_sub(dest_misalignment);
//             src = src.wrapping_sub(dest_misalignment);
//             n -= dest_misalignment;

//             let n_words = n & !WORD_MASK;
//             let src_misalignment = src as usize & WORD_MASK;
//             if core::hint::likely(src_misalignment == 0) {
//                 copy_backward_aligned_words(dest, src, n_words);
//             } else {
//                 copy_backward_misaligned_words(dest, src, n_words);
//             }
//             dest = dest.wrapping_sub(n_words);
//             src = src.wrapping_sub(n_words);
//             n -= n_words;
//         }
//         copy_backward_bytes(dest, src, n);
//     }

//     #[inline(always)]
//     pub unsafe fn set_bytes(mut s: *mut u8, c: u8, mut n: usize) {
//         #[inline(always)]
//         pub unsafe fn set_bytes_bytes(mut s: *mut u8, c: u8, n: usize) {
//             let end = s.wrapping_add(n);
//             while s < end {
//                 *s = c;
//                 s = s.wrapping_add(1);
//             }
//         }

//         #[inline(always)]
//         pub unsafe fn set_bytes_words(s: *mut u8, c: u8, n: usize) {
//             let mut broadcast = c as usize;
//             let mut bits = 8;
//             while bits < WORD_SIZE * 8 {
//                 broadcast |= broadcast << bits;
//                 bits <<= 1;
//             }

//             let mut s_usize = s as *mut usize;
//             let end = s.wrapping_add(n) as *mut usize;

//             while s_usize < end {
//                 *s_usize = broadcast;
//                 s_usize = s_usize.wrapping_add(1);
//             }
//         }

//         if core::hint::likely(n >= WORD_COPY_THRESHOLD) {
//             // Align s
//             // Because of n >= 2 * WORD_SIZE, dst_misalignment < n
//             let misalignment = (s as usize).wrapping_neg() & WORD_MASK;
//             set_bytes_bytes(s, c, misalignment);
//             s = s.wrapping_add(misalignment);
//             n -= misalignment;

//             let n_words = n & !WORD_MASK;
//             set_bytes_words(s, c, n_words);
//             s = s.wrapping_add(n_words);
//             n -= n_words;
//         }
//         set_bytes_bytes(s, c, n);
//     }

//     #[inline(always)]
//     pub unsafe fn compare_bytes(s1: *const u8, s2: *const u8, n: usize) -> u8 {
//         let mut i = 0;
//         while i < n {
//             let a = *s1.wrapping_add(i);
//             let b = *s2.wrapping_add(i);
//             if a != b {
//                 return a - b;
//             }
//             i += 1;
//         }
//         0
//     }

//     #[inline(always)]
//     pub unsafe fn c_string_length(mut s: *const core::ffi::c_char) -> usize {
//         let mut n = 0;
//         while *s != 0 {
//             n += 1;
//             s = s.wrapping_add(1);
//         }
//         n
//     }
// }

// #[no_mangle]
// pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
//     impls::copy_forward(dest, src, n);
//     dest
// }

// #[no_mangle]
// pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
//     if (dest as usize).wrapping_sub(src as usize) >= n {
//         impls::copy_forward(dest, src, n);
//     } else {
//         impls::copy_backward(dest, src, n);
//     }
//     dest
// }

// #[no_mangle]
// pub unsafe extern "C" fn memset(s: *mut u8, c: core::ffi::c_int, n: usize) -> *mut u8 {
//     impls::set_bytes(s, c as u8, n);
//     s
// }

// #[no_mangle]
// pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> core::ffi::c_int {
//     impls::compare_bytes(s1, s2, n) as core::ffi::c_int
// }

// #[no_mangle]
// pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> core::ffi::c_int {
//     memcmp(s1, s2, n)
// }

// #[no_mangle]
// pub unsafe extern "C" fn strlen(s: *const core::ffi::c_char) -> usize {
//     impls::c_string_length(s)
// }

// #[no_mangle]
// pub unsafe extern "C" fn abort() -> ! {
//     extern "C" {
//         fn _trap() -> !; // Hook into header trap
//     }
//     _trap()
// }

// #[no_mangle]
// pub unsafe extern "C" fn __mulsi3(a: u32, b: u32) -> u32 {
//     if a == 0 || b == 0 {
//         return 0;
//     }

//     let al = a as u16;
//     let ah = ((a as u32) >> 16) as u16;
//     let bl = b as u16;
//     let bh = ((b as u32) >> 16) as u16;
    
//     let (v0, c0) = al.widening_mul(bl);
//     let v1 = ah.wrapping_mul(bl);
//     let v2 = al.wrapping_mul(bh);

//     (
//         v0 as u32
//     ).wrapping_add(
//         (
//             v1 as u32
//         ).wrapping_add(
//             v2 as u32
//         ).wrapping_add(
//             c0 as u32
//         ) << 16
//     )
// }