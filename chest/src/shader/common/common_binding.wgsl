#define_import_path aurora::common_binding
#import aurora::common_type::{Camera, Scene}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> scene: Scene;
