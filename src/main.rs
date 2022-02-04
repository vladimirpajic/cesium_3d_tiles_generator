mod quadtree;
mod tiles;

use las::{Read, Reader, Color};
use std::path::{Path, PathBuf};
use crate::quadtree::{QuadTree, Aabb, Point};
use crate::tiles::{create_tile, TileSet, TileSetRoot, TileSetAsset, TileSetRootContent, TileSetRootBoundingVolume, TileSetRootChild};
use las::point::Classification;
use std::fs;
use std::io::Write;
use rayon::prelude::*;
use morton_encoding::morton_encode;

const CAPACITY: usize = 65536;

const LAS_PATH: &str = "C:\\temp\\01_LAS\\wgs84";

const OUTPUT_DIR: &str = "C:\\Cesium-1.88\\Apps\\tileset1";

fn main() {
    let las_path = Path::new(LAS_PATH);

    let output_dir = Path::new(OUTPUT_DIR);

    let mut global_tileset = TileSet {
        asset: TileSetAsset { version: "1.0".to_string() },
        geometric_error: 5000.0,
        root: TileSetRoot {
            content: TileSetRootContent { uri: "".to_string() },
            bounding_volume: TileSetRootBoundingVolume { bbox: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] },
            geometric_error: 2000.0,
            refine: "ADD".to_string(),
            children: Some(vec![]),
        }
    };

    let mut children = vec![];

    if las_path.is_dir() {
        let files = fs::read_dir(las_path).expect("IO Error");

        let mut las_files = vec![];
        for file_result in files {
            if let Ok(file) = file_result {
                let path = file.path();
                if path.is_file() && path.extension().unwrap_or_default().eq("las") {
                    las_files.push(path);
                }
            }
        };

        children = las_files.par_iter().map(|path| -> TileSetRootChild {
            let file_name = path.file_name().unwrap().to_str().unwrap().strip_suffix(".las").unwrap();

            let child_tileset = create_tileset_for_file(&path, &output_dir.join(file_name));

            TileSetRootChild {
                content: TileSetRootContent { uri: format!("{}/tileset.json", file_name) },
                bounding_volume: child_tileset.root.bounding_volume,
                geometric_error: child_tileset.root.geometric_error,
                refine: "ADD".to_string()
            }
        })
            .collect::<Vec<TileSetRootChild>>();
    }

    global_tileset.root.children = Some(children);

    global_tileset.root.bounding_volume.bbox = global_tileset.clone().root.children.unwrap().get(0).unwrap().bounding_volume.clone().bbox;

    let mut tileset_json_file = std::fs::File::create(output_dir.join("tileset.json")).unwrap();

    tileset_json_file.write_all(serde_json::to_string(&global_tileset).unwrap().into_bytes().as_slice());
}

fn create_tileset_for_file(source_path: &PathBuf, target_path: &PathBuf) -> TileSet {
    let mut reader =
        Reader::from_path(source_path).expect("Can't read LAS file.");

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

            let (x, y, z) = geodetic_to_geocentric(point.y, point.x, point.z);

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
                lod: 0,
                morton: 0,
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
    }, 1, CAPACITY as usize);

    for mut point in &mut points {
        let x_norm = (u32::MAX as f64 * (point.x - x_min) / (x_max - x_min)).round() as u32;
        let y_norm = (u32::MAX as f64 * (point.y - y_min) / (y_max - y_min)).round() as u32;
        point.morton = morton_encode([x_norm, y_norm]);
    }

    points.par_sort_by(|point1, point2| point1.morton.cmp(&point2.morton));

    let mut points_to_promote = vec![];

    for (index, point) in points.iter().enumerate() {
        if index % (4 * points.len() / CAPACITY as usize) == 0 {
            points_to_promote.push(point);
            continue;
        }

        quadtree.insert(&point, index, points.len());
    }

    create_tile(target_path, &quadtree)
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
