use core::ops::Mul;

use glam::camera::lh::proj::directx;

use crate::{Radians, Vec3, Vec4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(glam::Mat4);

impl Mat4 {
    pub const IDENTITY: Self = Self(glam::Mat4::IDENTITY);

    #[must_use]
    pub fn from_translation(translation: Vec3) -> Self {
        Self(glam::Mat4::from_translation(glam::Vec3::new(
            translation.x,
            translation.y,
            translation.z,
        )))
    }

    #[must_use]
    pub fn perspective_lh(vertical_fov: Radians, aspect: f32, near: f32, far: f32) -> Self {
        assert!(vertical_fov.get() > 0.0 && vertical_fov.get() < core::f32::consts::PI);
        assert!(aspect.is_finite() && aspect > 0.0);
        assert!(near.is_finite() && far.is_finite() && near > 0.0 && far > near);
        Self(directx::perspective(vertical_fov.get(), aspect, near, far))
    }

    #[must_use]
    pub fn to_cols_array(self) -> [f32; 16] {
        self.0.to_cols_array()
    }
}

impl Mul for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    fn mul(self, rhs: Vec4) -> Vec4 {
        let value = self.0 * glam::Vec4::new(rhs.x, rhs.y, rhs.z, rhs.w);
        Vec4::new(value.x, value.y, value.z, value.w)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mat4, Radians, Vec3, Vec4};

    #[test]
    fn left_handed_projection_preserves_perspective_and_zero_to_one_depth() {
        let projection =
            Mat4::perspective_lh(Radians::new(core::f32::consts::FRAC_PI_2), 2.0, 1.0, 9.0);
        let transform = projection * Mat4::from_translation(Vec3::new(2.0, -1.0, 4.0));
        let clip = transform * Vec4::new(0.0, 2.0, 0.0, 1.0);
        for (actual, expected) in [(clip.x, 1.0), (clip.y, 1.0), (clip.z, 3.375), (clip.w, 4.0)] {
            assert!((actual - expected).abs() < 1e-6);
        }
        for (distance, expected_depth) in [(1.0, 0.0), (9.0, 1.0)] {
            let clip = projection * Vec4::new(0.0, 0.0, distance, 1.0);
            assert!((clip.z / clip.w - expected_depth).abs() < 1e-6);
        }
        let farther = projection * Vec4::new(2.0, 1.0, 8.0, 1.0);
        assert!((clip.x / clip.w - 2.0 * farther.x / farther.w).abs() < 1e-6);
        let behind = projection * Vec4::new(0.0, 0.0, -4.0, 1.0);
        assert!(behind.w < 0.0);
        let translation = Mat4::from_translation(Vec3::new(2.0, -1.0, 4.0));
        assert_eq!(&translation.to_cols_array()[12..], &[2.0, -1.0, 4.0, 1.0]);
        assert_eq!(Mat4::IDENTITY * translation, translation);
    }
}
