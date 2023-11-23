use crate::{
    detour::{self, Base},
    state::StateRef,
    storage::{Allocation, BackTrace, BackTraceFrame, BackTraceSymbol, StackTrace},
    Configuration,
};

pub struct StorageAllocationHandler {
    state: StateRef,
}

impl StorageAllocationHandler {
    pub const fn new(state: StateRef) -> Self {
        Self { state }
    }
}

fn create_back_trace(skip: usize, count: usize, resolve_symbols_count: usize) -> Option<BackTrace> {
    if count == 0 {
        return None;
    }

    let mut bt = BackTrace {
        frames: Vec::with_capacity(20),
    };
    let mut cnt = 0;

    backtrace::trace(|frame| {
        if cnt >= skip {
            // Resolve symbols
            let resolved_symbols = if resolve_symbols_count > 0 {
                let mut s = Vec::with_capacity(3);

                backtrace::resolve_frame(frame, |symbol| {
                    if s.len() < resolve_symbols_count {
                        s.push(BackTraceSymbol {
                            name: symbol.name().and_then(|x| x.as_str().map(|y| y.to_owned())),
                            address: symbol.addr().map(|x| x as usize),
                        });
                    }
                });

                s
            } else {
                Vec::new()
            };

            bt.frames.push(BackTraceFrame {
                instruction_pointer: frame.ip() as usize,
                stack_pointer: frame.sp() as usize,
                module_base: frame.module_base_address().map(|x| x as usize),
                resolved_symbols,
            });
        }

        cnt += 1;

        bt.frames.len() < count
    });

    Some(bt)
}

fn create_stack_trace(address: usize, size: usize, offset: usize) -> Option<StackTrace> {
    if size == 0 {
        return None;
    }

    // FIXME: may crash if size of the stack trace requested is greater than the actual stack.
    // May happen during thread creation, when the stack is small enough.
    let address = address + (crate::wordsize() as usize) * offset;
    let slice = unsafe { core::slice::from_raw_parts(address as *const usize, size) };

    Some(StackTrace {
        base: address,
        trace: slice.to_vec(),
    })
}

fn creeate_stack_and_back_trace(
    base: &Base,
    configuration: &Configuration,
) -> (Option<StackTrace>, Option<BackTrace>) {
    let stack_trace = {
        let stack_base = base.address_of_return_address.or(base.stack_frame_address);

        if let Some(stack_base) = stack_base {
            assert_ne!(stack_base, 0);

            create_stack_trace(
                stack_base,
                configuration.stack_trace_size as usize,
                configuration.stack_trace_offset as usize,
            )
        } else {
            None
        }
    };

    let back_trace = create_back_trace(
        configuration.backtrace_frames_skip as usize,
        configuration.backtrace_frames_count as usize,
        configuration.backtrace_resolve_symbols_count as usize,
    );

    (stack_trace, back_trace)
}

impl detour::AllocationHandler for StorageAllocationHandler {
    fn on_allocation(&self, allocation: crate::detour::Allocation) {
        if let Some(base_address) = allocation.allocated_base_address {
            let configuration = self.state.get_configuration();
            let (stack_trace, back_trace) =
                creeate_stack_and_back_trace(&allocation.base, &configuration);

            self.state.lock_storage().store(Allocation {
                base_address,
                size: allocation.size,
                heap_handle: allocation.base.heap_handle,
                stack_trace,
                back_trace,
            });

            {
                // Update statistics
                let mut stats = self.state.lock_statistics();
                stats.total_allocations += 1;
            }
        }
    }

    fn on_reallocation(&self, reallocation: detour::Reallocation) {
        if let Some(base_address) = reallocation.allocation.allocated_base_address {
            let configuration = self.state.get_configuration();
            let (stack_trace, back_trace) =
                creeate_stack_and_back_trace(&reallocation.allocation.base, &configuration);

            {
                let mut storage = self.state.lock_storage();
                storage.remove(reallocation.base_address).ok();
                storage.store(Allocation {
                    base_address,
                    size: reallocation.allocation.size,
                    heap_handle: reallocation.allocation.base.heap_handle,
                    stack_trace,
                    back_trace,
                });
            }

            {
                // Update statistics
                let mut stats = self.state.lock_statistics();
                stats.total_reallocations += 1;
            }
        }
    }

    fn on_deallocation(&self, deallocation: crate::detour::Deallocation) {
        if deallocation.success {
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
