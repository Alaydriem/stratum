use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Instant;

use stratum_core::services::StructureParser;
use stratum_core::services::preview::{PreviewMesher, PreviewRasterizer, TexturePack};
use stratum_core::structs::preview::OrbitCamera;

// Renders the preview of each structure file to a PNG, for a visual check outside the game.
// Usage: preview_png <resource_packs dir> <out dir> <file.mcstructure>...
fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let packs = PathBuf::from(args.next().ok_or_else(|| Error::new(ErrorKind::InvalidInput, "packs dir"))?);
    let out = PathBuf::from(args.next().ok_or_else(|| Error::new(ErrorKind::InvalidInput, "out dir"))?);
    std::fs::create_dir_all(&out)?;
    let started = Instant::now();
    let mut pack = TexturePack::load(&packs);
    println!("texture load {:.1} ms, {} files", started.elapsed().as_secs_f64() * 1000.0, pack.file_count());
    let mut rasterizer = PreviewRasterizer::new();
    for file in args {
        let path = Path::new(&file);
        let bytes = std::fs::read(path)?;
        let structure =
            StructureParser::parse(&bytes).map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
        let started = Instant::now();
        let mesh = PreviewMesher::build(&structure, &mut pack)
            .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
        let mesh_ms = started.elapsed().as_secs_f64() * 1000.0;
        let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("structure");
        for (label, camera) in [("start", OrbitCamera::start()), ("back", OrbitCamera::new(225.0, 20.0))] {
            let started = Instant::now();
            let triangles = rasterizer.render(&mesh, camera, 640, 480);
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            println!(
                "{stem} {label}: mesh {mesh_ms:.1} ms, render {ms:.1} ms, {} quads, {triangles} triangles, missing {}",
                mesh.quads.len(),
                mesh.missing
            );
            let file = std::fs::File::create(out.join(format!("{stem}-{label}.png")))?;
            let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), rasterizer.width(), rasterizer.height());
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(|error| Error::other(error.to_string()))?;
            writer.write_image_data(rasterizer.pixels()).map_err(|error| Error::other(error.to_string()))?;
        }
    }
    Ok(())
}
