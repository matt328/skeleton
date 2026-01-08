use slotmap::SlotMap;

use super::image::{ImageKey, ImageLifetime, ImageSpec};

pub struct LogicalImage {
    pub sped: ImageSpec,
    pub lifetime: ImageLifetime,
    pub physical: Option<ImageKey>,
}

slotmap::new_key_type! {pub struct LogicalImageKey; }

/// AliasRegistry
pub struct LogicalImageRegistry {
    logical_images: SlotMap<LogicalImageKey, LogicalImage>,
}
