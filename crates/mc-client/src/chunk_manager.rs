//! Chunk management: dynamic loading/unloading around the player and worker thread meshing.

use glam::Vec3;
use mc_world::{BlockRegistry, Chunk, TerrainGenerator, CHUNK_X, CHUNK_Y, CHUNK_Z};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use wgpu::util::DeviceExt;

use crate::camera::Frustum;
use crate::mesh::{ChunkMesh, ChunkMeshContext};

pub const RENDER_DISTANCE: i32 = 6; // Chunks radius (13x13 chunks loaded)
pub const UNLOAD_DISTANCE: i32 = RENDER_DISTANCE + 2;

pub struct GpuChunkMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
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

pub struct ChunkManager {
    generator: TerrainGenerator,
    registry: BlockRegistry,
    loaded_chunks: HashMap<(i32, i32), Arc<Chunk>>,
    pub gpu_meshes: HashMap<(i32, i32), GpuChunkMesh>,
    pending_mesh_jobs: HashSet<(i32, i32)>,
    job_tx: Sender<MeshJob>,
    result_rx: Receiver<MeshResult>,
    pub player_chunk_x: i32,
    pub player_chunk_z: i32,
}

impl ChunkManager {
    pub fn new(seed: i64, num_workers: usize) -> Self {
        let (job_tx, job_rx) = channel::<MeshJob>();
        let (result_tx, result_rx) = channel::<MeshResult>();

        // Wrap receiver in Arc<Mutex> so worker threads can pull jobs
        let job_rx = Arc::new(std::sync::Mutex::new(job_rx));

        for _ in 0..num_workers {
            let rx = Arc::clone(&job_rx);
            let tx = result_tx.clone();
            thread::spawn(move || {
                loop {
                    let job = {
                        let Ok(guard) = rx.lock() else { break };
                        match guard.recv() {
                            Ok(job) => job,
                            Err(_) => break, // Channel closed
                        }
                    };

                    let context = ChunkMeshContext {
                        center: &job.center,
                        north: job.north.as_deref(),
                        south: job.south.as_deref(),
                        west: job.west.as_deref(),
                        east: job.east.as_deref(),
                    };

                    let mesh = context.build_mesh();
                    if tx
                        .send(MeshResult {
                            coords: job.coords,
                            mesh,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }

        Self {
            generator: TerrainGenerator::new(seed),
            registry: BlockRegistry::default(),
            loaded_chunks: HashMap::new(),
            gpu_meshes: HashMap::new(),
            pending_mesh_jobs: HashSet::new(),
            job_tx,
            result_rx,
            player_chunk_x: i32::MAX, // Force update on first frame
            player_chunk_z: i32::MAX,
        }
    }

    /// Update chunk loading and meshing around the player position
    pub fn update(&mut self, player_pos: Vec3, device: &wgpu::Device) {
        let cx = (player_pos.x / CHUNK_X as f32).floor() as i32;
        let cz = (player_pos.z / CHUNK_Z as f32).floor() as i32;

        // Drain mesh results from worker threads
        while let Ok(res) = self.result_rx.try_recv() {
            self.pending_mesh_jobs.remove(&res.coords);

            // Only upload if still within unload distance
            let dist_sq = (res.coords.0 - cx).pow(2) + (res.coords.1 - cz).pow(2);
            if dist_sq <= UNLOAD_DISTANCE * UNLOAD_DISTANCE && !res.mesh.is_empty() {
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
                        aabb_min: Vec3::new(min_x, 0.0, min_z),
                        aabb_max: Vec3::new(
                            min_x + CHUNK_X as f32,
                            CHUNK_Y as f32,
                            min_z + CHUNK_Z as f32,
                        ),
                    },
                );
            }
        }

        // If player has moved into a new chunk or on initial setup
        if cx != self.player_chunk_x || cz != self.player_chunk_z {
            self.player_chunk_x = cx;
            self.player_chunk_z = cz;

            // 1. Unload distant chunks
            self.loaded_chunks.retain(|&(x, z), _| {
                let d_sq = (x - cx).pow(2) + (z - cz).pow(2);
                d_sq <= UNLOAD_DISTANCE * UNLOAD_DISTANCE
            });

            self.gpu_meshes.retain(|&(x, z), _| {
                let d_sq = (x - cx).pow(2) + (z - cz).pow(2);
                d_sq <= UNLOAD_DISTANCE * UNLOAD_DISTANCE
            });

            self.pending_mesh_jobs.retain(|&(x, z)| {
                let d_sq = (x - cx).pow(2) + (z - cz).pow(2);
                d_sq <= UNLOAD_DISTANCE * UNLOAD_DISTANCE
            });

            // 2. Load / generate chunks within RENDER_DISTANCE
            let mut chunks_to_generate = Vec::new();
            for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
                for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                    if dx * dx + dz * dz <= RENDER_DISTANCE * RENDER_DISTANCE {
                        let target = (cx + dx, cz + dz);
                        if !self.loaded_chunks.contains_key(&target) {
                            chunks_to_generate.push(target);
                        }
                    }
                }
            }

            // Sort generation by distance to player (closest first)
            chunks_to_generate.sort_by_key(|&(x, z)| (x - cx).pow(2) + (z - cz).pow(2));
            for coords in chunks_to_generate {
                let chunk = self
                    .generator
                    .generate_chunk(coords.0, coords.1, &self.registry);
                self.loaded_chunks.insert(coords, Arc::new(chunk));
            }

            // 3. Queue chunks that need meshing
            for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
                for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                    if dx * dx + dz * dz <= RENDER_DISTANCE * RENDER_DISTANCE {
                        let target = (cx + dx, cz + dz);
                        if !self.gpu_meshes.contains_key(&target)
                            && !self.pending_mesh_jobs.contains(&target)
                        {
                            if let Some(center) = self.loaded_chunks.get(&target).cloned() {
                                let north =
                                    self.loaded_chunks.get(&(target.0, target.1 - 1)).cloned();
                                let south =
                                    self.loaded_chunks.get(&(target.0, target.1 + 1)).cloned();
                                let west =
                                    self.loaded_chunks.get(&(target.0 - 1, target.1)).cloned();
                                let east =
                                    self.loaded_chunks.get(&(target.0 + 1, target.1)).cloned();

                                self.pending_mesh_jobs.insert(target);
                                let _ = self.job_tx.send(MeshJob {
                                    coords: target,
                                    center,
                                    north,
                                    south,
                                    west,
                                    east,
                                });
                            }
                        }
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
}
