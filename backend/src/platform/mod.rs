mod windows;

pub use windows::{debug_message_fmt, TlsKey};

pub struct TlsSlotAcquisition {
    tls_key: TlsKey<usize>,
}

// Assumes initial value of a TLS slot to be 0.
impl TlsSlotAcquisition {
    pub fn new() -> Option<Self> {
        Some(Self {
            tls_key: TlsKey::new().ok()?,
        })
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
