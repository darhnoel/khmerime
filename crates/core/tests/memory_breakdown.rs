// TEMPORARY measurement harness — delete after sizing the iOS session-init footprint.
//
// A counting global allocator tracks live heap bytes (alloc - dealloc). We build
// the transliterator the way iOS does and print the cumulative heap at each stage,
// so we can see which structure dominates the ~58 MB session-init spike measured
// on device.
//
// Run:
//   cargo test -p khmerime_core --features no-search-index --test memory_breakdown -- --nocapture --test-threads=1

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

static LIVE: AtomicIsize = AtomicIsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let np = System.realloc(ptr, layout, new_size);
        if !np.is_null() {
            LIVE.fetch_add(new_size as isize - layout.size() as isize, Ordering::Relaxed);
        }
        np
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn live_mb() -> f64 {
    LIVE.load(Ordering::Relaxed) as f64 / 1_048_576.0
}

#[test]
fn measure_session_init_footprint_breakdown() {
    use khmerime_core::{DecoderConfig, Transliterator};

    let base = live_mb();

    // 1) Total for the exact iOS path.
    let ios = Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive())
        .expect("default data must load");
    eprintln!("=== iOS path (from_default_data_with_config) TOTAL: {:.1} MB", live_mb() - base);
    drop(ios);
    eprintln!("=== after drop (baseline check): {:.1} MB", live_mb() - base);

    // 2) Per-stage breakdown via the stage-logged shared constructor.
    let base2 = live_mb();
    let shared = Transliterator::from_default_shared_data_with_stage_logger(|stage, _elapsed_ms| {
        eprintln!("stage {:<28} cumulative {:>6.1} MB", stage, live_mb() - base2);
    })
    .expect("shared data must load");
    eprintln!("=== shared total: {:.1} MB", live_mb() - base2);

    std::hint::black_box(&shared);
}
