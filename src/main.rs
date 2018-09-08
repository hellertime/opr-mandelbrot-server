extern crate crossbeam;
extern crate env_logger;
extern crate image;
extern crate iron;
extern crate logger;
#[macro_use] extern crate mime;
extern crate num;
extern crate router;
extern crate urlencoded;

use image::ColorType;
use image::png::PNGEncoder;
use iron::prelude::*;
use iron::status;
use logger::Logger;
use num::Complex;
use router::Router;
use std::str::FromStr;
use urlencoded::UrlEncodedQuery;

/// Approximated Mandelbrot containment test
/// `limit` caps the number of iterations to try to
/// evaluate if Mandelbrot contains `c`
///
/// If `c` is decided to be a member returns `Some(i)`
/// with `i` being the number of iterations (up to `limit`)
/// the approximation ran for.
/// A result of `None` indicates the approximation iterated to the limit
fn approx_mandelbrot_test(c: Complex<f64>, limit: u32) -> Option<u32> {
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

/// Render pixel buffer
fn render(pixels: &mut [u8],
          bounds: (usize, usize),
          upper_left: Complex<f64>,
          lower_right: Complex<f64>) {
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0 .. bounds.1 {
        for col in 0 .. bounds.0 {
            let point = pixel_to_point(bounds, (col, row), upper_left, lower_right);
            pixels[row * bounds.0 + col] = match approx_mandelbrot_test(point, 255) {
                None => 0,
                Some(n) => 255 - n as u8
            };
        }
    }
}

fn get_index_page(_request: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();

    response.set_mut(status::Ok);
    response.set_mut(mime!(Text/Html; Charset=Utf8));
    response.set_mut(r#"
        <title>Mandlebrot</title>
        <img src="/mandelbrot.png?b=1000x750&u=-1.20,0.35&l=-1,0.20" height="750" width="1000"/>
    "#);

    Ok(response)
}

fn get_mandelbrot_image(request: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();

    let query_data = match request.get_ref::<UrlEncodedQuery>() {
        Err(e) => {
            response.set_mut(status::BadRequest);
            response.set_mut(format!("Error parsing query string: {:?}\n", e));
            return Ok(response);
        }
        Ok(map) => map
    };

    let bounds = match query_data.get("b") {
        None => {
            response.set_mut(status::BadRequest);
            response.set_mut(format!("missing the 'b' parameter\n"));
            return Ok(response);
        }
        Some(b) => match parse_pair(&b[0], 'x') {
            None => {
                response.set_mut(status::BadRequest);
                response.set_mut(format!("misformatted bounds\n"));
                return Ok(response);
            }
            Some(bounds) => bounds
        }
    };

    let lower_right = match query_data.get("l") {
        None => {
            response.set_mut(status::BadRequest);
            response.set_mut(format!("missing the 'l' parameter\n"));
            return Ok(response);
        }
        Some(l) => match parse_complex(&l[0]) {
            None => {
                response.set_mut(status::BadRequest);
                response.set_mut(format!("misformatted lower right\n"));
                return Ok(response);
            }
            Some(lower_right) => lower_right
        }
    };

    let upper_left = match query_data.get("u") {
        None => {
            response.set_mut(status::BadRequest);
            response.set_mut(format!("missing the 'u' parameter\n"));
            return Ok(response);
        }
        Some(u) => match parse_complex(&u[0]) {
            None => {
                response.set_mut(status::BadRequest);
                response.set_mut(format!("misformatted upper left\n"));
                return Ok(response);
            }
            Some(upper_left) => upper_left
        }
    };

    let mut pixels = vec![0; bounds.0 * bounds.1];

    {
        let threads = 8;
        let rows_per_band = bounds.1 / threads + 1;

        // this new scope hides this mutable borrow, so that write_image typechecks
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();

        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
                let band_lower_right = pixel_to_point(bounds, (bounds.0, top + height), upper_left, lower_right);

                spawner.spawn(move || {
                    render(band, band_bounds, band_upper_left, band_lower_right);
                });
            }
        });
    }

    let mut buf: Vec<u8> = Vec::new();

    // create a new lexical scope so the mutable borrow handed to the encoder will be dropped
    // we send pass buf to set_mut
    {
        let encoder = PNGEncoder::new(&mut buf);

        match encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8)) {
            Err(e) => {
                response.set_mut(status::InternalServerError);
                response.set_mut(format!("failed to encode png: {:?}\n", e));
                return Ok(response);
            }
            Ok(_) => ()
        }
    }

    response.set_mut(status::Ok);
    response.set_mut(mime!(Image/Png));
    response.set_mut(buf);
    Ok(response)
}

fn main() {
    env_logger::init();

    let (logger_before, logger_after) = Logger::new(None);

    let mut router = Router::new();

    router.get("/", get_index_page, "root");
    router.get("/mandelbrot.png", get_mandelbrot_image, "mandelbrot");

    let mut chain = Chain::new(router);

    chain.link_before(logger_before);
    chain.link_after(logger_after);
    
    println!("Serving on http://devbox.hemlock.hellertime.com:3000/...");
    Iron::new(chain).http("devbox.hemlock.hellertime.com:3000").unwrap();
}
