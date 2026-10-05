use core::any::TypeId;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use crate::arena::Arena;

pub struct PersistentMemory<'m> {
    root_type: &'m mut Option<TypeId>,
    root: &'m mut [MaybeUninit<u8>],
    single_thread: PhantomData<*mut ()>,
}

impl<'m> PersistentMemory<'m> {
    #[must_use]
    pub fn new(bytes: &'m mut [MaybeUninit<u8>]) -> Self {
        let mut arena = Arena::new(bytes);
        let root_type = arena.push(None);
        Self {
            root_type,
            root: arena.into_free(),
            single_thread: PhantomData,
        }
    }

    pub fn reborrow(&mut self) -> PersistentMemory<'_> {
        PersistentMemory {
            root_type: &mut *self.root_type,
            root: &mut *self.root,
            single_thread: PhantomData,
        }
    }

    pub fn root<T: 'static>(self, init: impl FnOnce() -> T) -> &'m mut T {
        let slot = Arena::new(self.root).slot::<T>();
        let root_type = TypeId::of::<T>();
        match *self.root_type {
            None => {
                let root = slot.write(init());
                *self.root_type = Some(root_type);
                root
            }
            Some(initialized) => {
                assert!(
                    initialized == root_type,
                    "persistent memory root was initialized as a different type"
                );
                // SAFETY: `root_type` becomes `Some` only after a `T` was written to this slot.
                // The slot's position depends only on the private `root` bytes and `T`'s layout,
                // and nothing else writes to those bytes.
                unsafe { slot.assume_init_mut() }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::PersistentMemory;

    trait AmbiguousIfSend<Marker> {
        fn check() {}
    }

    impl<T: ?Sized> AmbiguousIfSend<()> for T {}

    impl<T: ?Sized + Send> AmbiguousIfSend<u8> for T {}

    const _: fn() = <PersistentMemory<'static> as AmbiguousIfSend<_>>::check;

    #[derive(Debug, PartialEq, Eq)]
    enum Mode {
        Title,
        Playing { level: u8 },
    }

    #[test]
    fn root_is_initialized_once_and_survives_reborrows() {
        let mut bytes = [MaybeUninit::uninit(); 64];
        let mut memory = PersistentMemory::new(&mut bytes);
        assert_eq!(*memory.reborrow().root(|| Mode::Title), Mode::Title);
        *memory.reborrow().root(|| Mode::Title) = Mode::Playing { level: 3 };
        let root = memory
            .reborrow()
            .root(|| -> Mode { panic!("root initialized twice") });
        assert_eq!(*root, Mode::Playing { level: 3 });
    }

    #[test]
    #[should_panic(expected = "initialized as a different type")]
    fn root_of_another_type_panics() {
        let mut bytes = [MaybeUninit::uninit(); 64];
        let mut memory = PersistentMemory::new(&mut bytes);
        memory.reborrow().root(|| 1_u32);
        memory.root(|| 1_i32);
    }

    #[test]
    #[should_panic(expected = "arena exhausted")]
    fn root_larger_than_the_memory_panics() {
        let mut bytes = [MaybeUninit::uninit(); 64];
        PersistentMemory::new(&mut bytes).root(|| [0_u8; 64]);
    }
}
