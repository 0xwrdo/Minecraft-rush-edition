//! Clean-room implementation of Minecraft 1.0 Improved Perlin Noise,
//! Octave Noise, and Terrain Density Grid Sampling.
//!
//! Has NO graphics dependencies and runs entirely headless.

pub mod png;

use mc_rand::JavaRandom;
use std::path::Path;

/// Improved Perlin Noise generator matching Minecraft 1.0.0.
#[derive(Clone, Debug, PartialEq)]
pub struct PerlinNoise {
    pub permutations: [u8; 512],
    pub x_coord: f64,
    pub y_coord: f64,
    pub z_coord: f64,
}

impl PerlinNoise {
    /// Construct a new PerlinNoise generator from a `JavaRandom`.
    ///
    /// Exact documented RNG call order in Minecraft 1.0:
    /// 1. `x_coord = random.next_double() * 256.0`
    /// 2. `y_coord = random.next_double() * 256.0`
    /// 3. `z_coord = random.next_double() * 256.0`
    /// 4. For `i` in `0..256`:
    ///    - `j = random.next_int_bounded(256 - i) + i`
    ///    - swap `permutations[i]` and `permutations[j]`
    ///    - `permutations[i + 256] = permutations[i]`
    pub fn new(random: &mut JavaRandom) -> Self {
        // RNG Call 1: x_coord
        let x_coord = random.next_double() * 256.0;
        // RNG Call 2: y_coord
        let y_coord = random.next_double() * 256.0;
        // RNG Call 3: z_coord
        let z_coord = random.next_double() * 256.0;

        let mut permutations = [0u8; 512];
        #[allow(clippy::needless_range_loop)]
        for i in 0..256 {
            permutations[i] = i as u8;
        }

        // RNG Calls 4..259: 256 bounded integer calls
        for i in 0..256 {
            let j = random.next_int_bounded((256 - i) as i32) as usize + i;
            permutations.swap(i, j);
            // Minecraft duplicate fill: copied immediately as index i is finalized
            permutations[i + 256] = permutations[i];
        }

        Self {
            permutations,
            x_coord,
            y_coord,
            z_coord,
        }
    }

    #[inline]
    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    #[inline]
    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b - a)
    }

    #[inline]
    fn grad2d(hash: u8, x: f64, z: f64) -> f64 {
        let h = (hash & 15) as i32;
        let u = (1.0 - ((h & 8) >> 3) as f64) * x;
        let v = if h < 4 {
            0.0
        } else if h != 12 && h != 14 {
            z
        } else {
            x
        };
        (if (h & 1) == 0 { u } else { -u }) + (if (h & 2) == 0 { v } else { -v })
    }

    #[inline]
    fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
        let h = (hash & 15) as i32;
        let u = if h < 8 { x } else { y };
        let v = if h < 4 {
            y
        } else if h != 12 && h != 14 {
            z
        } else {
            x
        };
        (if (h & 1) == 0 { u } else { -u }) + (if (h & 2) == 0 { v } else { -v })
    }

    /// Sample a 2D noise slice into `noise_array`.
    ///
    /// Specialized 2D fast path matching Minecraft 1.0 when `y_size == 1`.
    #[allow(clippy::too_many_arguments)]
    pub fn sample_2d(
        &self,
        noise_array: &mut [f64],
        x_offset: f64,
        z_offset: f64,
        x_size: usize,
        z_size: usize,
        x_scale: f64,
        z_scale: f64,
        noise_scale: f64,
    ) {
        self.populate_noise_array(
            noise_array,
            x_offset,
            0.0,
            z_offset,
            x_size,
            1,
            z_size,
            x_scale,
            0.0,
            z_scale,
            noise_scale,
        );
    }

    /// Populate noise array matching Minecraft 1.0 `NoiseGeneratorPerlin.populateNoiseArray`.
    #[allow(clippy::too_many_arguments)]
    pub fn populate_noise_array(
        &self,
        noise_array: &mut [f64],
        x_offset: f64,
        y_offset: f64,
        z_offset: f64,
        x_size: usize,
        y_size: usize,
        z_size: usize,
        x_scale: f64,
        y_scale: f64,
        z_scale: f64,
        noise_scale: f64,
    ) {
        let inv_noise_scale = 1.0 / noise_scale;

        // 2D specialized path
        if y_size == 1 {
            let mut idx = 0;
            for x in 0..x_size {
                let dx_base = x_offset + x as f64 * x_scale + self.x_coord;
                let xi = dx_base.floor() as i64;
                let x_mask = (xi & 255) as usize;
                let dx = dx_base - xi as f64;
                let u = Self::fade(dx);

                for z in 0..z_size {
                    let dz_base = z_offset + z as f64 * z_scale + self.z_coord;
                    let zi = dz_base.floor() as i64;
                    let z_mask = (zi & 255) as usize;
                    let dz = dz_base - zi as f64;
                    let w = Self::fade(dz);

                    let a = self.permutations[x_mask] as usize;
                    let aa = self.permutations[a] as usize + z_mask;
                    let b = self.permutations[x_mask + 1] as usize;
                    let ba = self.permutations[b] as usize + z_mask;

                    let grad_a = Self::grad2d(self.permutations[aa], dx, dz);
                    let grad_b = Self::grad2d(self.permutations[ba], dx - 1.0, dz);
                    let lerp_a = Self::lerp(u, grad_a, grad_b);

                    let grad_c = Self::grad2d(self.permutations[aa + 1], dx, dz - 1.0);
                    let grad_d = Self::grad2d(self.permutations[ba + 1], dx - 1.0, dz - 1.0);
                    let lerp_b = Self::lerp(u, grad_c, grad_d);

                    let val = Self::lerp(w, lerp_a, lerp_b);
                    noise_array[idx] += val * inv_noise_scale;
                    idx += 1;
                }
            }
            return;
        }

        // 3D path with vertical slice caching (prev_yi optimization)
        let mut idx = 0;
        let mut prev_yi = -1i64;
        let mut lerp_a1 = 0.0;
        let mut lerp_a2 = 0.0;
        let mut lerp_b1 = 0.0;
        let mut lerp_b2 = 0.0;

        for x in 0..x_size {
            let dx_base = x_offset + x as f64 * x_scale + self.x_coord;
            let xi = dx_base.floor() as i64;
            let x_mask = (xi & 255) as usize;
            let dx = dx_base - xi as f64;
            let u = Self::fade(dx);

            for z in 0..z_size {
                let dz_base = z_offset + z as f64 * z_scale + self.z_coord;
                let zi = dz_base.floor() as i64;
                let z_mask = (zi & 255) as usize;
                let dz = dz_base - zi as f64;
                let w = Self::fade(dz);

                for y in 0..y_size {
                    let dy_base = y_offset + y as f64 * y_scale + self.y_coord;
                    let yi = dy_base.floor() as i64;
                    let y_mask = (yi & 255) as usize;
                    let dy = dy_base - yi as f64;
                    let v = Self::fade(dy);

                    if y == 0 || yi != prev_yi {
                        prev_yi = yi;
                        let a = self.permutations[x_mask] as usize + y_mask;
                        let aa = self.permutations[a] as usize + z_mask;
                        let ab = self.permutations[a + 1] as usize + z_mask;
                        let b = self.permutations[x_mask + 1] as usize + y_mask;
                        let ba = self.permutations[b] as usize + z_mask;
                        let bb = self.permutations[b + 1] as usize + z_mask;

                        lerp_a1 = Self::lerp(
                            u,
                            Self::grad(self.permutations[aa], dx, dy, dz),
                            Self::grad(self.permutations[ba], dx - 1.0, dy, dz),
                        );
                        lerp_a2 = Self::lerp(
                            u,
                            Self::grad(self.permutations[ab], dx, dy - 1.0, dz),
                            Self::grad(self.permutations[bb], dx - 1.0, dy - 1.0, dz),
                        );
                        lerp_b1 = Self::lerp(
                            u,
                            Self::grad(self.permutations[aa + 1], dx, dy, dz - 1.0),
                            Self::grad(self.permutations[ba + 1], dx - 1.0, dy, dz - 1.0),
                        );
                        lerp_b2 = Self::lerp(
                            u,
                            Self::grad(self.permutations[ab + 1], dx, dy - 1.0, dz - 1.0),
                            Self::grad(self.permutations[bb + 1], dx - 1.0, dy - 1.0, dz - 1.0),
                        );
                    }

                    let lerp_y1 = Self::lerp(v, lerp_a1, lerp_a2);
                    let lerp_y2 = Self::lerp(v, lerp_b1, lerp_b2);
                    let val = Self::lerp(w, lerp_y1, lerp_y2);

                    noise_array[idx] += val * inv_noise_scale;
                    idx += 1;
                }
            }
        }
    }
}

/// Multi-octave Perlin noise wrapper.
#[derive(Clone, Debug, PartialEq)]
pub struct OctaveNoise {
    pub generators: Vec<PerlinNoise>,
    pub octaves: usize,
}

impl OctaveNoise {
    /// Construct an `OctaveNoise` with `octaves` layers from `JavaRandom`.
    ///
    /// Exact RNG call order in Minecraft 1.0:
    /// Sequential creation of `octaves` instances of `PerlinNoise` from the same RNG.
    pub fn new(random: &mut JavaRandom, octaves: usize) -> Self {
        let mut generators = Vec::with_capacity(octaves);
        for _ in 0..octaves {
            generators.push(PerlinNoise::new(random));
        }
        Self {
            generators,
            octaves,
        }
    }

    /// Generate multi-octave noise summing layers with doubled frequency and halved amplitude.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_noise_octaves(
        &self,
        noise_array: &mut [f64],
        x_offset: i32,
        y_offset: i32,
        z_offset: i32,
        x_size: usize,
        y_size: usize,
        z_size: usize,
        x_scale: f64,
        y_scale: f64,
        z_scale: f64,
    ) {
        for val in noise_array.iter_mut() {
            *val = 0.0;
        }

        let mut d = 1.0;
        for i in 0..self.octaves {
            let mut cur_x = x_offset as f64 * d * x_scale;
            let cur_y = y_offset as f64 * d * y_scale;
            let mut cur_z = z_offset as f64 * d * z_scale;

            let mut xi = cur_x.floor() as i64;
            let mut zi = cur_z.floor() as i64;
            cur_x -= xi as f64;
            cur_z -= zi as f64;

            // TODO(verify): Minecraft 1.0 coordinates modulo 16,777,216 (2^24) in octaves
            xi %= 16_777_216;
            zi %= 16_777_216;
            cur_x += xi as f64;
            cur_z += zi as f64;

            self.generators[i].populate_noise_array(
                noise_array,
                cur_x,
                cur_y,
                cur_z,
                x_size,
                y_size,
                z_size,
                x_scale * d,
                y_scale * d,
                z_scale * d,
                d,
            );
            d /= 2.0;
        }
    }
}

/// 5x17x5 Coarse Density Sampler with Trilinear Interpolation to 16x128x16 fine resolution.
pub struct TerrainDensitySampler;

impl TerrainDensitySampler {
    /// Trilinearly interpolates a coarse 5x17x5 grid to a 16x128x16 fine block density grid.
    ///
    /// Grid dimensions:
    /// - Horizontal step: 4 blocks (4 intervals of 4 blocks = 16 blocks)
    /// - Vertical step: 8 blocks (16 intervals of 8 blocks = 128 blocks)
    ///
    /// Fine array indexing matches Minecraft 1.0 chunk block order:
    /// `idx = y | (z << 7) | (x << 11)`
    pub fn interpolate_chunk(coarse: &[f64; 5 * 17 * 5], fine: &mut [f64; 16 * 128 * 16]) {
        for grid_x in 0..4 {
            for grid_z in 0..4 {
                for grid_y in 0..16 {
                    let d000 = coarse[(grid_x * 5 + grid_z) * 17 + grid_y];
                    let d001 = coarse[(grid_x * 5 + (grid_z + 1)) * 17 + grid_y];
                    let d010 = coarse[(grid_x * 5 + grid_z) * 17 + (grid_y + 1)];
                    let d011 = coarse[(grid_x * 5 + (grid_z + 1)) * 17 + (grid_y + 1)];
                    let d100 = coarse[((grid_x + 1) * 5 + grid_z) * 17 + grid_y];
                    let d101 = coarse[((grid_x + 1) * 5 + (grid_z + 1)) * 17 + grid_y];
                    let d110 = coarse[((grid_x + 1) * 5 + grid_z) * 17 + (grid_y + 1)];
                    let d111 = coarse[((grid_x + 1) * 5 + (grid_z + 1)) * 17 + (grid_y + 1)];

                    for sub_y in 0..8 {
                        let ty = sub_y as f64 / 8.0;
                        let y = grid_y * 8 + sub_y;

                        let d00 = d000 + (d010 - d000) * ty;
                        let d01 = d001 + (d011 - d001) * ty;
                        let d10 = d100 + (d110 - d100) * ty;
                        let d11 = d101 + (d111 - d101) * ty;

                        for sub_x in 0..4 {
                            let tx = sub_x as f64 / 4.0;
                            let x = grid_x * 4 + sub_x;

                            let d0 = d00 + (d10 - d00) * tx;
                            let d1 = d01 + (d11 - d01) * tx;

                            for sub_z in 0..4 {
                                let tz = sub_z as f64 / 4.0;
                                let z = grid_z * 4 + sub_z;

                                let density = d0 + (d1 - d0) * tz;
                                let idx = y | (z << 7) | (x << 11);
                                fine[idx] = density;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Dumps a 2D floating-point noise slice as an 8-bit grayscale PNG image.
///
/// Normalizes the minimum and maximum noise values in the slice to `[0, 255]`.
pub fn dump_noise_slice_png(
    slice: &[f64],
    width: u32,
    height: u32,
    path: &Path,
) -> std::io::Result<()> {
    assert_eq!(
        slice.len(),
        (width * height) as usize,
        "Slice length must equal width * height"
    );

    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    for &val in slice {
        if val < min_val {
            min_val = val;
        }
        if val > max_val {
            max_val = val;
        }
    }

    let range = if (max_val - min_val).abs() < 1e-9 {
        1.0
    } else {
        max_val - min_val
    };

    let mut pixels = Vec::with_capacity(slice.len());
    for &val in slice {
        let normalized = ((val - min_val) / range * 255.0).clamp(0.0, 255.0) as u8;
        pixels.push(normalized);
    }

    png::save_grayscale_png(path, width, height, &pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin_determinism() {
        let mut r1 = JavaRandom::new(12345);
        let perlin1 = PerlinNoise::new(&mut r1);

        let mut r2 = JavaRandom::new(12345);
        let perlin2 = PerlinNoise::new(&mut r2);

        assert_eq!(perlin1, perlin2);

        let mut out1 = [0.0; 16];
        let mut out2 = [0.0; 16];
        perlin1.sample_2d(&mut out1, 5.0, 5.0, 4, 4, 0.1, 0.1, 1.0);
        perlin2.sample_2d(&mut out2, 5.0, 5.0, 4, 4, 0.1, 0.1, 1.0);

        assert_eq!(out1, out2);
    }

    #[test]
    fn test_perlin_2d_reference_fixed_coords() {
        let mut r = JavaRandom::new(12345);
        let perlin = PerlinNoise::new(&mut r);
        let mut out = [0.0; 9];
        perlin.sample_2d(&mut out, 10.0, 20.0, 3, 3, 0.1, 0.1, 1.0);

        let expected = [
            0.5837657371955226,
            0.6451790060151252,
            0.6780499572750542,
            0.45100284344622144,
            0.518278901956207,
            0.5662467989428863,
            0.3042759303587461,
            0.3772600633385487,
            0.43734009880170643,
        ];

        for (a, b) in out.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-14, "Mismatch: {} != {}", a, b);
        }
    }

    #[test]
    fn test_perlin_3d_reference_fixed_coords() {
        let mut r = JavaRandom::new(54321);
        let perlin = PerlinNoise::new(&mut r);
        let mut out = [0.0; 12];
        perlin.populate_noise_array(&mut out, 5.0, 10.0, 15.0, 2, 3, 2, 0.2, 0.2, 0.2, 1.0);

        let expected = [
            0.4393295944629251,
            0.2573487800310885,
            0.18027111441555732,
            0.2835922354502935,
            -0.026460273165973158,
            -0.15778248785653648,
            0.43554768806873634,
            0.2779848539161731,
            0.2112493824756077,
            0.34458134735797563,
            0.0759925752552004,
            -0.0377677440886726,
        ];

        for (a, b) in out.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-14, "Mismatch: {} != {}", a, b);
        }
    }

    #[test]
    fn test_octave_3d_reference_fixed_coords() {
        let mut r = JavaRandom::new(99999);
        let oct = OctaveNoise::new(&mut r, 4);
        let mut out = [0.0; 8];
        oct.generate_noise_octaves(&mut out, 1, 2, 3, 2, 2, 2, 0.5, 0.5, 0.5);

        let expected = [
            0.02044790947527919,
            0.19778901071743116,
            -0.07259362481748766,
            0.8549219744767926,
            -1.5007277164817743,
            -1.0718347056258426,
            -1.2635432127780475,
            -0.13283151929716408,
        ];

        for (a, b) in out.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-14, "Mismatch: {} != {}", a, b);
        }
    }

    #[test]
    fn test_dump_noise_slice_png() {
        let mut r = JavaRandom::new(42);
        let oct = OctaveNoise::new(&mut r, 4);
        let width = 64;
        let height = 64;
        let mut slice = vec![0.0; width * height];
        oct.generate_noise_octaves(&mut slice, 0, 0, 0, width, 1, height, 0.05, 0.0, 0.05);

        let output_path = Path::new("target/test_noise_slice.png");
        std::fs::create_dir_all("target").unwrap();
        dump_noise_slice_png(&slice, width as u32, height as u32, output_path).unwrap();

        assert!(output_path.exists());
        let meta = std::fs::metadata(output_path).unwrap();
        assert!(
            meta.len() > 100,
            "PNG file should contain valid header and data"
        );
    }
}
