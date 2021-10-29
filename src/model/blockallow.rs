pub enum BlockAllow {
    Allowed,
    AllowedBy(usize),
    Blocked,
    BlockedBy(usize),
}

impl BlockAllow {
    pub fn allow(index: usize, depth: usize) -> BlockAllow {
        match depth {
            0 => BlockAllow::Allowed,
            _ => BlockAllow::AllowedBy(index),
        }
    }
    pub fn block(index: usize, depth: usize) -> BlockAllow {
        match depth {
            0 => BlockAllow::Blocked,
            _ => BlockAllow::BlockedBy(index),
        }
    }
}
