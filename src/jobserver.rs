use std::sync::OnceLock;

use log::{debug, trace};

use jobslot::Client;

pub static JOBSERVER: OnceLock<Client> = OnceLock::new();

pub fn maybe_init_fromenv() {
    if let Some(client) = unsafe { Client::from_env() } {
        debug!("laze: jobserver inherited");
        let _ = JOBSERVER.set(client);
    }
}

pub(crate) fn maybe_set_limit(limit: usize) {
    JOBSERVER.get_or_init(|| {
        trace!("laze: configured jobserver with limit {limit}");
        Client::new_with_fifo(limit).expect("jobserver created")
    });
}
