extern crate num;

use num::Complex;
use std::str::FromStr;

/// Approximated Mandelbrot containment test
/// `limit` caps the number of iterations to try to
/// evaluate if Mandelbrot contains `c`
///
/// If `c` is decided to be a member returns `Some(i)`
/// with `i` being the number of iterations (up to `limit`)
/// the approximation ran for.
/// A result of `None` indicates the approximation iterated to the limit
fn approx_mandlebrot_test(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 }; // `re` is cartesian x, `im` cartesian y
    for i in 0..limit {
        z = z * z + c; // `z` outside a ball of radius two, centered at the origin will grow to inf
        if z.norm_sqr() > 4.0 { // compare the squared distance, cheaper than using sqroot
            return Some(i);
        }
    }
    None
}

/// Parse a delimited string into a pair
/// `s`should be a string delimited by `separator`
/// the format should be <left><sep><right> any other
/// form will be rejected
/// if so, the result will be `Some((l,r))` where `l` and `r`
/// hold the values of `T` parsed from the input
/// and invalid parse results in `None`
fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T,T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l,r)),
                _ => None
            }
        }
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("",          ','), None);
    assert_eq!(parse_pair::<i32>("10,",       ','), None);
    assert_eq!(parse_pair::<i32>(",10",       ','), None);
    assert_eq!(parse_pair::<i32>("10,20",     ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy",   ','), None);
    assert_eq!(parse_pair::<f64>("0.5x",      'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5",   'x'), Some((0.5, 1.5)));
    assert_eq!(parse_pair::<f64>("0.5 x 1.5", 'x'), None);
}

/// Parse a particular complex number representation
/// Format will be <real>,<imaginary>
fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex{ re, im }),
        None => None
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(parse_complex("1.25,-0.0625"), Some(Complex { re: 1.25, im: -0.0625 }));
    assert_eq!(parse_complex(",-0.0625"), None);
}

/// Translate a pixel coordinate to the complex plane
///
/// `bounds` sets the limits of the pixel space
/// `pixel` is the x,y location in catesian space of the pixel
/// `upper_left` and `lower_right` define the area of our image in complex terms
fn pixel_to_point(bounds: (usize, usize),
                  pixel: (usize, usize),
                  upper_left: Complex<f64>,
                  lower_right: Complex<f64>) -> Complex<f64> {
    let (width, height) = (lower_right.re - upper_left.re, upper_left.im - lower_right.im);

    Complex {
        re: upper_left.re + pixel.0 as f64 * width  / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64
        // subtract pixel.1, since it increases as we decrease the pixel space
        // but the imaginary component increases as we increase the complex space
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100, 100), (25, 75), Complex { re: -1.0, im: 1.0 }, Complex { re: 1.0, im: -1.0 }), Complex { re: -0.5, im: -0.5 });
}

fn main() {
    println!("Hello, world!");
}
