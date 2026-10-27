use std::time::{Duration, Instant};

use stratum_core::errors::PreviewError;
use stratum_core::services::preview::PreviewService;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::preview::{OrbitCamera, PreviewFrame, PreviewRequest, PreviewSource};

use crate::support::StructureFixture;

fn request(serial: u64, file_name: &str) -> PreviewRequest {
    PreviewRequest {
        serial,
        structure: StructureId(serial),
        file_name: file_name.to_owned(),
        camera: OrbitCamera::start(),
        width: 32,
        height: 24,
    }
}

fn wait(service: &PreviewService) -> PreviewFrame {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(frame) = service.take() {
            return frame;
        }
        assert!(Instant::now() < deadline, "no preview frame in time");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_request_comes_back_as_a_picture_of_the_asked_size() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([2, 2, 2]).block(0, 0, 0, "minecraft:stone").bytes();
    std::fs::write(dir.path().join("one.mcstructure"), bytes).unwrap();
    let service = PreviewService::new(PreviewSource::new(None, dir.path().to_path_buf()));
    service.submit(request(1, "one.mcstructure")).unwrap();
    let frame = wait(&service);
    assert_eq!(frame.serial, 1);
    let image = frame.outcome.unwrap();
    assert_eq!((image.width, image.height), (32, 24));
    assert_eq!(image.rgba.len(), 32 * 24 * 4);
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|texel| texel[3] == 255));
}

#[test]
fn a_missing_file_comes_back_as_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let service = PreviewService::new(PreviewSource::new(None, dir.path().to_path_buf()));
    service.submit(request(1, "gone.mcstructure")).unwrap();
    let frame = wait(&service);
    assert!(matches!(frame.outcome, Err(PreviewError::Structure(_))));
}
