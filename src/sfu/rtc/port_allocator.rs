use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
struct PortAllocatorInner {
    start: u16,
    end: u16,
    free: BTreeSet<u16>,
}

#[derive(Debug, Clone)]
pub struct PortAllocator {
    inner: Arc<Mutex<PortAllocatorInner>>,
}

impl PortAllocator {
    pub fn new(start: u16, end: u16) -> Self {
        assert!(start <= end);

        Self {
            inner: Arc::new(Mutex::new(PortAllocatorInner {
                start,
                end,
                free: (start..=end).collect(),
            })),
        }
    }

    pub fn acquire(&self) -> Option<PortLease> {
        let mut inner = self.inner.lock().unwrap();

        let port = inner.free.first().copied()?;
        inner.free.remove(&port);

        Some(PortLease {
            port,
            allocator: Arc::clone(&self.inner),
        })
    }

    pub fn available(&self) -> usize {
        self.inner.lock().unwrap().free.len()
    }

    pub fn capacity(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        (inner.end - inner.start + 1) as usize
    }
}

#[derive(Debug)]
pub struct PortLease {
    port: u16,
    allocator: Arc<Mutex<PortAllocatorInner>>,
}

impl PortLease {
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for PortLease {
    fn drop(&mut self) {
        let mut inner = self.allocator.lock().unwrap();

        if self.port >= inner.start && self.port <= inner.end {
            inner.free.insert(self.port);
        }
    }
}
