extern crate clap;
extern crate fidget;

use clap::Parser;
use fidget::context::{Context, Tree};
use fidget::jit::JitShape;
use fidget::mesh::{Octree, Settings};
use fidget::shape::Bounds;
use fidget::vm::VmData;
use indexmap::IndexMap;
use nalgebra::base::{Vector2, Vector3};
use std::f64::consts::PI;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

static EPS: f64 = 1e-8;

fn bounded_box(x_min: f64, y_min: f64, z_min: f64, x_max: f64, y_max: f64, z_max: f64) -> Tree {
    let x_radius = (x_max - x_min) / 2f64;
    let x_avg = (x_max + x_min) / 2f64;
    let y_radius = (y_max - y_min) / 2f64;
    let y_avg = (y_max + y_min) / 2f64;
    let z_radius = (z_max - z_min) / 2f64;
    let z_avg = (z_max + z_min) / 2f64;
    (((Tree::x().abs()).max(Tree::y().abs())).max(Tree::z().abs()) - 1f64).remap_xyz(
        Tree::x() / x_radius - x_avg,
        Tree::y() / y_radius - y_avg,
        Tree::z() / z_radius - z_avg,
    )
}

enum LayerType {
    Standard,
}

struct Layer {
    z_height: f64,
    layer_type: LayerType,
}

impl Layer {
    fn new(z_height: f64, layer_type: LayerType) -> Layer {
        Layer {
            z_height,
            layer_type,
        }
    }
}

#[derive(Debug, Clone)]
struct ExtrusionPath {
    path_width: f64,
    path_height: f64,
    z_height: f64,
    extruder_cross_sectional_area_per_mm: f64,
    paths: Vec<Vector2<f32>>,
}

impl ExtrusionPath {
    fn new(
        path_width: f64,
        path_height: f64,
        z_height: f64,
        extruder_cross_sectional_area_per_mm: f64,
        start: Vector2<f32>,
    ) -> ExtrusionPath {
        let mut paths = Vec::new();
        paths.push(start);
        ExtrusionPath {
            path_width: path_width,
            path_height: path_height,
            z_height: z_height,
            extruder_cross_sectional_area_per_mm: extruder_cross_sectional_area_per_mm,
            paths: paths,
        }
    }

    fn add_to_path(&mut self, point: Vector2<f32>) {
        self.paths.push(point);
    }

    fn first_point_in_path(&self) -> Option<&Vector2<f32>> {
        self.paths.first()
    }

    fn last_point_in_path(&self) -> Option<&Vector2<f32>> {
        self.paths.last()
    }

    fn first_point_in_path_as_bits(&self) -> Option<[u32; 2]> {
        let fir = self.paths.first();
        match fir {
            Some(f) => Some([f[0].to_bits(), f[1].to_bits()]),
            None => None,
        }
    }

    fn last_point_in_path_as_bits(&self) -> Option<[u32; 2]> {
        let las = self.paths.last();
        match las {
            Some(l) => Some([l[0].to_bits(), l[1].to_bits()]),
            None => None,
        }
    }

    fn write_gcode<F: std::io::Write>(&self, out: &mut F) -> Result<(), std::io::Error> {
        let extrusion_cross_section_area = (self.path_width - self.path_height) * self.path_height
            + PI * (self.path_height / 2.).powi(2);
        match self.paths.first() {
            Some(first_point) => {
                write!(out, "G1 Z{:.6}\n", self.z_height)?;
                write!(
                    out,
                    "G1 X{:.6} Y{:.6} Z{:.6}\n",
                    first_point.x, first_point.y, self.z_height
                )?;
            }
            None => return Ok(()),
        }
        for (point_0, point_1) in self.paths.windows(2).map(|s| (s[0], s[1])) {
            let extrusion_volume =
                ((point_1 - point_0).norm() as f64) * extrusion_cross_section_area;
            let extruder_distance = extrusion_volume / self.extruder_cross_sectional_area_per_mm;
            write!(
                out,
                "G1 X{:.6} Y{:.6} E{:.6}\n",
                point_1.x, point_1.y, extruder_distance
            )?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Slicer {
    object_tree: Tree,
    nozzle_diameter: f64,
    layer_height: f64,
    filament_diameter: f64,
    extrusion_width_scalar: f64,
    perimeters: u64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    z_min: f64,
    z_max: f64,
}

impl Slicer {
    fn new(
        tree: Tree,
        nozzle_diameter: f64,
        layer_height: f64,
        filament_diameter: f64,
        extrusion_width_scalar: f64,
        perimeters: u64,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        z_min: f64,
        z_max: f64,
    ) -> Slicer {
        Slicer {
            object_tree: tree,
            nozzle_diameter: nozzle_diameter,
            layer_height: layer_height,
            filament_diameter: filament_diameter,
            extrusion_width_scalar: extrusion_width_scalar,
            perimeters: perimeters,
            x_min: x_min,
            x_max: x_max,
            y_min: y_min,
            y_max: y_max,
            z_min: z_min,
            z_max: z_max,
        }
    }

    fn slice(&mut self) {
        let z_range = self.z_max - self.z_min;

        let layer_count = z_range / self.layer_height;

        // Leaving this general so other types of layers (support, interface) could be added.
        let mut layer_index: f64 = 0.;
        let mut layers = Vec::<Layer>::new();
        while layer_index < layer_count {
            layers.push(Layer::new(
                z_range * layer_index / layer_count - self.z_min,
                LayerType::Standard,
            ));
            layer_index += 1.;
        }

        let extruder_cross_sectional_area_per_mm = PI * (self.filament_diameter / 2.).powi(2);

        let mut extrusion_paths = Vec::<ExtrusionPath>::new();
        for layer in layers {
            match layer.layer_type {
                LayerType::Standard => {
                    let extrusion_width = self.extrusion_width_scalar * self.nozzle_diameter;
                    let path_spacing = extrusion_width - self.layer_height * (1. - PI / 4.);
                    // Create box to intersect tree with.
                    // TODO: Investigate if there is a bug here.
                    let layer_z_height = layer.z_height + self.layer_height / 2.;
                    let layer_box =
                        bounded_box(self.x_min, self.y_min, 0., self.x_max, self.y_max, 2.);
                    // Subtract a multiple of extrusion widths from the object, then intersect it
                    // with the tree.
                    for perimeter in (0..self.perimeters).rev() {
                        // this cloning might be slow, unsure if this is an Arc or not
                        let mut perimeter_tree = (self.object_tree.clone().remap_xyz(
                            Tree::x(),
                            Tree::y(),
                            Tree::constant(layer_z_height),
                        ) + path_spacing
                            * ((perimeter as f64) + 1. / 2.))
                            .max(layer_box.clone());
                        let mut perimeter_context = Context::new();
                        let perimeter_node = perimeter_context.import(&perimeter_tree);
                        let perimeter_vmdata =
                            VmData::<255>::new(&perimeter_context, &[perimeter_node]).unwrap();
                        let mut temp_vmdata = fs::File::create(format!(
                            "debug_data/vmdata_{:.2}.bin",
                            layer.z_height
                        ))
                        .unwrap();
                        bincode::serialize_into(temp_vmdata, &perimeter_vmdata);
                        perimeter_tree = perimeter_context
                            .export(perimeter_node)
                            .expect("No Mr. Bond, I expect a tree.");
                        let perimeter_shape = JitShape::from(perimeter_tree);
                        let mut temp_settings = fs::File::create(format!(
                            "debug_data/settings_{:.2}",
                            layer.z_height
                        ))
                        .unwrap();
                        write!(&mut temp_settings, "depth: {}\n", 8);
                        write!(
                            &mut temp_settings,
                            "center x: {}\n",
                            (((self.x_max + self.x_min) / 2.) as f32)
                        );
                        write!(
                            &mut temp_settings,
                            "center y: {}\n",
                            (((self.y_max + self.y_min) / 2.) as f32)
                        );
                        write!(
                            &mut temp_settings,
                            "center z: {}\n",
                            (0. as f32)
                        );
                        write!(
                            &mut temp_settings,
                            "size: {}\n",
                            (((self.x_max - self.x_min).max(self.y_max - self.y_min) + EPS) as f32)
                        );
                        let perimeter_octree_settings = Settings {
                            depth: 8,
                            // TODO: fix bounds
                            bounds: Bounds {
                                center: Vector3::new(
                                    ((self.x_max + self.x_min) / 2.) as f32,
                                    ((self.y_max + self.y_min) / 2.) as f32,
                                    0.,
                                ),
                                size: ((self.x_max - self.x_min).max(self.y_max - self.y_min) + EPS)
                                    as f32,
                            },
                            ..Default::default()
                        };
                        let o = Octree::build(&perimeter_shape, perimeter_octree_settings);
                        // Produce a mesh that contains a path that we will extract to use as the
                        // perimter path. I know this is doing a huge amount more computation than
                        // needed for this task, this is a proof of concept.
                        let perimeter_mesh = o.walk_dual(perimeter_octree_settings);
                        let mut temp_stl =
                            fs::File::create(format!("debug_data/temp_{:.2}.stl", layer.z_height))
                                .unwrap();
                        perimeter_mesh.write_stl(&mut temp_stl);
                        // Extract path from mesh. Iterate over all triangles. This would not be
                        // necissary if the result was 2D; maybe ask fidget to support it.
                        let mut edge_map_as_bits = IndexMap::new();
                        // Filter triangles to only those that contain two vertices on the current
                        // layer - this subset of triangles must contain the paths.
                        for triangle in perimeter_mesh.triangles.iter().filter(|tri| {
                            let num_vertices_at_layer: u8 = (0..=2)
                                .map(|i| {
                                    ((perimeter_mesh.vertices[tri[i]].z as f64).abs() < EPS) as u8
                                })
                                .sum();
                            num_vertices_at_layer == 2
                        }) {
                            // Append any edges that lie in the same plane as the current layer.
                            for (edge_0_index, edge_1_index) in vec![
                                (triangle[0], triangle[1]),
                                (triangle[1], triangle[2]),
                                (triangle[2], triangle[0]),
                            ]
                            .into_iter()
                            {
                                if ((perimeter_mesh.vertices[edge_0_index].z as f64).abs()) < EPS
                                    && ((perimeter_mesh.vertices[edge_1_index].z as f64).abs())
                                        < EPS
                                {
                                    edge_map_as_bits.insert(
                                        [
                                            perimeter_mesh.vertices[edge_0_index].x.to_bits(),
                                            perimeter_mesh.vertices[edge_0_index].y.to_bits(),
                                        ],
                                        [
                                            perimeter_mesh.vertices[edge_1_index].x.to_bits(),
                                            perimeter_mesh.vertices[edge_1_index].y.to_bits(),
                                        ],
                                    );
                                }
                            }
                        }
                        // This whole block of code disgusts me. It could be reordered to be more
                        // concise, but eh.
                        match edge_map_as_bits.first() {
                            Some((key, _)) => {
                                let mut curr_path = ExtrusionPath::new(
                                    extrusion_width,
                                    self.layer_height,
                                    layer.z_height + self.layer_height,
                                    extruder_cross_sectional_area_per_mm,
                                    Vector2::new(f32::from_bits(key[0]), f32::from_bits(key[1])),
                                );
                                while !edge_map_as_bits.is_empty() {
                                    // unwrap should be fine, these always have at least one value
                                    // in the path.
                                    let last_point =
                                        curr_path.last_point_in_path_as_bits().unwrap();
                                    let next_point = edge_map_as_bits.swap_remove(&last_point);
                                    match next_point {
                                        Some(p) => {
                                            curr_path.add_to_path(Vector2::new(
                                                f32::from_bits(p[0]),
                                                f32::from_bits(p[1]),
                                            ));
                                            if edge_map_as_bits.is_empty() {
                                                // TODO: get rid of this clone; memswap?
                                                extrusion_paths.push(curr_path.clone());
                                            }
                                        }
                                        None => {
                                            // TODO: get rid of this clone; memswap?
                                            extrusion_paths.push(curr_path.clone());
                                            match edge_map_as_bits.first() {
                                                Some((key, _)) => {
                                                    curr_path = ExtrusionPath::new(
                                                        extrusion_width,
                                                        self.layer_height,
                                                        layer.z_height + self.layer_height,
                                                        extruder_cross_sectional_area_per_mm,
                                                        Vector2::new(
                                                            f32::from_bits(key[0]),
                                                            f32::from_bits(key[1]),
                                                        ),
                                                    );
                                                }
                                                // map is empty and loop will break
                                                None => (),
                                            }
                                        }
                                    }
                                }
                            }
                            None => (),
                        }
                    }
                }
            }
        }
        // TODO: Make this return instead of write.
        let mut output_gcode = fs::File::create("output.gcode").unwrap();
        for extrusion_path in extrusion_paths {
            extrusion_path.write_gcode(&mut output_gcode);
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Cli {
    /// Path to surface.
    #[arg(short, long, value_parser = clap::value_parser!(PathBuf))]
    file: PathBuf,

    /// Nozzle diameter.
    #[arg(short, long, default_value = "0.40")]
    nozzle_diameter: f64,

    /// Layer height.
    #[arg(short, long, default_value = "0.20")]
    layer_height: f64,

    /// Filament diameter.
    #[arg(short, long, default_value = "1.75")]
    filament_diameter: f64,

    /// Extrusion width scalar; extrusions are this multiplied by the nozzle diameter.
    #[arg(short, long, default_value = "1.05")]
    extrusion_width_scalar: f64,

    /// Number of perimeters.
    #[arg(short, long, default_value = "1")]
    perimeters: u64,

    /// X axis minimum.
    #[arg(short, long, default_value = "-5")]
    x_min: f64,

    /// X axis maximum.
    #[arg(short, long, default_value = "5")]
    x_max: f64,

    /// Y axis minimum.
    #[arg(short, long, default_value = "-5")]
    y_min: f64,

    /// Y axis maximum.
    #[arg(short, long, default_value = "5")]
    y_max: f64,

    /// Z axis minimum.
    #[arg(short, long, default_value = "0")]
    z_min: f64,

    /// Z axis maximum.
    #[arg(short, long, default_value = "5")]
    z_max: f64,
}

fn main() {
    let args = Cli::parse();

    //println!("Input path: {}", args.file.display());

    let rhai_def = fs::read_to_string(&args.file).expect("Unable to read file.");

    //println!("{}", rhai_def);

    let tree_def = fidget::rhai::eval(&rhai_def).expect("Object definition invalid.");

    let mut slicer = Slicer::new(
        tree_def,
        args.nozzle_diameter,
        args.layer_height,
        args.filament_diameter,
        args.extrusion_width_scalar,
        args.perimeters,
        args.x_min,
        args.x_max,
        args.y_min,
        args.y_max,
        args.z_min,
        args.z_max,
    );
    slicer.slice();
}
