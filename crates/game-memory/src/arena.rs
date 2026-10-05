use core::mem::{self, MaybeUninit};
use core::slice;

pub struct Arena<'m> {
    free: &'m mut [MaybeUninit<u8>],
}

impl<'m> Arena<'m> {
    #[must_use]
    pub fn new(bytes: &'m mut [MaybeUninit<u8>]) -> Self {
        Self { free: bytes }
    }

    pub fn push<T>(&mut self, value: T) -> &'m mut T {
        self.slot().write(value)
    }

    pub fn push_slice<T: Copy>(&mut self, len: usize, value: T) -> &'m mut [T] {
        let size = mem::size_of::<T>()
            .checked_mul(len)
            .unwrap_or_else(|| panic!("arena slice of {len} elements overflows usize"));
        let bytes = self.take(size, mem::align_of::<T>());
        // SAFETY: `take` returned `size_of::<T>() * len` bytes aligned for `T` and borrowed
        // exclusively for 'm, and `MaybeUninit<T>` has no validity requirements.
        let slots: &mut [MaybeUninit<T>] =
            unsafe { slice::from_raw_parts_mut(bytes.as_mut_ptr().cast(), len) };
        for slot in &mut *slots {
            slot.write(value);
        }
        // SAFETY: the loop above initialized every element.
        unsafe { slots.assume_init_mut() }
    }

    pub fn temporary(&mut self) -> Arena<'_> {
        Arena {
            free: &mut *self.free,
        }
    }

    pub(crate) fn slot<T>(&mut self) -> &'m mut MaybeUninit<T> {
        const {
            assert!(
                !mem::needs_drop::<T>(),
                "arena values never run destructors"
            );
        }
        let bytes = self.take(mem::size_of::<T>(), mem::align_of::<T>());
        // SAFETY: `take` returned `size_of::<T>()` bytes aligned for `T` and borrowed exclusively
        // for 'm, and `MaybeUninit<T>` has no validity requirements.
        unsafe { &mut *bytes.as_mut_ptr().cast() }
    }

    pub(crate) fn into_free(self) -> &'m mut [MaybeUninit<u8>] {
        self.free
    }

    fn take(&mut self, size: usize, align: usize) -> &'m mut [MaybeUninit<u8>] {
        let free = mem::take(&mut self.free);
        let available = free.len();
        let padding = free.as_ptr().align_offset(align);
        let end = padding
            .checked_add(size)
            .filter(|&end| end <= available)
            .unwrap_or_else(|| {
                panic!("arena exhausted: {size} bytes requested, {available} bytes free")
            });
        let (taken, rest) = free.split_at_mut(end);
        self.free = rest;
        &mut taken[padding..]
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::Arena;

    #[test]
    fn push_aligns_and_keeps_values_apart() {
        let mut bytes = [MaybeUninit::uninit(); 64];
        let mut arena = Arena::new(&mut bytes);
        let byte = arena.push(7_u8);
        let word = arena.push(0x0102_0304_0506_0708_u64);
        let slice = arena.push_slice(3, 9_u16);
        *byte += 1;
        slice[1] = 10;
        assert_eq!(*byte, 8);
        assert_eq!(*word, 0x0102_0304_0506_0708);
        assert_eq!(slice, [9, 10, 9]);
        assert!(core::ptr::from_mut(word).is_aligned());
    }

    #[test]
    fn zero_sized_values_take_no_space() {
        let mut bytes: [MaybeUninit<u8>; 0] = [];
        let mut arena = Arena::new(&mut bytes);
        arena.push(());
        assert_eq!(arena.push_slice(1000, ()).len(), 1000);
    }

    #[test]
    fn temporary_space_is_reused_after_the_scope() {
        let mut bytes = [MaybeUninit::uninit(); 8];
        let mut arena = Arena::new(&mut bytes);
        {
            let mut scratch = arena.temporary();
            assert_eq!(scratch.push_slice(8, 1_u8), [1; 8]);
        }
        assert_eq!(arena.push_slice(8, 2_u8), [2; 8]);
    }

    #[test]
    #[should_panic(expected = "arena exhausted: 2 bytes requested, 1 bytes free")]
    fn exhaustion_panics() {
        let mut bytes = [MaybeUninit::uninit(); 1];
        Arena::new(&mut bytes).push_slice(2, 0_u8);
    }

    #[test]
    #[should_panic(expected = "overflows usize")]
    fn slice_size_overflow_panics() {
        let mut bytes = [MaybeUninit::uninit(); 1];
        Arena::new(&mut bytes).push_slice(usize::MAX, 0_u16);
    }
}
