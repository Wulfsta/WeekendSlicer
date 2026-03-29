//! mesh2frep
//!

use nalgebra::{Matrix3, Vector3, Vector4};
use tritet::{InputDataTetMesh, Tetgen};

const BBOX_EXPANSION: f64 = 2.0;
const RHAI_UNIT_TETRAHEDRON: &str = include_str!("rhai/unit_tetrahedron.rhai");

/// A closed, orientable triangulated surface mesh.
///
/// Vertex coordinates are `f32` (native STL precision). Face normals are
/// assumed outward-facing per the STL convention (right-hand rule).
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Each component is a 0-based index into `vertices`.
    pub triangles: Vec<Vector3<usize>>,
    pub vertices: Vec<Vector3<f32>>,
}

struct BBox {
    min: Vector3<f64>,
    max: Vector3<f64>,
}

impl BBox {
    fn from_vertices(verts: &[Vector3<f32>]) -> Self {
        let mut min = Vector3::from_element(f64::MAX);
        let mut max = Vector3::from_element(f64::MIN);
        for v in verts {
            let vf = v.cast::<f64>();
            for i in 0..3 {
                min[i] = min[i].min(vf[i]);
                max[i] = max[i].max(vf[i]);
            }
        }
        BBox { min, max }
    }

    /// Expand by a fixed `margin` on every side.
    fn expanded_by(&self, margin: f64) -> Self {
        let m = Vector3::from_element(margin);
        BBox {
            min: self.min - m,
            max: self.max + m,
        }
    }

    /// The 8 corners in the fixed order used by [`inward_triangles`].
    ///
    ///         7 ── 6
    ///        /|   /|      y
    ///       4 ── 5 |      |  z
    ///       | 3 ─| 2      | /
    ///       |/   |/       |/___x
    ///       0 ── 1
    fn corners(&self) -> [Vector3<f64>; 8] {
        let (x0, y0, z0) = (self.min.x, self.min.y, self.min.z);
        let (x1, y1, z1) = (self.max.x, self.max.y, self.max.z);
        [
            Vector3::new(x0, y0, z0), // 0
            Vector3::new(x1, y0, z0), // 1
            Vector3::new(x1, y0, z1), // 2
            Vector3::new(x0, y0, z1), // 3
            Vector3::new(x0, y1, z0), // 4
            Vector3::new(x1, y1, z0), // 5
            Vector3::new(x1, y1, z1), // 6
            Vector3::new(x0, y1, z1), // 7
        ]
    }

    /// 12 triangles (6 faces × 2) with inward-facing normals so the box acts
    /// as a closed containing shell for TetGen.
    fn inward_triangles() -> [Vector3<usize>; 12] {
        [
            Vector3::new(0, 2, 1),
            Vector3::new(0, 3, 2), // -Y bottom
            Vector3::new(4, 5, 6),
            Vector3::new(4, 6, 7), // +Y top
            Vector3::new(0, 1, 5),
            Vector3::new(0, 5, 4), // -Z front
            Vector3::new(3, 6, 2),
            Vector3::new(3, 7, 6), // +Z back
            Vector3::new(0, 4, 7),
            Vector3::new(0, 7, 3), // -X left
            Vector3::new(1, 2, 6),
            Vector3::new(1, 6, 5), // +X right
        ]
    }
}

/// Return a point likely inside the STL surface.
///
/// Picks the largest face by area, takes its centroid, then steps opposite the
/// direction of the normal by epsilon. May fail to pick an interior point in
/// extreme cases.
fn interior_seed(mesh: &Mesh, epsilon: f64) -> Result<Vector3<f64>, &'static str> {
    let best_tri = mesh
        .triangles
        .iter()
        .max_by(|p, q| {
            let area = |t: &Vector3<usize>| -> f64 {
                let [a, b, c] = [t.x, t.y, t.z].map(|i| mesh.vertices[i].cast::<f64>());
                (b - a).cross(&(c - a)).norm_squared()
            };
            area(p).partial_cmp(&area(q)).unwrap()
        })
        .ok_or("mesh has no triangles")?;

    let [a, b, c] = [best_tri.x, best_tri.y, best_tri.z].map(|i| mesh.vertices[i].cast::<f64>());
    let centroid = Matrix3::from_columns(&[a, b, c]).column_mean();
    let outward_normal = (b - a).cross(&(c - a)).normalize();

    Ok(centroid - outward_normal * epsilon)
}

/// A point strictly in the exterior gap, trivially correct by construction.
fn exterior_seed(stl_bbox: &BBox, expansion: f64) -> Vector3<f64> {
    stl_bbox.min - Vector3::from_element(expansion * 0.5)
}

/// Tetrahedralise a closed manifold surface into a volume mesh with two regions.
pub fn mesh2frep(mesh: &Mesh) -> Result<String, &'static str> {
    let stl_bbox = BBox::from_vertices(&mesh.vertices);
    let bbox = stl_bbox.expanded_by(BBOX_EXPANSION);

    // Epsilon = 0.1% of the STL bounding diagonal.
    // TODO: pass this in as optional parameter.
    let epsilon = (bbox.max - bbox.min).norm() * 0.001;

    let inner = interior_seed(mesh, epsilon)?;
    let outer = exterior_seed(&stl_bbox, BBOX_EXPANSION);

    // Merge STL vertices with the 8 bbox corners.
    let box_offset = mesh.vertices.len();
    let box_corners = bbox.corners();

    let points: Vec<(i32, f64, f64, f64)> = mesh
        .vertices
        .iter()
        .map(|v| (0_i32, v.x as f64, v.y as f64, v.z as f64))
        .chain(box_corners.iter().map(|v| (0_i32, v.x, v.y, v.z)))
        .collect();

    // STL triangles → facet marker 1; box triangles → facet marker 2.
    let facets: Vec<(i32, Vec<usize>)> = mesh
        .triangles
        .iter()
        .map(|t| (1_i32, vec![t.x, t.y, t.z]))
        .chain(BBox::inward_triangles().iter().map(|t| {
            (
                2_i32,
                vec![box_offset + t.x, box_offset + t.y, box_offset + t.z],
            )
        }))
        .collect();

    let input = InputDataTetMesh {
        points,
        facets,
        holes: vec![],
        regions: vec![
            (1, inner.x, inner.y, inner.z, None),
            (2, outer.x, outer.y, outer.z, None),
        ],
    };

    let tetgen = Tetgen::from_input_data(&input)
        .map_err(|_| "TetGen failed to construct input; is the surface a closed manifold?")?;

    tetgen
        .generate_mesh(false, false, None, None)
        .map_err(|_| "TetGen failed to generate the mesh")?;

    // Read back vertices.
    let vertices: Vec<Vector3<f64>> = (0..tetgen.out_npoint())
        .map(|i| {
            Vector3::new(
                tetgen.out_point(i, 0),
                tetgen.out_point(i, 1),
                tetgen.out_point(i, 2),
            )
        })
        .collect();

    // Read back tetrahedra, split by region marker.
    assert_eq!(tetgen.out_cell_npoint(), 4, "Expected linear tet4 cells");

    let mut interior = Vec::new();
    let mut exterior = Vec::new();

    for c in 0..tetgen.out_ncell() {
        let tet = Vector4::new(
            tetgen.out_cell_point(c, 0),
            tetgen.out_cell_point(c, 1),
            tetgen.out_cell_point(c, 2),
            tetgen.out_cell_point(c, 3),
        );
        match tetgen.out_cell_marker(c) {
            1 => interior.push(tet),
            2 => exterior.push(tet),
            m => eprintln!("warning: unexpected region marker {} on tet {}", m, c),
        }
    }

    let mut rhai_frep = String::new();

    rhai_frep.push_str(
        "// Generated file representing a mesh transformed into a functional representation.\n",
    );
    rhai_frep.push('\n');
    rhai_frep.push_str(RHAI_UNIT_TETRAHEDRON);
    rhai_frep.push('\n');

    rhai_frep.push_str("let interior_tet_list = [\n");
    for tetrahedron in interior {
        let tet_orig = vertices[tetrahedron[0]];
        let tetrahedron_x_transform = vertices[tetrahedron[1]] - tet_orig;
        let tetrahedron_y_transform = vertices[tetrahedron[2]] - tet_orig;
        let tetrahedron_z_transform = vertices[tetrahedron[3]] - tet_orig;
        let mut itm = Matrix3::from_columns(&[
            tetrahedron_x_transform,
            tetrahedron_y_transform,
            tetrahedron_z_transform,
        ]);
        if !itm.try_inverse_mut() {
            return Err("ahhhhh");
        };

        let (x0, y0, z0) = (itm[(0, 0)], itm[(0, 1)], itm[(0, 2)]);
        let (x1, y1, z1) = (itm[(1, 0)], itm[(1, 1)], itm[(1, 2)]);
        let (x2, y2, z2) = (itm[(2, 0)], itm[(2, 1)], itm[(2, 2)]);
        let (tx, ty, tz) = (tet_orig.x, tet_orig.y, tet_orig.z);

        rhai_frep.push_str(&format!(
            "    remap(unit_tetrahedron(), (x - {tx}) * {x0} + (y - {ty}) * {y0} + (z - {tz}) * {z0}, (x - {tx}) * {x1} + (y - {ty}) * {y1} + (z - {tz}) * {z1}, (x - {tx}) * {x2} + (y - {ty}) * {y2} + (z - {tz}) * {z2}),\n"
        ));
    }
    rhai_frep.push_str("];");
    rhai_frep.push('\n');
    rhai_frep.push('\n');
    // TODO: Make this pretty.
    rhai_frep.push_str("let result = interior_tet_list[0];\n");
    rhai_frep.push_str("for i in 1..interior_tet_list.len() {\n");
    rhai_frep.push_str("    result = min(result, interior_tet_list[i]);\n");
    rhai_frep.push_str("}\n");
    rhai_frep.push_str("result");

    Ok(rhai_frep)
}
