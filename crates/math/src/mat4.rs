use core::ops::Mul;

use glam::camera::lh::proj::directx;
use glam::camera::lh::view;

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
    pub fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let eye = glam::Vec3::new(eye.x, eye.y, eye.z);
        let target = glam::Vec3::new(target.x, target.y, target.z);
        let up = glam::Vec3::new(up.x, up.y, up.z);
        assert!(eye.is_finite() && target.is_finite());
        assert!(up.is_normalized());
        let direction = (target - eye)
            .try_normalize()
            .expect("camera eye and target define a finite nonzero direction");
        assert!(
            up.cross(direction).length_squared() > 0.0,
            "camera up must not be parallel to the view direction"
        );
        let matrix = view::look_to_mat4(eye, direction, up);
        assert!(matrix.is_finite());
        Self(matrix)
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
    fn left_handed_view_places_eye_at_origin_and_target_in_front() {
        let view = Mat4::look_at_lh(
            Vec3::new(2.0, 4.0, -3.0),
            Vec3::new(2.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        for (point, expected) in [
            (Vec4::new(2.0, 4.0, -3.0, 1.0), [0.0, 0.0, 0.0, 1.0]),
            (Vec4::new(2.0, 1.0, 1.0, 1.0), [0.0, 0.0, 5.0, 1.0]),
            (Vec4::new(5.0, 4.0, -3.0, 1.0), [3.0, 0.0, 0.0, 1.0]),
            (Vec4::new(2.0, 8.0, 0.0, 1.0), [0.0, 5.0, 0.0, 1.0]),
        ] {
            let actual = view * point;
            for (actual, expected) in [actual.x, actual.y, actual.z, actual.w]
                .into_iter()
                .zip(expected)
            {
                assert!((actual - expected).abs() < 1e-6);
            }
        }
        let sideways = Mat4::look_at_lh(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(5.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(
            sideways * Vec4::new(6.0, 4.0, 0.0, 1.0),
            Vec4::new(3.0, 2.0, 5.0, 1.0)
        );
    }

    #[test]
    #[should_panic(expected = "camera up must not be parallel to the view direction")]
    fn view_rejects_up_parallel_to_direction() {
        let _ = Mat4::look_at_lh(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(1.0, 4.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
    }

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
