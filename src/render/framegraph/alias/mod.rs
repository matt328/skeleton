mod data;
mod registry;
mod resolved;

pub use data::{ImageDesc, ImageFormat, ImageSize};

pub use registry::{AliasRegistry, ImageResolveContext};

pub use resolved::ResolvedRegistry;
