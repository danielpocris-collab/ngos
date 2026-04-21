use crate::RenderError;
use alloc::vec::Vec;
use ngos_gfx_translate::RgbaColor;

fn sqrt_f32(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x / 2.0;
    for _ in 0..10 {
        guess = (guess + x / guess) / 2.0;
    }
    guess
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub color: RgbaColor,
}

impl Vertex {
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        tex_coord: [f32; 2],
        color: RgbaColor,
    ) -> Self {
        Vertex {
            position,
            normal,
            tex_coord,
            color,
        }
    }

    pub fn position_only(position: [f32; 3]) -> Self {
        Vertex {
            position,
            normal: [0.0, 1.0, 0.0],
            tex_coord: [0.0, 0.0],
            color: RgbaColor {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeshId(pub u32);

impl MeshId {
    pub fn new(id: u32) -> Self {
        MeshId(id)
    }
}

#[derive(Debug, Clone)]
pub struct IndexBuffer {
    indices: Vec<u32>,
}

impl IndexBuffer {
    pub fn new(indices: Vec<u32>) -> Result<Self, RenderError> {
        if indices.is_empty() {
            return Err(RenderError::InvalidIndexCount { count: 0 });
        }
        if indices.len() % 3 != 0 {
            return Err(RenderError::InvalidIndexCount {
                count: indices.len(),
            });
        }
        Ok(IndexBuffer { indices })
    }

    pub fn triangles(&self) -> impl Iterator<Item = [u32; 3]> + '_ {
        self.indices
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    id: MeshId,
    vertices: Vec<Vertex>,
    index_buffer: Option<IndexBuffer>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Mesh {
    pub fn new(id: MeshId, vertices: Vec<Vertex>) -> Result<Self, RenderError> {
        if vertices.is_empty() {
            return Err(RenderError::InvalidVertexCount { count: 0 });
        }
        Ok(Mesh {
            id,
            vertices,
            index_buffer: None,
        })
    }

    pub fn with_indices(mut self, indices: Vec<u32>) -> Result<Self, RenderError> {
        self.index_buffer = Some(IndexBuffer::new(indices)?);
        Ok(self)
    }

    pub fn id(&self) -> MeshId {
        self.id
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_buffer(&self) -> Option<&IndexBuffer> {
        self.index_buffer.as_ref()
    }

    pub fn triangle_count(&self) -> usize {
        match &self.index_buffer {
            Some(ib) => ib.len() / 3,
            None => self.vertices.len() / 3,
        }
    }

    pub fn bounds(&self) -> MeshBounds {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for vertex in &self.vertices {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex.position[axis]);
                max[axis] = max[axis].max(vertex.position[axis]);
            }
        }
        MeshBounds { min, max }
    }

    pub fn center(&self) -> [f32; 3] {
        let bounds = self.bounds();
        [
            (bounds.min[0] + bounds.max[0]) * 0.5,
            (bounds.min[1] + bounds.max[1]) * 0.5,
            (bounds.min[2] + bounds.max[2]) * 0.5,
        ]
    }

    pub fn get_triangle(&self, index: usize) -> Option<[Vertex; 3]> {
        match &self.index_buffer {
            Some(ib) => {
                let indices: Vec<[u32; 3]> = ib.triangles().collect();
                indices.get(index).map(|&[i0, i1, i2]| {
                    [
                        self.vertices[i0 as usize],
                        self.vertices[i1 as usize],
                        self.vertices[i2 as usize],
                    ]
                })
            }
            None => {
                let start = index * 3;
                if start + 2 >= self.vertices.len() {
                    return None;
                }
                Some([
                    self.vertices[start],
                    self.vertices[start + 1],
                    self.vertices[start + 2],
                ])
            }
        }
    }

    pub fn compute_normal(p0: &[f32; 3], p1: &[f32; 3], p2: &[f32; 3]) -> [f32; 3] {
        let edge1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let edge2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let mut normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];
        let len = sqrt_f32(normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]);
        if len > 0.0 {
            normal[0] /= len;
            normal[1] /= len;
            normal[2] /= len;
        }
        normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_color() -> RgbaColor {
        RgbaColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    fn test_vertex(x: f32, y: f32, z: f32) -> Vertex {
        Vertex::new([x, y, z], [0.0, 1.0, 0.0], [0.0, 0.0], test_color())
    }

    #[test]
    fn vertex_creation() {
        let v = test_vertex(1.0, 2.0, 3.0);
        assert_eq!(v.position, [1.0, 2.0, 3.0]);
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn vertex_position_only() {
        let v = Vertex::position_only([1.0, 0.0, 0.0]);
        assert_eq!(v.position, [1.0, 0.0, 0.0]);
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
        assert_eq!(v.tex_coord, [0.0, 0.0]);
    }

    #[test]
    fn mesh_id_creation() {
        let id = MeshId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn index_buffer_valid() {
        let ib = IndexBuffer::new(vec![0, 1, 2, 2, 1, 3]).unwrap();
        assert_eq!(ib.len(), 6);
        assert_eq!(ib.triangles().count(), 2);
    }

    #[test]
    fn index_buffer_empty_rejected() {
        let err = IndexBuffer::new(vec![]).unwrap_err();
        assert!(matches!(err, RenderError::InvalidIndexCount { count: 0 }));
    }

    #[test]
    fn index_buffer_non_multiple_of_three_rejected() {
        let err = IndexBuffer::new(vec![0, 1, 2, 3]).unwrap_err();
        assert!(matches!(err, RenderError::InvalidIndexCount { count: 4 }));
    }

    #[test]
    fn mesh_creation() {
        let vertices = vec![
            test_vertex(0.0, 0.0, 0.0),
            test_vertex(1.0, 0.0, 0.0),
            test_vertex(0.0, 1.0, 0.0),
        ];
        let mesh = Mesh::new(MeshId::new(1), vertices).unwrap();
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn mesh_empty_vertices_rejected() {
        let err = Mesh::new(MeshId::new(1), vec![]).unwrap_err();
        assert!(matches!(err, RenderError::InvalidVertexCount { count: 0 }));
    }

    #[test]
    fn mesh_with_indices() {
        let vertices = vec![
            test_vertex(0.0, 0.0, 0.0),
            test_vertex(1.0, 0.0, 0.0),
            test_vertex(0.0, 1.0, 0.0),
            test_vertex(1.0, 1.0, 0.0),
        ];
        let mesh = Mesh::new(MeshId::new(1), vertices)
            .unwrap()
            .with_indices(vec![0, 1, 2, 2, 1, 3])
            .unwrap();
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        assert!(mesh.index_buffer().is_some());
    }

    #[test]
    fn mesh_get_triangle_indexed() {
        let vertices = vec![
            test_vertex(0.0, 0.0, 0.0),
            test_vertex(1.0, 0.0, 0.0),
            test_vertex(0.0, 1.0, 0.0),
            test_vertex(1.0, 1.0, 0.0),
        ];
        let mesh = Mesh::new(MeshId::new(1), vertices)
            .unwrap()
            .with_indices(vec![0, 1, 2, 2, 1, 3])
            .unwrap();
        let tri0 = mesh.get_triangle(0).unwrap();
        assert_eq!(tri0[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(tri0[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(tri0[2].position, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn mesh_get_triangle_out_of_bounds() {
        let vertices = vec![
            test_vertex(0.0, 0.0, 0.0),
            test_vertex(1.0, 0.0, 0.0),
            test_vertex(0.0, 1.0, 0.0),
        ];
        let mesh = Mesh::new(MeshId::new(1), vertices).unwrap();
        assert!(mesh.get_triangle(1).is_none());
    }

    #[test]
    fn compute_normal_flat_triangle() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [1.0, 0.0, 0.0];
        let p2 = [0.0, 1.0, 0.0];
        let normal = Mesh::compute_normal(&p0, &p1, &p2);
        assert!((normal[0] - 0.0).abs() < 1e-4);
        assert!((normal[1] - 0.0).abs() < 1e-4);
        assert!((normal[2] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn compute_normal_normalized() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [2.0, 0.0, 0.0];
        let p2 = [0.0, 2.0, 0.0];
        let normal = Mesh::compute_normal(&p0, &p1, &p2);
        let len = sqrt_f32(normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]);
        assert!((len - 1.0).abs() < 1e-4);
    }
}
