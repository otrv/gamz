#![no_std]

mod arena;
mod persistent;

pub use arena::Arena;
pub use persistent::PersistentMemory;

pub struct GameMemory<'m> {
    pub persistent: PersistentMemory<'m>,
    pub transient: Arena<'m>,
}
