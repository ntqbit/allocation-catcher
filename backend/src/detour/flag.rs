use lazy_static::lazy_static;

use crate::platform::TlsKey;

pub struct AcquisitionGuard<'a> {
    tls: &'a TlsSlotAcquisition,
    acquisition: usize,
}

impl<'a> AcquisitionGuard<'a> {
    const fn new(tls: &'a TlsSlotAcquisition, acquisition: usize) -> Self {
        Self { tls, acquisition }
    }

    pub fn forget(self) {
        core::mem::forget(self)
    }
}

impl<'a> Drop for AcquisitionGuard<'a> {
    fn drop(&mut self) {
        self.tls.release(self.acquisition);
    }
}

struct TlsSlotAcquisition {
    tls_key: TlsKey<usize>,
}

// Assumes initial value of a TLS slot to be 0.
impl TlsSlotAcquisition {
    pub fn new() -> Self {
        Self {
            tls_key: TlsKey::new().expect("failed to allocate tls slot"),
        }
    }

    pub fn get(&self) -> usize {
        self.tls_key.get()
    }

    pub fn acquire(&self, acquisition: usize) -> usize {
        let val = self.get();
        self.tls_key.set(val | acquisition);
        !val & acquisition
    }

    pub fn release(&self, acquisition: usize) {
        self.tls_key.set(self.get() & !acquisition)
    }
}

pub struct DetourFlagSet {
    tls_slot_acquisition: TlsSlotAcquisition,
}

impl DetourFlagSet {
    pub fn new() -> Self {
        Self {
            tls_slot_acquisition: TlsSlotAcquisition::new(),
        }
    }

    pub fn acquire(&self, flag: impl Into<usize>) -> Option<AcquisitionGuard> {
        let mask = 1 << flag.into();
        let acquisition = self.tls_slot_acquisition.acquire(mask);

        if acquisition != 0 {
            Some(AcquisitionGuard::new(
                &self.tls_slot_acquisition,
                acquisition,
            ))
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn is_acquired(&self, flag: impl Into<usize>) -> bool {
        self.tls_slot_acquisition.get() & (1 << flag.into()) != 0
    }
}

lazy_static! {
    static ref DETOUR_FLAG_SET: DetourFlagSet = DetourFlagSet::new();
}

pub enum DetourFlag {
    // Setting this flag disables detour handling
    Lock,
}

impl Into<usize> for DetourFlag {
    fn into(self) -> usize {
        match self {
            DetourFlag::Lock => 0,
        }
    }
}

pub fn flag_set() -> &'static DetourFlagSet {
    &DETOUR_FLAG_SET
}
