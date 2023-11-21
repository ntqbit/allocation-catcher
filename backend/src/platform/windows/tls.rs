use std::marker::PhantomData;

use winapi::{
    shared::minwindef::{DWORD, LPVOID},
    um::processthreadsapi::{TlsAlloc, TlsFree, TlsGetValue, TlsSetValue, TLS_OUT_OF_INDEXES},
};

pub struct TlsKey<T> {
    slot: DWORD,
    _marker: PhantomData<T>,
}

impl<T> TlsKey<T>
where
    T: From<usize> + Into<usize>,
{
    pub fn new() -> Result<Self, ()> {
        let slot = unsafe { TlsAlloc() };
        if slot == TLS_OUT_OF_INDEXES {
            Err(())
        } else {
            Ok(Self {
                slot,
                _marker: PhantomData,
            })
        }
    }

    pub fn get(&self) -> T {
        T::from(unsafe { TlsGetValue(self.slot) as usize })
    }

    pub fn set(&self, val: T) {
        unsafe {
            TlsSetValue(self.slot, val.into() as LPVOID);
        }
    }
}

impl<T> Drop for TlsKey<T> {
    fn drop(&mut self) {
        unsafe {
            TlsFree(self.slot);
        }
    }
}
