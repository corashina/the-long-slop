use super::Drive;
use crate::browser::{game_performance_counters, game_performance_frame};
use bevy::prelude::*;

pub(super) fn publish(drive: Res<Drive>) {
    if !game_performance_frame(drive.physics_ms, drive.world_ms) {
        return;
    }
    let runtime = &drive.runtime;
    let counters = runtime.stream.performance_counters();
    let snapshot = runtime.physics.snapshot(runtime.vehicle);
    game_performance_counters(
        drive.frame_steps,
        snapshot.wheels_contact.into_iter().filter(|v| *v).count() as u32,
        runtime.physics.rapier().bodies.len() as u32,
        runtime.physics.collider_count() as u32,
        counters.roads as u32,
        counters.terrain as u32,
        counters.objects as u32,
        counters.jobs as u32,
        counters.pending as u32,
        counters.render_cpu_bytes as f64,
        counters.render_gpu_bytes as f64,
        runtime.weather.storms().len() as u32,
        runtime.weather.influence().strength,
        runtime.weather.particle_count() as u32,
    );
}
