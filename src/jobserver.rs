use std::sync::OnceLock;

use jobslot::Client;

pub static JOBSERVER: OnceLock<Client> = OnceLock::new();

pub fn maybe_init_fromenv(verbose: u8) {
    if let Some(client) = unsafe { Client::from_env() } {
        if verbose > 0 {
            println!("laze: jobserver inherited");
        }
        let _ = JOBSERVER.set(client);
    }
}

pub(crate) fn maybe_set_limit(limit: usize, verbose: u8) {
    JOBSERVER.get_or_init(|| {
        if verbose > 1 {
            println!("laze: configured jobserver with limit {limit}");
        }
        Client::new_with_fifo(limit).expect("jobserver created")
    });
}
