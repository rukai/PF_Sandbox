use pf_sandbox_lib::fighter::{ActionFrame, LinkType, ColboxOrLink, CollisionBox, CollisionBoxLink};
use pf_sandbox_lib::geometry::Rect;
use pf_sandbox_lib::stage::Surface;
use pf_sandbox_lib::package::{Package, PackageUpdate};
use player::RenderShield;
use graphics;
use player::RenderPlayer;
use game::SurfaceSelection;

use treeflection::Node;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::device::Device;

use lyon::path::default::Path;
use lyon::path::builder::FlatPathBuilder;
use lyon::math::point;
use lyon::tessellation::{VertexBuffers, FillVertex};
use lyon::tessellation::{FillTessellator, FillOptions};
use lyon::tessellation::{VertexConstructor, BuffersBuilder};

use std::collections::{HashSet, HashMap};
use std::f32::consts;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub edge: f32,
    pub render_id: u32,
}
impl_vertex!(Vertex, position, edge, render_id);

fn vertex(x: f32, y: f32) -> Vertex {
    Vertex {
        position: [x, y],
        edge: 1.0,
        render_id: 0,
    }
}

#[derive(Debug, Clone)]
pub struct ColorVertex {
    pub position:  [f32; 2],
    pub color:     [f32; 4],
}
impl_vertex!(ColorVertex, position, color);

fn colorvertex(x: f32, y: f32, color: [f32; 4]) -> ColorVertex {
    ColorVertex {
        position: [x, y],
        color
    }
}

struct StageVertexConstructor;
impl VertexConstructor<FillVertex, ColorVertex> for StageVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> ColorVertex {
        ColorVertex {
            position: vertex.position.to_array(),
            color:    [0.16, 0.16, 0.16, 1.0]
        }
    }
}

#[derive(Clone)]
pub struct Buffers {
    pub vertex: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub index:  Arc<CpuAccessibleBuffer<[u16]>>,
}

#[derive(Clone)]
pub struct ColorBuffers {
    pub vertex: Arc<CpuAccessibleBuffer<[ColorVertex]>>,
    pub index:  Arc<CpuAccessibleBuffer<[u16]>>,
}

impl Buffers {
    /// Creates a single circle with radius 1 around the origin
    pub fn new_circle(device: Arc<Device>) -> Buffers {
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();

        let iterations = 40;

        vertices.push(Vertex { position: [0.0, 0.0], edge: 0.0, render_id: 0});
        for i in 0..iterations {
            let angle = i as f32 * 2.0 * consts::PI / (iterations as f32);
            let (sin, cos) = angle.sin_cos();
            vertices.push(Vertex { position: [cos, sin], edge: 1.0, render_id: 0});
            indices.push(0);
            indices.push(i + 1);
            indices.push((i + 1) % iterations + 1);
        }

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }

    /// Creates a single triangle with sides of length 1
    pub fn new_triangle(device: Arc<Device>) -> Buffers {
        let h = ((3.0/4.0) as f32).sqrt();
        let vertices = [
            Vertex { position: [0.0,    h  ], edge: 0.0, render_id: 0 },
            Vertex { position: [h/-2.0, 0.0], edge: 0.0, render_id: 0 },
            Vertex { position: [h/2.0,  0.0], edge: 0.0, render_id: 0 }
        ];

        let indices = [0, 1, 2];
        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }

    pub fn new_shield(device: Arc<Device>, shield: &RenderShield) -> Buffers {
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();

        let triangles = match shield.distort {
            0 => 100,
            1 => 20,
            2 => 10,
            3 => 8,
            4 => 7,
            5 => 6,
            _ => 5
        };

        // triangles are drawn meeting at the centre, forming a circle
        vertices.push(Vertex { position: [0.0, 0.0], edge: 0.0, render_id: 0});
        for i in 0..triangles {
            let angle = i as f32 * 2.0 * consts::PI / (triangles as f32);
            let (sin, cos) = angle.sin_cos();
            let x = cos * shield.radius;
            let y = sin * shield.radius;
            vertices.push(Vertex { position: [x, y], edge: 1.0, render_id: 0});
            indices.push(0);
            indices.push(i + 1);
            indices.push((i + 1) % triangles + 1);
        }

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }

    pub fn new_spawn_point(device: Arc<Device>) -> Buffers {
        let vertices: [Vertex; 11] = [
            // vertical bar
            vertex(-0.15, -4.0),
            vertex( 0.15, -4.0),
            vertex(-0.15,  4.0),
            vertex( 0.15,  4.0),

            // horizontal bar
            vertex(-4.0, -0.15),
            vertex(-4.0,  0.15),
            vertex( 4.0, -0.15),
            vertex( 4.0,  0.15),

            // arrow head
            vertex(4.2, 0.0),
            vertex(3.0, -1.0),
            vertex(3.0, 1.0),
        ];

        let indices: [u16; 15] = [
            // vertical bar
            0, 1, 2,
            3, 2, 1,

            // horizontal bar
            4, 5, 6,
            7, 6, 5,

            // arrow head
            8, 9, 10
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }

    pub fn new_arrow(device: Arc<Device>) -> Buffers {
        let vertices: [Vertex; 7] = [
            // stick
            vertex(-0.7, 0.0),
            vertex(0.7, 0.0),
            vertex(-0.7, 10.0),
            vertex(0.7, 10.0),

            // head
            vertex(0.0, 12.0),
            vertex(-2.2, 10.0),
            vertex(2.2, 10.0),
        ];

        let indices: [u16; 9] = [
            // stick
            0, 1, 2,
            1, 2, 3,

            //head
            4, 5, 6
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }

    pub fn rect_buffers(device: Arc<Device>, rect: Rect) -> Buffers {
        let left  = rect.left();
        let right = rect.right();
        let bot   = rect.bot();
        let top   = rect.top();

        let vertices: [Vertex; 4] = [
            vertex(left,  bot),
            vertex(right, bot),
            vertex(right, top),
            vertex(left,  top),
        ];

        let indices: [u16; 6] = [
            0, 1, 2,
            0, 2, 3
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }

    pub fn rect_outline_buffers(device: Arc<Device>, rect: &Rect) -> Buffers {
        let width = 0.5;
        let left  = rect.left();
        let right = rect.right();
        let bot   = rect.bot();
        let top   = rect.top();

        let vertices: [Vertex; 8] = [
            // outer rectangle
            vertex(left,  bot),
            vertex(right, bot),
            vertex(right, top),
            vertex(left,  top),

            // inner rectangle
            vertex(left+width,  bot+width),
            vertex(right-width, bot+width),
            vertex(right-width, top-width),
            vertex(left+width,  top-width),
        ];

        let indices: [u16; 24] = [
            0, 4, 1, 1, 4, 5, // bottom edge
            1, 5, 2, 2, 5, 6, // right edge
            2, 6, 3, 3, 7, 6, // top edge
            3, 7, 0, 0, 4, 7, // left edge
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }

    pub fn new_selected_surfaces(device: Arc<Device>, surfaces: &[Surface], selected_surfaces: &HashSet<SurfaceSelection>) -> Option<ColorBuffers> {
        if surfaces.len() == 0 {
            return None;
        }

        let mut vertices: Vec<ColorVertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut indice_count = 0;
        let color = [0.0, 1.0, 0.0, 1.0];
        for (i, surface) in surfaces.iter().enumerate() {
            let x_mid = (surface.x1 + surface.x2) / 2.0;
            let y_mid = (surface.y1 + surface.y2) / 2.0;

            let angle = surface.render_angle() - 90f32.to_radians();
            let d_x = angle.cos() / 4.0;
            let d_y = angle.sin() / 4.0;

            if selected_surfaces.contains(&SurfaceSelection::P1(i)) {
                vertices.push(colorvertex(x_mid      + d_x, y_mid      + d_y, color));
                vertices.push(colorvertex(surface.x1 + d_x, surface.y1 + d_y, color));
                vertices.push(colorvertex(surface.x1 - d_x, surface.y1 - d_y, color));
                vertices.push(colorvertex(x_mid      - d_x, y_mid      - d_y, color));

                indices.push(indice_count + 0);
                indices.push(indice_count + 1);
                indices.push(indice_count + 2);
                indices.push(indice_count + 0);
                indices.push(indice_count + 2);
                indices.push(indice_count + 3);
                indice_count += 4;
            }
            if selected_surfaces.contains(&SurfaceSelection::P2(i)) {
                vertices.push(colorvertex(x_mid      + d_x, y_mid      + d_y, color));
                vertices.push(colorvertex(surface.x2 + d_x, surface.y2 + d_y, color));
                vertices.push(colorvertex(surface.x2 - d_x, surface.y2 - d_y, color));
                vertices.push(colorvertex(x_mid      - d_x, y_mid      - d_y, color));

                indices.push(indice_count + 0);
                indices.push(indice_count + 1);
                indices.push(indice_count + 2);
                indices.push(indice_count + 0);
                indices.push(indice_count + 2);
                indices.push(indice_count + 3);
                indice_count += 4;
            }
        }

        Some(ColorBuffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        })
    }

    pub fn new_surfaces(device: Arc<Device>, surfaces: &[Surface]) -> Option<ColorBuffers> {
        if surfaces.len() == 0 {
            return None;
        }

        let mut vertices: Vec<ColorVertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut indice_count = 0;

        for surface in surfaces {
            let r = if surface.is_pass_through() { 0.4 } else if surface.floor.is_some() { 0.6 } else { 0.0 };
            let g = if surface.ceiling { 0.5 } else { 0.0 };
            let b = if surface.wall { 0.5 } else { 0.0 };
            let color = [1.0 - g - b, 1.0 - r - b, 1.0 - r - g, 1.0];

            let angle = surface.render_angle() - 90f32.to_radians();
            let d_x = angle.cos() / 4.0;
            let d_y = angle.sin() / 4.0;

            vertices.push(colorvertex(surface.x1 + d_x, surface.y1 + d_y, color));
            vertices.push(colorvertex(surface.x2 + d_x, surface.y2 + d_y, color));
            vertices.push(colorvertex(surface.x2 - d_x, surface.y2 - d_y, color));
            vertices.push(colorvertex(surface.x1 - d_x, surface.y1 - d_y, color));

            indices.push(indice_count + 0);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 0);
            indices.push(indice_count + 2);
            indices.push(indice_count + 3);
            indice_count += 4;
        }

        Some(ColorBuffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        })
    }

    // TODO: Combine new_surfaces(..) and new_surfaces_fill(..), waiting on: https://github.com/nical/lyon/issues/224
    pub fn new_surfaces_fill(device: Arc<Device>, surfaces: &[Surface]) -> Option<ColorBuffers> {
        if surfaces.len() == 0 {
            return None;
        }

        let mut builder = Path::builder();
        let mut used: Vec<usize> = vec!();
        let mut cant_loop: Vec<usize> = vec!(); // optimization, so we dont have to keep rechecking surfaces that will never loop

        for (i, surface) in surfaces.iter().enumerate() {
            if used.contains(&i) {
                continue;
            }

            let mut loop_elements: Vec<usize> = vec!(i);
            let mut found_loop = false;
            let mut prev_surface = surface;
            if !cant_loop.contains(&i) {
                'loop_search: loop {
                    for (j, check_surface) in surfaces.iter().enumerate() {
                        if  i != j && !loop_elements.contains(&j) && !used.contains(&j) &&
                            (
                                check_surface.x1 == prev_surface.x1 && check_surface.y1 == prev_surface.y1 ||
                                check_surface.x1 == prev_surface.x2 && check_surface.y1 == prev_surface.y2 ||
                                check_surface.x2 == prev_surface.x1 && check_surface.y2 == prev_surface.y1 ||
                                check_surface.x2 == prev_surface.x2 && check_surface.y2 == prev_surface.y2
                            )
                        {
                            loop_elements.push(j);
                            if  loop_elements.len() > 2 &&
                                (
                                    check_surface.x1 == surface.x1 && check_surface.y1 == surface.y1 ||
                                    check_surface.x1 == surface.x2 && check_surface.y1 == surface.y2 ||
                                    check_surface.x2 == surface.x1 && check_surface.y2 == surface.y1 ||
                                    check_surface.x2 == surface.x2 && check_surface.y2 == surface.y2
                                )
                            {
                                found_loop = true;
                                break 'loop_search // completed a loop
                            }
                            else {
                                prev_surface = check_surface;
                                continue 'loop_search; // found a loop element, start the loop_search again to find the next loop element.
                            }
                        }
                    }
                    break 'loop_search // loop search exhausted
                }
            }

            if found_loop {
                let mut loop_elements_iter = loop_elements.iter().cloned();
                let first_surface_i = loop_elements_iter.next().unwrap();
                used.push(first_surface_i);

                let first_surface = &surfaces[first_surface_i];
                let second_surface = &surfaces[loop_elements[1]];
                let start_p1 = first_surface.x1 == second_surface.x1 && first_surface.y1 == second_surface.y1 ||
                               first_surface.x1 == second_surface.x2 && first_surface.y1 == second_surface.y2;
                let mut prev_x = if start_p1 { first_surface.x1 } else { first_surface.x2 };
                let mut prev_y = if start_p1 { first_surface.y1 } else { first_surface.y2 };
                builder.move_to(point(prev_x, prev_y));

                for j in loop_elements_iter {
                    let surface = &surfaces[j];
                    if surface.x1 == prev_x && surface.y1 == prev_y {
                        prev_x = surface.x2;
                        prev_y = surface.y2;
                    }
                    else {
                        prev_x = surface.x1;
                        prev_y = surface.y1;
                    }
                    builder.line_to(point(prev_x, prev_y));
                    used.push(j);
                }
                builder.close();
            }
            else {
                for j in loop_elements {
                    cant_loop.push(j);
                }
            }
            used.push(i);
        }

        let path = builder.build();
        let mut tessellator = FillTessellator::new();
        let mut mesh = VertexBuffers::new();
        tessellator.tessellate_path(
            path.path_iter(),
            &FillOptions::tolerance(0.01),
            &mut BuffersBuilder::new(&mut mesh, StageVertexConstructor)
        ).unwrap();

        Some(ColorBuffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), mesh.vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), mesh.indices.iter().cloned()).unwrap(),
        })
    }

    fn new_fighter_frame(device: Arc<Device>, frame: &ActionFrame) -> Option<Buffers> {
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
                    Buffers::gen_link(&mut vertices, &mut indices, link, colbox1, colbox2, &mut index_count);
                }
            }
        }

        Some(Buffers {
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
        })
    }

    pub fn gen_colbox(vertices: &mut Vec<Vertex>, indices: &mut Vec<u16>, colbox: &CollisionBox, index_count: &mut u16, render_id: u32) {
        let triangles = 60;
        // triangles are drawn meeting at the centre, forming a circle
        let point = &colbox.point;
        vertices.push(Vertex { position: [point.0, point.1], edge: 0.0, render_id});
        for i in 0..triangles {
            let angle = i as f32 * 2.0 * consts::PI / (triangles as f32);
            let (sin, cos) = angle.sin_cos();
            let x = point.0 + cos * colbox.radius;
            let y = point.1 + sin * colbox.radius;
            vertices.push(Vertex { position: [x, y], edge: 1.0, render_id });
            indices.push(*index_count);
            indices.push(*index_count + i + 1);
            indices.push(*index_count + (i + 1) % triangles + 1);
        }
        *index_count += triangles + 1;
    }

    pub fn gen_link(vertices: &mut Vec<Vertex>, indices: &mut Vec<u16>, link: &CollisionBoxLink, colbox1: &CollisionBox, colbox2: &CollisionBox, index_count: &mut u16) {
        let render_id1 = graphics::get_render_id(&colbox1.role);
        let render_id2 = graphics::get_render_id(&colbox2.role);

        let render_id_link = render_id1;
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
                vertices.push(Vertex { position: [link_x1, link_y1], edge: 1.0, render_id: render_id_link });
                vertices.push(Vertex { position: [link_x2, link_y2], edge: 1.0, render_id: render_id_link });
                vertices.push(Vertex { position: [link_x3, link_y3], edge: 1.0, render_id: render_id_link });
                vertices.push(Vertex { position: [link_x4, link_y4], edge: 1.0, render_id: render_id_link });
                vertices.push(Vertex { position: [link_x5, link_y5], edge: 0.0, render_id: render_id_link });
                vertices.push(Vertex { position: [link_x6, link_y6], edge: 0.0, render_id: render_id_link });

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

                let triangles = 30;

                // draw colbox1, triangles are drawn meeting at the centre, forming a circle
                vertices.push(Vertex { position: [x1, y1], edge: 0.0, render_id: render_id2 });
                for i in 0..triangles + 1 {
                    let angle = angle2 + i as f32 * consts::PI / (triangles as f32);
                    let (sin, cos) = angle.sin_cos();
                    let x = x1 + cos * colbox1.radius;
                    let y = y1 + sin * colbox1.radius;
                    vertices.push(Vertex { position: [x, y], edge: 1.0, render_id: render_id1 });
                }
                for i in 0..triangles {
                    indices.push(*index_count);
                    indices.push(*index_count + i + 1);
                    indices.push(*index_count + i + 2);
                }
                *index_count += triangles + 2;

                // draw colbox2, triangles are drawn meeting at the centre, forming a circle
                vertices.push(Vertex { position: [x2, y2], edge: 0.0, render_id: render_id2 });
                for i in 0..triangles + 1 {
                    let angle = angle1 + i as f32 * consts::PI / (triangles as f32);
                    let (sin, cos) = angle.sin_cos();
                    let x = x2 + cos * colbox2.radius;
                    let y = y2 + sin * colbox2.radius;
                    vertices.push(Vertex { position: [x, y], edge: 1.0, render_id: render_id2 });
                }
                for i in 0..triangles {
                    indices.push(*index_count);
                    indices.push(*index_count + i + 1);
                    indices.push(*index_count + i + 2);
                }
                *index_count += triangles + 2;
            },
            LinkType::Simple => { },
        }
    }

    pub fn new_player(device: Arc<Device>, player: &RenderPlayer) -> Buffers {
        let mid_y = (player.ecb.top + player.ecb.bottom) / 2.0;
        let vertices: [Vertex; 12] = [
            // ecb
            vertex(0.0,              player.ecb.bottom),
            vertex(player.ecb.left,  mid_y),
            vertex(player.ecb.right, mid_y),
            vertex(0.0,              player.ecb.top),

            // horizontal bps
            vertex(-4.0, -0.15),
            vertex(-4.0,  0.15),
            vertex( 4.0, -0.15),
            vertex( 4.0,  0.15),

            // vertical bps
            vertex(-0.15, -4.0),
            vertex( 0.15, -4.0),
            vertex(-0.15,  4.0),
            vertex( 0.15,  4.0),
        ];

        let indices: [u16; 18] = [
            1,  2,  0,
            1,  2,  3,
            4,  5,  6,
            7,  6,  5,
            8,  9,  10,
            11, 10, 13,
        ];

        Buffers {
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
        }
    }
}

pub struct PackageBuffers {
    pub stages:      HashMap<String, Option<ColorBuffers>>, // Only used in menu, ingame stages are recreated every frame
    pub stages_fill: HashMap<String, Option<ColorBuffers>>, // Only used in menu, ingame stages are recreated every frame
    pub fighters:    HashMap<String, Vec<Vec<Option<Buffers>>>>, // fighters <- actions <- frames
    pub package:     Option<Package>,
}

impl PackageBuffers {
    pub fn new() -> PackageBuffers {
        PackageBuffers {
            stages:      HashMap::new(),
            stages_fill: HashMap::new(),
            fighters:    HashMap::new(),
            package:     None,
        }
    }

    /// Update internal copy of the package and buffers
    pub fn update(&mut self, device: Arc<Device>, package_updates: Vec<PackageUpdate>) {
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
                                frame_buffers.push(Buffers::new_fighter_frame(device.clone(), frame));
                            }
                            action_buffers.push(frame_buffers);
                        }
                        self.fighters.insert(key.clone(), action_buffers);
                    }

                    for (key, stage) in package.stages.key_value_iter() {
                        self.stages     .insert(key.clone(), Buffers::new_surfaces     (device.clone(), &stage.surfaces));
                        self.stages_fill.insert(key.clone(), Buffers::new_surfaces_fill(device.clone(), &stage.surfaces));
                    }
                    self.package = Some(package);
                }
                PackageUpdate::Command { runner, fighter_context, stage_context } => {
                    if let &mut Some(ref mut package) = &mut self.package {
                        // TODO: apply context from fighter_context and stage_context

                        package.node_step(runner);

                        self.stages = HashMap::new();
                        self.fighters = HashMap::new();

                        for (key, fighter) in package.fighters.key_value_iter() {
                            let mut action_buffers: Vec<Vec<Option<Buffers>>> = vec!();
                            for action in &fighter.actions[..] {
                                let mut frame_buffers: Vec<Option<Buffers>> = vec!();
                                for frame in &action.frames[..] {
                                    frame_buffers.push(Buffers::new_fighter_frame(device.clone(), frame));
                                }
                                action_buffers.push(frame_buffers);
                            }
                            self.fighters.insert(key.clone(), action_buffers);
                        }

                        for (key, stage) in package.stages.key_value_iter() {
                            self.stages     .insert(key.clone(), Buffers::new_surfaces     (device.clone(), &stage.surfaces));
                            self.stages_fill.insert(key.clone(), Buffers::new_surfaces_fill(device.clone(), &stage.surfaces));
                        }
                    }
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
                    let buffers = Buffers::new_fighter_frame(device.clone(), &frame);
                    self.fighters.get_mut(fighter).unwrap()[action].insert(frame_index, buffers);
                    if let &mut Some(ref mut package) = &mut self.package {
                        package.fighters[fighter].actions[action].frames.insert(frame_index, frame);
                    }
                }
                PackageUpdate::DeleteStage { index, key } => {
                    self.stages.remove(&key);
                    self.stages_fill.remove(&key);
                    if let &mut Some(ref mut package) = &mut self.package {
                        package.stages.remove(index);
                    }
                }
                PackageUpdate::InsertStage { index, key, stage } => {
                    self.stages     .insert(key.clone(), Buffers::new_surfaces     (device.clone(), &stage.surfaces));
                    self.stages_fill.insert(key.clone(), Buffers::new_surfaces_fill(device.clone(), &stage.surfaces));
                    if let &mut Some(ref mut package) = &mut self.package {
                        package.stages.insert(index, key, stage);
                    }
                }
            }
        }
    }

    pub fn new_fighter_frame_colboxes(&self, device: Arc<Device>, fighter: &str, action: usize, frame: usize, selected: &HashSet<usize>) -> Buffers {
        let mut vertices: Vec<Vertex> = vec!();
        let mut indices: Vec<u16> = vec!();
        let mut index_count = 0;

        if let &Some(ref package) = &self.package {
            let colboxes = &package.fighters[fighter].actions[action].frames[frame].colboxes;
            for (i, colbox) in colboxes.iter().enumerate() {
                if selected.contains(&i) {
                    Buffers::gen_colbox(&mut vertices, &mut indices, colbox, &mut index_count, 0);
                }
            }
        }

        Buffers {
            index:  CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), indices.iter().cloned()).unwrap(),
            vertex: CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned()).unwrap(),
        }
    }
}
