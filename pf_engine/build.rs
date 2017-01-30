extern crate vulkano_shaders;
fn main() {
    vulkano_shaders::build_glsl_shaders([
        ("src/shaders/generic-vertex.glsl",   vulkano_shaders::ShaderType::Vertex),
        ("src/shaders/generic-fragment.glsl", vulkano_shaders::ShaderType::Fragment),
    ].iter().cloned());
}
