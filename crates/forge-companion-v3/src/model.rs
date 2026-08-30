//! glTF model loading — mesh, skeleton, animation clips.
//!
//! Loads a .glb file and extracts:
//! - Vertex data (position, normal, joints, weights) for skinned rendering
//! - Skeleton (joint hierarchy, inverse bind matrices)
//! - Animation clips (keyframed transforms per joint)
//!
//! Ported 2026-08-17 from `F:\NewRepo\crates\link-companion\src\model.rs`.

use std::collections::HashMap;

/// A skinned vertex — position, normal, joint indices, joint weights.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    /// Position in model space.
    pub position: [f32; 3],
    /// Surface normal.
    pub normal: [f32; 3],
    /// Texture coordinates.
    pub uv: [f32; 2],
    /// Joint indices for skinning.
    pub joints: [u32; 4],
    /// Joint weights (sum to 1.0).
    pub weights: [f32; 4],
}

/// A joint in the skeleton hierarchy.
#[derive(Clone, Debug)]
pub struct Joint {
    /// Joint name.
    pub name: String,
    /// Parent joint index, if any.
    pub parent: Option<usize>,
    /// Inverse bind pose matrix (16 f32 values in row-major order).
    pub inverse_bind: [f32; 16],
    /// Local transform (relative to parent).
    pub local_transform: Transform,
}

/// Decomposed transform for interpolation.
#[derive(Clone, Debug)]
pub struct Transform {
    /// Translation in local space.
    pub translation: [f32; 3],
    /// Rotation as quaternion (x, y, z, w).
    pub rotation: [f32; 4],
    /// Scale (uniform or per-axis).
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform {
    /// Convert to a 4x4 matrix (row-major).
    pub fn to_mat4(&self) -> [f32; 16] {
        let [tx, ty, tz] = self.translation;
        let [sx, sy, sz] = self.scale;
        let q = self.rotation;

        // Build rotation matrix from quaternion
        let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let wx = w * x;
        let wy = w * y;
        let wz = w * z;

        let mut mat = [0.0; 16];
        // Rotation part scaled by scale
        mat[0] = (1.0 - 2.0 * (yy + zz)) * sx;
        mat[1] = (2.0 * (xy - wz)) * sx;
        mat[2] = (2.0 * (xz + wy)) * sx;
        mat[3] = 0.0;

        mat[4] = (2.0 * (xy + wz)) * sy;
        mat[5] = (1.0 - 2.0 * (xx + zz)) * sy;
        mat[6] = (2.0 * (yz - wx)) * sy;
        mat[7] = 0.0;

        mat[8] = (2.0 * (xz - wy)) * sz;
        mat[9] = (2.0 * (yz + wx)) * sz;
        mat[10] = (1.0 - 2.0 * (xx + yy)) * sz;
        mat[11] = 0.0;

        mat[12] = tx;
        mat[13] = ty;
        mat[14] = tz;
        mat[15] = 1.0;

        mat
    }

    /// Linearly interpolate between two transforms.
    pub fn lerp(&self, other: &Transform, t: f32) -> Transform {
        let t = t.clamp(0.0, 1.0);
        Transform {
            translation: [
                self.translation[0] * (1.0 - t) + other.translation[0] * t,
                self.translation[1] * (1.0 - t) + other.translation[1] * t,
                self.translation[2] * (1.0 - t) + other.translation[2] * t,
            ],
            rotation: slerp(&self.rotation, &other.rotation, t),
            scale: [
                self.scale[0] * (1.0 - t) + other.scale[0] * t,
                self.scale[1] * (1.0 - t) + other.scale[1] * t,
                self.scale[2] * (1.0 - t) + other.scale[2] * t,
            ],
        }
    }
}

/// Spherical linear interpolation between two quaternions.
fn slerp(q1: &[f32; 4], q2: &[f32; 4], t: f32) -> [f32; 4] {
    let q1 = *q1;
    let mut q2 = *q2;
    let mut dot = q1[0] * q2[0] + q1[1] * q2[1] + q1[2] * q2[2] + q1[3] * q2[3];

    // If dot product is negative, negate one quaternion to take shorter path
    if dot < 0.0 {
        q2[0] = -q2[0];
        q2[1] = -q2[1];
        q2[2] = -q2[2];
        q2[3] = -q2[3];
        dot = -dot;
    }

    dot = dot.clamp(-1.0, 1.0);
    let theta = dot.acos();
    let sin_theta = theta.sin();

    if sin_theta < 0.001 {
        // Very close, use linear interpolation
        return [
            q1[0] * (1.0 - t) + q2[0] * t,
            q1[1] * (1.0 - t) + q2[1] * t,
            q1[2] * (1.0 - t) + q2[2] * t,
            q1[3] * (1.0 - t) + q2[3] * t,
        ];
    }

    let w1 = ((1.0 - t) * theta).sin() / sin_theta;
    let w2 = (t * theta).sin() / sin_theta;

    [
        q1[0] * w1 + q2[0] * w2,
        q1[1] * w1 + q2[1] * w2,
        q1[2] * w1 + q2[2] * w2,
        q1[3] * w1 + q2[3] * w2,
    ]
}

/// A single keyframe in an animation channel.
#[derive(Clone, Debug)]
pub struct Keyframe {
    /// Time in seconds.
    pub time: f32,
    /// Transform at this time.
    pub transform: Transform,
}

/// An animation clip — named collection of keyframe tracks per joint.
#[derive(Clone, Debug)]
pub struct AnimationClip {
    /// Clip name.
    pub name: String,
    /// Duration in seconds.
    pub duration: f32,
    /// Tracks indexed by joint index. None if that joint isn't animated in this clip.
    pub tracks: Vec<Option<Vec<Keyframe>>>,
}

impl AnimationClip {
    /// Sample the clip at a given time, returning a transform per joint.
    /// Joints without tracks return None.
    pub fn sample(&self, time: f32) -> Vec<Option<Transform>> {
        self.tracks
            .iter()
            .map(|track| {
                let frames = track.as_ref()?;
                if frames.is_empty() {
                    return None;
                }
                if frames.len() == 1 {
                    return Some(frames[0].transform.clone());
                }

                // Clamp time to clip duration.
                let t = time.clamp(0.0, self.duration);

                // Find surrounding keyframes.
                let mut prev_idx = 0;
                for (i, kf) in frames.iter().enumerate() {
                    if kf.time <= t {
                        prev_idx = i;
                    }
                }
                let next_idx = (prev_idx + 1).min(frames.len() - 1);

                if prev_idx == next_idx {
                    return Some(frames[prev_idx].transform.clone());
                }

                let prev = &frames[prev_idx];
                let next = &frames[next_idx];
                let segment_duration = next.time - prev.time;
                let factor = if segment_duration > 0.0 {
                    (t - prev.time) / segment_duration
                } else {
                    0.0
                };

                Some(prev.transform.lerp(&next.transform, factor))
            })
            .collect()
    }
}

/// Loaded texture image — RGBA pixels.
#[derive(Clone, Debug)]
pub struct TextureData {
    /// RGBA pixel data.
    pub pixels: Vec<u8>,
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
}

/// Complete loaded model — mesh + skeleton + clips + texture.
pub struct CompanionModel {
    /// All vertices.
    pub vertices: Vec<Vertex>,
    /// Triangle indices.
    pub indices: Vec<u32>,
    /// Skeleton joints.
    pub joints: Vec<Joint>,
    /// Animation clips.
    pub clips: Vec<AnimationClip>,
    /// Maps bone name → joint index for procedural bone lookup.
    pub bone_map: HashMap<String, usize>,
    /// Diffuse texture from the first material, if present.
    pub texture: Option<TextureData>,
}

impl CompanionModel {
    /// Load a glTF binary (.glb) file.
    ///
    /// v3: glTF loading deferred — generator.rs births the model procedurally (001 spec).
    /// This stub returns None. File loading is restored when a gltf-compatible zero-dep loader is available.
    pub fn load(_path: &str) -> Result<Self, String> {
        Err("glTF loading not available in v3 — use generate_painter() instead".into())
    }

    /// Find a clip by name.
    pub fn clip(&self, name: &str) -> Option<&AnimationClip> {
        self.clips.iter().find(|c| c.name == name)
    }

    /// Compute world-space joint matrices for the current pose.
    /// `local_overrides` allows procedural bones to override baked transforms.
    pub fn compute_joint_matrices(
        &self,
        sampled: &[Option<Transform>],
        local_overrides: &HashMap<usize, Transform>,
    ) -> Vec<[f32; 16]> {
        let n = self.joints.len();
        let mut world_matrices = vec![[0.0; 16]; n];
        let mut final_matrices = vec![[0.0; 16]; n];

        for i in 0..n {
            // Start with the joint's rest pose.
            let local = if let Some(ov) = local_overrides.get(&i) {
                ov.to_mat4()
            } else if let Some(Some(sampled_tf)) = sampled.get(i) {
                sampled_tf.to_mat4()
            } else {
                self.joints[i].local_transform.to_mat4()
            };

            world_matrices[i] = match self.joints[i].parent {
                Some(parent) => mat4_mul(&world_matrices[parent], &local),
                None => local,
            };

            final_matrices[i] = mat4_mul(&world_matrices[i], &self.joints[i].inverse_bind);
        }

        final_matrices
    }
}

/// Multiply two 4x4 matrices (row-major).
fn mat4_mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for i in 0..4 {
        for j in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[i * 4 + k] * b[k * 4 + j];
            }
            result[i * 4 + j] = sum;
        }
    }
    result
}
