use crate::Radians;
use core::ops::{Add, Mul, Sub};
use glam::{DVec4, Vec4};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizeError {
    ZeroLength,
    NonFinite,
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    #[must_use]
    pub fn length(self) -> f32 {
        Vec2x4::from_array([self; 4]).length()[0]
    }

    pub fn try_normalize(self) -> Result<Self, NormalizeError> {
        Vec2x4::from_array([self; 4]).try_normalize()[0]
    }

    #[must_use]
    pub fn lerp(self, other: Self, weight: f32) -> Self {
        Vec2x4::from_array([self; 4])
            .lerp(Vec2x4::from_array([other; 4]), [weight; 4])
            .to_array()[0]
    }

    #[must_use]
    pub fn rotate(self, angle: Radians) -> Self {
        Vec2x4::from_array([self; 4]).rotate([angle; 4]).to_array()[0]
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, scale: f32) -> Self {
        Self::new(self.x * scale, self.y * scale)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vec2x4 {
    x: Vec4,
    y: Vec4,
}

impl Vec2x4 {
    #[inline]
    #[must_use]
    pub fn from_array(values: [Vec2; 4]) -> Self {
        Self {
            x: Vec4::from_array(values.map(|value| value.x)),
            y: Vec4::from_array(values.map(|value| value.y)),
        }
    }

    #[must_use]
    pub fn to_array(self) -> [Vec2; 4] {
        let x = self.x.to_array();
        let y = self.y.to_array();
        core::array::from_fn(|lane| Vec2::new(x[lane], y[lane]))
    }

    #[inline]
    #[must_use]
    pub fn dot(self, other: Self) -> [f32; 4] {
        (self.x * other.x + self.y * other.y).to_array()
    }

    #[inline]
    #[must_use]
    pub fn length(self) -> [f32; 4] {
        let scale = self.x.abs().max(self.y.abs());
        let divisor = Vec4::select(scale.cmpgt(Vec4::ZERO), scale, Vec4::ONE);
        let x = self.x / divisor;
        let y = self.y / divisor;
        let mut length = scale * (x * x + y * y).sqrt();
        let finite = self.x.is_finite_mask() & self.y.is_finite_mask();
        let large = finite & scale.cmpgt(Vec4::splat(f32::MAX * 0.5));
        if large.any() {
            let x = DVec4::from(self.x);
            let y = DVec4::from(self.y);
            length = Vec4::select(large, (x * x + y * y).sqrt().as_vec4(), length);
        }
        Vec4::select(finite, length, self.x.abs() + self.y.abs()).to_array()
    }

    pub fn try_normalize(self) -> [Result<Vec2, NormalizeError>; 4] {
        let scale = self.x.abs().max(self.y.abs());
        let finite = self.x.is_finite_mask() & self.y.is_finite_mask();
        let zero = scale.cmpeq(Vec4::ZERO);
        let valid = finite & !zero;
        let divisor = Vec4::select(valid, scale, Vec4::ONE);
        let x = Vec4::select(valid, self.x, Vec4::ZERO) / divisor;
        let y = Vec4::select(valid, self.y, Vec4::ZERO) / divisor;
        let length = Vec4::select(valid, (x * x + y * y).sqrt(), Vec4::ONE);
        let normalized = Self {
            x: x / length,
            y: y / length,
        }
        .to_array();
        let finite = finite.bitmask();
        let zero = zero.bitmask();
        core::array::from_fn(|lane| {
            if finite & (1 << lane) == 0 {
                Err(NormalizeError::NonFinite)
            } else if zero & (1 << lane) != 0 {
                Err(NormalizeError::ZeroLength)
            } else {
                Ok(normalized[lane])
            }
        })
    }

    #[must_use]
    pub fn lerp(self, other: Self, weights: [f32; 4]) -> Self {
        let weights = Vec4::from_array(weights);
        let start = weights.cmpeq(Vec4::ZERO);
        let end = weights.cmpeq(Vec4::ONE);
        let weights = DVec4::from(weights);
        let interpolate = |a: Vec4, b: Vec4| {
            let value = DVec4::from(a) + (DVec4::from(b) - DVec4::from(a)) * weights;
            Vec4::select(start, a, Vec4::select(end, b, value.as_vec4()))
        };
        Self {
            x: interpolate(self.x, other.x),
            y: interpolate(self.y, other.y),
        }
    }

    #[must_use]
    pub fn rotate(self, angles: [Radians; 4]) -> Self {
        let (sin, cos) = Vec4::from_array(angles.map(Radians::get)).sin_cos();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
        }
    }
}

impl Add for Vec2x4 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2x4 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<[f32; 4]> for Vec2x4 {
    type Output = Self;

    fn mul(self, scales: [f32; 4]) -> Self {
        let scales = Vec4::from_array(scales);
        Self {
            x: self.x * scales,
            y: self.y * scales,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{NormalizeError, Radians, Vec2, Vec2x4};
    use core::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4, PI};

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 2.0e-6 * expected.abs().max(1.0),
            "{actual} != {expected}"
        );
    }

    fn vector_close(actual: Vec2, expected: Vec2) {
        close(actual.x, expected.x);
        close(actual.y, expected.y);
    }

    #[test]
    fn arithmetic_preserves_independent_lanes_and_unclamped_weights() {
        let a = [
            Vec2::new(2.0, -3.0),
            Vec2::new(-4.0, 5.0),
            Vec2::new(6.0, 7.0),
            Vec2::new(-8.0, -9.0),
        ];
        let b = [
            Vec2::new(-1.0, 4.0),
            Vec2::new(3.0, -2.0),
            Vec2::new(5.0, -6.0),
            Vec2::new(-7.0, 8.0),
        ];
        let weights = [-1.0, 0.0, 0.25, 1.5];
        let expected = [
            Vec2::new(5.0, -10.0),
            Vec2::new(-4.0, 5.0),
            Vec2::new(5.75, 3.75),
            Vec2::new(-6.5, 16.5),
        ];
        let batch = Vec2x4::from_array(a)
            .lerp(Vec2x4::from_array(b), weights)
            .to_array();
        let dot = Vec2x4::from_array(a).dot(Vec2x4::from_array(b));
        for lane in 0..4 {
            vector_close(batch[lane], expected[lane]);
            vector_close(a[lane].lerp(b[lane], weights[lane]), expected[lane]);
            let expected_dot = [-14.0, -22.0, -12.0, -16.0][lane];
            close(dot[lane], expected_dot);
            close(a[lane].dot(b[lane]), expected_dot);
        }
    }

    #[test]
    fn interpolation_preserves_finite_endpoints_and_avoids_intermediate_overflow() {
        let a = [
            Vec2::new(-f32::MAX, f32::MAX),
            Vec2::new(-f32::MAX, f32::MAX),
            Vec2::new(-f32::MAX, f32::MAX),
            Vec2::new(1.0e8, -f32::MAX),
        ];
        let b = [
            Vec2::new(f32::MAX, -f32::MAX),
            Vec2::new(f32::MAX, -f32::MAX),
            Vec2::new(f32::MAX, -f32::MAX),
            Vec2::new(1.0, f32::from_bits(1)),
        ];
        let weights = [0.0, 0.5, 1.0, 1.0];
        let expected = [a[0], Vec2::ZERO, b[2], b[3]];
        let batch = Vec2x4::from_array(a)
            .lerp(Vec2x4::from_array(b), weights)
            .to_array();
        for lane in 0..4 {
            assert_eq!(batch[lane], expected[lane]);
            assert_eq!(a[lane].lerp(b[lane], weights[lane]), expected[lane]);
        }

        let a = [
            Vec2::new(f32::MAX * 0.5, -f32::MAX * 0.5),
            Vec2::new(f32::MAX * 0.25, -f32::MAX * 0.25),
            Vec2::new(f32::MAX, -f32::MAX),
            Vec2::new(-0.0, f32::from_bits(1)),
        ];
        let b = [a[1], a[0], a[2], Vec2::new(f32::MAX, -f32::MAX)];
        let weights = [3.0, -2.0, f32::MAX, 0.0];
        let expected = [
            Vec2::new(-f32::MAX * 0.25, f32::MAX * 0.25),
            Vec2::new(-f32::MAX * 0.25, f32::MAX * 0.25),
            a[2],
            a[3],
        ];
        let batch = Vec2x4::from_array(a)
            .lerp(Vec2x4::from_array(b), weights)
            .to_array();
        for lane in 0..4 {
            assert_eq!(batch[lane], expected[lane]);
            assert_eq!(a[lane].lerp(b[lane], weights[lane]), expected[lane]);
        }
        assert_eq!(batch[3].x.to_bits(), (-0.0_f32).to_bits());
    }

    #[test]
    fn lengths_and_directions_resist_overflow_and_underflow() {
        let tiny = f32::from_bits(1);
        let values = [
            Vec2::new(3.0, 4.0),
            Vec2::new(-5.0, 12.0),
            Vec2::new(f32::MAX, f32::MAX),
            Vec2::new(0.0, -tiny),
        ];
        let lengths = Vec2x4::from_array(values).length();
        close(lengths[0], 5.0);
        close(lengths[1], 13.0);
        assert_eq!(lengths[2].to_bits(), f32::INFINITY.to_bits());
        assert_eq!(lengths[3].to_bits(), tiny.to_bits());
        let expected = [
            Vec2::new(0.6, 0.8),
            Vec2::new(-5.0 / 13.0, 12.0 / 13.0),
            Vec2::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2),
            Vec2::new(0.0, -1.0),
        ];
        let normalized = Vec2x4::from_array(values).try_normalize();
        for lane in 0..4 {
            vector_close(normalized[lane].unwrap(), expected[lane]);
            vector_close(values[lane].try_normalize().unwrap(), expected[lane]);
            assert_eq!(values[lane].length().to_bits(), lengths[lane].to_bits());
        }

        let near_max = Vec2::new(f32::from_bits(0x7f7f_eeab), f32::from_bits(0x7cbc_5dbd));
        let values = [
            near_max,
            Vec2::new(f32::from_bits(0x7f7f_eeac), near_max.y),
            Vec2::new(3.0, 4.0),
            Vec2::new(-near_max.y, -near_max.x),
        ];
        let lengths = Vec2x4::from_array(values).length();
        let expected_lengths = [f32::MAX, f32::INFINITY, 5.0, f32::MAX]
            .into_iter()
            .enumerate();
        for (lane, expected) in expected_lengths {
            assert_eq!(lengths[lane].to_bits(), expected.to_bits());
            assert_eq!(values[lane].length().to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn invalid_directions_fail_per_lane_without_poisoning_valid_neighbors() {
        let values = [
            Vec2::new(-0.0, 0.0),
            Vec2::new(f32::NAN, 1.0),
            Vec2::new(1.0, f32::NEG_INFINITY),
            Vec2::new(-8.0, 6.0),
        ];
        let result = Vec2x4::from_array(values).try_normalize();
        assert_eq!(result[0], Err(NormalizeError::ZeroLength));
        assert_eq!(result[1], Err(NormalizeError::NonFinite));
        assert_eq!(result[2], Err(NormalizeError::NonFinite));
        vector_close(result[3].unwrap(), Vec2::new(-0.8, 0.6));
        for lane in 0..3 {
            assert_eq!(values[lane].try_normalize(), result[lane]);
        }
        let lengths = Vec2x4::from_array(values).length();
        assert_eq!(lengths[0].to_bits(), 0.0_f32.to_bits());
        assert!(lengths[1].is_nan());
        assert_eq!(lengths[2].to_bits(), f32::INFINITY.to_bits());
        close(lengths[3], 10.0);
    }

    #[test]
    fn rotations_use_each_lanes_angle_and_preserve_orientation() {
        let values = [
            Vec2::new(3.0, 4.0),
            Vec2::new(-2.0, 5.0),
            Vec2::new(6.0, -7.0),
            Vec2::new(-8.0, -9.0),
        ];
        let angles = [0.0, FRAC_PI_2, -FRAC_PI_2, PI].map(Radians::new);
        let expected = [
            Vec2::new(3.0, 4.0),
            Vec2::new(-5.0, -2.0),
            Vec2::new(-7.0, -6.0),
            Vec2::new(8.0, 9.0),
        ];
        let rotated = Vec2x4::from_array(values).rotate(angles).to_array();
        for lane in 0..4 {
            vector_close(rotated[lane], expected[lane]);
            vector_close(values[lane].rotate(angles[lane]), expected[lane]);
        }
        vector_close(
            Vec2::new(1.0, 0.0).rotate(Radians::new(FRAC_PI_4)),
            Vec2::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2),
        );

        let values = [
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
        ];
        let raw_angles = [1.0e8, -1.0e8, f32::MAX, -f32::MAX];
        let angles = raw_angles.map(Radians::new);
        let expected = [
            Vec2::new(-0.363_385_08, 0.931_639),
            Vec2::new(0.931_639, -0.363_385_08),
            Vec2::new(0.853_021, -0.521_876_5),
            Vec2::new(0.521_876_5, -0.853_021),
        ];
        let rotated = Vec2x4::from_array(values).rotate(angles).to_array();
        for lane in 0..4 {
            assert_eq!(angles[lane].get().to_bits(), raw_angles[lane].to_bits());
            vector_close(rotated[lane], expected[lane]);
            vector_close(values[lane].rotate(angles[lane]), expected[lane]);
        }
    }
}
