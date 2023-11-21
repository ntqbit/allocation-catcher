use std::sync::{Mutex, MutexGuard};

use crate::{allocations_storage::AllocationsStorage, proto};

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

#[derive(Debug, Default, Clone)]
pub struct Statistics {
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub total_deallocations_non_allocated: usize,
}

impl Statistics {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub struct State {
    configuration: Mutex<Configuration>,
    storage: Mutex<Box<dyn AllocationsStorage>>,
    statistics: Mutex<Box<Statistics>>,
}

impl State {
    pub fn new(configuration: Configuration, storage: Box<dyn AllocationsStorage>) -> Self {
        Self {
            configuration: Mutex::new(configuration),
            storage: Mutex::new(storage),
            statistics: Mutex::new(Box::new(Statistics::default())),
        }
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

    pub fn lock_storage(&self) -> MutexGuard<'_, Box<dyn AllocationsStorage>> {
        self.storage.lock().expect("unexpected storage lock poison")
    }

    pub fn lock_statistics(&self) -> MutexGuard<'_, Box<Statistics>> {
        self.statistics
            .lock()
            .expect("unexpected statistics lock poison")
    }
}
