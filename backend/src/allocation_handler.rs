use std::sync::{Arc, Mutex};

use crate::{
    allocations_storage::{Allocation, AllocationsStorage, CallStack},
    detour,
};

pub struct AllocationHandlerImpl {
    storage: Arc<Mutex<dyn AllocationsStorage>>,
}

impl AllocationHandlerImpl {
    pub const fn new(storage: Arc<Mutex<dyn AllocationsStorage>>) -> Self {
        Self { storage }
    }
}

impl detour::AllocationHandler for AllocationHandlerImpl {
    fn on_allocation(&self, allocation: crate::detour::Allocation) {
        if let Some(base_address) = allocation.allocated_base_address {
            self.storage.lock().unwrap().store(Allocation {
                base_address,
                size: allocation.size,
                heap_handle: allocation.heap_handle,
                call_stack: CallStack {},
            });
        }
    }

    fn on_deallocation(&self, deallocation: crate::detour::Deallocation) {
        if deallocation.success {
            self.storage
                .lock()
                .unwrap()
                .remove(deallocation.base_address);
        }
    }
}
