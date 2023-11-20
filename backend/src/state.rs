use std::sync::{Mutex, MutexGuard};

use crate::{allocations_storage::AllocationsStorage, platform::TlsSlotAcquisition, proto};

#[derive(Debug, Clone)]
pub struct Configuration {
    pub stack_trace_offset: usize,
    pub stack_trace_size: usize,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            stack_trace_offset: 0x0,
            stack_trace_size: 0x10,
        }
    }
}

impl From<proto::Configuration> for Configuration {
    fn from(value: proto::Configuration) -> Self {
        Self {
            stack_trace_offset: value.stack_trace_offset as usize,
            stack_trace_size: value.stack_trace_size as usize,
        }
    }
}

impl From<Configuration> for proto::Configuration {
    fn from(value: Configuration) -> Self {
        Self {
            stack_trace_offset: value.stack_trace_offset as u64,
            stack_trace_size: value.stack_trace_size as u64,
        }
    }
}

pub type StateRef = &'static State;

#[repr(usize)]
pub enum Slot {
    Alloc = 0,
    Free = 1,
}

pub struct State {
    configuration: Mutex<Configuration>,
    storage: Mutex<Box<dyn AllocationsStorage>>,
    tls_slot_acquisition: TlsSlotAcquisition,
}

pub struct AcquisitionGuard<'a> {
    tls: &'a TlsSlotAcquisition,
    acquisition: usize,
}

impl<'a> AcquisitionGuard<'a> {
    pub const fn new(tls: &'a TlsSlotAcquisition, acquisition: usize) -> Self {
        Self { tls, acquisition }
    }
}

impl<'a> Drop for AcquisitionGuard<'a> {
    fn drop(&mut self) {
        unsafe {
            self.tls.release(self.acquisition);
        }
    }
}

impl State {
    pub fn new(configuration: Configuration, storage: Box<dyn AllocationsStorage>) -> Self {
        Self {
            configuration: Mutex::new(configuration),
            storage: Mutex::new(storage),
            tls_slot_acquisition: unsafe {
                TlsSlotAcquisition::new().expect("failed to allocate tls slot")
            },
        }
    }

    // Methods for preventing recursive detour call.
    pub fn acquire_slot(&self, slot: Slot) -> Option<AcquisitionGuard> {
        let mask = 1 << (slot as usize);
        let acquisition = unsafe { self.tls_slot_acquisition.acquire(mask) };

        if acquisition != 0 {
            Some(AcquisitionGuard::new(
                &self.tls_slot_acquisition,
                acquisition,
            ))
        } else {
            None
        }
    }

    pub fn acquire_all(&self) -> AcquisitionGuard {
        AcquisitionGuard::new(&self.tls_slot_acquisition, unsafe {
            self.tls_slot_acquisition.acquire(!0)
        })
    }

    pub fn set_configuration(&self, configuration: Configuration) {
        *self
            .configuration
            .lock()
            .expect("unexpected configuration lock poison") = configuration;
    }

    pub fn get_configuration(&self) -> Configuration {
        self.configuration
            .lock()
            .expect("unexpected configuration lock poison")
            .clone()
    }

    pub fn get_storage(&self) -> MutexGuard<'_, Box<dyn AllocationsStorage>> {
        self.storage.lock().expect("unexpected storage lock poison")
    }
}
