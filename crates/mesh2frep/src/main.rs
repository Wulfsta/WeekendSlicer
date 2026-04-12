use clap::Parser;
use mesh2frep::{Mesh, mesh2frep};
use nalgebra::Vector3;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use stl_io::IndexedMesh;

/// Convert a closed manifold STL surface into a rhai script representing a f-rep parsable by fidget.
#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to the input STL file.
    input: PathBuf,

    /// Path to the output rhai script. [default: frep.rhai]
    #[arg(short, long, default_value = "frep.rhai")]
    output: PathBuf,
}

fn load_stl(path: &PathBuf) -> Result<Mesh, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(path)?;
    let indexed: IndexedMesh = stl_io::read_stl(&mut BufReader::new(file))?;

    Ok(Mesh {
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
    })
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    eprintln!("Reading STL: {}", args.input.display());
    let mesh = load_stl(&args.input)?;
    eprintln!(
        "  Surface: {} vertices, {} triangles",
        mesh.vertices.len(),
        mesh.triangles.len()
    );

    let rhai_frep = mesh2frep(&mesh)?;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?;
    file.write_all(rhai_frep.as_bytes())?;

    Ok(())
}
