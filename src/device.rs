#[derive(Debug)]
pub struct Device {
    id: u32,
}

impl Device {
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    pub fn create_command_pool(&self, owner: &'static str) {
        log::debug!("Device {}: create command pool for {}", self.id, owner);
    }

    pub fn submit(&self, owner: &'static str) {
        log::debug!("Device {}: submit from {}", self.id, owner);
    }
}
