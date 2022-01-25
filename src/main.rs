mod quadtree;
mod tiles;

use las::{Read, Reader, Color};
use std::path::Path;
use crate::quadtree::{QuadTree, Aabb, Point};
use crate::tiles::create_tile;
use las::point::Classification;
use proj::Proj;

const MAX_POINTS_PER_TILE: u64 = 4096;

fn main() {
    let mut reader =
        Reader::from_path(Path::new("C:\\tmp\\BETHEL_MARKET_300FT_ROW_FINAL_200423.las")).expect("Can't read LAS file.");

    let crs_source = "EPSG:6559";

    let crs_target = "EPSG:4979";

    let crs_transformation = Proj::new_known_crs(&crs_source, &crs_target, None).unwrap();

    let mut points = vec![];

    let mut x_min = f64::MAX;
    let mut x_max = f64::MIN;
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    let mut z_min = f64::MAX;
    let mut z_max = f64::MIN;

    for point in reader.points() {
        if let Ok(point) = point {
            let color = if let Some(color) = point.color {
                color
            } else {
                match point.classification {
                    Classification::Ground => Color::new(0x8888, 0x8888, 0x0000),
                    Classification::LowVegetation => Color::new(0x0000, 0x8888, 0x0000),
                    Classification::MediumVegetation => Color::new(0x0000, 0xcccc, 0x0000),
                    Classification::HighVegetation => Color::new(0x0000, 0xffff, 0x0000),
                    Classification::Building => Color::new(0xffff, 0x0000, 0x0000),
                    _ => Color::new(0xffff, 0xffff, 0x0000),
                }
            };

            let (lon, lat) = crs_transformation.convert((point.x, point.y)).unwrap();

            let (x, y, z) = geodetic_to_geocentric(lat, lon, point.z);

            if x < x_min {
                x_min = x;
            }

            if x > x_max {
                x_max = x;
            }

            if y < y_min {
                y_min = y;
            }

            if y > y_max {
                y_max = y;
            }

            if z < z_min {
                z_min = z;
            }

            if z > z_max {
                z_max = z;
            }

            points.push(Point {
                x,
                y,
                z,
                r: color.red,
                g: color.green,
                b: color.blue
            });
        }
    }

    let half_width = (x_max - x_min) / 2.0;

    let half_length = (y_max - y_min) / 2.0;

    let half_height = (z_max - z_min) / 2.0;

    let mut quadtree = QuadTree::new(Aabb {
        x_center: x_min + half_width,
        y_center: y_min + half_length,
        z_center: z_min + half_height,
        half_width,
        half_length,
        half_height,
    }, 1, 16384);

    for point in points {
        quadtree.insert(&point);
    }

    create_tile(Path::new("C:\\Cesium-1.88\\Apps\\tileset"), &quadtree);
}

fn geodetic_to_geocentric(lat: f64, lon: f64, h: f64) -> (f64, f64, f64) {
    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians();
    let t: f64 = 1.0 - 1.0 / 298.257223563;
    let nn = 6378137.0 / (1.0 - (1.0 - t.powf(2.0)) * lat_rad.sin().powf(2.0)).sqrt();
    let x = (nn + h) * lat_rad.cos() * lon_rad.cos();
    let y = (nn + h) * lat_rad.cos() * lon_rad.sin();
    let z = (t.powf(2.0) * nn + h) * lat_rad.sin();

    (x, y, z)
}
