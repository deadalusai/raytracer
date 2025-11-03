use std::ops::Mul;

use crate::types::V3;

#[derive(Copy, Clone)]
pub struct Matrix([[f32; 4]; 4]);

impl Default for Matrix {
    fn default() -> Self {
        // Identity matrix
        Matrix([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl Matrix {

    pub fn translate(x: f32, y: f32, z: f32) -> Matrix {
        Matrix([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, z, 1.0],
        ])
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Matrix {
        Matrix([
            [x, 0.0, 0.0, 0.0],
            [0.0, y, 0.0, 0.0],
            [0.0, 0.0, z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotate_x(theta_rads: f32) -> Matrix {
        let sin = theta_rads.sin();
        let cos = theta_rads.cos();
        Matrix([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, cos, sin, 0.0],
            [0.0, -sin, cos, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotate_y(theta_rads: f32) -> Matrix {
        let sin = theta_rads.sin();
        let cos = theta_rads.cos();
        Matrix([
            [cos, 0.0, -sin, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [sin, 0.0, cos, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotate_z(theta_rads: f32) -> Matrix {
        let sin = theta_rads.sin();
        let cos = theta_rads.cos();
        Matrix([
            [cos, -sin, 0.0, 0.0],
            [sin, cos, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    // pub fn orthographic(width: f32, height: f32, depth: f32) -> Matrix {
    //     let x = 2.0 / width;
    //     let y = -2.0 / height;
    //     let z = -2.0 / depth;
    //     Matrix([
    //         [x, 0.0, 0.0, 0.0],
    //         [0.0, y, 0.0, 0.0],
    //         [0.0, 0.0, z, 0.0],
    //         [-1.0, 1.0, -1.0, 1.0],
    //     ])
    // }

    pub fn multiply(Matrix(m1): &Matrix, Matrix(m2): &Matrix) -> Matrix {
        let mut result = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += m1[i][k] * m2[k][j];
                }
            }
        }
        Matrix(result)
    }
}

impl Mul for Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Self) -> Self::Output {
        Matrix::multiply(&self, &rhs)
    }
}

// V3 * Matrix
impl Mul<Matrix> for V3 {
    type Output = V3;

    fn mul(self, Matrix(m): Matrix) -> Self::Output {
        V3(
            m[0][0]*self.0 + m[1][0]*self.1 + m[2][0]*self.2 + m[3][0],
            m[0][1]*self.0 + m[1][1]*self.1 + m[2][1]*self.2 + m[3][1],
            m[0][2]*self.0 + m[1][2]*self.1 + m[2][2]*self.2 + m[3][2],
         // m[0][3]*self.0 + m[1][3]*self.1 + m[2][3]*self.2 + m[3][3], // not used
        )
    }
}

// Transformation composition

pub struct MatrixBuilder {
    matrix: Matrix,
}

impl MatrixBuilder {
    pub fn new() -> Self {
        Self { matrix: Matrix::default() }
    }

    // Add translation
    pub fn translate(mut self, x: f32, y: f32, z: f32) -> Self {
        self.matrix = self.matrix * Matrix::translate(x, y, z);
        self
    }

    /// Add scale
    pub fn scale(mut self, x: f32, y: f32, z: f32) -> Self {
        self.matrix = self.matrix * Matrix::scale(x, y, z);
        self
    }

    /// Add rotation on the X axis
    pub fn rotate_x(mut self, theta_rads: f32) -> Self {
        self.matrix = self.matrix * Matrix::rotate_x(theta_rads);
        self
    }

    /// Add rotation on the Y axis
    pub fn rotate_y(mut self, theta_rads: f32) -> Self {
        self.matrix = self.matrix * Matrix::rotate_y(theta_rads);
        self
    }

    /// Add rotation on the Z axis
    pub fn rotate_z(mut self, theta_rads: f32) -> Self {
        self.matrix = self.matrix * Matrix::rotate_z(theta_rads);
        self
    }

    // /// Adds an orthographic projection transformation and return the completed transformation matrix.
    // pub fn orthographic(self, width: f32, height: f32, depth: f32) -> Matrix {
    //     self.matrix * Matrix::orthographic(width, height, depth)
    // }

    /// Return the completed transformation matrix
    pub fn done(self) -> Matrix {
        self.matrix
    }
}

#[cfg(test)]
mod test {
    use crate::types::V3;
    use crate::matrix::MatrixBuilder;
    use super::Matrix;

    macro_rules! assert_approx_eq {
        ($a:expr, $b:expr) => {
            assert_approx_eq!($a, $b, EPSILON=0.000001);
        };
        ($a:expr, $b:expr, EPSILON=$epsilon:expr) => {
            match (&$a, &$b, &$epsilon) {
                (a, b, e) => {
                    if (*a - *b).abs() > *e || (*b - *a).abs() > *e {
                        panic!("assertion {} ~== {} failed\n  left: {:?}\n right: {:?}", stringify!($a), stringify!($b), a, b);
                    }
                }
            }
        };
    }

    #[test]
    fn translate_1() {
        let p1 = V3(-1.0, -1.0, -1.0);
        let p2 = p1 * Matrix::translate(1.0, 1.0, 1.0);
        assert_eq!(p2, V3::ZERO);
    }

    #[test]
    fn translate_2() {
        let p2 = V3::ZERO * Matrix::translate(1.0, -1.0, 2.0);
        assert_eq!(p2, V3(1.0, -1.0, 2.0));
    }

    #[test]
    fn rotate_1() {
        let p1 = V3(0.0, 1.0, 0.0);
        let p2 = p1 * Matrix::rotate_x(90_f32.to_radians());
        assert_approx_eq!(p2.0, 0.0);
        assert_approx_eq!(p2.1, 0.0);
        assert_approx_eq!(p2.2, 1.0);
    }

    #[test]
    fn rotate_2() {
        let p1 = V3(1.0, 0.0, 0.0);
        let p2 = p1 * Matrix::rotate_y(270_f32.to_radians());
        assert_approx_eq!(p2.0, 0.0);
        assert_approx_eq!(p2.1, 0.0);
        assert_approx_eq!(p2.2, 1.0);
    }

    #[test]
    fn rotate_3() {
        let p1 = V3(1.0, 0.0, 0.0);
        let p2 = p1 * Matrix::rotate_z(-90_f32.to_radians());
        assert_approx_eq!(p2.0, 0.0);
        assert_approx_eq!(p2.1, 1.0);
        assert_approx_eq!(p2.2, 0.0);
    }

    #[test]
    fn scale_1() {
        let p1 = V3(1.0, -1.0, 1.0);
        let p2 = p1 * Matrix::scale(2.0, 2.0, 2.0);
        assert_approx_eq!(p2.0, 2.0);
        assert_approx_eq!(p2.1, -2.0);
        assert_approx_eq!(p2.2, 2.0);
    }

    #[test]
    fn scale_2() {
        let p1 = V3(1.0, -1.0, 1.0);
        let p2 = p1 * Matrix::scale(0.5, 0.5, 0.5);
        assert_approx_eq!(p2.0, 0.5);
        assert_approx_eq!(p2.1, -0.5);
        assert_approx_eq!(p2.2, 0.5);
    }

    #[test]
    fn transform_composition() {
        let matrix = MatrixBuilder::new()
            .scale(2.0, 2.0, 2.0)
            .rotate_x(90_f32.to_radians())
            .translate(1.0, 1.0, 1.0)
            .done();

        let p1 = V3(1.0, 1.0, 1.0);
        let p2 = p1 * matrix;
        assert_approx_eq!(p2.0, 3.0);
        assert_approx_eq!(p2.1, -1.0);
        assert_approx_eq!(p2.2, 3.0);
    }
}
