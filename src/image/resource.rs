use ash::vk;
use smallvec::SmallVec;

use crate::image::{
    ImageKey, ImageViewKey,
    spec::{ImageSpec, ImageViewSpec},
};

pub struct OwnedImageInfo {
    pub allocation: vk_mem::Allocation,
    pub spec: ImageSpec,
    pub allocation_info: vk_mem::AllocationInfo,
    pub views: SmallVec<[ImageViewKey; 2]>,
}

pub struct Image {
    pub vk_image: vk::Image,
    pub owned: Option<OwnedImageInfo>,
}

impl Image {
    pub fn vk_image(&self) -> vk::Image {
        self.vk_image
    }
}

pub struct OwnedImageViewInfo {
    pub spec: ImageViewSpec,
    pub debug_name: Option<&'static str>,
}

pub struct ImageView {
    pub vk_image_view: vk::ImageView,
    pub owned: Option<OwnedImageViewInfo>,
}

/*
    TODO:
    figure out api for ImageManager and AliasRegistry.
    consider possibly AliasRegistry and ResolvedAliasRegistry.
    ImageManager
        - register_swapchain_images -> Vec<(ImageKey, ImageViewKey)>
            - Needs to handle swapchain resizing / image recreation
        - recreate_swapchain_images(SwapchainContext)
        - get_image(ImageKey)
        - get_image_view(ImageViewKey)
        - create_image(ImageSpec) -> ImageKey
        - create_image_view(ImageViewSpec) -> ImageViewKey
        - destroy_image(ImageKey)
        - destroy_image_view(ImageViewKey)
        - increment_image_use(ImageKey)
        - decrement_image_use(ImageKey)
    AliasRegistry
        - accumulates ImageAlias->ImageDesc mapping, also holds External Image Alias->(ImageKey, ImageViewKey)
        - is created once when compiling the framegraph and then thrown away after a ResolvedImageRegistry is produced
        - declare_image(ImageAlias, ImageDesc)
            - maps alias to ImageDesc
            - errors if any alias is ever mapped to more than one Desc
        - declare_external_image(ImageAlias, Vec<(ImageKey, ImageViewKey)>)
        - resolve(frame_count,
                  image_manager,
                  allocator,
                  device,
                  swapchain_context) -> ResolvedAliasRegistry
    ResolvedImageRegistry
        - contains static map of ImageAlias to (ImageKey, ImageViewKey)
        - immutable after creation
        - get_image_key(ImageAlias, FrameIndex)
        - get_image_view_key(ImageViewKey, FrameIndex)

*/
