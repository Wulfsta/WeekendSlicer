use std::fs::{File, write};
use std::io::BufReader;
use std::path::PathBuf;
use std::process;

use clap::Parser;
use mesh2frep::{Mesh, mesh2frep};
use nalgebra::Vector3;
use stl_io::IndexedMesh;

/// Convert a closed manifold STL surface into a rhai script representing a f-rep parsable by
/// fidget.
#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to the input STL file (ASCII or binary).
    input: PathBuf,
}

fn load_stl(path: &PathBuf) -> Mesh {
    let file = File::open(path).unwrap_or_else(|e| {
        eprintln!("Error: cannot open '{}': {}", path.display(), e);
        process::exit(1);
    });
    let indexed: IndexedMesh = stl_io::read_stl(&mut BufReader::new(file)).unwrap_or_else(|e| {
        eprintln!("Error: failed to parse STL '{}': {}", path.display(), e);
        process::exit(1);
    });

    Mesh {
        vertices: indexed
            .vertices
            .iter()
            .map(|v| Vector3::new(v[0], v[1], v[2]))
            .collect(),
        triangles: indexed
            .faces
            .iter()
            .map(|f| Vector3::new(f.vertices[0], f.vertices[1], f.vertices[2]))
            .collect(),
    }
}

fn main() {
    let args = Args::parse();

    eprintln!("Reading STL: {}", args.input.display());
    let mesh = load_stl(&args.input);
    eprintln!(
        "  Surface: {} vertices, {} triangles",
        mesh.vertices.len(),
        mesh.triangles.len()
    );

    let rhai_frep = mesh2frep(&mesh).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    });

    write("debug_data/frep.rhai", rhai_frep).expect("Unable to write file");
}
