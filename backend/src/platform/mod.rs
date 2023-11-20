mod windows;

pub use windows::TlsKey;

pub struct TlsSlotAcquisition {
    tls_key: TlsKey<usize>,
}

impl TlsSlotAcquisition {
    pub unsafe fn new() -> Option<Self> {
        let tls_key = TlsKey::new().ok()?;
        tls_key.set(0);

        Some(Self { tls_key })
    }

    pub unsafe fn acquire(&self, slot: usize) -> bool {
        let val = self.tls_key.get();
        if val & (1 << slot) != 0 {
            false
        } else {
            self.tls_key.set(val | (1 << slot));
            true
        }
    }

    pub unsafe fn release(&self, slot: usize) {
        self.tls_key.set(self.tls_key.get() & !(1 << slot));
    }
}
