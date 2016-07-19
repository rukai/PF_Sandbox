use ::stage::Stage;
use ::fighter::Fighter;
use ::player::RenderPlayer;

use glium;
use glium::backend::glutin_backend::GlutinFacade;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
}
implement_vertex!(Vertex, position);

pub struct Buffers {
    pub vertex: glium::VertexBuffer<Vertex>,
    pub index: glium::IndexBuffer<u16>,
}

impl Buffers {
    pub fn new(display: &GlutinFacade) -> Buffers {
        Buffers {
            vertex: glium::VertexBuffer::empty_dynamic(display, 1000).unwrap(),
            index: glium::IndexBuffer::empty_dynamic(display, glium::index::PrimitiveType::TrianglesList, 1000).unwrap(),
        }
    }

    pub fn new_stage(display: &GlutinFacade, stage: &Stage) -> Buffers {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut indice_count = 0;
        for platform in &stage.platforms {
            let x1 = (platform.x - platform.w / 2.0) as f32;
            let y1 = (platform.y - platform.h / 2.0) as f32;
            let x2 = (platform.x + platform.w / 2.0) as f32;
            let y2 = (platform.y + platform.h / 2.0) as f32;

            vertices.push(Vertex { position: [x1, y1] });
            vertices.push(Vertex { position: [x1, y2] });
            vertices.push(Vertex { position: [x2, y1] });
            vertices.push(Vertex { position: [x2, y2] });

            indices.push(indice_count + 0);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 3);
            indice_count += 4;
        }

        Buffers {
            vertex: glium::VertexBuffer::new(display, &vertices).unwrap(),
            index: glium::IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &indices).unwrap(),
        }
    }

    pub fn new_fighter(display: &GlutinFacade, fighter: &Fighter) -> Buffers {
        Buffers::new(display)
    }

    pub fn new_player(display: &GlutinFacade, player: &RenderPlayer) -> Buffers {
        let ecb_w = player.ecb_w as f32;
        let ecb_y = player.ecb_y as f32;
        let ecb_top = player.ecb_top as f32;
        let ecb_bottom = player.ecb_bottom as f32;

        // ecb
        let vertex0 = Vertex { position: [ 0.0, ecb_y + ecb_bottom] };
        let vertex1 = Vertex { position: [-ecb_w/2.0, ecb_y] };
        let vertex2 = Vertex { position: [ ecb_w/2.0, ecb_y] };
        let vertex3 = Vertex { position: [ 0.0, ecb_y + ecb_top] };

        // horizontal bps
        let vertex4 = Vertex { position: [-4.0,-0.15] };
        let vertex5 = Vertex { position: [-4.0, 0.15] };
        let vertex6 = Vertex { position: [ 4.0,-0.15] };
        let vertex7 = Vertex { position: [ 4.0, 0.15] };

        // vertical bps
        let vertex8  = Vertex { position: [-0.15,-4.0] };
        let vertex9  = Vertex { position: [ 0.15,-4.0] };
        let vertex10 = Vertex { position: [-0.15, 4.0] };
        let vertex11 = Vertex { position: [ 0.15, 4.0] };

        let shape = vec![vertex0, vertex1, vertex2, vertex3, vertex4, vertex5, vertex6, vertex7, vertex8, vertex9, vertex10, vertex11];
        let indices: [u16; 18] = [
            1,  2,  0,
            1,  2,  3,
            4,  5,  6,
            7,  6,  5,
            8,  9,  10,
            11, 10, 13,
        ];

        let vertices = glium::VertexBuffer::new(display, &shape).unwrap();
        let indices = glium::IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &indices).unwrap();

        Buffers {
            vertex: vertices,
            index: indices,
        }
    }
}

pub struct PackageBuffers {
    pub stages: Vec<Buffers>,
    pub fighters: Vec<Buffers>,
}

impl PackageBuffers {
    pub fn new(display: &GlutinFacade, fighters: &Vec<Fighter>, stages: &Vec<Stage>) -> PackageBuffers {
        let mut stage_buffers:   Vec<Buffers> = vec!();
        let mut fighter_buffers: Vec<Buffers> = vec!();

        for fighter in fighters {
            fighter_buffers.push(Buffers::new_fighter(display, fighter));
        }

        for stage in stages {
            stage_buffers.push(Buffers::new_stage(display, stage));
        }

        PackageBuffers {
            stages: stage_buffers,
            fighters: fighter_buffers,
        }
    }
}
