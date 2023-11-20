use crate::{
    allocations_storage::{Allocation, CallStack},
    detour,
    state::{Slot, StateRef},
};

pub struct AllocationHandlerImpl {
    state: StateRef,
}

impl AllocationHandlerImpl {
    pub const fn new(state: StateRef) -> Self {
        Self { state }
    }
}

impl detour::AllocationHandler for AllocationHandlerImpl {
    fn on_allocation(&self, allocation: crate::detour::Allocation) {
        if let Some(_guard) = self.state.acquire_slot(Slot::Alloc) {
            if let Some(base_address) = allocation.allocated_base_address {
                let _configuration = self.state.get_configuration();
                // TODO: add stack trace

                self.state.get_storage().store(Allocation {
                    base_address,
                    size: allocation.size,
                    heap_handle: allocation.heap_handle,
                    call_stack: CallStack {},
                });
            }
        }
    }

    fn on_deallocation(&self, deallocation: crate::detour::Deallocation) {
        if let Some(_guard) = self.state.acquire_slot(Slot::Free) {
            if deallocation.success {
                let _configuration = self.state.get_configuration();
                // TODO: add stack trace

                self.state.get_storage().remove(deallocation.base_address);
            }
        }
    }
}
