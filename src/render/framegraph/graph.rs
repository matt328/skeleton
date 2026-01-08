pub trait RenderPass {}

pub struct FrameGraph {
    render_passes: Vec<Box<dyn RenderPass>>,
}

impl FrameGraph {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            render_passes: vec![],
        })
    }
}
