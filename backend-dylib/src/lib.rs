#![feature(type_alias_impl_trait)]

mod platform;

use std::{net::SocketAddr, sync::Arc};

use static_cell::make_static;

use platform::handle_panic;

use allocation_catcher_backend::{spawn_thread, AllocationCatcher, StorageAllocationHandler};
use allocation_catcher_backend_server::{serve_tcp, SimpleServer};

static mut ALLOCATION_CATCHER: Option<AllocationCatcher> = None;

fn initialize() {
    std::panic::set_hook(Box::new(|panic_info| handle_panic(panic_info)));

    assert!(unsafe { ALLOCATION_CATCHER.is_none() });

    let state = unsafe {
        let allocation_catcher = AllocationCatcher::init(Default::default());
        let state = allocation_catcher.state();
        let allocation_handler = make_static!(StorageAllocationHandler::new(state));
        allocation_catcher.set_allocation_handler(allocation_handler);
        allocation_catcher.enable();
        ALLOCATION_CATCHER = Some(allocation_catcher);
        state
    };

    spawn_thread(|| {
        serve_tcp(
            &SocketAddr::from(([0, 0, 0, 0], 9940)),
            Arc::new(SimpleServer::new(state)),
        )
    });
}

fn deinitialize() {
    assert!(unsafe { ALLOCATION_CATCHER.is_some() });
}
