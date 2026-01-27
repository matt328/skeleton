use std::fmt;

use ash::vk;

use crate::{
    image::{CompositeImageKey, CompositeImageViewKey, ImageLifetime},
    render::framegraph::graph::ImageAlias,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImageFormat {
    SwapchainColor,
    _Depth,
    _HDRColor,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ImageSize {
    _Absolute { width: u32, height: u32 },
    SwapchainRelative { scale: f32 },
    _Relative(ImageAlias, f32),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ImageDesc {
    pub format: ImageFormat,
    pub size: ImageSize,
    pub usage: vk::ImageUsageFlags,
    pub lifetime: ImageLifetime,
    pub samples: vk::SampleCountFlags,
}

pub struct ImageKeys {
    pub image: CompositeImageKey,
    pub view: CompositeImageViewKey,
}

impl fmt::Display for ImageDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageDesc(format={}, size={}, usage={:?}, samples={:?}, lifetime={:?})",
            self.format, self.size, self.usage, self.samples, self.lifetime,
        )
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ImageFormat::SwapchainColor => "SwapchainColor",
            ImageFormat::_Depth => "Depth",
            ImageFormat::_HDRColor => "HDRColor",
        };
        f.write_str(s)
    }
}

impl fmt::Display for ImageSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ImageSize::_Absolute { width, height } => {
                write!(f, "{}x{}", width, height)
            }
            ImageSize::SwapchainRelative { scale } => {
                write!(f, "Swapchain * {:.2}", scale)
            }
            ImageSize::_Relative(alias, scale) => {
                write!(f, "{:?} * {:.2}", alias, scale)
            }
        }
    }
}
