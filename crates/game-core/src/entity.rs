use core::fmt;
use core::marker::PhantomData;
use core::num::NonZeroU64;

pub struct EntityId<T> {
    index: usize,
    generation: NonZeroU64,
    entity: PhantomData<fn() -> T>,
}

impl<T> Copy for EntityId<T> {}

impl<T> Clone for EntityId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for EntityId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for EntityId<T> {}

impl<T> fmt::Debug for EntityId<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EntityId")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

enum Slot<T> {
    Vacant { next: usize },
    Occupied { generation: NonZeroU64, value: T },
}

pub struct EntityStorage<T: Copy, const CAPACITY: usize> {
    slots: [Slot<T>; CAPACITY],
    first_free: usize,
    len: usize,
    next_generation: NonZeroU64,
}

impl<T: Copy, const CAPACITY: usize> Default for EntityStorage<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const CAPACITY: usize> EntityStorage<T, CAPACITY> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|index| Slot::Vacant { next: index + 1 }),
            first_free: 0,
            len: 0,
            next_generation: NonZeroU64::MIN,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, value: T) -> Result<EntityId<T>, T> {
        if self.first_free == CAPACITY {
            return Err(value);
        }
        let Slot::Vacant { next } = self.slots[self.first_free] else {
            panic!("entity free list points to an occupied slot");
        };
        let generation = self.next_generation;
        let next_generation = generation
            .checked_add(1)
            .expect("entity generations exhausted");
        let id = EntityId {
            index: self.first_free,
            generation,
            entity: PhantomData,
        };
        self.slots[id.index] = Slot::Occupied { generation, value };
        self.first_free = next;
        self.len += 1;
        self.next_generation = next_generation;
        Ok(id)
    }

    #[must_use]
    pub fn get(&self, id: EntityId<T>) -> Option<&T> {
        match self.slots.get(id.index)? {
            Slot::Occupied { generation, value } if *generation == id.generation => Some(value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: EntityId<T>) -> Option<&mut T> {
        match self.slots.get_mut(id.index)? {
            Slot::Occupied { generation, value } if *generation == id.generation => Some(value),
            _ => None,
        }
    }

    pub fn remove(&mut self, id: EntityId<T>) -> Option<T> {
        let value = *self.get(id)?;
        self.slots[id.index] = Slot::Vacant {
            next: self.first_free,
        };
        self.first_free = id.index;
        self.len -= 1;
        Some(value)
    }

    pub fn clear(&mut self) {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            *slot = Slot::Vacant { next: index + 1 };
        }
        self.first_free = 0;
        self.len = 0;
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            if let Slot::Occupied { generation, value } = slot {
                Some((
                    EntityId {
                        index,
                        generation: *generation,
                        entity: PhantomData,
                    },
                    value,
                ))
            } else {
                None
            }
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId<T>, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                if let Slot::Occupied { generation, value } = slot {
                    Some((
                        EntityId {
                            index,
                            generation: *generation,
                            entity: PhantomData,
                        },
                        value,
                    ))
                } else {
                    None
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::EntityStorage;

    #[test]
    fn full_storage_returns_the_value_and_reuse_rejects_old_handles() {
        let mut entities = EntityStorage::<_, 1>::new();
        let old = entities.insert(17).unwrap();
        assert_eq!(entities.insert(29), Err(29));
        assert_eq!(entities.get(old), Some(&17));
        assert_eq!(entities.len(), 1);
        assert_eq!(entities.remove(old), Some(17));
        assert!(entities.is_empty());
        assert_eq!(entities.get(old), None);
        assert_eq!(entities.remove(old), None);
        let new = entities.insert(31).unwrap();
        assert_ne!(old, new);
        assert_eq!(entities.get_mut(old), None);
        assert_eq!(entities.remove(old), None);
        *entities.get_mut(new).unwrap() = 43;
        assert_eq!(entities.get(new), Some(&43));
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn iteration_visits_survivors_past_holes_and_mutates_in_place() {
        let mut entities = EntityStorage::<_, 4>::new();
        let removed_first = entities.insert(3).unwrap();
        let survivor_first = entities.insert(11).unwrap();
        let removed_middle = entities.insert(23).unwrap();
        let survivor_last = entities.insert(47).unwrap();
        assert_eq!(entities.remove(removed_first), Some(3));
        assert_eq!(entities.remove(removed_middle), Some(23));
        let mut visits = entities.iter();
        assert_eq!(visits.next(), Some((survivor_first, &11)));
        assert_eq!(visits.next(), Some((survivor_last, &47)));
        assert_eq!(visits.next(), None);
        drop(visits);
        for (id, value) in entities.iter_mut() {
            *value += if id == survivor_first { 2 } else { 5 };
        }
        assert_eq!(entities.get(survivor_first), Some(&13));
        assert_eq!(entities.get(survivor_last), Some(&52));
        assert_eq!(entities.len(), 2);
        let replacement_first = entities.insert(61).unwrap();
        let replacement_second = entities.insert(79).unwrap();
        assert_eq!(entities.insert(97), Err(97));
        assert_eq!(entities.get(replacement_first), Some(&61));
        assert_eq!(entities.get(replacement_second), Some(&79));
        assert_eq!(entities.get(removed_first), None);
        assert_eq!(entities.get(removed_middle), None);
        assert_eq!(entities.iter().map(|(_, value)| value).sum::<i32>(), 205);
    }

    #[test]
    fn clear_invalidates_handles_and_recovers_every_slot() {
        let mut entities = EntityStorage::<_, 3>::new();
        let old = core::array::from_fn::<_, 3, _>(|n| entities.insert(n).unwrap());
        assert_eq!(entities.remove(old[1]), Some(1));
        entities.clear();
        entities.clear();
        assert!(entities.is_empty());
        assert_eq!(entities.iter().count(), 0);
        let new = core::array::from_fn::<_, 3, _>(|n| entities.insert(n + 10).unwrap());
        for id in old {
            assert_eq!(entities.get(id), None);
            assert_eq!(entities.get_mut(id), None);
            assert_eq!(entities.remove(id), None);
        }
        for (n, id) in new.into_iter().enumerate() {
            assert_eq!(entities.get(id), Some(&(n + 10)));
        }
        assert_eq!(entities.insert(99), Err(99));
        assert_eq!(entities.len(), 3);
    }

    #[test]
    fn zero_capacity_is_always_full_and_empty() {
        let mut entities = EntityStorage::<_, 0>::new();
        assert_eq!(entities.insert(7), Err(7));
        entities.clear();
        assert!(entities.is_empty());
        assert_eq!(entities.iter().count(), 0);
        assert_eq!(entities.iter_mut().count(), 0);
    }
}
