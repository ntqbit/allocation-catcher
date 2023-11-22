use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included};

use common::proto;

pub type Address = usize;
pub type HeapHandle = usize;

#[derive(Debug, Clone)]
pub struct StackTrace {
    pub base: usize,
    pub trace: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct BackTraceSymbol {
    pub name: Option<String>,
    pub address: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct BackTraceFrame {
    pub instruction_pointer: usize,
    pub stack_pointer: usize,
    pub module_base: Option<usize>,
    pub resolved_symbols: Vec<BackTraceSymbol>,
}

#[derive(Debug, Clone)]
pub struct BackTrace {
    pub frames: Vec<BackTraceFrame>,
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub base_address: Address,
    pub size: usize,
    pub heap_handle: HeapHandle,
    pub stack_trace: Option<StackTrace>,
    pub back_trace: Option<BackTrace>,
}

impl From<&StackTrace> for proto::StackTrace {
    fn from(value: &StackTrace) -> Self {
        Self {
            stack_pointer: value.base as u64,
            wordsize: crate::wordsize(),
            trace: value.trace.iter().map(|&x| x as u64).collect(),
        }
    }
}

impl From<&BackTraceSymbol> for proto::BackTraceSymbol {
    fn from(value: &BackTraceSymbol) -> Self {
        Self {
            name: value.name.as_ref().map(|x| x.clone()),
            address: value.address.map(|x| x as u64),
        }
    }
}

impl From<&BackTraceFrame> for proto::BackTraceFrame {
    fn from(value: &BackTraceFrame) -> Self {
        Self {
            instruction_pointer: value.instruction_pointer as u64,
            stack_pointer: value.stack_pointer as u64,
            module_base: value.module_base.map(|x| x as u64),
            resolved_symbols: value.resolved_symbols.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<&BackTrace> for proto::BackTrace {
    fn from(value: &BackTrace) -> Self {
        Self {
            frames: value.frames.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<&Allocation> for proto::Allocation {
    fn from(value: &Allocation) -> Self {
        Self {
            base_address: value.base_address as u64,
            size: value.size as u64,
            heap_handle: value.heap_handle as u64,
            stack_trace: value.stack_trace.as_ref().map(|x| x.into()),
            back_trace: value.back_trace.as_ref().map(|x| x.into()),
        }
    }
}

pub trait AllocationsStorage: Sync + Send {
    fn store(&mut self, allocation: Allocation);

    fn remove(&mut self, address: Address) -> Result<(), ()>;

    fn find(&self, address: Address) -> Option<&Allocation>;

    fn find_range<'a>(
        &'a self,
        lower: Address,
        upper: Address,
    ) -> Box<dyn Iterator<Item = &Allocation> + 'a>;

    fn dump<'a>(&'a self) -> Box<dyn Iterator<Item = &Allocation> + 'a>;

    fn clear(&mut self);

    fn count(&self) -> usize;
}

pub struct BtreeMapStorage {
    map: BTreeMap<Address, Allocation>,
}

impl BtreeMapStorage {
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

impl AllocationsStorage for BtreeMapStorage {
    fn store(&mut self, allocation: Allocation) {
        self.map.insert(allocation.base_address, allocation);
    }

    fn remove(&mut self, address: Address) -> Result<(), ()> {
        if self.map.remove(&address).is_some() {
            Ok(())
        } else {
            Err(())
        }
    }

    fn find(&self, address: Address) -> Option<&Allocation> {
        self.map.get(&address)
    }

    fn find_range<'a>(
        &'a self,
        lower: Address,
        upper: Address,
    ) -> Box<dyn Iterator<Item = &Allocation> + 'a> {
        if lower > upper {
            Box::new(core::iter::empty())
        } else {
            Box::new(
                self.map
                    .range((Included(lower), Excluded(upper)))
                    .map(|(_, k)| k),
            )
        }
    }

    fn dump<'a>(&'a self) -> Box<dyn Iterator<Item = &Allocation> + 'a> {
        Box::new(self.map.iter().map(|(_, k)| k))
    }

    fn clear(&mut self) {
        self.map.clear();
    }

    fn count(&self) -> usize {
        self.map.len()
    }
}
