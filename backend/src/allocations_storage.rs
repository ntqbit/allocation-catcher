use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included};

use crate::proto;

pub type Address = usize;
pub type HeapHandle = usize;

#[derive(Debug, Clone)]
pub struct CallStack {}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub base_address: Address,
    pub size: usize,
    pub heap_handle: HeapHandle,
    pub call_stack: CallStack,
}

impl From<&Allocation> for proto::Allocation {
    fn from(value: &Allocation) -> Self {
        Self {
            base_address: value.base_address as u64,
            size: value.size as u64,
            // TODO: complete
        }
    }
}

pub trait AllocationsStorage: Sync + Send {
    fn store(&mut self, allocation: Allocation);

    fn remove(&mut self, address: Address);

    fn find(&self, address: Address) -> Option<&Allocation>;

    fn find_range<'a>(
        &'a self,
        lower: Address,
        upper: Address,
    ) -> Box<dyn Iterator<Item = &Allocation> + 'a>;

    fn dump<'a>(&'a self) -> Box<dyn Iterator<Item = &Allocation> + 'a>;

    fn clear(&mut self);
}

pub struct StorageImpl {
    map: BTreeMap<Address, Allocation>,
}

impl StorageImpl {
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

impl AllocationsStorage for StorageImpl {
    fn store(&mut self, allocation: Allocation) {
        self.map.insert(allocation.base_address, allocation);
    }

    fn remove(&mut self, address: Address) {
        self.map.remove(&address);
    }

    fn find(&self, address: Address) -> Option<&Allocation> {
        self.map.get(&address)
    }

    fn find_range<'a>(
        &'a self,
        lower: Address,
        upper: Address,
    ) -> Box<dyn Iterator<Item = &Allocation> + 'a> {
        Box::new(
            self.map
                .range((Included(lower), Excluded(upper)))
                .map(|(_, k)| k),
        )
    }

    fn dump<'a>(&'a self) -> Box<dyn Iterator<Item = &Allocation> + 'a> {
        Box::new(self.map.iter().map(|(_, k)| k))
    }

    fn clear(&mut self) {
        self.map.clear();
    }
}
