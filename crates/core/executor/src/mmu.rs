use std::mem::size_of;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use vec_map::VecMap;

use crate::events::MemoryRecord;

pub const LOG_PAGE_SIZE: usize = 12;
pub const LOG_BLOCK_LEN: usize = LOG_PAGE_SIZE - size_of::<Option<MemoryRecord>>().ilog2() as usize;
pub const BLOCK_LEN: usize = 1 << LOG_PAGE_SIZE;
pub const BLOCK_MASK: usize = BLOCK_LEN - 1;

pub const MAX_PAGE_COUNT: usize = 1 << (u32::BITS as usize - LOG_BLOCK_LEN);

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page(VecMap<MemoryRecord>);

impl Default for Page {
    fn default() -> Self {
        Self(VecMap::with_capacity(BLOCK_LEN))
    }
}

pub use vecmap_mmu::VecMapMmu;

pub mod vecmap_mmu {
    use std::mem::{replace, take};

    use serde::{Deserialize, Serialize};
    use vec_map::VecMap;

    use super::{Page, BLOCK_MASK, LOG_BLOCK_LEN, MAX_PAGE_COUNT};
    use crate::events::MemoryRecord;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct VecMapMmu {
        pub page_table: VecMap<Page>,
    }

    impl VecMapMmu {
        pub fn new() -> Self {
            Self { page_table: VecMap::with_capacity(MAX_PAGE_COUNT) }
        }

        pub fn get(&self, index: usize) -> Option<&MemoryRecord> {
            let (upper, lower) = Self::split_index(index);
            self.page_table.get(upper)?.0.get(lower)
        }

        pub fn get_mut(&mut self, index: usize) -> Option<&mut MemoryRecord> {
            let (upper, lower) = Self::split_index(index);
            self.page_table.get_mut(upper)?.0.get_mut(lower)
        }

        pub fn insert(&mut self, index: usize, value: MemoryRecord) -> Option<MemoryRecord> {
            let (upper, lower) = Self::split_index(index);
            self.page_table.entry(upper).or_insert_with(Page::default).0.insert(lower, value)
        }

        pub fn remove(&mut self, index: usize) -> Option<MemoryRecord> {
            let (upper, lower) = Self::split_index(index);
            self.page_table.get_mut(upper)?.0.remove(lower)
        }

        pub fn entry(&mut self, index: usize) -> Entry<'_> {
            let (upper, lower) = Self::split_index(index);
            let page_table_entry = self.page_table.entry(upper);
            if let vec_map::Entry::Occupied(occ_entry) = page_table_entry {
                if occ_entry.get().0.contains_key(lower) {
                    Entry::Occupied(OccupiedEntry { lower, page_table_occupied_entry: occ_entry })
                } else {
                    Entry::Vacant(VacantEntry {
                        index,
                        page_table_entry: vec_map::Entry::Occupied(occ_entry),
                    })
                }
            } else {
                Entry::Vacant(VacantEntry { index, page_table_entry })
            }
        }

        pub fn keys(&self) -> impl Iterator<Item = usize> + '_ {
            self.page_table
                .iter()
                .flat_map(|(upper, page)| page.0.iter().map(move |(lower, _)| upper + lower))
        }

        #[inline]
        fn split_index(index: usize) -> (usize, usize) {
            (index >> LOG_BLOCK_LEN, index & BLOCK_MASK)
        }
    }

    pub enum Entry<'a> {
        Vacant(VacantEntry<'a>),
        Occupied(OccupiedEntry<'a>),
    }

    pub struct VacantEntry<'a> {
        index: usize,
        page_table_entry: vec_map::Entry<'a, Page>,
    }

    impl<'a> VacantEntry<'a> {
        pub fn insert(self, value: MemoryRecord) -> &'a mut MemoryRecord {
            // By construction, the slot in the page is `None`.
            match self
                .page_table_entry
                .or_insert_with(Default::default)
                .0
                .entry(self.index & BLOCK_MASK)
            {
                vec_map::Entry::Vacant(entry) => entry.insert(value),
                vec_map::Entry::Occupied(entry) => {
                    panic!(
                        "entry at {} should be vacant, but found {:?}",
                        self.index,
                        entry.into_mut()
                    )
                }
            }
        }

        pub fn into_key(self) -> usize {
            self.index
        }

        pub fn key(&self) -> &usize {
            &self.index
        }
    }

    pub struct OccupiedEntry<'a> {
        lower: usize,
        page_table_occupied_entry: vec_map::OccupiedEntry<'a, Page>,
    }

    impl<'a> OccupiedEntry<'a> {
        pub fn get(&self) -> &MemoryRecord {
            self.page_table_occupied_entry.get().0.get(self.lower).unwrap()
        }

        pub fn get_mut(&mut self) -> &mut MemoryRecord {
            self.page_table_occupied_entry.get_mut().0.get_mut(self.lower).unwrap()
        }

        pub fn insert(&mut self, value: MemoryRecord) -> MemoryRecord {
            replace(self.get_mut(), value)
        }

        pub fn into_mut(self) -> &'a mut MemoryRecord {
            self.page_table_occupied_entry.into_mut().0.get_mut(self.lower).unwrap()
        }

        pub fn remove(mut self) -> MemoryRecord {
            self.page_table_occupied_entry.get_mut().0.remove(self.lower).unwrap()
        }
    }

    impl FromIterator<(usize, MemoryRecord)> for VecMapMmu {
        fn from_iter<T: IntoIterator<Item = (usize, MemoryRecord)>>(iter: T) -> Self {
            let mut mmu = Self::default();
            for (k, v) in iter {
                mmu.insert(k, v);
            }
            mmu
        }
    }
}
