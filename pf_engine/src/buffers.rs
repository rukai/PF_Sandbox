use ::stage::Stage;
use ::fighter::{ActionFrame, LinkType};
use ::player::RenderPlayer;
use ::package::{PackageUpdate};
use ::game::RenderRect;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::device::{Device, Queue};

use std::f32::consts;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub edge: f32, // maybe I can use this to determine where to render outline
}
impl_vertex!(Vertex, position, edge);

pub struct Buffers {
    pub vertex: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub index:  Arc<CpuAccessibleBuffer<[u16]>>,
}

impl Buffers {
    /// Returns only a VertexBuffer
    /// Use with PrimitiveToplogy::LineStrip
    pub fn rect_buffers(device: &Arc<Device>, queue: &Arc<Queue>, rect: RenderRect) -> Buffers {
        let width = 0.5;
        let min_x = rect.p1.0.min(rect.p2.0);
        let min_y = rect.p1.1.min(rect.p2.1);
        let max_x = rect.p1.0.max(rect.p2.0);
        let max_y = rect.p1.1.max(rect.p2.1);

        let vertices: Vec<Vertex> = vec!(
            // outer rectangle
            Vertex { position: [min_x, min_y], edge: 0.0},
            Vertex { position: [max_x, min_y], edge: 0.0},
            Vertex { position: [max_x, max_y], edge: 0.0},
            Vertex { position: [min_x, max_y], edge: 0.0},
            // inner rectangle
            Vertex { position: [min_x+width, min_y+width], edge: 0.0},
            Vertex { position: [max_x-width, min_y+width], edge: 0.0},
            Vertex { position: [max_x-width, max_y-width], edge: 0.0},
            Vertex { position: [min_x+width, max_y-width], edge: 0.0},
        );
        let indices: [u16; 24] = [
            0, 4, 1, 1, 4, 5, // bottom edge
            1, 5, 2, 2, 5, 6, // right edge
            2, 6, 3, 3, 7, 6, // top edge
            3, 7, 0, 0, 4, 7, // left edge
        ];
        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
        }
    }

    fn new_stage(device: &Arc<Device>, queue: &Arc<Queue>, stage: &Stage) -> Buffers {
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut indice_count = 0;
        for platform in &stage.platforms[..] {
            let x1 = platform.x - platform.w / 2.0;
            let y1 = platform.y - platform.h / 2.0;
            let x2 = platform.x + platform.w / 2.0;
            let y2 = platform.y + platform.h / 2.0;

            vertices.push(Vertex { position: [x1, y1], edge: 0.0});
            vertices.push(Vertex { position: [x1, y2], edge: 0.0});
            vertices.push(Vertex { position: [x2, y1], edge: 0.0});
            vertices.push(Vertex { position: [x2, y2], edge: 0.0});

            indices.push(indice_count + 0);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 3);
            indice_count += 4;
        }

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
        }
    }

    fn new_fighter_frame(device: &Arc<Device>, queue: &Arc<Queue>, frame: &ActionFrame) -> Option<Buffers> {
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut index_count = 0;
        let triangles = 20;

        if frame.colboxes.len() == 0 {
            return None;
        }

        for colbox in &frame.colboxes[..] {
            // Draw a colbox, at the point
            // triangles are drawn meeting at the centre, forming a circle
            let point = &colbox.point;
            for i in 0..triangles {
                let angle: f32 = ((i * 2) as f32) * consts::PI / (triangles as f32);
                let x = point.0 + angle.cos() * colbox.radius;
                let y = point.1 + angle.sin() * colbox.radius;
                vertices.push(Vertex { position: [x, y], edge: 1.0});
                indices.push(index_count);
                indices.push(index_count + i);
                indices.push(index_count + (i + 1) % triangles);
            }
            index_count += triangles;
        }

        for link in &frame.colbox_links {
            match link.link_type {
                LinkType::MeldFirst | LinkType::MeldSecond => {
                    // draw a rectangle connecting two colboxes
                    let (x1, y1)   = frame.colboxes[link.one].point;
                    let (x2, y2)   = frame.colboxes[link.two].point;
                    let one_radius = frame.colboxes[link.one].radius;
                    let two_radius = frame.colboxes[link.two].radius;

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
                    vertices.push(Vertex { position: [link_x1, link_y1], edge: 0.0});
                    vertices.push(Vertex { position: [link_x2, link_y2], edge: 0.0});
                    vertices.push(Vertex { position: [link_x3, link_y3], edge: 0.0});
                    vertices.push(Vertex { position: [link_x4, link_y4], edge: 0.0});

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
        Some(Buffers {
            index:  CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
            vertex: CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
        })
    }

    pub fn new_player(device: &Arc<Device>, queue: &Arc<Queue>, player: &RenderPlayer) -> Buffers {
        // ecb
        let vertex0 = Vertex { position: [ player.ecb.bot_x,   player.ecb.bot_y], edge: 0.0};
        let vertex1 = Vertex { position: [ player.ecb.left_x,  player.ecb.left_y], edge: 0.0};
        let vertex2 = Vertex { position: [ player.ecb.right_x, player.ecb.right_y], edge: 0.0};
        let vertex3 = Vertex { position: [ player.ecb.top_x,   player.ecb.top_y], edge: 0.0};

        // horizontal bps
        let vertex4 = Vertex { position: [-4.0,-0.15], edge: 0.0};
        let vertex5 = Vertex { position: [-4.0, 0.15], edge: 0.0};
        let vertex6 = Vertex { position: [ 4.0,-0.15], edge: 0.0};
        let vertex7 = Vertex { position: [ 4.0, 0.15], edge: 0.0};

        // vertical bps
        let vertex8  = Vertex { position: [-0.15,-4.0], edge: 0.0};
        let vertex9  = Vertex { position: [ 0.15,-4.0], edge: 0.0};
        let vertex10 = Vertex { position: [-0.15, 4.0], edge: 0.0};
        let vertex11 = Vertex { position: [ 0.15, 4.0], edge: 0.0};

        let vertices = vec![vertex0, vertex1, vertex2, vertex3, vertex4, vertex5, vertex6, vertex7, vertex8, vertex9, vertex10, vertex11];
        let indices: [u16; 18] = [
            1,  2,  0,
            1,  2,  3,
            4,  5,  6,
            7,  6,  5,
            8,  9,  10,
            11, 10, 13,
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device, &BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
        }
    }
}

pub struct PackageBuffers {
    pub stages: Vec<Buffers>,
    pub fighters: Vec<Vec<Vec<Option<Buffers>>>>, // fighters <- actions <- frames
}

impl PackageBuffers {
    pub fn new() -> PackageBuffers {
        let package_buffers = PackageBuffers {
            stages:   vec!(),
            fighters: vec!(),
        };
        package_buffers
    }

    pub fn update(&mut self, device: &Arc<Device>, queue: &Arc<Queue>, package_updates: Vec<PackageUpdate>) {
        for update in package_updates {
            match update {
                PackageUpdate::Package (package) => {
                    self.stages = vec!();
                    self.fighters = vec!();
                    let mut i = 0;

                    for fighter in &package.fighters[..] { // TODO: Whats up with the deref coercion?
                        let mut action_buffers: Vec<Vec<Option<Buffers>>> = vec!();
                        for action in &fighter.actions[..] {
                            let mut frame_buffers: Vec<Option<Buffers>> = vec!();
                            for frame in &action.frames[..] {
                                println!("i:{}", i);
                                i += 1;
                                frame_buffers.push(Buffers::new_fighter_frame(device, queue, frame));
                            }
                            action_buffers.push(frame_buffers);
                        }
                        self.fighters.push(action_buffers);
                    }

                    for stage in &package.stages[..] {
                        self.stages.push(Buffers::new_stage(device, queue, &stage));
                    }
                }
                PackageUpdate::DeleteFighterFrame { fighter, action, frame_index } => {
                    self.fighters[fighter][action].remove(frame_index);
                }
                PackageUpdate::InsertFighterFrame { fighter, action, frame_index, frame } => {
                    let buffers = Buffers::new_fighter_frame(device, queue, &frame);
                    self.fighters[fighter][action].insert(frame_index, buffers);
                }
                PackageUpdate::DeleteStage { stage_index } => {
                    self.stages.remove(stage_index);
                }
                PackageUpdate::InsertStage { stage_index, stage } => {
                    self.stages.insert(stage_index, Buffers::new_stage(device, queue, &stage));
                }
            }
        }
    }
}
