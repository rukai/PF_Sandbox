use std::collections::HashSet;
use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::device::{Device, Queue};

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
}

impl_vertex!(Vertex, position);

pub struct Buffers {
}

impl Buffers {
    pub fn new() -> Buffers {
        Buffers { }
    }
}
