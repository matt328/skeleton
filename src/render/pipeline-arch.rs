use std::ffi::CString;

use ash::vk;

use crate::render::shader::{ShaderId, ShaderManager};

pub struct GraphicsPipelineDesc {
    pub vertex_id: ShaderId,
    pub fragment_id: ShaderId,
    pub topology: vk::PrimitiveTopology,
    pub color_formats: Vec<vk::Format>,
    pub depth_format: Option<vk::Format>,
}

pub struct GraphicsPipeline {
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

pub fn destroy_graphics_pipeline(pipeline: GraphicsPipeline, device: &ash::Device) {
    unsafe {
        device.destroy_pipeline_layout(pipeline.layout, None);
        device.destroy_pipeline(pipeline.pipeline, None);
    }
}

pub fn create_graphics_pipeline(
    device: &ash::Device,
    desc: &GraphicsPipelineDesc,
    shader_manager: &ShaderManager,
) -> anyhow::Result<GraphicsPipeline> {
    let pipeline_layout =
        unsafe { device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)? };

    let mut rendering_info =
        vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&desc.color_formats);

    if let Some(depth) = desc.depth_format {
        rendering_info = rendering_info.depth_attachment_format(depth);
    }

    let input_assembly =
        vk::PipelineInputAssemblyStateCreateInfo::default().topology(desc.topology);

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let raster = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .line_width(1.0);

    let multisample = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA);

    let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
        .attachments(std::slice::from_ref(&color_blend_attachment));

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let vert_module = shader_manager.module(desc.vertex_id);
    let frag_module = shader_manager.module(desc.fragment_id);

    let entry = CString::new("main")?;

    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&entry),
    ];

    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&raster)
        .multisample_state(&multisample)
        .color_blend_state(&color_blend)
        .dynamic_state(&dynamic_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info);

    let pipeline = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .map_err(|e| anyhow::anyhow!("failed to create pipeline: {e:?}"))?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no pipeline returned"))?
    };

    Ok(GraphicsPipeline {
        layout: pipeline_layout,
        pipeline,
    })
}
