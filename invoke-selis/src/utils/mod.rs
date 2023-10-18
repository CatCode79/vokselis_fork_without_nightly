use std::{
    collections::HashMap,
    hash::Hash,
    num::NonZeroU64,
    ops::{Deref, DerefMut},
};

pub(crate) mod frame_counter;
pub(crate) mod input;

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

#[derive(Debug, Clone, Copy)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
    pub unpadded_bytes_per_row: u32,
    pub padded_bytes_per_row: u32,
}

impl ImageDimensions {
    pub fn new(width: u32, height: u32, align: u32) -> Self {
        let height = height.saturating_sub(height % 2);
        let width = width.saturating_sub(width % 2);
        let bytes_per_pixel = std::mem::size_of::<[u8; 4]>() as u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }

    pub fn linear_size(&self) -> u64 {
        self.padded_bytes_per_row as u64 * self.height as u64
    }
}
