use ::fighter::{ActionFrame, LinkType, ColboxOrLink, CollisionBox, CollisionBoxLink};
use ::graphics;
use ::graphics::RenderRect;
use ::package::{Package, PackageUpdate};
use ::player::RenderPlayer;
use ::stage::Stage;

use std::collections::{HashSet, HashMap};
use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::device::{Device, Queue};

use std::f32::consts;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub edge: f32,
    pub render_id: f32,
}

impl_vertex!(Vertex, position, edge, render_id);

fn vertex(x: f32, y: f32) -> Vertex {
    Vertex {
        position: [x, y],
        edge: 1.0,
        render_id: 0.0,
    }
}

pub struct Buffers {
    pub vertex: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub index:  Arc<CpuAccessibleBuffer<[u16]>>,
}

impl Buffers {
    pub fn rect_buffers(device: Arc<Device>, queue: Arc<Queue>, rect: RenderRect) -> Buffers {
        let min_x = rect.p1.0.min(rect.p2.0);
        let min_y = rect.p1.1.min(rect.p2.1);
        let max_x = rect.p1.0.max(rect.p2.0);
        let max_y = rect.p1.1.max(rect.p2.1);

        let vertices: [Vertex; 4] = [
            vertex(min_x, min_y),
            vertex(max_x, min_y),
            vertex(max_x, max_y),
            vertex(min_x, max_y),
        ];

        let indices: [u16; 6] = [
            0, 1, 2,
            0, 2, 3
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
        }
    }

    pub fn rect_outline_buffers(device: Arc<Device>, queue: Arc<Queue>, rect: RenderRect) -> Buffers {
        let width = 0.5;
        let min_x = rect.p1.0.min(rect.p2.0);
        let min_y = rect.p1.1.min(rect.p2.1);
        let max_x = rect.p1.0.max(rect.p2.0);
        let max_y = rect.p1.1.max(rect.p2.1);

        let vertices: [Vertex; 8] = [
            // outer rectangle
            vertex(min_x, min_y),
            vertex(max_x, min_y),
            vertex(max_x, max_y),
            vertex(min_x, max_y),

            // inner rectangle
            vertex(min_x+width, min_y+width),
            vertex(max_x-width, min_y+width),
            vertex(max_x-width, max_y-width),
            vertex(min_x+width, max_y-width),
        ];

        let indices: [u16; 24] = [
            0, 4, 1, 1, 4, 5, // bottom edge
            1, 5, 2, 2, 5, 6, // right edge
            2, 6, 3, 3, 7, 6, // top edge
            3, 7, 0, 0, 4, 7, // left edge
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
        }
    }

    fn new_stage(device: Arc<Device>, queue: Arc<Queue>, stage: &Stage) -> Option<Buffers> {
        if stage.platforms.len() == 0 {
            return None;
        }

        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut indice_count = 0;
        for platform in &stage.platforms[..] {
            vertices.push(vertex(platform.x1, platform.y1));
            vertices.push(vertex(platform.x2, platform.y2));
            vertices.push(vertex(platform.x1, platform.y1 - 2.0));
            vertices.push(vertex(platform.x2, platform.y2 - 2.0));

            indices.push(indice_count + 0);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 3);
            indice_count += 4;
        }

        Some(Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
        })
    }

    fn new_fighter_frame(device: Arc<Device>, queue: Arc<Queue>, frame: &ActionFrame) -> Option<Buffers> {
        if frame.colboxes.len() == 0 {
            return None;
        }

        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut index_count = 0;

        for colbox_or_link in frame.get_colboxes_and_links() {
            match colbox_or_link {
                ColboxOrLink::Colbox (ref colbox) => {
                    let render_id = graphics::get_render_id(&colbox.role);
                    Buffers::gen_colbox(&mut vertices, &mut indices, colbox, &mut index_count, render_id);
                }
                ColboxOrLink::Link (ref link) => {
                    let colbox1 = &frame.colboxes[link.one];
                    let colbox2 = &frame.colboxes[link.two];
                    let render_id1 = graphics::get_render_id(&colbox1.role);
                    Buffers::gen_colbox(&mut vertices, &mut indices, colbox1, &mut index_count, render_id1);
                    let render_id2 = graphics::get_render_id(&colbox2.role);
                    Buffers::gen_colbox(&mut vertices, &mut indices, colbox2, &mut index_count, render_id2);
                    Buffers::gen_link(&mut vertices, &mut indices, link, colbox1, colbox2, &mut index_count, render_id1);
                }
            }
        }

        Some(Buffers {
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
        })
    }

    pub fn gen_colbox(vertices: &mut Vec<Vertex>, indices: &mut Vec<u16>, colbox: &CollisionBox, index_count: &mut u16, render_id: f32) {
        // TODO: maybe bake damage into an extra field on vertex and use to change hitbox render
        let triangles = 25;
        // triangles are drawn meeting at the centre, forming a circle
        let point = &colbox.point;
        vertices.push(Vertex { position: [point.0, point.1], edge: 0.0, render_id: render_id});
        for i in 0..triangles {
            let angle: f32 = ((i * 2) as f32) * consts::PI / (triangles as f32);
            let x = point.0 + angle.cos() * colbox.radius;
            let y = point.1 + angle.sin() * colbox.radius;
            vertices.push(Vertex { position: [x, y], edge: 1.0, render_id: render_id});
            indices.push(*index_count);
            indices.push(*index_count + i);
            indices.push(*index_count + (i + 1) % triangles);
        }
        indices.push(*index_count);
        indices.push(*index_count + 1);
        indices.push(*index_count + triangles - 1);
        *index_count += triangles + 1;
    }

    pub fn gen_link(vertices: &mut Vec<Vertex>, indices: &mut Vec<u16>, link: &CollisionBoxLink, colbox1: &CollisionBox, colbox2: &CollisionBox, index_count: &mut u16, render_id: f32) {
        match link.link_type {
            LinkType::MeldFirst | LinkType::MeldSecond => {
                // draw a rectangle connecting two colboxes
                let (x1, y1)   = colbox1.point;
                let (x2, y2)   = colbox2.point;
                let one_radius = colbox1.radius;
                let two_radius = colbox2.radius;

                let mid_angle = (y1 - y2).atan2(x1 - x2);

                let angle1 = mid_angle + consts::FRAC_PI_2;
                let angle2 = mid_angle - consts::FRAC_PI_2;

                // rectangle as 4 points
                let link_x1 = x1 + angle1.cos() * one_radius;
                let link_x2 = x1 + angle2.cos() * one_radius;
                let link_x3 = x2 + angle1.cos() * two_radius;
                let link_x4 = x2 + angle2.cos() * two_radius;
                let link_x5 = x1;
                let link_x6 = x2;

                let link_y1 = y1 + angle1.sin() * one_radius;
                let link_y2 = y1 + angle2.sin() * one_radius;
                let link_y3 = y2 + angle1.sin() * two_radius;
                let link_y4 = y2 + angle2.sin() * two_radius;
                let link_y5 = y1;
                let link_y6 = y2;

                // rectangle into buffers
                vertices.push(Vertex { position: [link_x1, link_y1], edge: 1.0, render_id: render_id});
                vertices.push(Vertex { position: [link_x2, link_y2], edge: 1.0, render_id: render_id});
                vertices.push(Vertex { position: [link_x3, link_y3], edge: 1.0, render_id: render_id});
                vertices.push(Vertex { position: [link_x4, link_y4], edge: 1.0, render_id: render_id});
                vertices.push(Vertex { position: [link_x5, link_y5], edge: 0.0, render_id: render_id});
                vertices.push(Vertex { position: [link_x6, link_y6], edge: 0.0, render_id: render_id});

                indices.push(*index_count);
                indices.push(*index_count + 4);
                indices.push(*index_count + 5);

                indices.push(*index_count + 0);
                indices.push(*index_count + 2);
                indices.push(*index_count + 5);

                indices.push(*index_count + 1);
                indices.push(*index_count + 3);
                indices.push(*index_count + 4);

                indices.push(*index_count + 3);
                indices.push(*index_count + 4);
                indices.push(*index_count + 5);
                *index_count += 6;
            },
            LinkType::Simple => { },
        }
    }

    pub fn new_player(device: Arc<Device>, queue: Arc<Queue>, player: &RenderPlayer) -> Buffers {
        // ecb
        let vertex0 = vertex(player.ecb.bot_x,   player.ecb.bot_y);
        let vertex1 = vertex(player.ecb.left_x,  player.ecb.left_y);
        let vertex2 = vertex(player.ecb.right_x, player.ecb.right_y);
        let vertex3 = vertex(player.ecb.top_x,   player.ecb.top_y);

        // horizontal bps
        let vertex4 = vertex(-4.0, -0.15);
        let vertex5 = vertex(-4.0,  0.15);
        let vertex6 = vertex( 4.0, -0.15);
        let vertex7 = vertex( 4.0,  0.15);

        // vertical bps
        let vertex8  = vertex(-0.15, -4.0);
        let vertex9  = vertex( 0.15, -4.0);
        let vertex10 = vertex(-0.15,  4.0);
        let vertex11 = vertex( 0.15,  4.0);

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
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
        }
    }
}

pub struct PackageBuffers {
    pub stages:   HashMap<String, Option<Buffers>>,
    pub fighters: HashMap<String, Vec<Vec<Option<Buffers>>>>, // fighters <- actions <- frames
    pub package:  Option<Package>,
}

impl PackageBuffers {
    pub fn new() -> PackageBuffers {
        PackageBuffers {
            stages:   HashMap::new(),
            fighters: HashMap::new(),
            package:  None,
        }
    }

    /// Update internal copy of the package and buffers
    pub fn update(&mut self, device: Arc<Device>, queue: Arc<Queue>, package_updates: Vec<PackageUpdate>) {
        for update in package_updates {
            match update {
                PackageUpdate::Package (package) => {
                    self.stages = HashMap::new();
                    self.fighters = HashMap::new();

                    for (key, fighter) in package.fighters.key_value_iter() {
                        let mut action_buffers: Vec<Vec<Option<Buffers>>> = vec!();
                        for action in &fighter.actions[..] {
                            let mut frame_buffers: Vec<Option<Buffers>> = vec!();
                            for frame in &action.frames[..] {
                                frame_buffers.push(Buffers::new_fighter_frame(device.clone(), queue.clone(), frame));
                            }
                            action_buffers.push(frame_buffers);
                        }
                        self.fighters.insert(key.clone(), action_buffers);
                    }

                    for (key, stage) in package.stages.key_value_iter() {
                        self.stages.insert(key.clone(), Buffers::new_stage(device.clone(), queue.clone(), &stage));
                    }
                    self.package = Some(package);
                }
                PackageUpdate::DeleteFighterFrame { fighter, action, frame_index } => {
                    let fighter: &str = &fighter;
                    self.fighters.get_mut(fighter).unwrap()[action].remove(frame_index);
                    if let &mut Some(ref mut package) = &mut self.package {
                        package.fighters[fighter].actions[action].frames.remove(frame_index);
                    }
                }
                PackageUpdate::InsertFighterFrame { fighter, action, frame_index, frame } => {
                    let fighter: &str = &fighter;
                    let buffers = Buffers::new_fighter_frame(device.clone(), queue.clone(), &frame);
                    self.fighters.get_mut(fighter).unwrap()[action].insert(frame_index, buffers);
                    if let &mut Some(ref mut package) = &mut self.package {
                        package.fighters[fighter].actions[action].frames.insert(frame_index, frame);
                    }
                }
                PackageUpdate::DeleteStage { index, key } => {
                    self.stages.remove(&key);
                    if let &mut Some(ref mut package) = &mut self.package {
                        package.stages.remove(index);
                    }
                }
                PackageUpdate::InsertStage { index, key, stage } => {
                    self.stages.insert(key.clone(), Buffers::new_stage(device.clone(), queue.clone(), &stage));
                    if let &mut Some(ref mut package) = &mut self.package {
                        package.stages.insert(index, key, stage);
                    }
                }
            }
        }
    }

    pub fn fighter_frame_colboxes(&self, device: Arc<Device>, queue: Arc<Queue>, fighter: &str, action: usize, frame: usize, selected: &HashSet<usize>) -> Buffers {
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut index_count = 0;

        if let &Some(ref package) = &self.package {
            let colboxes = &package.fighters[fighter].actions[action].frames[frame].colboxes;
            for (i, colbox) in colboxes.iter().enumerate() {
                if selected.contains(&i) {
                    Buffers::gen_colbox(&mut vertices, &mut indices, colbox, &mut index_count, 0.0);
                }
            }
        }

        Buffers {
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), indices.iter().cloned()).unwrap(),
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), Some(queue.family()), vertices.iter().cloned()).unwrap(),
        }
    }

}
