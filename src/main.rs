mod quadtree;
mod spatial_extent;
mod tiles;

use crate::quadtree::{Aabb, Point, QuadTree};
use crate::spatial_extent::SpatialExtent;
use crate::tiles::{
    create_tile, package_points, TileSet, TileSetAsset, TileSetRoot, TileSetRootBoundingVolume,
    TileSetRootChild, TileSetRootContent,
};
use las::point::Classification;
use las::{Color, Read, Reader};
use morton_encoding::morton_encode;
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const CAPACITY: usize = 100000;

const LAS_PATH: &str = "C:\\temp\\B2";

const OUTPUT_DIR: &str = "C:\\Cesium-1.88\\Apps\\tileset2";

fn main() {
    let las_path = Path::new(LAS_PATH);

    let output_dir = Path::new(OUTPUT_DIR);

    let mut global_tileset = TileSet {
        asset: TileSetAsset {
            version: "1.0".to_string(),
        },
        geometric_error: 5000.0,
        root: TileSetRoot {
            content: TileSetRootContent {
                uri: "root.pnts".to_string(),
            },
            bounding_volume: TileSetRootBoundingVolume {
                bbox: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            },
            geometric_error: 2000.0,
            refine: "ADD".to_string(),
            children: Some(vec![]),
        },
    };

    let mut children = vec![];

    if las_path.is_dir() {
        let files = fs::read_dir(las_path).expect("IO Error");

        let mut las_files = vec![];
        for file_result in files {
            if let Ok(file) = file_result {
                let path = file.path();
                if path.is_file()
                    && (path.extension().unwrap_or_default().eq("laz")
                        || path.extension().unwrap_or_default().eq("las"))
                {
                    las_files.push(path);
                }
            }
        }

        children = las_files
            .par_iter()
            .map(|path| -> (TileSetRootChild, Vec<Point>) {
                let file_name = path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .strip_suffix(".laz")
                    .unwrap();

                let (child_tileset, points_to_promote) =
                    create_tileset_for_file(&path, &output_dir.join(file_name));

                (
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: format!("{}/tileset.json", file_name),
                        },
                        bounding_volume: child_tileset.root.bounding_volume,
                        geometric_error: child_tileset.root.geometric_error,
                        refine: "ADD".to_string(),
                    },
                    points_to_promote,
                )
            })
            .collect::<Vec<(TileSetRootChild, Vec<Point>)>>();
    }

    println!("All LAS/LAZ files are processed");

    println!("Creating root tile set");

    let mut bbox = SpatialExtent::default();

    let mut global_tileset_points = vec![];

    let mut global_tileset_root_children = vec![];

    for child in children {
        global_tileset_root_children.push(child.0);
        for point in child.1 {
            bbox.update(&point);

            global_tileset_points.push(point);
        }
    }

    let half_width = (bbox.x_max - bbox.x_min) / 2.0;

    let half_length = (bbox.y_max - bbox.y_min) / 2.0;

    let half_height = (bbox.z_max - bbox.z_min) / 2.0;

    global_tileset.root.children = Some(global_tileset_root_children);

    let mut global_quadtree = QuadTree::new(
        Aabb {
            x_center: bbox.x_min + half_width,
            y_center: bbox.y_min + half_length,
            z_center: bbox.z_min + half_height,
            half_width,
            half_length,
            half_height,
        },
        1,
        global_tileset_points.len(),
    );

    for (index, point) in global_tileset_points.iter().enumerate() {
        global_quadtree.insert(point, index, global_tileset_points.len());
    }

    global_tileset.root.geometric_error = 0.05
        * (global_quadtree.bounds.half_width.powf(2.0_f64)
            + global_quadtree.bounds.half_length.powf(2.0_f64))
        .sqrt();

    global_tileset.root.bounding_volume = TileSetRootBoundingVolume::new(&global_quadtree);

    global_tileset.geometric_error = global_tileset.root.geometric_error * 5.0;

    let root_pnts = package_points(&&global_quadtree);

    println!("Saving root tile set");

    let mut pnts_file = std::fs::File::create(Path::new(OUTPUT_DIR).join("root.pnts")).unwrap();
    pnts_file.write_all(root_pnts.as_slice());

    let mut tileset_json_file = std::fs::File::create(output_dir.join("tileset.json")).unwrap();
    tileset_json_file.write_all(
        serde_json::to_string(&global_tileset)
            .unwrap()
            .into_bytes()
            .as_slice(),
    );

    println!("SUCCESS: Point cloud 3D tiles created successfully");
}

fn create_tileset_for_file(source_path: &PathBuf, target_path: &PathBuf) -> (TileSet, Vec<Point>) {
    let mut reader = Reader::from_path(source_path).expect("Can't read LAS file.");

    println!(
        "Processing LAS file {:?} with {} points",
        source_path.file_name().unwrap_or_default(),
        reader.header().number_of_points()
    );

    let mut points = vec![];

    let mut bbox = SpatialExtent::default();

    for point in reader.points() {
        if let Ok(las_point) = point {
            let color = if let Some(color) = las_point.color {
                color
            } else {
                Color::new(0xffff, 0xffff, 0x0000)
            };

            let (x, y, z) = geodetic_to_geocentric(las_point.y, las_point.x, las_point.z);

            let point = Point {
                morton: 0,
                x,
                y,
                z,
                r: color.red,
                g: color.green,
                b: color.blue,
                classification: u8::from(las_point.classification),
                is_edge_of_flight_line: las_point.is_edge_of_flight_line,
                is_synthetic: las_point.is_synthetic,
                is_key_point: las_point.is_key_point,
                is_withheld: las_point.is_withheld,
                is_overlap: las_point.is_overlap,
            };

            bbox.update(&point);

            points.push(point);
        }
    }

    let half_width = (bbox.x_max - bbox.x_min) / 2.0;

    let half_length = (bbox.y_max - bbox.y_min) / 2.0;

    let half_height = (bbox.z_max - bbox.z_min) / 2.0;

    let mut quadtree = QuadTree::new(
        Aabb {
            x_center: bbox.x_min + half_width,
            y_center: bbox.y_min + half_length,
            z_center: bbox.z_min + half_height,
            half_width,
            half_length,
            half_height,
        },
        1,
        CAPACITY as usize,
    );

    for mut point in &mut points {
        let x_norm =
            (u32::MAX as f64 * (point.x - bbox.x_min) / (bbox.x_max - bbox.x_min)).round() as u32;
        let y_norm =
            (u32::MAX as f64 * (point.y - bbox.y_min) / (bbox.y_max - bbox.y_min)).round() as u32;
        point.morton = morton_encode([x_norm, y_norm]);
    }

    points.par_sort_by(|point1, point2| point1.morton.cmp(&point2.morton));

    let mut points_to_promote = vec![];

    for (index, point) in points.iter().enumerate() {
        if 4 * points.len() / CAPACITY as usize > 0 {
            if index % (4 * points.len() / CAPACITY as usize) == 0 {
                points_to_promote.push(point.to_owned());
                continue;
            }
        }

        quadtree.insert(&point, index, points.len());
    }

    println!(
        "Creating tile set {:?}",
        target_path.file_name().unwrap_or_default()
    );

    let tile_set = create_tile(target_path, &quadtree);

    println!(
        "Tile set {:?} created",
        target_path.file_name().unwrap_or_default()
    );

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
