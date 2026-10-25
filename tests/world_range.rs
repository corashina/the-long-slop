//! Stream selection stays inside the limits along 30 km of road at low and high draw distances.
use the_long_slop::{
    reference::ReferenceManifest,
    starting_area::GarageRuntime,
    world::{GameMode, RoadSequence, StreamLimits, terrain_evaluator},
};

const ROUTE: f64 = 30_000.;
const STEP: f64 = 250.;

/// Points every `STEP` metres along the source road, with the road segment index.
fn route(reference: &ReferenceManifest, seed: i32) -> Vec<([f64; 3], u32)> {
    let mut sequence =
        RoadSequence::new(&reference.world, seed, GameMode::default().biome_override()).unwrap();
    let terrain = terrain_evaluator(&reference.world, seed);
    let first = sequence.advance(&terrain).unwrap();
    let mut points = vec![(first.position(), first.index)];
    let (mut last, mut travelled, mut next) = (first.position(), 0., STEP);
    while next <= ROUTE {
        let segment = sequence.advance(&terrain).unwrap();
        let point = segment.position();
        let span = ((point[0] - last[0]).powi(2) + (point[2] - last[2]).powi(2)).sqrt();
        while next <= ROUTE && travelled + span >= next {
            let t = (next - travelled) / span;
            points.push((
                std::array::from_fn(|i| last[i] + (point[i] - last[i]) * t),
                segment.index,
            ));
            next += STEP;
        }
        travelled += span;
        last = point;
    }
    points
}

fn drive(reference: &ReferenceManifest, distance: f32) {
    let mut run = GarageRuntime::new(reference, GameMode::default(), distance).unwrap();
    let stream = &mut run.stream;
    let limits = stream.limits();
    let ahead = reference.world.road.gen_at_start;
    let mut peak = StreamLimits {
        roads: 0,
        tiles: [0; 3],
        objects: 0,
        active_cpu: 0,
        active_gpu: 0,
        terrain_job: 0,
    };
    for (point, index) in route(reference, stream.seed()) {
        stream.request_at(point, point);
        let mut previous = None;
        for call in 0.. {
            assert!(
                call < 100_000,
                "{distance} m: selection stalled at {point:?}"
            );
            // Keep the source road ahead of the request, as at startup.
            while stream.initial_generation_ready() && stream.coverage().road_end < index + ahead {
                stream.advance_generation();
            }
            stream.prepare(0.);
            let failure = stream.frame().failure;
            assert!(failure.is_none(), "{distance} m at {point:?}: {failure:?}");
            let now = stream.selected();
            peak.roads = peak.roads.max(now.roads);
            for t in 0..3 {
                peak.tiles[t] = peak.tiles[t].max(now.tiles[t]);
            }
            peak.objects = peak.objects.max(now.objects);
            peak.active_cpu = peak.active_cpu.max(now.active_cpu);
            peak.active_gpu = peak.active_gpu.max(now.active_gpu);
            if stream.selection_complete() && previous == Some(now) {
                break;
            }
            previous = Some(now);
        }
    }
    println!("{distance} m\n  limits: {limits:?}\n  peak:   {peak:?}");
    assert!(peak.roads <= limits.roads);
    assert!((0..3).all(|t| peak.tiles[t] <= limits.tiles[t]));
    assert!(peak.objects <= limits.objects);
    assert!(peak.active_cpu <= limits.active_cpu);
    assert!(peak.active_gpu <= limits.active_gpu);
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn selection_stays_inside_limits_along_the_road() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let largest = GarageRuntime::new(&reference, GameMode::default(), 500.)
        .unwrap()
        .stream
        .largest_fitting()
        .unwrap();
    println!("largest fitting: {largest} m");
    for distance in [500., 2500., largest] {
        drive(&reference, distance);
    }
}
