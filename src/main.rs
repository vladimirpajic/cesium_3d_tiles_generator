mod quadtree;
mod tiles;

use las::{Read, Reader, Color};
use std::path::{Path, PathBuf};
use crate::quadtree::{QuadTree, Aabb, Point};
use crate::tiles::{create_tile, TileSet, TileSetRoot, TileSetAsset, TileSetRootContent, TileSetRootBoundingVolume, TileSetRootChild, package_points};
use las::point::Classification;
use std::fs;
use std::io::Write;
use rayon::prelude::*;
use morton_encoding::morton_encode;

const CAPACITY: usize = 100000;

const LAS_PATH: &str = "C:\\data\\Campus Novi Sad\\Laser\\wgs84";

const OUTPUT_DIR: &str = "C:\\Cesium-1.88\\Apps\\tileset1";

fn main() {
    let las_path = Path::new(LAS_PATH);

    let output_dir = Path::new(OUTPUT_DIR);

    let mut global_tileset = TileSet {
        asset: TileSetAsset { version: "1.0".to_string() },
        geometric_error: 5000.0,
        root: TileSetRoot {
            content: TileSetRootContent { uri: "root.pnts".to_string() },
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

        children = las_files.par_iter().map(|path| -> (TileSetRootChild, Vec<Point>) {
            let file_name = path.file_name().unwrap().to_str().unwrap().strip_suffix(".las").unwrap();

            let (child_tileset, points_to_promote) = create_tileset_for_file(&path, &output_dir.join(file_name));

            (
            TileSetRootChild {
                content: TileSetRootContent { uri: format!("{}/tileset.json", file_name) },
                bounding_volume: child_tileset.root.bounding_volume,
                geometric_error: child_tileset.root.geometric_error,
                refine: "ADD".to_string()
            },
            points_to_promote,
            )
        })
            .collect::<Vec<(TileSetRootChild,  Vec<Point>)>>();
    }

    let mut x_min = f64::MAX;
    let mut x_max = f64::MIN;
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    let mut z_min = f64::MAX;
    let mut z_max = f64::MIN;

    let mut global_tileset_points = vec![];

    let mut global_tileset_root_children = vec![];

    for child in children {
        global_tileset_root_children.push(child.0);
        for point in child.1 {
            if point.x < x_min {
                x_min = point.x;
            }

            if point.x > x_max {
                x_max = point.x;
            }

            if point.y < y_min {
                y_min = point.y;
            }

            if point.y > y_max {
                y_max = point.y;
            }

            if point.z < z_min {
                z_min = point.z;
            }

            if point.z > z_max {
                z_max = point.z;
            }

            global_tileset_points.push(point);
        }
    }

    let half_width = (x_max - x_min) / 2.0;

    let half_length = (y_max - y_min) / 2.0;

    let half_height = (z_max - z_min) / 2.0;

    global_tileset.root.children = Some(global_tileset_root_children);

    let mut global_quadtree = QuadTree::new(Aabb {
        x_center: x_min + half_width,
        y_center: y_min + half_length,
        z_center: z_min + half_height,
        half_width,
        half_length,
        half_height,
    }, 1, global_tileset_points.len());

    for (index, point) in global_tileset_points.iter().enumerate() {
        global_quadtree.insert(point, index, global_tileset_points.len());
    }

    global_tileset.root.geometric_error = 0.1 * (global_quadtree.bounds.half_width.powf(2.0_f64) + global_quadtree.bounds.half_length.powf(2.0_f64)).sqrt();

    global_tileset.root.bounding_volume = TileSetRootBoundingVolume::new(&global_quadtree);

    global_tileset.geometric_error = global_tileset.root.geometric_error * 5.0;

    let root_pnts = package_points(&&global_quadtree);

    let mut pnts_file = std::fs::File::create(Path::new(OUTPUT_DIR).join("root.pnts")).unwrap();
    pnts_file.write_all(root_pnts.as_slice());

    let mut tileset_json_file = std::fs::File::create(output_dir.join("tileset.json")).unwrap();
    tileset_json_file.write_all(serde_json::to_string(&global_tileset).unwrap().into_bytes().as_slice());
}

fn create_tileset_for_file(source_path: &PathBuf, target_path: &PathBuf) -> (TileSet, Vec<Point>) {
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
            points_to_promote.push(point.to_owned());
            continue;
        }

        quadtree.insert(&point, index, points.len());
    }

    let tile_set = create_tile(target_path, &quadtree);

    (tile_set, points_to_promote)
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
