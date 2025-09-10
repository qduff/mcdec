use std::{
    fmt::Debug,
    ops::{Bound, Range, RangeBounds},
};


/// `SparseMap` provides a bidirectional mapping between a "sparse" address (the key)
/// and a "dense" index (its position in the sorted set). 
#[derive(Clone, PartialEq, Eq)]
pub struct SparseMap {
    data: Vec<usize>,
}

impl Debug for SparseMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SparseMap {{ len: {}, range: {:?}..={:?} }}",
            self.data.len(),
            self.data.first().unwrap_or(&0),
            self.data.last().unwrap_or(&0)
        )
    }
}

impl SparseMap {
    /// Creates a new `SparseMap` from a vector of keys.
    /// The keys will be sorted and duplicates will be removed.
    pub fn new(mut keys: Vec<usize>) -> Self {
        keys.sort_unstable();
        keys.dedup();
        Self { data: keys }
    }

    /// Creates a new `SparseMap` from a vector of keys that are already
    /// sorted and unique. This is faster as it avoids the sorting and deduplication steps.
    pub fn new_presorted(sorted_unique_keys: Vec<usize>) -> Self {
        Self {
            data: sorted_unique_keys,
        }
    }

    /// Returns the sparse address for a given dense index.
    pub fn get_sparse_address(&self, dense_index: usize) -> Option<usize> {
        self.data.get(dense_index).copied()
    }

    /// Returns the dense index for a given sparse address.
    pub fn get_dense_index(&self, sparse_key: usize) -> Option<usize> {
        self.data.binary_search(&sparse_key).ok()
    }

    /// Returns the last sparse address in the map.
    pub fn last_address(&self) -> Option<usize> {
        self.data.last().copied()
    }

    /// Returns the first sparse address in the map.
    pub fn first_address(&self) -> Option<usize> {
        self.data.first().copied()
    }

    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a slice containing all the sparse addresses.
    pub fn as_slice(&self) -> &[usize] {
        &self.data
    }

    /// Creates a new `SparseMap` by cloning a sub-slice of this map's data.
    /// The range is for the dense indices.
    fn clone_from_slice(&self, range: Range<usize>) -> Self {
        Self {
            data: self.data[range].to_vec(),
        }
    }
}

/// A container for items (like code instructions) that have non-contiguous addresses.
///
/// It uses a `SparseMap` to map the dense `Vec` index (0, 1, 2...) to a
/// sparse address (e.g., 0x100, 0x104, 0x105...).
///
/// Slicing this container creates a new, independent `AddressedContainer` by cloning
/// the underlying items and their corresponding address mappings.
#[derive(Clone, PartialEq, Eq)]
pub struct AddressedContainer<T> {
    items: Vec<T>,
    mapping: SparseMap,
}

impl<T: Debug> Debug for AddressedContainer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddressedContainer")
            .field("len", &self.items.len())
            .field("address_range", &(self.start_addr(), self.end_addr()))
            .finish()
    }
}

impl<T> AddressedContainer<T> {
    /// Creates a new `AddressedContainer` from items and their corresponding address mapping.
    pub fn new(items: Vec<T>, mapping: SparseMap) -> Self {
        assert_eq!(
            items.len(),
            mapping.len(),
            "The number of items must match the number of addresses in the mapping."
        );
        Self { items, mapping }
    }
    
    /// Returns an iterator that yields the address and a reference to each item.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.items
            .iter()
            .enumerate()
            .map(move |(index, item_ref)| {
                // unwrap is safe as same len!!!
                (self.mapping.get_sparse_address(index).unwrap(), item_ref)
            })
    }
    
    /// Creates a new `AddressedContainer` by CLONING a subslice with range being
    ///  the dense **index**, not the sparse address.
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> Option<Self>
    where
        T: Clone,
    {
        let start = match range.start_bound() {
            Bound::Included(&s) => s,
            Bound::Excluded(&s) => s + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&e) => e + 1,
            Bound::Excluded(&e) => e,
            Bound::Unbounded => self.items.len(),
        };

        if start > end || end > self.items.len() {
            return None;
        }

        let new_items = self.items[start..end].to_vec();
        let new_mapping = self.mapping.clone_from_slice(start..end);

        Some(Self {
            items: new_items,
            mapping: new_mapping,
        })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn start_addr(&self) -> Option<usize> {
        self.mapping.first_address()
    }

    pub fn end_addr(&self) -> Option<usize> {
        self.mapping.last_address()
    }

    /// Returns a reference to the item at the given sparse `addr`.
    pub fn item_at_address(&self, addr: usize) -> Option<&T> {
        let index = self.mapping.get_dense_index(addr)?;
        self.items.get(index)
    }

    /// Returns a reference to the item at the given dense `index`.
    pub fn item_at_idx(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// Converts a sparse `addr` to its corresponding dense `index`.
    pub fn addr_to_idx(&self, addr: usize) -> Option<usize> {
        self.mapping.get_dense_index(addr)
    }

    /// Converts a dense `index` to its corresponding sparse `addr`.
    pub fn idx_to_addr(&self, index: usize) -> Option<usize> {
        self.mapping.get_sparse_address(index)
    }
    
    /// Returns the sparse address of the item that is `offset` positions away from
    /// the item at the given `addr`. A negative offset moves backwards.
    pub fn addr_offset_by_idx(&self, addr: usize, offset: isize) -> Option<usize> {
        let current_index = self.addr_to_idx(addr)? as isize;
        let target_index = current_index.checked_add(offset)?;
        if target_index < 0 {
            return None;
        }
        self.idx_to_addr(target_index as usize)
    }

    pub fn last(&self) -> Option<&T> {
        self.items.last()
    }
}
