use std::ffi::CString;

use ash::vk;

const VERT_SPV: &[u8] = include_bytes!("triangle.vert.spv");
const FRAG_SPV: &[u8] = include_bytes!("triangle.frag.spv");

fn spv_u32(bytes: &[u8]) -> anyhow::Result<&[u32]> {
    bytemuck::try_cast_slice(bytes)
        .map_err(|e| anyhow::anyhow!("invalid SPIR-V bytecode alignment: {e:?}"))
}

pub fn create_default_pipeline(
    device: &ash::Device,
    swapchain_format: vk::Format,
) -> anyhow::Result<(
    vk::PipelineLayout,
    vk::Pipeline,
    vk::ShaderModule,
    vk::ShaderModule,
)> {
    let pipeline_layout =
        unsafe { device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)? };
    let formats = [swapchain_format];
    let mut rendering_info =
        vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&formats);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

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

    let vert_module = create_shader_module(device, spv_u32(VERT_SPV)?)?;
    let frag_module = create_shader_module(device, spv_u32(FRAG_SPV)?)?;

    let module_name = CString::new("main")?;

    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&module_name),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&module_name),
    ];

    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&[])
        .vertex_attribute_descriptions(&[]);

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .input_assembly_state(&input_assembly)
        .vertex_input_state(&vertex_input)
        .viewport_state(&viewport_state)
        .rasterization_state(&raster)
        .multisample_state(&multisample)
        .color_blend_state(&color_blend)
        .dynamic_state(&dynamic_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info);

    let pipelines = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .map_err(|e| anyhow::anyhow!("failed to create graphics pipeline: {:?}", e))?
    };

    let pipeline = pipelines.first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("no pipeline returned from create_graphics_pipelines"))?;

    Ok((pipeline_layout, pipeline, frag_module, vert_module))
}

fn create_shader_module(device: &ash::Device, code: &[u32]) -> anyhow::Result<vk::ShaderModule> {
    Ok(unsafe {
        device.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(code), None)?
    })
}
