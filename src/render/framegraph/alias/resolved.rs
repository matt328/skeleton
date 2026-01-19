use std::collections::HashMap;

use crate::{
    image::{CompositeImageKey, CompositeImageViewKey},
    render::framegraph::graph::ImageAlias,
};

pub struct ResolvedRegistry {
    pub images: HashMap<ImageAlias, CompositeImageKey>,
    pub image_views: HashMap<ImageAlias, CompositeImageViewKey>,
}
