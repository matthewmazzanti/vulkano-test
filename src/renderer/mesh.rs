extern crate lyon;
use lyon::math::{point, Point};
use lyon::path::Path;
use lyon::path::builder::*;
use lyon::tessellation::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 2],
}

pub fn ship_mesh() -> Vec<Vertex> {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, -1.0));
    builder.line_to(point(-1.0, 1.0));
    builder.line_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0));

    let path = builder.build();
    let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    {
        // Compute the tessellation.
        tessellator.tessellate_path(
            &path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |pos: Point, _: FillAttributes| {
                Vertex {
                    pos: pos.to_array(),
                }
            }),
        ).unwrap();
    }

    geometry.indices
        .iter()
        .map(|i| geometry.vertices[usize::from(*i)])
        .collect::<Vec<Vertex>>()
}

pub fn asteroid_mesh() -> Vec<Vertex> {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
    builder.cubic_bezier_to(point(2.0, 2.0), point(0.0, 2.0), point(0.0, 0.0));
    builder.close();

    let path = builder.build();// Build a simple path.

    let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    {
        // Compute the tessellation.
        tessellator.tessellate_path(
            &path,
            &FillOptions::default().with_tolerance(0.001),
            &mut BuffersBuilder::new(&mut geometry, |pos: Point, _: FillAttributes| {
                Vertex {
                    pos: pos.to_array(),
                }
            }),
        ).unwrap();
    }

    geometry.indices
        .iter()
        .map(|i| geometry.vertices[usize::from(*i)])
        .collect::<Vec<Vertex>>()
}

