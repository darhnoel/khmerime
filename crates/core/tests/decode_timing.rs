//! Throwaway latency probe for the weighted-span decode (no model).
//! Run: cargo test -p khmerime_core decode_timing --release -- --nocapture

use std::collections::HashMap;
use std::time::Instant;

use khmerime_core::{DecoderConfig, DecoderMode, Transliterator};

#[test]
fn decode_timing() {
    // Mix of clean single words and harder multi-span blobs.
    let inputs = [
        "knhom", "sopheap", "mothyoubay", "vinichchhai", "salarien",
        "brolaom", "bongaengnih", "suntarakatha", "knhomtowsala", "akhosrok",
    ];
    let history: HashMap<String, usize> = HashMap::new();

    for (label, base) in [
        ("interactive (beam_width=2)", DecoderConfig::shadow_interactive()),
        ("default/offline (beam_width=6)", DecoderConfig::default()),
    ] {
        let mut cfg = base;
        cfg.mode = DecoderMode::Wfst; // isolate the weighted-span path
        let t = Transliterator::from_default_data_with_config(cfg).expect("load");

        for inp in &inputs {
            let _ = t.suggest(inp, &history); // warmup
        }

        let mut us: Vec<u128> = Vec::new();
        for _ in 0..50 {
            for inp in &inputs {
                let s = Instant::now();
                let _ = t.suggest(inp, &history);
                us.push(s.elapsed().as_micros());
            }
        }
        us.sort_unstable();
        let med = us[us.len() / 2] as f64 / 1000.0;
        let p95 = us[us.len() * 95 / 100] as f64 / 1000.0;
        let max = *us.last().unwrap() as f64 / 1000.0;
        println!(
            "{label:32} median {med:6.3} ms  p95 {p95:6.3} ms  max {max:6.3} ms  ({} calls)",
            us.len()
        );
    }
}
