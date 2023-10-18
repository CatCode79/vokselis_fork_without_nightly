pub(crate) mod frame_counter;
pub(crate) mod input;

use std::{
    collections::HashMap,
    hash::Hash,
    num::NonZeroU64,
    ops::{Deref, DerefMut},
};

pub fn dispatch_optimal(len: u32, subgroup_size: u32) -> u32 {
    let padded_size = (subgroup_size - len % subgroup_size) % subgroup_size;
    (len + padded_size) / subgroup_size
}

pub trait NonZeroSized: Sized {
    const SIZE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(std::mem::size_of::<Self>() as _) };
}
/// Holds invariants? Nah!
impl<T> NonZeroSized for T where T: Sized {}

/// A hash map with a [HashSet](std::collections::HashSet) to hold unique values
#[derive(Debug)]
pub(crate) struct ContiniousHashMap<K, V>(HashMap<K, Vec<V>>);

impl<K, V> Deref for ContiniousHashMap<K, V> {
    type Target = HashMap<K, Vec<V>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> DerefMut for ContiniousHashMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V> ContiniousHashMap<K, V> {
    /// Creates an empty [ContiniousHashMap]
    ///
    /// The hash map is initially created with a capacity of 0,
    /// so it will not allocate until it is first inserted into.
    #[allow(unused)]
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl<K: Eq + Hash, V> ContiniousHashMap<K, V> {
    /// Inserts a key-value pair into the map.
    ///
    /// If the mep already contain this key this method will add
    /// a value instead of rewriting an old value.
    #[allow(unused)]
    pub(crate) fn push_value(&mut self, key: K, value: V) {
        self.0.entry(key).or_insert_with(Vec::new).push(value);
    }
}

impl<K, V> Default for ContiniousHashMap<K, V> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}
