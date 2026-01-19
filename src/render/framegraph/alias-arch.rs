use anyhow::Context;
use ash::vk;

use crate::{
    image::{
        ImageKey, ImageLifetime, ImageManager, ImageSpec, ImageViewKey, ImageViewSpec, ResizePolicy,
    },
    render::{
        framegraph::{graph::ImageAlias, image::ImageDesc},
        swapchain::SwapchainContext,
    },
};
use std::collections::HashMap;

pub enum BuiltinImageDecl {
    Swapchain,
}

struct DeclaredImageDecl {
    desc: ImageDesc,
}

enum AliasImage {
    Declared(DeclaredImageDecl),
    Builtin(BuiltinImageDecl),
}

pub struct AliasRegistry {
    images: HashMap<ImageAlias, AliasImage>,
}

impl AliasRegistry {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            images: HashMap::default(),
        })
    }

    pub fn declare_image(&mut self, alias: ImageAlias, desc: ImageDesc) -> anyhow::Result<()> {
        self.images
            .insert(alias, AliasImage::Declared(DeclaredImageDecl { desc }));
        Ok(())
    }

    pub fn require_image(&mut self, alias: ImageAlias) -> anyhow::Result<()> {
        self.images
            .get(&alias)
            .with_context(|| format!("No image registered with alias {:?}", alias))?;
        Ok(())
    }

    pub fn declare_builtin(
        &mut self,
        alias: ImageAlias,
        image: BuiltinImageDecl,
    ) -> anyhow::Result<()> {
        self.images.insert(alias, AliasImage::Builtin(image));
        Ok(())
    }

    pub fn build_resources(
        self,
        frame_count: u32,
        image_manager: &mut ImageManager,
        allocator: &vk_mem::Allocator,
        device: &ash::Device,
        swapchain: &SwapchainContext,
    ) -> anyhow::Result<ResolvedAliasRegistry> {
        let mut resolved = HashMap::new();

        for (alias, entry) in self.images {
            let res_image = match entry {
                AliasImage::Declared(decl) => {
                    let count = if decl.desc.lifetime == ImageLifetime::Transient {
                        frame_count
                    } else {
                        1
                    };

                    let mut images = Vec::with_capacity(count as usize);
                    let mut views = Vec::with_capacity(count as usize);

                    for _ in 0..count {
                        let (img, view) =
                            image_manager.create_image_and_view(device, allocator, &decl.desc)?;
                        images.push(img);
                        views.push(view);
                    }

                    ResolvedImage::Owned(OwnedImageResolved {
                        images,
                        views,
                        desc: decl.desc,
                    })
                }
                AliasImage::Builtin(builtin) => match builtin {
                    BuiltinImageDecl::Swapchain => {
                        ResolvedImage::Builtin(BuiltinImageResolved::Swapchain {
                            images: swapchain.images.clone(),
                            views: swapchain.image_views.clone(),
                            format: swapchain.swapchain_format,
                            extent: swapchain.swapchain_extent,
                        })
                    }
                },
            };

            resolved.insert(alias, res_image);
        }

        Ok(ResolvedAliasRegistry { images: resolved })
    }
}

#[derive(Clone)]
pub struct OwnedImageResolved {
    pub images: Vec<ImageKey>,    // per-frame
    pub views: Vec<ImageViewKey>, // per-frame
    pub desc: ImageDesc,
}

#[derive(Clone)]
pub enum BuiltinImageResolved {
    Swapchain {
        images: Vec<vk::Image>,
        views: Vec<vk::ImageView>,
        format: vk::Format,
        extent: vk::Extent2D,
    },
}

#[derive(Clone)]
pub enum ResolvedImage {
    Owned(OwnedImageResolved),
    BuiltinSwapchain {
        images: Vec<vk::Image>,
        views: Vec<vk::ImageView>,
        format: vk::Format,
        extent: vk::Extent2D,
    },
}

pub struct ResolvedAliasRegistry {
    images: HashMap<ImageAlias, ResolvedImage>,
}

impl ResolvedAliasRegistry {
    pub fn image_view(&self, alias: ImageAlias, frame_index: u32) -> anyhow::Result<ImageViewKey> {
        let entry = self
            .images
            .get(&alias)
            .with_context(|| format!("No image registered with alias {:?}", alias))?;

        match entry {
            ResolvedImage::Owned(owned) => {
                owned.views.get(frame_index as usize).copied().context("")
            }
            ResolvedImage::BuiltinSwapchain { views, .. } => {
                views.get(frame_index as usize).copied().context("")
            }
        }
    }

    pub fn image(&self, alias: ImageAlias, frame_index: u32) -> anyhow::Result<ImageKey> {
        let entry = self
            .images
            .get(&alias)
            .with_context(|| format!("No image registered with alias {:?}", alias))?;

        match entry {
            ResolvedImage::Owned(owned) => owned
                .images
                .get(frame_index as usize)
                .copied()
                .with_context(|| {
                    format!(
                        "Frame index {} out of bounds for alias {:?}",
                        frame_index, alias
                    )
                }),
            ResolvedImage::Builtin(BuiltinImageResolved::Swapchain { images, .. }) => {
                images.get(frame_index as usize).copied().with_context(|| {
                    format!(
                        "Frame index {} out of bounds for swapchain alias {:?}",
                        frame_index, alias
                    )
                })
            }
        }
    }
}

fn desc_to_spec(
    desc: ImageDesc,
    resolved_extent: vk::Extent3D,
    resize_policy: ResizePolicy,
    initial_layout: vk::ImageLayout,
) -> ImageSpec {
    ImageSpec {
        format: desc.format.resolve(),
        extent: resolved_extent,
        usage: desc.usage,
        samples: desc.samples,
        lifetime: desc.lifetime,
        resize_policy,
        initial_layout,
        mips: 1,
        layers: 1,
        debug_name: None,
    }
}

fn image_view_spec(image_key: ImageKey, image_spec: &ImageSpec) -> ImageViewSpec {
    let (aspect_mask, view_type) = if image_spec
        .usage
        .intersects(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
    {
        (vk::ImageAspectFlags::COLOR, vk::ImageViewType::TYPE_2D)
    } else {
        let view_type = if image_spec.layers == 1 {
            vk::ImageViewType::TYPE_2D
        } else {
            vk::ImageViewType::TYPE_2D_ARRAY
        };
        (vk::ImageAspectFlags::DEPTH, view_type)
    };

    ImageViewSpec {
        image_key,
        format: image_spec.format,
        aspect_mask,
        view_type,
        level_count: image_spec.mips,
        layer_count: image_spec.layers,
        ..Default::default()
    }
}
