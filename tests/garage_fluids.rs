//! Critical G04 snapshot allocation check against the installed single-Lada export.
use my_summer_slop::{reference::ReferenceManifest, starting_area::GarageRuntime};

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {
    static TRACK_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct AllocationCounter;
fn record_allocation() {
    if TRACK_ALLOCATIONS.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for AllocationCounter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: AllocationCounter = AllocationCounter;

#[test]
#[ignore = "requires the installed single-Lada export"]
fn g04_ordered_snapshot_capture_and_copy_do_not_allocate() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let r = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let run = GarageRuntime::new(&r).unwrap();
    let oil = r.vehicles[0].engine.oil_tank;
    let warm = run.area.items.generated_snapshot(oil, None).unwrap();
    assert!(warm.fluids.is_some());
    ALLOCATIONS.with(|count| count.set(0));
    TRACK_ALLOCATIONS.with(|track| track.set(true));
    for _ in 0..256 {
        let snapshot = std::hint::black_box(run.area.items.generated_snapshot(oil, None).unwrap());
        let copy = std::hint::black_box(snapshot.clone());
        std::hint::black_box(snapshot == copy);
    }
    TRACK_ALLOCATIONS.with(|track| track.set(false));
    let count = ALLOCATIONS.with(Cell::get);
    assert_eq!(
        count, 0,
        "repeated fluid snapshot capture and copy must not allocate"
    );
}
