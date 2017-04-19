extern crate vulkano_shaders;

fn main() {
    vulkano_shaders::build_glsl_shaders([
        ("src/shaders/vertex.glsl",   vulkano_shaders::ShaderType::Vertex),
        ("src/shaders/fragment.glsl", vulkano_shaders::ShaderType::Fragment),
    ].iter().cloned());
}
