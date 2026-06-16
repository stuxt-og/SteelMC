//! Spawn chunk generation.
//!
//! During server startup, generates chunks around the spawn position until
//! the 7×7 Full area is complete.
//!
//! Set `PREGEN_RADIUS` environment variable to generate a larger area (e.g., 128).

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::sleep;

use steel_core::chunk::chunk_access::ChunkStatus;
use steel_core::chunk::chunk_pyramid::GENERATION_PYRAMID;
use steel_core::chunk::chunk_request::{
    ChunkRequest, ChunkRequestHandle, ChunkRequestState, ChunkTicketKind,
};
use steel_core::server::Server;
use steel_core::world::World;
use steel_utils::{ChunkPos, SectionPos};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "slow_chunk_gen")]
use std::sync::atomic::Ordering;
#[cfg(feature = "slow_chunk_gen")]
use steel_core::chunk::chunk_holder::SLOW_CHUNK_GEN;

/// Vanilla spawn chunk radius — chunks within this radius reach Full status.
const SPAWN_RADIUS: i32 = 3;
const PREGEN_WINDOW_SIZE: i32 = 32;
const PREGEN_ACTIVE_WINDOWS: usize = 2;
const FULL_DEPENDENCY_RADIUS: i32 = GENERATION_PYRAMID
    .get_step_to(ChunkStatus::Full)
    .accumulated_dependencies
    .get_radius_of(ChunkStatus::Empty) as i32;

#[derive(Clone, Copy, Debug)]
struct PregenWindow {
    min_x: i32,
    max_x: i32,
    min_z: i32,
    max_z: i32,
}

impl PregenWindow {
    fn positions(self) -> Vec<ChunkPos> {
        let mut positions = Vec::with_capacity(self.chunk_count());
        for z in self.min_z..=self.max_z {
            for x in self.min_x..=self.max_x {
                positions.push(ChunkPos::new(x, z));
            }
        }
        positions
    }

    const fn chunk_count(self) -> usize {
        (self.width() * self.height()) as usize
    }

    const fn width(self) -> i32 {
        self.max_x - self.min_x + 1
    }

    const fn height(self) -> i32 {
        self.max_z - self.min_z + 1
    }

    const fn protected_rect(self) -> PregenRect {
        PregenRect {
            min_x: self.min_x - FULL_DEPENDENCY_RADIUS,
            max_x: self.max_x + FULL_DEPENDENCY_RADIUS,
            min_z: self.min_z - FULL_DEPENDENCY_RADIUS,
            max_z: self.max_z + FULL_DEPENDENCY_RADIUS,
        }
    }
}

#[derive(Clone, Copy)]
struct PregenRect {
    min_x: i32,
    max_x: i32,
    min_z: i32,
    max_z: i32,
}

impl PregenRect {
    const fn overlaps(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_z <= other.max_z
            && self.max_z >= other.min_z
    }
}

struct ActivePregenWindow {
    window: PregenWindow,
    handle: ChunkRequestHandle,
    ready_chunks: usize,
    ready: bool,
    counted: bool,
}

impl ActivePregenWindow {
    fn new(world: &Arc<World>, window: PregenWindow) -> Self {
        let handle = world.chunk_map.request_chunks(ChunkRequest {
            status: ChunkStatus::Full,
            positions: window.positions(),
            ticket_kind: ChunkTicketKind::Pregen,
        });
        Self {
            window,
            handle,
            ready_chunks: 0,
            ready: false,
            counted: false,
        }
    }

    fn poll(&mut self, world: &Arc<World>) {
        match self.handle.poll() {
            ChunkRequestState::Ready => {
                self.ready_chunks = self.window.chunk_count();
                self.ready = true;
            }
            ChunkRequestState::Pending { ready, .. } => {
                self.ready_chunks = ready;
            }
            ChunkRequestState::Cancelled => {
                self.handle = world.chunk_map.request_chunks(ChunkRequest {
                    status: ChunkStatus::Full,
                    positions: self.window.positions(),
                    ticket_kind: ChunkTicketKind::Pregen,
                });
                self.ready_chunks = 0;
                self.ready = false;
                self.counted = false;
            }
        }
    }
}

/// Gets the pregeneration radius from environment variable, or returns default spawn radius.
fn get_pregen_radius() -> i32 {
    use std::env;
    env::var("PREGEN_RADIUS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(SPAWN_RADIUS)
}

/// Generates spawn chunks.
///
/// Adds a ticket at the world spawn position so that a 7×7 area of chunks
/// reaches `Full` status. The generation system is pumped in a loop until
/// completion.
///
/// Set `PREGEN_RADIUS` environment variable to generate a larger area.
pub async fn generate_spawn_chunks(server: &Arc<Server>) -> bool {
    let overworld = server.overworld();
    let pregen_radius = get_pregen_radius();

    // For large pregeneration, use center at 0,0; otherwise use spawn position
    let center_chunk = if pregen_radius > SPAWN_RADIUS {
        ChunkPos::new(0, 0)
    } else {
        let spawn_pos = overworld.level_data.read().data().spawn_pos();
        ChunkPos::new(
            SectionPos::block_to_section_coord(spawn_pos.0.x),
            SectionPos::block_to_section_coord(spawn_pos.0.z),
        )
    };

    pregen_overworld(overworld, center_chunk, pregen_radius, &server.cancel_token).await
}

async fn pregen_overworld(
    world: &Arc<World>,
    center_chunk: ChunkPos,
    pregen_radius: i32,
    cancel_token: &CancellationToken,
) -> bool {
    let total_chunks = ((pregen_radius * 2 + 1) * (pregen_radius * 2 + 1)) as usize;

    log::info!(
        "Preparing spawn area: {} chunks (radius {}) around chunk ({}, {})",
        total_chunks,
        pregen_radius,
        center_chunk.0.x,
        center_chunk.0.y,
    );

    #[cfg(feature = "slow_chunk_gen")]
    SLOW_CHUNK_GEN.store(true, Ordering::Relaxed);

    let elapsed = {
        let start = Instant::now();
        let completed = generate_pregen(world, center_chunk, pregen_radius, cancel_token).await;
        (start.elapsed(), completed)
    };

    #[cfg(feature = "slow_chunk_gen")]
    SLOW_CHUNK_GEN.store(false, Ordering::Relaxed);

    let elapsed_secs = elapsed.0.as_secs_f64();
    let chunks_per_second = if elapsed_secs > 0.0 {
        total_chunks as f64 / elapsed_secs
    } else {
        0.0
    };
    if elapsed.1 {
        log::info!(
            "Spawn area prepared: {total_chunks} chunks in {elapsed_secs:.2}s ({chunks_per_second:.1} chunks/s)",
        );
    } else {
        log::info!("Spawn area preparation cancelled after {elapsed_secs:.2}s");
    }
    elapsed.1
}

fn build_pregen_windows(center_chunk: ChunkPos, radius: i32) -> VecDeque<PregenWindow> {
    let min_x = center_chunk.0.x - radius;
    let max_x = center_chunk.0.x + radius;
    let min_z = center_chunk.0.y - radius;
    let max_z = center_chunk.0.y + radius;
    let mut windows = VecDeque::new();

    let mut z = min_z;
    while z <= max_z {
        let window_max_z = (z + PREGEN_WINDOW_SIZE - 1).min(max_z);
        let mut x = min_x;
        while x <= max_x {
            let window_max_x = (x + PREGEN_WINDOW_SIZE - 1).min(max_x);
            windows.push_back(PregenWindow {
                min_x: x,
                max_x: window_max_x,
                min_z: z,
                max_z: window_max_z,
            });
            x = window_max_x + 1;
        }
        z = window_max_z + 1;
    }

    windows
}

/// Generates chunks with progress reporting for pregeneration.
async fn generate_pregen(
    world: &Arc<World>,
    center_chunk: ChunkPos,
    radius: i32,
    cancel_token: &CancellationToken,
) -> bool {
    let total_chunks = ((radius * 2 + 1) * (radius * 2 + 1)) as usize;
    let mut pending_windows = build_pregen_windows(center_chunk, radius);
    let mut active_windows = Vec::with_capacity(PREGEN_ACTIVE_WINDOWS + 1);
    let mut last_report = Instant::now();
    let mut last_completed = 0usize;
    let mut completed = 0usize;
    let start = Instant::now();

    log::info!(
        "Pregeneration windowing: {PREGEN_WINDOW_SIZE}x{PREGEN_WINDOW_SIZE} target chunks, {PREGEN_ACTIVE_WINDOWS} active windows, dependency halo {FULL_DEPENDENCY_RADIUS} chunks",
    );

    fill_active_windows(world, &mut pending_windows, &mut active_windows);

    while completed < total_chunks {
        if cancel_token.is_cancelled() {
            release_all_windows(world, &mut active_windows);
            return false;
        }

        world.chunk_map.tick_scheduling();

        for active in &mut active_windows {
            active.poll(world);
        }

        let newly_ready_count = active_windows
            .iter()
            .filter(|active| active.ready && !active.counted)
            .count();
        for _ in 0..newly_ready_count {
            activate_next_window(world, &mut pending_windows, &mut active_windows);
        }

        for active in &mut active_windows {
            if active.ready && !active.counted {
                completed += active.window.chunk_count();
                active.counted = true;
            }
        }
        fill_active_windows(world, &mut pending_windows, &mut active_windows);
        world.chunk_map.tick_scheduling();
        release_unneeded_completed_windows(world, &pending_windows, &mut active_windows);

        if completed == total_chunks {
            break;
        }

        // Report progress every 5 seconds for large pregen
        if radius > SPAWN_RADIUS && last_report.elapsed() >= Duration::from_secs(5) {
            let ready_in_active = active_windows
                .iter()
                .filter(|active| !active.counted)
                .map(|active| active.ready_chunks)
                .sum::<usize>();
            let current_completed = (completed + ready_in_active).min(total_chunks);
            let elapsed = start.elapsed().as_secs_f64();
            let chunks_per_sec = if elapsed > 0.0 {
                (current_completed.saturating_sub(last_completed)) as f64
                    / last_report.elapsed().as_secs_f64()
            } else {
                0.0
            };
            let percent = (current_completed as f64 / total_chunks as f64) * 100.0;
            let remaining = total_chunks.saturating_sub(current_completed);
            let eta = if chunks_per_sec > 0.0 && remaining > 0 {
                remaining as f64 / chunks_per_sec
            } else {
                0.0
            };
            log::info!(
                "Progress: {current_completed}/{total_chunks} ({percent:.1}%), {chunks_per_sec:.1} chunks/s, ETA: {eta:.0}s",
            );
            last_report = Instant::now();
            last_completed = current_completed;
        }

        tokio::select! {
            () = cancel_token.cancelled() => {
                release_all_windows(world, &mut active_windows);
                return false;
            }
            () = sleep(Duration::from_millis(10)) => {}
        }
    }

    release_all_windows(world, &mut active_windows);
    true
}

fn fill_active_windows(
    world: &Arc<World>,
    pending_windows: &mut VecDeque<PregenWindow>,
    active_windows: &mut Vec<ActivePregenWindow>,
) {
    while active_windows.iter().filter(|active| !active.ready).count() < PREGEN_ACTIVE_WINDOWS {
        if !activate_next_window(world, pending_windows, active_windows) {
            break;
        }
    }
}

fn activate_next_window(
    world: &Arc<World>,
    pending_windows: &mut VecDeque<PregenWindow>,
    active_windows: &mut Vec<ActivePregenWindow>,
) -> bool {
    let Some(window) = pending_windows.pop_front() else {
        return false;
    };

    active_windows.push(ActivePregenWindow::new(world, window));
    true
}

fn release_unneeded_completed_windows(
    world: &Arc<World>,
    pending_windows: &VecDeque<PregenWindow>,
    active_windows: &mut Vec<ActivePregenWindow>,
) {
    let incomplete_windows = active_windows
        .iter()
        .filter(|active| !active.ready)
        .map(|active| active.window)
        .collect::<Vec<_>>();

    active_windows.retain(|active| {
        if !active.ready {
            return true;
        }

        let protected = active.window.protected_rect();
        let overlaps_incomplete = incomplete_windows
            .iter()
            .any(|window| protected.overlaps(window.protected_rect()));
        let overlaps_pending = pending_windows
            .iter()
            .any(|window| protected.overlaps(window.protected_rect()));

        overlaps_incomplete || overlaps_pending
    });

    world.chunk_map.tick_scheduling();
}

fn release_all_windows(world: &Arc<World>, active_windows: &mut Vec<ActivePregenWindow>) {
    active_windows.clear();
    world.chunk_map.tick_scheduling();
}
