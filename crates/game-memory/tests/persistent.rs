use std::mem::MaybeUninit;
use std::panic::{self, AssertUnwindSafe};

use game_memory::PersistentMemory;

#[test]
fn panicking_init_leaves_the_root_uninitialized() {
    let mut bytes = [MaybeUninit::uninit(); 64];
    let mut memory = PersistentMemory::new(&mut bytes);
    let init = panic::catch_unwind(AssertUnwindSafe(|| {
        memory.reborrow().root(|| -> u32 { panic!("init failed") });
    }));
    assert!(init.is_err());
    assert_eq!(*memory.root(|| 5_u32), 5);
}
