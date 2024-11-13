use aurora_core::render::flow::RenderNode;

#[derive(Default)]
pub struct BilateralFiltering {}

impl RenderNode for BilateralFiltering {
    // fn require_shader(&self) -> Option<(&'static [&'static str], &'static str)> {
    //     Some([
    //         &[],
    //         include_str!("../shader/post_processing/image_effects/bilateral_filtering.wgsl"),
    //     ])
    // }
}
