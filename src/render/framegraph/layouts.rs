use ash::vk;
use tracing_subscriber::field::debug;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ImageState {
    pub layout: vk::ImageLayout,
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
}

use std::fmt;

impl fmt::Display for ImageState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Convert layout
        let layout_str = match self.layout {
            vk::ImageLayout::UNDEFINED => "UNDEFINED",
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => "COLOR_ATTACHMENT_OPTIMAL",
            vk::ImageLayout::PRESENT_SRC_KHR => "PRESENT_SRC_KHR",
            vk::ImageLayout::GENERAL => "GENERAL",
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => "TRANSFER_SRC_OPTIMAL",
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => "TRANSFER_DST_OPTIMAL",
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => "SHADER_READ_ONLY_OPTIMAL",
            _ => "OTHER",
        };

        // Convert stage flags
        let mut stages = Vec::new();
        let stage_flags = self.stage;
        if stage_flags.contains(vk::PipelineStageFlags2::TOP_OF_PIPE) {
            stages.push("TOP_OF_PIPE");
        }
        if stage_flags.contains(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT) {
            stages.push("COLOR_ATTACHMENT_OUTPUT");
        }
        if stage_flags.contains(vk::PipelineStageFlags2::BOTTOM_OF_PIPE) {
            stages.push("BOTTOM_OF_PIPE");
        }
        if stage_flags.contains(vk::PipelineStageFlags2::TRANSFER) {
            stages.push("TRANSFER");
        }
        if stage_flags.contains(vk::PipelineStageFlags2::COMPUTE_SHADER) {
            stages.push("COMPUTE_SHADER");
        }
        if stages.is_empty() {
            stages.push("NONE");
        }

        // Convert access flags
        let mut access = Vec::new();
        let access_flags = self.access;
        if access_flags.contains(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE) {
            access.push("COLOR_ATTACHMENT_WRITE");
        }
        if access_flags.contains(vk::AccessFlags2::COLOR_ATTACHMENT_READ) {
            access.push("COLOR_ATTACHMENT_READ");
        }
        if access_flags.contains(vk::AccessFlags2::TRANSFER_READ) {
            access.push("TRANSFER_READ");
        }
        if access_flags.contains(vk::AccessFlags2::TRANSFER_WRITE) {
            access.push("TRANSFER_WRITE");
        }
        if access_flags.contains(vk::AccessFlags2::SHADER_READ) {
            access.push("SHADER_READ");
        }
        if access_flags.contains(vk::AccessFlags2::SHADER_WRITE) {
            access.push("SHADER_WRITE");
        }
        if access.is_empty() {
            access.push("NONE");
        }
        // Print like formatted JSON
        writeln!(f, "{{")?;
        writeln!(f, "  \"layout\": \"{}\",", layout_str)?;
        writeln!(f, "  \"stage\": [")?;
        for s in &stages {
            writeln!(f, "    \"{}\",", s)?;
        }
        writeln!(f, "  ],")?;
        writeln!(f, "  \"access\": [")?;
        for a in &access {
            writeln!(f, "    \"{}\",", a)?;
        }
        writeln!(f, "  ]")?;
        write!(f, "}}")
    }
}

impl ImageState {
    pub const UNDEFINED: ImageState = ImageState {
        layout: vk::ImageLayout::UNDEFINED,
        stage: vk::PipelineStageFlags2::TOP_OF_PIPE,
        access: vk::AccessFlags2::NONE,
    };

    pub const COLOR_ATTACHMENT_WRITE: ImageState = ImageState {
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
    };

    pub const PRESENT: ImageState = ImageState {
        layout: vk::ImageLayout::PRESENT_SRC_KHR,
        stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
    };
}

pub const COLOR_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

pub const DEPTH_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::DEPTH,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

pub fn transition_image(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    range: vk::ImageSubresourceRange,
    old: ImageState,
    new: ImageState,
    debug_name: &str,
) {
    // log_image_transition(old, new, debug_name);
    let barrier = vk::ImageMemoryBarrier2::default()
        .image(image)
        .subresource_range(range)
        .src_stage_mask(old.stage)
        .src_access_mask(old.access)
        .old_layout(old.layout)
        .dst_stage_mask(new.stage)
        .dst_access_mask(new.access)
        .new_layout(new.layout);

    let dep_info =
        vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));

    unsafe {
        device.cmd_pipeline_barrier2(cmd, &dep_info);
    }
}

pub fn log_image_transition(old: ImageState, new: ImageState, debug_name: &str) {
    fn layout_str(layout: vk::ImageLayout) -> &'static str {
        match layout {
            vk::ImageLayout::UNDEFINED => "UNDEFINED",
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => "COLOR_ATTACHMENT_OPTIMAL",
            vk::ImageLayout::PRESENT_SRC_KHR => "PRESENT_SRC_KHR",
            vk::ImageLayout::GENERAL => "GENERAL",
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => "TRANSFER_SRC_OPTIMAL",
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => "TRANSFER_DST_OPTIMAL",
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => "SHADER_READ_ONLY_OPTIMAL",
            _ => "OTHER",
        }
    }

    fn stage_str(stage: vk::PipelineStageFlags2) -> String {
        let mut stages = Vec::new();
        if stage.contains(vk::PipelineStageFlags2::TOP_OF_PIPE) {
            stages.push("TOP_OF_PIPE");
        }
        if stage.contains(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT) {
            stages.push("COLOR_ATTACHMENT_OUTPUT");
        }
        if stage.contains(vk::PipelineStageFlags2::BOTTOM_OF_PIPE) {
            stages.push("BOTTOM_OF_PIPE");
        }
        if stage.contains(vk::PipelineStageFlags2::TRANSFER) {
            stages.push("TRANSFER");
        }
        if stage.contains(vk::PipelineStageFlags2::COMPUTE_SHADER) {
            stages.push("COMPUTE_SHADER");
        }
        if stages.is_empty() {
            stages.push("NONE");
        }
        stages.join(" | ")
    }

    fn access_str(access: vk::AccessFlags2) -> String {
        let mut access_flags = Vec::new();
        if access.contains(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE) {
            access_flags.push("COLOR_ATTACHMENT_WRITE");
        }
        if access.contains(vk::AccessFlags2::COLOR_ATTACHMENT_READ) {
            access_flags.push("COLOR_ATTACHMENT_READ");
        }
        if access.contains(vk::AccessFlags2::TRANSFER_READ) {
            access_flags.push("TRANSFER_READ");
        }
        if access.contains(vk::AccessFlags2::TRANSFER_WRITE) {
            access_flags.push("TRANSFER_WRITE");
        }
        if access.contains(vk::AccessFlags2::SHADER_READ) {
            access_flags.push("SHADER_READ");
        }
        if access.contains(vk::AccessFlags2::SHADER_WRITE) {
            access_flags.push("SHADER_WRITE");
        }
        if access_flags.is_empty() {
            access_flags.push("NONE");
        }
        access_flags.join(" | ")
    }

    log::debug!(
        "{}\n     layout: {} -> {}\n     stage:  {} -> {}\n     access: {} -> {}",
        debug_name,
        layout_str(old.layout),
        layout_str(new.layout),
        stage_str(old.stage),
        stage_str(new.stage),
        access_str(old.access),
        access_str(new.access)
    );
}
