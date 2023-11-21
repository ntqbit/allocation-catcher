use crate::{
    allocations_storage::{Allocation, CallStack},
    detour,
    state::StateRef,
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
        if let Some(base_address) = allocation.allocated_base_address {
            let _configuration = self.state.get_configuration();
            // TODO: add stack trace

            self.state.lock_storage().store(Allocation {
                base_address,
                size: allocation.size,
                heap_handle: allocation.heap_handle,
                call_stack: CallStack {},
            });

            {
                // Update statistics
                let mut stats = self.state.lock_statistics();
                stats.total_allocations += 1;
            }
        }
    }

    fn on_deallocation(&self, deallocation: crate::detour::Deallocation) {
        if deallocation.success {
            let _configuration = self.state.get_configuration();
            // TODO: add stack trace

            let removed = self
                .state
                .lock_storage()
                .remove(deallocation.base_address)
                .is_ok();

            {
                // Update statistics
                let mut stats = self.state.lock_statistics();
                stats.total_deallocations += 1;
                if !removed {
                    stats.total_deallocations_non_allocated += 1;
                }
            }
        }
    }
}
