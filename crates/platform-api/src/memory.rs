use core::mem::MaybeUninit;

pub struct PersistentMemory<'a> {
    pub bytes: &'a mut [MaybeUninit<u8>],
}

pub struct TransientMemory<'a> {
    pub bytes: &'a mut [MaybeUninit<u8>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRange {
    pub offset: usize,
    pub len: usize,
}
