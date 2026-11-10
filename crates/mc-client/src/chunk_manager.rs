//! Chunk management: dynamic loading/unloading around the player and worker thread generation & meshing.

use glam::Vec3;
use mc_game::BlockQuery;
use mc_world::{BlockId, BlockRegistry, Chunk, TerrainGenerator, CHUNK_X, CHUNK_Y, CHUNK_Z};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use wgpu::util::DeviceExt;

use crate::camera::Frustum;
use crate::mesh::{ChunkMesh, ChunkMeshContext};
use crate::ui::state::LoadStage;

pub const RENDER_DISTANCE: i32 = 6; // Chunks radius (13x13 chunks loaded)
pub const UNLOAD_DISTANCE: i32 = RENDER_DISTANCE + 2;

pub struct GpuChunkMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub opaque_index_count: u32,
    pub water_index_offset: u32,
    pub water_index_count: u32,
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
}

pub struct MeshJob {
    pub coords: (i32, i32),
    pub center: Arc<Chunk>,
    pub north: Option<Arc<Chunk>>,
    pub south: Option<Arc<Chunk>>,
    pub west: Option<Arc<Chunk>>,
    pub east: Option<Arc<Chunk>>,
}

pub struct MeshResult {
    pub coords: (i32, i32),
    pub mesh: ChunkMesh,
}

pub enum WorkerJob {
    Generate {
        coords: (i32, i32),
        seed: i64,
        world_dir: Option<PathBuf>,
    },
    Mesh(MeshJob),
}

pub enum WorkerResult {
    Generated {
        coords: (i32, i32),
        chunk: Arc<Chunk>,
    },
    Meshed(MeshResult),
}

pub struct ChunkManager {
    pub registry: BlockRegistry,
    loaded_chunks: HashMap<(i32, i32), Arc<Chunk>>,
    pub gpu_meshes: HashMap<(i32, i32), GpuChunkMesh>,
    pending_gen_jobs: HashSet<(i32, i32)>,
    pending_mesh_jobs: HashSet<(i32, i32)>,
    loading_spawn_chunks: HashSet<(i32, i32)>,
    finished_spawn_chunks: HashSet<(i32, i32)>,
    job_tx: Sender<WorkerJob>,
    result_rx: Receiver<WorkerResult>,
    pub player_chunk_x: i32,
    pub player_chunk_z: i32,
    pub world_dir: Option<PathBuf>,
    pub seed: i64,
    pub render_distance: i32,
}

impl ChunkManager {
    pub fn new(seed: i64, num_workers: usize) -> Self {
        Self::with_world(seed, num_workers, None)
    }

    pub fn with_world(seed: i64, num_workers: usize, world_dir: Option<PathBuf>) -> Self {
        let (job_tx, job_rx) = channel::<WorkerJob>();
        let (result_tx, result_rx) = channel::<WorkerResult>();

        // Wrap receiver in Arc<Mutex> so worker threads can pull jobs
        let job_rx = Arc::new(std::sync::Mutex::new(job_rx));

        for _ in 0..num_workers.max(2) {
            let rx = Arc::clone(&job_rx);
            let tx = result_tx.clone();
            thread::spawn(move || {
                let mut cached_gen: Option<(i64, Arc<TerrainGenerator>)> = None;
                loop {
                    let job = {
                        let Ok(guard) = rx.lock() else { break };
                        match guard.recv() {
                            Ok(job) => job,
                            Err(_) => break, // Channel closed
                        }
                    };

                    match job {
                        WorkerJob::Generate {
                            coords,
                            seed,
                            world_dir,
                        } => {
                            if cached_gen.as_ref().map(|(s, _)| *s) != Some(seed) {
                                cached_gen = Some((seed, Arc::new(TerrainGenerator::new(seed))));
                            }
                            let gen = &cached_gen.as_ref().unwrap().1;

                            let registry = BlockRegistry::default();
                            let mut chunk = if let Some(ref dir) = world_dir {
                                let rx = coords.0 >> 5;
                                let rz = coords.1 >> 5;
                                let region_path =
                                    dir.join("region").join(format!("r.{}.{}.mcr", rx, rz));
                                let mut from_disk = None;
                                if region_path.is_file() {
                                    if let Ok(mut region) = mc_world::RegionFile::open(&region_path)
                                    {
                                        if let Ok(Some(c)) = region.read_chunk(coords.0, coords.1) {
                                            from_disk = Some(c);
                                        }
                                    }
                                }
                                from_disk.unwrap_or_else(|| {
                                    let mut c = gen.generate_chunk(coords.0, coords.1, &registry);
                                    mc_world::lighting::LightingEngine::init_chunk_skylight(
                                        &mut c, &registry,
                                    );
                                    c
                                })
                            } else {
                                let mut c = gen.generate_chunk(coords.0, coords.1, &registry);
                                mc_world::lighting::LightingEngine::init_chunk_skylight(
                                    &mut c, &registry,
                                );
                                c
                            };
                            mc_world::lighting::LightingEngine::init_chunk_skylight(
                                &mut chunk, &registry,
                            );
                            if tx
                                .send(WorkerResult::Generated {
                                    coords,
                                    chunk: Arc::new(chunk),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        WorkerJob::Mesh(job) => {
                            let context = ChunkMeshContext {
                                center: &job.center,
                                north: job.north.as_deref(),
                                south: job.south.as_deref(),
                                west: job.west.as_deref(),
                                east: job.east.as_deref(),
                            };

                            let mesh = context.build_mesh();
                            if tx
                                .send(WorkerResult::Meshed(MeshResult {
                                    coords: job.coords,
                                    mesh,
                                }))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            });
        }

        Self {
            registry: BlockRegistry::default(),
            loaded_chunks: HashMap::new(),
            gpu_meshes: HashMap::new(),
            pending_gen_jobs: HashSet::new(),
            pending_mesh_jobs: HashSet::new(),
            loading_spawn_chunks: HashSet::new(),
            finished_spawn_chunks: HashSet::new(),
            job_tx,
            result_rx,
            player_chunk_x: i32::MAX, // Force update on first frame
            player_chunk_z: i32::MAX,
            world_dir,
            seed,
            render_distance: 8,
        }
    }

    /// Reset chunk manager with a new world seed and save directory
    pub fn set_world(&mut self, seed: i64, world_dir: Option<PathBuf>) {
        self.seed = seed;
        self.world_dir = world_dir;
        self.loaded_chunks.clear();
        self.gpu_meshes.clear();
        self.pending_gen_jobs.clear();
        self.pending_mesh_jobs.clear();
        self.loading_spawn_chunks.clear();
        self.finished_spawn_chunks.clear();
        self.player_chunk_x = i32::MAX;
        self.player_chunk_z = i32::MAX;
    }

    /// Flush and save all loaded chunks to McRegion (.mcr) files
    pub fn save_all_chunks(&self) {
        if let Some(ref dir) = self.world_dir {
            let region_dir = dir.join("region");
            let _ = std::fs::create_dir_all(&region_dir);
            for (&coords, chunk_arc) in &self.loaded_chunks {
                let rx = coords.0 >> 5;
                let rz = coords.1 >> 5;
                let region_path = region_dir.join(format!("r.{}.{}.mcr", rx, rz));
                if let Ok(mut region) = mc_world::RegionFile::open(&region_path) {
                    let _ = region.write_chunk(chunk_arc);
                }
            }
        }
    }

    /// Request meshing or remeshing for a chunk if loaded and not currently pending
    pub fn queue_mesh_or_remesh(&mut self, coords: (i32, i32)) {
        if !self.loaded_chunks.contains_key(&coords) || self.pending_mesh_jobs.contains(&coords) {
            return;
        }

        if let Some(center) = self.loaded_chunks.get(&coords).cloned() {
            let north = self.loaded_chunks.get(&(coords.0, coords.1 - 1)).cloned();
            let south = self.loaded_chunks.get(&(coords.0, coords.1 + 1)).cloned();
            let west = self.loaded_chunks.get(&(coords.0 - 1, coords.1)).cloned();
            let east = self.loaded_chunks.get(&(coords.0 + 1, coords.1)).cloned();

            self.pending_mesh_jobs.insert(coords);
            let _ = self.job_tx.send(WorkerJob::Mesh(MeshJob {
                coords,
                center,
                north,
                south,
                west,
                east,
            }));
        }
    }

    /// Request meshing for a chunk if loaded and not currently pending or meshed
    pub fn try_queue_mesh(&mut self, coords: (i32, i32)) {
        if self.gpu_meshes.contains_key(&coords) {
            return;
        }
        self.queue_mesh_or_remesh(coords);
    }

    /// Drain results from worker threads and upload new GPU meshes
    pub fn drain_worker_results(&mut self, device: &wgpu::Device) {
        while let Ok(res) = self.result_rx.try_recv() {
            match res {
                WorkerResult::Generated { coords, chunk } => {
                    self.pending_gen_jobs.remove(&coords);
                    self.loaded_chunks.insert(coords, chunk);

                    // Try meshing center and remesh its 4 neighbours to seal any border seams
                    self.try_queue_mesh(coords);
                    for &(nx, nz) in &[
                        (coords.0, coords.1 - 1),
                        (coords.0, coords.1 + 1),
                        (coords.0 - 1, coords.1),
                        (coords.0 + 1, coords.1),
                    ] {
                        self.queue_mesh_or_remesh((nx, nz));
                    }
                }
                WorkerResult::Meshed(res) => {
                    self.pending_mesh_jobs.remove(&res.coords);

                    if !res.mesh.is_empty() {
                        let v_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("Chunk VB ({}, {})", res.coords.0, res.coords.1)),
                            contents: bytemuck::cast_slice(&res.mesh.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                        let i_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("Chunk IB ({}, {})", res.coords.0, res.coords.1)),
                            contents: bytemuck::cast_slice(&res.mesh.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                        let min_x = (res.coords.0 * CHUNK_X as i32) as f32;
                        let min_z = (res.coords.1 * CHUNK_Z as i32) as f32;

                        self.gpu_meshes.insert(
                            res.coords,
                            GpuChunkMesh {
                                vertex_buffer: v_buf,
                                index_buffer: i_buf,
                                index_count: res.mesh.indices.len() as u32,
                                opaque_index_count: res.mesh.opaque_index_count,
                                water_index_offset: res.mesh.water_index_offset,
                                water_index_count: res.mesh.water_index_count,
                                aabb_min: Vec3::new(min_x, 0.0, min_z),
                                aabb_max: Vec3::new(
                                    min_x + CHUNK_X as f32,
                                    CHUNK_Y as f32,
                                    min_z + CHUNK_Z as f32,
                                ),
                            },
                        );
                    }

                    if self.loading_spawn_chunks.contains(&res.coords) {
                        self.finished_spawn_chunks.insert(res.coords);
                    }
                }
            }
        }
    }

    /// Begin requesting all chunks in a square radius R around spawn coordinates
    pub fn start_loading_spawn_area(&mut self, spawn_x: i32, spawn_z: i32, radius: i32) {
        let center_cx = spawn_x >> 4;
        let center_cz = spawn_z >> 4;
        let r = radius.clamp(2, 8);

        self.loading_spawn_chunks.clear();
        self.finished_spawn_chunks.clear();

        for dx in -r..=r {
            for dz in -r..=r {
                let coords = (center_cx + dx, center_cz + dz);
                self.loading_spawn_chunks.insert(coords);

                if self.gpu_meshes.contains_key(&coords) {
                    self.finished_spawn_chunks.insert(coords);
                } else if self.loaded_chunks.contains_key(&coords) {
                    self.try_queue_mesh(coords);
                } else if !self.pending_gen_jobs.contains(&coords) {
                    self.pending_gen_jobs.insert(coords);
                    let _ = self.job_tx.send(WorkerJob::Generate {
                        coords,
                        seed: self.seed,
                        world_dir: self.world_dir.clone(),
                    });
                }
            }
        }
    }

    /// Poll loading progress: returns current stage, number of chunks finished, and total chunks
    pub fn poll_loading(&mut self, device: &wgpu::Device) -> (LoadStage, usize, usize) {
        self.drain_worker_results(device);

        let total = self.loading_spawn_chunks.len();
        let done = self.finished_spawn_chunks.len();

        let stage = if !self.pending_gen_jobs.is_empty() {
            LoadStage::BuildingTerrain
        } else if !self.pending_mesh_jobs.is_empty() {
            LoadStage::BuildingMeshes
        } else {
            LoadStage::LoadingLevel
        };

        (stage, done, total)
    }

    /// Cancel loading screen jobs
    pub fn cancel_loading(&mut self) {
        self.loading_spawn_chunks.clear();
        self.finished_spawn_chunks.clear();
    }

    /// Find the highest solid non-liquid block with 2 air blocks above for safe spawn
    pub fn find_safe_spawn(&self, spawn_x: i32, spawn_z: i32) -> Vec3 {
        let check_col = |x: i32, z: i32| -> Option<f32> {
            for y in (1..126).rev() {
                let b = self.get_block(x, y, z);
                if b != BlockId::AIR
                    && b != BlockId::WATER
                    && b != BlockId::FLOWING_WATER
                    && b != BlockId::LAVA
                    && b != BlockId::FLOWING_LAVA
                {
                    let above1 = self.get_block(x, y + 1, z);
                    let above2 = self.get_block(x, y + 2, z);
                    if above1 == BlockId::AIR && above2 == BlockId::AIR {
                        return Some((y + 1) as f32);
                    }
                }
            }
            None
        };

        if let Some(y) = check_col(spawn_x, spawn_z) {
            return Vec3::new(spawn_x as f32 + 0.5, y, spawn_z as f32 + 0.5);
        }

        // Search in a small expanding spiral if spawn column is submerged
        for r in 1..=16 {
            for dx in -r..=r {
                for dz in -r..=r {
                    if dx * dx + dz * dz <= r * r {
                        if let Some(y) = check_col(spawn_x + dx, spawn_z + dz) {
                            return Vec3::new(
                                (spawn_x + dx) as f32 + 0.5,
                                y,
                                (spawn_z + dz) as f32 + 0.5,
                            );
                        }
                    }
                }
            }
        }

        Vec3::new(spawn_x as f32 + 0.5, 65.0, spawn_z as f32 + 0.5)
    }

    /// Update chunk loading and meshing around the player position
    pub fn update(&mut self, player_pos: Vec3, device: &wgpu::Device) {
        let cx = (player_pos.x / CHUNK_X as f32).floor() as i32;
        let cz = (player_pos.z / CHUNK_Z as f32).floor() as i32;

        let r_dist = self.render_distance.clamp(2, 16);
        let u_dist = r_dist + 2;

        self.drain_worker_results(device);

        // If player has moved into a new chunk or on initial setup
        if cx != self.player_chunk_x || cz != self.player_chunk_z {
            self.player_chunk_x = cx;
            self.player_chunk_z = cz;

            // 1. Unload distant chunks
            self.loaded_chunks.retain(|&(x, z), _| {
                let d_sq = (x - cx).pow(2) + (z - cz).pow(2);
                d_sq <= u_dist * u_dist
            });

            self.gpu_meshes.retain(|&(x, z), _| {
                let d_sq = (x - cx).pow(2) + (z - cz).pow(2);
                d_sq <= u_dist * u_dist
            });

            self.pending_gen_jobs.retain(|&(x, z)| {
                let d_sq = (x - cx).pow(2) + (z - cz).pow(2);
                d_sq <= u_dist * u_dist
            });

            self.pending_mesh_jobs.retain(|&(x, z)| {
                let d_sq = (x - cx).pow(2) + (z - cz).pow(2);
                d_sq <= u_dist * u_dist
            });

            // 2. Queue chunks within render distance for background generation
            let mut chunks_to_generate = Vec::new();
            for dx in -r_dist..=r_dist {
                for dz in -r_dist..=r_dist {
                    if dx * dx + dz * dz <= r_dist * r_dist {
                        let target = (cx + dx, cz + dz);
                        if !self.loaded_chunks.contains_key(&target)
                            && !self.pending_gen_jobs.contains(&target)
                        {
                            chunks_to_generate.push(target);
                        }
                    }
                }
            }

            // Sort generation by distance to player (closest first)
            chunks_to_generate.sort_by_key(|&(x, z)| (x - cx).pow(2) + (z - cz).pow(2));
            for coords in chunks_to_generate {
                self.pending_gen_jobs.insert(coords);
                let _ = self.job_tx.send(WorkerJob::Generate {
                    coords,
                    seed: self.seed,
                    world_dir: self.world_dir.clone(),
                });
            }

            // 3. Queue loaded chunks for meshing
            for dx in -r_dist..=r_dist {
                for dz in -r_dist..=r_dist {
                    if dx * dx + dz * dz <= r_dist * r_dist {
                        let target = (cx + dx, cz + dz);
                        self.try_queue_mesh(target);
                    }
                }
            }
        }
    }

    /// Get list of visible GPU meshes passing frustum culling
    pub fn get_visible_meshes<'a>(&'a self, frustum: &Frustum) -> Vec<&'a GpuChunkMesh> {
        self.gpu_meshes
            .values()
            .filter(|mesh| frustum.intersects_aabb(mesh.aabb_min, mesh.aabb_max))
            .collect()
    }

    pub fn loaded_chunks_count(&self) -> usize {
        self.loaded_chunks.len()
    }

    pub fn meshed_chunks_count(&self) -> usize {
        self.gpu_meshes.len()
    }

    /// Read block at world coordinates
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> BlockId {
        if !(0..128).contains(&y) {
            return BlockId::AIR;
        }
        let cx = x.div_euclid(16);
        let cz = z.div_euclid(16);
        let bx = x.rem_euclid(16) as usize;
        let bz = z.rem_euclid(16) as usize;

        if let Some(chunk) = self.loaded_chunks.get(&(cx, cz)) {
            chunk.get_block(bx, y as usize, bz)
        } else {
            BlockId::AIR
        }
    }

    /// Place or destroy a block at world coordinates and trigger remeshing
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block_id: BlockId) -> bool {
        if !(0..128).contains(&y) {
            return false;
        }
        let cx = x.div_euclid(16);
        let cz = z.div_euclid(16);
        let bx = x.rem_euclid(16) as usize;
        let bz = z.rem_euclid(16) as usize;

        let coords = (cx, cz);
        if let Some(chunk_arc) = self.loaded_chunks.get_mut(&coords) {
            let chunk = Arc::make_mut(chunk_arc);
            chunk.set_block(bx, y as usize, bz, block_id);

            // Request remesh without dropping the old mesh until new one is uploaded
            self.queue_mesh_or_remesh(coords);

            // If on chunk border, remesh adjacent neighbor
            if bx == 0 {
                self.queue_mesh_or_remesh((cx - 1, cz));
            } else if bx == 15 {
                self.queue_mesh_or_remesh((cx + 1, cz));
            }
            if bz == 0 {
                self.queue_mesh_or_remesh((cx, cz - 1));
            } else if bz == 15 {
                self.queue_mesh_or_remesh((cx, cz + 1));
            }

            true
        } else {
            false
        }
    }
}

impl BlockQuery for ChunkManager {
    fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.get_block(x, y, z)
    }
}
