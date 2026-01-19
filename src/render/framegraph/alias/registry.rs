use std::collections::HashMap;

use ash::vk;

use crate::{
    image::{
        CompositeImageKey, CompositeImageViewKey, ImageKey, ImageManager, ImageSpec, ImageViewKey,
        ImageViewSpec, ResizePolicy,
    },
    render::framegraph::{
        alias::{
            data::{ImageDesc, ImageFormat, ImageKeys, ImageSize},
            resolved::ResolvedRegistry,
        },
        graph::ImageAlias,
    },
};

pub struct AliasRegistry {
    declared: HashMap<ImageAlias, ImageDesc>,
    externals: HashMap<ImageAlias, Vec<ImageKeys>>,
}

impl Default for AliasRegistry {
    fn default() -> Self {
        Self {
            declared: Default::default(),
            externals: Default::default(),
        }
    }
}

impl AliasRegistry {
    pub fn declare_image(&mut self, alias: ImageAlias, desc: ImageDesc) -> anyhow::Result<()> {
        if let Some(existing) = self.declared.get(&alias) {
            if *existing != desc {
                anyhow::bail!(
                    "ImageAlias {:?} declared with conflicting descriptions:\n\
                 existing: {}\n\
                 new:      {}",
                    alias,
                    existing,
                    desc
                );
            }
            return Ok(());
        }

        self.declared.insert(alias, desc);
        Ok(())
    }

    pub fn declare_external_image(
        &mut self,
        alias: ImageAlias,
        keys: Vec<(ImageKey, ImageViewKey)>,
    ) -> anyhow::Result<()> {
        let image_keys: Vec<ImageKeys> = keys
            .into_iter()
            .map(|(image, view)| ImageKeys { image, view })
            .collect();
        self.externals.insert(alias, image_keys);
        Ok(())
    }

    pub fn resolve(
        &mut self,
        image_manager: &mut ImageManager,
        allocator: &vk_mem::Allocator,
        ctx: &ImageResolveContext,
    ) -> anyhow::Result<ResolvedRegistry> {
        let mut images: HashMap<ImageAlias, CompositeImageKey> = HashMap::default();

        let mut image_views: HashMap<ImageAlias, CompositeImageViewKey> = HashMap::default();

        for (alias, desc) in self.declared.iter() {
            let spec = create_image_spec(desc, ctx)?;
            let image_key = image_manager.create_image(allocator, spec, ctx.frame_count)?;
            images.insert(*alias, image_key);

            let view_spec = create_image_view_spec(image_key, &spec)?;
            let view_key =
                image_manager.create_image_view(ctx.device, view_spec, ctx.frame_count)?;
            image_views.insert(*alias, view_key);
        }

        for (alias, keys) in self.externals.iter() {
            for k in keys {
                images.insert(*alias, CompositeImageKey::External(k.image));
                image_views.insert(*alias, CompositeImageViewKey::External(k.view));
            }
        }

        Ok(ResolvedRegistry {
            images,
            image_views,
        })
    }
}

pub struct ImageResolveContext<'a> {
    pub device: &'a ash::Device,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_format: vk::Format,
    pub resolve_alias: &'a dyn Fn(ImageAlias) -> vk::Extent2D,
    pub default_resize_policy: ResizePolicy,
    pub default_initial_layout: vk::ImageLayout,
    pub frame_count: u32,
}

fn create_image_view_spec(
    image_key: CompositeImageKey,
    spec: &ImageSpec,
) -> anyhow::Result<ImageViewSpec> {
    Ok(ImageViewSpec::new(image_key)
        .view_type(derive_view_type(spec.layers))
        .aspect(derive_aspect_mask(spec.format))
        .mip_range(0, 1)
        .format(spec.format)
        .layers(0, 1))
}

fn derive_view_type(layers: u32) -> vk::ImageViewType {
    match layers {
        1 => vk::ImageViewType::TYPE_2D,
        _ => vk::ImageViewType::TYPE_2D_ARRAY,
    }
}

fn derive_aspect_mask(format: vk::Format) -> vk::ImageAspectFlags {
    match format {
        vk::Format::D32_SFLOAT | vk::Format::D16_UNORM => vk::ImageAspectFlags::DEPTH,

        vk::Format::D24_UNORM_S8_UINT | vk::Format::D32_SFLOAT_S8_UINT => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }

        _ => vk::ImageAspectFlags::COLOR,
    }
}

fn create_image_spec(desc: &ImageDesc, ctx: &ImageResolveContext) -> anyhow::Result<ImageSpec> {
    let format = match desc.format {
        ImageFormat::SwapchainColor => ctx.swapchain_format,
        ImageFormat::Depth => vk::Format::D32_SFLOAT,
        ImageFormat::HDRColor => vk::Format::R16G16B16A16_SFLOAT,
    };

    let extent2d = match desc.size {
        ImageSize::Absolute { width, height } => vk::Extent2D { width, height },

        ImageSize::SwapchainRelative { scale } => vk::Extent2D {
            width: (ctx.swapchain_extent.width as f32 * scale) as u32,
            height: (ctx.swapchain_extent.height as f32 * scale) as u32,
        },

        ImageSize::Relative(alias, scale) => {
            let base = (ctx.resolve_alias)(alias);
            vk::Extent2D {
                width: (base.width as f32 * scale) as u32,
                height: (base.height as f32 * scale) as u32,
            }
        }
    };

    let extent = vk::Extent3D {
        width: extent2d.width,
        height: extent2d.height,
        depth: 1,
    };

    let resize_policy = match desc.size {
        ImageSize::Absolute { .. } => ResizePolicy::Fixed,
        _ => ctx.default_resize_policy,
    };

    let spec = ImageSpec::default()
        .format(format)
        .extent(extent)
        .usage(desc.usage)
        .samples(desc.samples)
        .resize_policy(resize_policy)
        .lifetime(desc.lifetime)
        .initial_layout(ctx.default_initial_layout);

    Ok(spec)
}
