use crate::{
    allocations_storage::{Allocation, BackTrace, BackTraceFrame, BackTraceSymbol, StackTrace},
    detour,
    state::{Configuration, StateRef},
};

pub struct AllocationHandlerImpl {
    state: StateRef,
}

impl AllocationHandlerImpl {
    pub const fn new(state: StateRef) -> Self {
        Self { state }
    }
}

fn back_trace(skip: u32, count: u32, resolve_symbols_count: u32) -> BackTrace {
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
                    if s.len() < (resolve_symbols_count as usize) {
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

        bt.frames.len() < (count as usize)
    });

    bt
}

fn create_back_trace(configuration: &Configuration) -> Option<BackTrace> {
    if configuration.backtrace_frames_count > 0 {
        Some(back_trace(
            configuration.backtrace_frames_skip,
            configuration.backtrace_frames_count,
            configuration.backtrace_resolve_symbols_count,
        ))
    } else {
        None
    }
}

fn create_stack_trace(configuration: &Configuration) -> Option<StackTrace> {
    if configuration.stack_trace_size == 0 {
        return None;
    }

    // Find stack base address.
    let mut base = None;
    backtrace::trace(|frame| {
        base = Some(frame.sp() as usize);
        false
    });

    if let Some(base) = base {
        let address = base + (crate::wordsize() as usize) * configuration.stack_trace_offset;
        let slice = unsafe {
            core::slice::from_raw_parts(address as *const usize, configuration.stack_trace_size)
        };

        Some(StackTrace {
            base: address,
            trace: slice.to_vec(),
        })
    } else {
        None
    }
}

impl detour::AllocationHandler for AllocationHandlerImpl {
    fn on_allocation(&self, allocation: crate::detour::Allocation) {
        if let Some(base_address) = allocation.allocated_base_address {
            let configuration = self.state.get_configuration();
            let stack_trace = create_stack_trace(&configuration);
            let back_trace = create_back_trace(&configuration);

            self.state.lock_storage().store(Allocation {
                base_address,
                size: allocation.size,
                heap_handle: allocation.heap_handle,
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
