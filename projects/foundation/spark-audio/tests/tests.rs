//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_audio::*;

#[test]
fn queue_drains_into_silent_bus() {
    let bus = AudioBus::silent();
    let mut q = AudioQueue::new();
    q.play_tone(Tone::new(440.0, 10, 0.5));
    q.play_file("nope.wav");
    assert_eq!(q.len(), 2);
    let mut errs = Vec::new();
    q.flush(&bus, Some(&mut errs));
    assert!(q.is_empty());
    assert!(errs.is_empty());
}

#[test]
fn pcm_stream_loops_then_stops() {
    let pcm = PcmAudio { sample_rate: 4, channels: 1, samples: vec![0.1, 0.2] };
    let mut looping = PcmStream::new(&pcm, true);
    assert_eq!(looping.next(), Some(0.1));
    assert_eq!(looping.next(), Some(0.2));
    assert_eq!(looping.next(), Some(0.1));
    let mut once = PcmStream::new(&pcm, false);
    assert_eq!(once.next(), Some(0.1));
    assert_eq!(once.next(), Some(0.2));
    assert_eq!(once.next(), None);
}

#[test]
fn muted_start_pcm_returns_none() {
    let bus = AudioBus::silent();
    let pcm = PcmAudio { sample_rate: 8, channels: 1, samples: vec![0.0, 1.0] };
    assert!(bus.start_pcm(&pcm, 1.0, 1.0, false).unwrap().is_none());
}
