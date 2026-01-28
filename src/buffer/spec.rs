#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationStrategy {
    Linear,
    Resizable,
    Arena,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferLifetime {
    PerFrame,
    Global,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferType {
    DeviceArena,
    Device,
    HostTransient,
    IndirectCommand,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferUsage {
    Storage,
    Uniform,
    Transfer,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BufferSpec {
    pub allocation_strategy: AllocationStrategy,
    pub lifetime: BufferLifetime,
    pub usage: BufferUsage,
    pub initial_size: usize,
    pub item_stride: usize,
    pub debug_name: Option<String>,
}
