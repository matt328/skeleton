use std::collections::HashMap;

use crate::image::ImageSpec;

type ImageHandle = u32;
type ImageViewHandle = u32;
type Alias = String;

enum ImageLifetime {
    Transient,
    Persistent,
    Swapchain,
}

struct AliasImageEntry {
    lifetime: ImageLifetime,
    image_handles: Vec<ImageHandle>,
}

struct AliasImageViewEntry {
    lifetime: ImageLifetime,
    image_view_handles: Vec<ImageViewHandle>,
}

pub struct AliasRegistry {
    image_specs: HashMap<Alias, ImageSpec>,
    image_entries: HashMap<Alias, AliasImageEntry>,

    image_view_entries: HashMap<Alias, AliasImageViewEntry>,
}
