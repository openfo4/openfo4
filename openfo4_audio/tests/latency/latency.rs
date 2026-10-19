use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use openfo4_audio::backend::Backend;
use openfo4_audio::channel::Receiver;
use openfo4_audio::effects::Volume;
use openfo4_audio::sound::Frame;
use openfo4_audio::sound_data::{Settings, SoundData};
use openfo4_audio::source::AudioSource;
use openfo4_audio::AudioManager;

#[test]
fn test_latency() {
    let sound = SoundData {
        frames: vec![
            Frame {
                left: 1.0,
                right: 1.0,
            };
            48_000
        ],
        sample_rate: 48_000,
        volume: Volume::default(),
    };

    let backend = LatencyTestBackend::default();

    let mut manager = AudioManager::new(backend.clone());

    // Fill the initial buffer.
    for _ in 0..48_000 {
        manager.update();
    }

    manager.play(AudioSource::from_data(sound), Settings::default());

    let now = Instant::now();
    loop {
        manager.update();

        if backend.cell.load(Ordering::Acquire) {
            break;
        }
    }

    println!("Sound played after {:?}", now.elapsed());
}

#[derive(Clone, Debug, Default)]
pub struct LatencyTestBackend {
    cell: Arc<AtomicBool>,
}

impl Backend for LatencyTestBackend {
    fn create_output_stream(&mut self, mut rx: Receiver) {
        let cell = self.cell.clone();

        std::thread::spawn(move || loop {
            if let Some(frame) = rx.recv() {
                if frame.left == 1.0 && frame.right == 1.0 {
                    cell.store(true, Ordering::Release);
                }
            }
        });
    }
}
