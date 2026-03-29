//! mesh2frep

use nalgebra::{Matrix3, Rotation3, Vector3, Vector4};
use rayon::prelude::*;
use tritet::{InputDataTetMesh, Tetgen};

const RHAI_UGF_MESH: &str = include_str!("rhai/ugf_mesh.rhai");

/// A closed, orientable triangulated surface mesh.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub triangles: Vec<Vector3<usize>>,
    pub vertices: Vec<Vector3<f32>>,
}

/// Tetrahedralise a closed manifold surface and represent it as a unit gradient function.
// TODO: Add error types
pub fn mesh2frep(mesh: &Mesh) -> Result<String, &'static str> {
    // Points: only the STL vertices (no bbox corners).
    let points: Vec<(i32, f64, f64, f64)> = mesh
        .vertices
        .iter()
        .map(|v| (0_i32, v.x as f64, v.y as f64, v.z as f64))
        .collect();

    // Facets: only the STL triangles, all with marker 1.
    let facets: Vec<(i32, Vec<usize>)> = mesh
        .triangles
        .iter()
        .map(|t| (1_i32, vec![t.x, t.y, t.z]))
        .collect();

    let input = InputDataTetMesh {
        points,
        facets,
        holes: vec![],
        regions: vec![],
    };

    let tetgen = Tetgen::from_input_data(&input)
        .map_err(|_| "TetGen failed to construct input; is the surface a closed manifold?")?;

    tetgen
        .generate_mesh(false, false, None, None)
        .map_err(|_| "TetGen failed to generate the mesh")?;

    let vertices: Vec<Vector3<f64>> = (0..tetgen.out_npoint())
        .map(|i| {
            Vector3::new(
                tetgen.out_point(i, 0),
                tetgen.out_point(i, 1),
                tetgen.out_point(i, 2),
            )
        })
        .collect();

    assert_eq!(tetgen.out_cell_npoint(), 4, "Expected linear tet4 cells");

    let mut interior: Vec<Vector4<usize>> = Vec::new();
    for c in 0..tetgen.out_ncell() {
        let tet = Vector4::new(
            tetgen.out_cell_point(c, 0),
            tetgen.out_cell_point(c, 1),
            tetgen.out_cell_point(c, 2),
            tetgen.out_cell_point(c, 3),
        );
        match tetgen.out_cell_marker(c) {
            1 => interior.push(tet),
            m => eprintln!("warning: unexpected region marker {} on tet {}", m, c),
        }
    }

    let n_marked = tetgen.out_n_marked_face();
    let mut surface_triangles: Vec<[usize; 3]> = Vec::new();

    for i in 0..n_marked {
        let mut pts = [0_i32; 6];
        let (marker, _cell) = tetgen.out_marked_face(i, &mut pts);
        if marker == 1 {
            // pts[0..3] are the three corner indices for a linear (non-o2) mesh.
            surface_triangles.push([pts[0] as usize, pts[1] as usize, pts[2] as usize]);
        }
    }

    let mut rhai_frep = String::new();

    rhai_frep.push_str(
        "// Generated file representing a mesh transformed into a functional representation.\n",
    );
    rhai_frep.push('\n');
    rhai_frep.push_str(RHAI_UGF_MESH);
    rhai_frep.push('\n');

    rhai_frep.push_str("let tetrahedrons = [\n");
    let tetrahedrons_string: String = interior.par_iter().map(|tetrahedron| {
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
            return Err("degenerate matrix");
        };

        let (x0, y0, z0) = (itm[(0, 0)], itm[(0, 1)], itm[(0, 2)]);
        let (x1, y1, z1) = (itm[(1, 0)], itm[(1, 1)], itm[(1, 2)]);
        let (x2, y2, z2) = (itm[(2, 0)], itm[(2, 1)], itm[(2, 2)]);
        let (tx, ty, tz) = (tet_orig.x, tet_orig.y, tet_orig.z);

        Ok(format!(
            "    remap(bounded_unit_tetrahedron(), (x - {tx}) * {x0} + (y - {ty}) * {y0} + (z - {tz}) * {z0}, (x - {tx}) * {x1} + (y - {ty}) * {y1} + (z - {tz}) * {z1}, (x - {tx}) * {x2} + (y - {ty}) * {y2} + (z - {tz}) * {z2}),\n"
        ).to_string())
    }).collect::<Result<Vec<_>, _>>()?.join("");
    rhai_frep.push_str(&tetrahedrons_string);
    rhai_frep.push_str("];");
    rhai_frep.push('\n');
    rhai_frep.push('\n');

    let eps = 1e-8;

    rhai_frep.push_str("let triangles = [\n");
    let triangles_string: String = surface_triangles.par_iter().map(|triangle|{
        let triangle_v0 = vertices[triangle[0]];
        let triangle_v1 = vertices[triangle[1]];
        let triangle_v2 = vertices[triangle[2]];
        let shift_e1 = triangle_v1 - triangle_v0;
        let shift_e2 = triangle_v2 - triangle_v0;
        let normal = shift_e1.cross(&shift_e2);
        let unit_normal = normal / normal.norm();
        // linear transform from unit x, y, z vectors to triangle edges one, two, and unit normal
        let mut itm = Matrix3::from_columns(&[shift_e1, shift_e2, unit_normal]);
        if !itm.try_inverse_mut() {
            return Err("degenerate matrix");
        };

        let (tx, ty, tz) = (triangle_v0.x, triangle_v0.y, triangle_v0.z);

        let (x0, y0, z0) = (itm[(0, 0)], itm[(0, 1)], itm[(0, 2)]);
        let (x1, y1, z1) = (itm[(1, 0)], itm[(1, 1)], itm[(1, 2)]);
        let (x2, y2, z2) = (itm[(2, 0)], itm[(2, 1)], itm[(2, 2)]);

        // linear transform of unit vectors to normalized triangle edges
        let shift_e1_norm = shift_e1.norm();
        let unit_shift_e1 = shift_e1 / shift_e1_norm;
        let shift_e2_norm = shift_e2.norm();
        let unit_shift_e2 = shift_e2 / shift_e2_norm;
        let unit_capsule_axis = Vector3::<f64>::new(0.0, 0.0, 1.0);

        let e1_itm: Matrix3<f64>;
        if (unit_capsule_axis + unit_shift_e1).norm() > eps {
            e1_itm = Rotation3::rotation_between(&unit_capsule_axis, &unit_shift_e1)
                .ok_or("degenerate rotation matrix e1")?
                .inverse()
                .into();
        } else {
            e1_itm = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0);
        }

        let (e1_x0, e1_y0, e1_z0) = (e1_itm[(0, 0)], e1_itm[(0, 1)], e1_itm[(0, 2)]);
        let (e1_x1, e1_y1, e1_z1) = (e1_itm[(1, 0)], e1_itm[(1, 1)], e1_itm[(1, 2)]);
        let (e1_x2, e1_y2, e1_z2) = (e1_itm[(2, 0)], e1_itm[(2, 1)], e1_itm[(2, 2)]);

        let e2_itm: Matrix3<f64>;
        if (unit_capsule_axis + unit_shift_e2).norm() > eps {
            e2_itm = Rotation3::rotation_between(&unit_capsule_axis, &unit_shift_e2)
                .ok_or("degenerate rotation matrix e2")?
                .inverse()
                .into();
        } else {
            e2_itm = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0);
        }

        let (e2_x0, e2_y0, e2_z0) = (e2_itm[(0, 0)], e2_itm[(0, 1)], e2_itm[(0, 2)]);
        let (e2_x1, e2_y1, e2_z1) = (e2_itm[(1, 0)], e2_itm[(1, 1)], e2_itm[(1, 2)]);
        let (e2_x2, e2_y2, e2_z2) = (e2_itm[(2, 0)], e2_itm[(2, 1)], e2_itm[(2, 2)]);

        let shift_e3 = shift_e2 - shift_e1;
        let shift_e3_norm = shift_e3.norm();
        let unit_shift_e3 = shift_e3 / shift_e3_norm;

        let e3_itm: Matrix3<f64>;
        if (unit_capsule_axis + unit_shift_e3).norm() > eps {
            e3_itm = Rotation3::rotation_between(&unit_capsule_axis, &unit_shift_e3)
                .ok_or("degenerate rotation matrix e3")?
                .inverse()
                .into();
        } else {
            e3_itm = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0);
        }

        let (e3_x0, e3_y0, e3_z0) = (e3_itm[(0, 0)], e3_itm[(0, 1)], e3_itm[(0, 2)]);
        let (e3_x1, e3_y1, e3_z1) = (e3_itm[(1, 0)], e3_itm[(1, 1)], e3_itm[(1, 2)]);
        let (e3_x2, e3_y2, e3_z2) = (e3_itm[(2, 0)], e3_itm[(2, 1)], e3_itm[(2, 2)]);

        let e3_translation = triangle_v0 + shift_e1;
        let (e3_tx, e3_ty, e3_tz) = (e3_translation.x, e3_translation.y, e3_translation.z);

        let mut triangle_frep = String::new();
        triangle_frep.push_str(&format!(
            "    unit_gradient_function_triangle_helper((x - {tx}) * {x0} + (y - {ty}) * {y0} + (z - {tz}) * {z0}, (x - {tx}) * {x1} + (y - {ty}) * {y1} + (z - {tz}) * {z1}, (x - {tx}) * {x2} + (y - {ty}) * {y2} + (z - {tz}) * {z2}, "
        ));
        triangle_frep.push_str(&format!("min(remap(vertical_capsule(0, {shift_e1_norm}), (x - {tx}) * {e1_x0} + (y - {ty}) * {e1_y0} + (z - {tz}) * {e1_z0}, (x - {tx}) * {e1_x1} + (y - {ty}) * {e1_y1} + (z - {tz}) * {e1_z1}, (x - {tx}) * {e1_x2} + (y - {ty}) * {e1_y2} + (z - {tz}) * {e1_z2}), "));
        triangle_frep.push_str(&format!("min(remap(vertical_capsule(0, {shift_e2_norm}), (x - {tx}) * {e2_x0} + (y - {ty}) * {e2_y0} + (z - {tz}) * {e2_z0}, (x - {tx}) * {e2_x1} + (y - {ty}) * {e2_y1} + (z - {tz}) * {e2_z1}, (x - {tx}) * {e2_x2} + (y - {ty}) * {e2_y2} + (z - {tz}) * {e2_z2}), "));
        triangle_frep.push_str(&format!("remap(vertical_capsule(0, {shift_e3_norm}), (x - {e3_tx}) * {e3_x0} + (y - {e3_ty}) * {e3_y0} + (z - {e3_tz}) * {e3_z0}, (x - {e3_tx}) * {e3_x1} + (y - {e3_ty}) * {e3_y1} + (z - {e3_tz}) * {e3_z1}, (x - {e3_tx}) * {e3_x2} + (y - {e3_ty}) * {e3_y2} + (z - {e3_tz}) * {e3_z2})))),\n"));
        Ok(triangle_frep)
    }).collect::<Result<Vec<_>, _>>()?.join("");

    rhai_frep.push_str(&triangles_string);
    rhai_frep.push_str("];");
    rhai_frep.push('\n');
    rhai_frep.push('\n');

    rhai_frep.push_str("ugf_mesh(tetrahedrons, triangles)");

    Ok(rhai_frep)
}
