use ash::vk;

use super::{frame::Frame, render_packet::RenderData};

pub fn record_commands(
    device: &ash::Device,
    frame: &Frame,
    _render_data: &RenderData,
) -> anyhow::Result<()> {
    unsafe {
        for &cmd in &frame.command_buffers {
            device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;
        }
    }

    Ok(())
}
