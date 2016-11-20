use ::stage::Stage;
use ::fighter::{ActionFrame, LinkType};
use ::player::RenderPlayer;
use ::package::{Package, PackageUpdate};
use ::game::RenderRect;

use glium;
use glium::backend::glutin_backend::GlutinFacade;

use std::f32::consts;

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
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut indice_count = 0;
        for platform in &stage.platforms[..] {
            let x1 = platform.x - platform.w / 2.0;
            let y1 = platform.y - platform.h / 2.0;
            let x2 = platform.x + platform.w / 2.0;
            let y2 = platform.y + platform.h / 2.0;

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

    /// Returns only a VertexBuffer
    /// Use with index::PrimitiveType::LineStrip
    pub fn rect_vertices(display: &GlutinFacade, rect: RenderRect) -> glium::VertexBuffer<Vertex> {
        let vertices: Vec<Vertex> = vec!(
            Vertex { position: [rect.p1.0, rect.p1.1] },
            Vertex { position: [rect.p1.0, rect.p2.1] },
            Vertex { position: [rect.p2.0, rect.p2.1] },
            Vertex { position: [rect.p2.0, rect.p1.1] },
            Vertex { position: [rect.p1.0, rect.p1.1] },
        );
        glium::VertexBuffer::immutable(display, &vertices).unwrap()
    }

    fn new_fighter_frame(display: &GlutinFacade, frame: &ActionFrame) -> Buffers {
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut index_count = 0;
        let triangles = 20;

        for colbox in &frame.colboxes[..] {
            // Draw a colbox, at the point
            // triangles are drawn meeting at the centre, forming a circle
            let point = &colbox.point;
            for i in 0..triangles {
                let angle: f32 = ((i * 2) as f32) * consts::PI / (triangles as f32);
                let x = point.0 + angle.cos() * colbox.radius;
                let y = point.1 + angle.sin() * colbox.radius;
                vertices.push(Vertex { position: [x, y] });
                indices.push(index_count);
                indices.push(index_count + i);
                indices.push(index_count + (i + 1) % triangles);
            }
            index_count += triangles;
        }

        for link in &frame.colbox_links {
            match link.link_type {
                LinkType::Meld => {
                    // draw a rectangle connecting two colboxes
                    let (x1, y1) = frame.colboxes[link.one].point;
                    let (x2, y2) = frame.colboxes[link.two].point;
                    let one_radius     = frame.colboxes[link.one].radius;
                    let two_radius     = frame.colboxes[link.two].radius;

                    let mid_angle = (y1 - y2).atan2(x1 - x2);

                    let angle1 = mid_angle + consts::FRAC_PI_2;
                    let angle2 = mid_angle - consts::FRAC_PI_2;

                    // rectangle as 4 points
                    let link_x1 = x1 + angle1.cos() * one_radius;
                    let link_x2 = x1 + angle2.cos() * one_radius;
                    let link_x3 = x2 + angle1.cos() * two_radius;
                    let link_x4 = x2 + angle2.cos() * two_radius;

                    let link_y1 = y1 + angle1.sin() * one_radius;
                    let link_y2 = y1 + angle2.sin() * one_radius;
                    let link_y3 = y2 + angle1.sin() * two_radius;
                    let link_y4 = y2 + angle2.sin() * two_radius;

                    // rectangle into buffers
                    vertices.push(Vertex { position: [link_x1, link_y1] });
                    vertices.push(Vertex { position: [link_x2, link_y2] });
                    vertices.push(Vertex { position: [link_x3, link_y3] });
                    vertices.push(Vertex { position: [link_x4, link_y4] });

                    indices.push(index_count);
                    indices.push(index_count + 1);
                    indices.push(index_count + 2);

                    indices.push(index_count + 1);
                    indices.push(index_count + 2);
                    indices.push(index_count + 3);
                    index_count += 4;
                },
                LinkType::Simple => { },
            }
        }

        Buffers {
            vertex: glium::VertexBuffer::new(display, &vertices).unwrap(),
            index: glium::IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &indices).unwrap(),
        }
    }

    pub fn new_player(display: &GlutinFacade, player: &RenderPlayer) -> Buffers {
        let ecb_w = player.ecb_w;
        let ecb_y = player.ecb_y;
        let ecb_top = player.ecb_top;
        let ecb_bottom = player.ecb_bottom;

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
    pub fighters: Vec<Vec<Vec<Buffers>>>, // fighters <- actions <- frames
}

impl PackageBuffers {
    pub fn new(display: &GlutinFacade, package: Package) -> PackageBuffers {
        let mut package_buffers = PackageBuffers {
            stages:   vec!(),
            fighters: vec!(),
        };
        package_buffers.update(display, vec!(PackageUpdate::Package(package)));
        package_buffers
    }

    pub fn update(&mut self, display: &GlutinFacade, package_updates: Vec<PackageUpdate>) {
        for update in package_updates {
            match update {
                PackageUpdate::Package (package) => {
                    self.stages = vec!();
                    self.fighters = vec!();

                    for fighter in &package.fighters[..] { // TODO: Whats up with the deref coercion?
                        let mut action_buffers: Vec<Vec<Buffers>> = vec!();
                        for action in &fighter.actions[..] {
                            let mut frame_buffers: Vec<Buffers> = vec!();
                            for frame in &action.frames[..] {
                                frame_buffers.push(Buffers::new_fighter_frame(display, frame));
                            }
                            action_buffers.push(frame_buffers);
                        }
                        self.fighters.push(action_buffers);
                    }

                    for stage in &package.stages[..] {
                        self.stages.push(Buffers::new_stage(display, &stage));
                    }
                }
                PackageUpdate::DeleteFighterFrame { fighter, action, frame_index } => {
                    self.fighters[fighter][action].remove(frame_index);
                }
                PackageUpdate::InsertFighterFrame { fighter, action, frame_index, frame } => {
                    let buffers = Buffers::new_fighter_frame(display, &frame);
                    self.fighters[fighter][action].insert(frame_index, buffers);
                }
                PackageUpdate::DeleteStage { stage_index } => {
                    self.stages.remove(stage_index);
                }
                PackageUpdate::InsertStage { stage_index, stage } => {
                    self.stages.insert(stage_index, Buffers::new_stage(display, &stage));
                }
            }
        }
    }
}
