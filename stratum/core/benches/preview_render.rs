#![allow(clippy::expect_used)]

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};
use stratum_core::services::StructureParser;
use stratum_core::services::preview::{PreviewMesher, PreviewRasterizer, TexturePack};
use stratum_core::structs::preview::OrbitCamera;

// Reads the inputs: the game's resource packs from `STRATUM_PREVIEW_PACKS` (no textures when unset) and the
// structure files from `STRATUM_PREVIEW_FILES` (`;` between paths), or the repo's two samples.
struct BenchInputs;

impl BenchInputs {
    fn pack() -> TexturePack {
        let Ok(dir) = std::env::var("STRATUM_PREVIEW_PACKS") else {
            println!("STRATUM_PREVIEW_PACKS unset, rendering with no textures");
            return TexturePack::empty();
        };
        let started = Instant::now();
        let pack = TexturePack::load(Path::new(&dir));
        println!(
            "texture pack load: {:.1} ms, {} packs, {} files, {} bytes",
            started.elapsed().as_secs_f64() * 1000.0,
            pack.pack_count(),
            pack.file_count(),
            pack.archive_bytes()
        );
        pack
    }

    fn files() -> Vec<PathBuf> {
        match std::env::var("STRATUM_PREVIEW_FILES") {
            Ok(list) => list.split(';').filter(|part| !part.is_empty()).map(PathBuf::from).collect(),
            Err(_) => {
                let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples");
                vec![samples.join("tint.mcstructure"), samples.join("easy-place-full.mcstructure")]
            }
        }
    }
}

// The mesh build and one 640 by 480 render from the start view, per structure.
fn bench_preview(c: &mut Criterion) {
    let mut group = c.benchmark_group("preview_render");
    group.sample_size(20);
    let mut pack = BenchInputs::pack();
    for path in BenchInputs::files() {
        let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("structure").to_owned();
        let bytes = std::fs::read(&path).expect("structure file reads");
        let structure = StructureParser::parse(&bytes).expect("structure parses");
        let mesh = PreviewMesher::build(&structure, &mut pack).expect("mesh builds");
        let mut rasterizer = PreviewRasterizer::new();
        let triangles = rasterizer.render(&mesh, OrbitCamera::start(), 640, 480);
        println!(
            "{name}: size {}x{}x{}, {} quads, {} triangles drawn, {} missing blocks",
            mesh.size.x,
            mesh.size.y,
            mesh.size.z,
            mesh.quads.len(),
            triangles,
            mesh.missing
        );
        group.bench_function(format!("mesh_{name}"), |b| {
            b.iter(|| PreviewMesher::build(black_box(&structure), &mut pack).expect("mesh builds"))
        });
        group.bench_function(format!("render_640x480_{name}"), |b| {
            b.iter(|| rasterizer.render(black_box(&mesh), OrbitCamera::start(), 640, 480))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_preview);
criterion_main!(benches);
