#![no_std]

mod arena;

pub use arena::Arena;

pub struct GameMemory<'m> {
    pub transient: Arena<'m>,
}
