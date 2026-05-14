use rand::Rng;
use std::collections::HashSet;
use std::net::UdpSocket;

pub struct PortAllocator {
    used_ports: HashSet<u16>,
    min: u16,
    max: u16,
}

impl PortAllocator {
    pub fn new(min: u16, max: u16) -> Self {
        Self {
            used_ports: HashSet::new(),
            min,
            max,
        }
    }

    pub fn allocate_port(&mut self) -> Option<u16> {
        let mut rng = rand::thread_rng();

        for _ in 0..1000 {
            let port = rng.gen_range(self.min..=self.max);

            // is in use?
            if self.used_ports.contains(&port) {
                continue;
            }

            // OS-check
            if UdpSocket::bind(("0.0.0.0", port)).is_ok() {
                self.used_ports.insert(port);
                return Some(port);
            }
        }

        None
    }

    pub fn release_port(&mut self, port: u16) {
        self.used_ports.remove(&port);
    }
}