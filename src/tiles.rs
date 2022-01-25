use serde::Serialize;
use las::{Point, Color};
use crate::quadtree::{QuadTree, Aabb};
use std::path::{Path, PathBuf};
use std::borrow::BorrowMut;
use std::io::Write;

const MAGIC: &str = "pnts";

const VERSION: u32 = 1;

pub struct Header<'a> {
    pub magic: &'a str,
    pub version: u32,
    pub byte_length: u32,
    feature_table_json_byte_length: u32,
    feature_table_binary_byte_length: u32,
    batch_table_json_byte_length: u32,
    batch_table_binary_byte_length: u32
}

#[derive(Serialize, Debug)]
pub struct AttributePosition {
    #[serde(rename = "byteOffset")]
    byte_offset: u32,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct FeatureTableHeader {
    pub points_length: u32,
    pub rtc_center: Vec<f32>,
    pub position: AttributePosition,
    pub rgb: AttributePosition,
}

#[derive(Serialize, Debug)]
pub struct TileSetAsset {
    pub version: String,
}

#[derive(Serialize, Debug)]
pub struct TileSetRootContent {
    pub uri: String,
}

#[derive(Serialize, Debug)]
pub struct TileSetRootBoundingVolume {
    #[serde(rename = "box")]
    pub bbox: [f64; 12]
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TileSetRootChild {
    pub content: TileSetRootContent,
    pub bounding_volume: TileSetRootBoundingVolume,
    pub geometric_error: f64,
    pub refine: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TileSetRoot {
    pub content: TileSetRootContent,
    pub bounding_volume: TileSetRootBoundingVolume,
    pub geometric_error: f64,
    pub refine: String,
    pub children: Option<[TileSetRootChild; 4]>
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TileSet {
    asset: TileSetAsset,
    geometric_error: f64,
    root: TileSetRoot,
}

impl TileSetRootBoundingVolume {
    pub fn new(quadtree: &QuadTree) -> Self {
        let aabb = &quadtree.bounds;

        if let Some(first_point) = &quadtree.points.get(0) {
            let mut z_min = first_point.z;

            let mut z_max = first_point.z;

            for p in &quadtree.points {
                if p.z < z_min {
                    z_min = p.z;
                } else if p.z > z_max {
                    z_max = p.z;
                }
            }

            let half_height = (z_max - z_min) / 2.0_f64;

            Self {
                bbox: [
                    aabb.x_center, aabb.y_center, z_min + half_height,
                    aabb.half_width, 0.0, 0.0,
                    0.0, aabb.half_length, 0.0,
                    0.0, 0.0, half_height,
                ]
            }
        } else {
            Self {
                bbox: [
                    aabb.x_center, aabb.y_center, aabb.z_center,
                    aabb.half_width, 0.0, 0.0,
                    0.0, aabb.half_length, 0.0,
                    0.0, 0.0, 0.0,
                ]
            }
        }
    }
}

impl TileSetRoot {
    pub fn new(quadtree: &QuadTree) -> Self {
        let content = TileSetRootContent {
            uri: "root.pnts".to_string()
        };

        let children = match &quadtree.children {
            Some(children) => {
                Some([
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "Tiles/0/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&children.get(0).unwrap()),
                        geometric_error: 1000_f64 / 2_f64.powf(quadtree.depth as f64 + 1_f64),
                        refine: "ADD".to_string()
                    },
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "Tiles/1/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&children.get(1).unwrap()),
                        geometric_error: 1000_f64 / 2_f64.powf(quadtree.depth as f64 + 1_f64),
                        refine: "ADD".to_string()
                    },
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "Tiles/2/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&children.get(2).unwrap()),
                        geometric_error: 1000_f64 / 2_f64.powf(quadtree.depth as f64 + 1_f64),
                        refine: "ADD".to_string()
                    },
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "Tiles/3/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&children.get(3).unwrap()),
                        geometric_error: 1000_f64 / 2_f64.powf(quadtree.depth as f64 + 1_f64),
                        refine: "ADD".to_string()
                    }
                ])
            },
            None => None
        };

        TileSetRoot {
            content,
            bounding_volume: TileSetRootBoundingVolume::new(&quadtree),
            geometric_error: 1000_f64 / 2_f64.powf(quadtree.depth as f64),
            refine: "ADD".to_string(),
            children
        }
    }
}

pub fn create_tile(base_dir: &Path, quadtree: &QuadTree) -> Option<Aabb> {
    let points_length = quadtree.points.len();

    let mut coordinates_serialized: Vec<u8> = Vec::with_capacity(points_length * 12);

    let mut colors_serialized: Vec<u8> = Vec::with_capacity(points_length);

    for point in &quadtree.points {
        let x_relative = (point.x - quadtree.bounds.x_center) as f32;
        let y_relative = (point.y - quadtree.bounds.y_center) as f32;
        let z_relative = (point.z - quadtree.bounds.z_center) as f32;

        coordinates_serialized.append(x_relative.to_le_bytes().to_vec().as_mut());
        coordinates_serialized.append(y_relative.to_le_bytes().to_vec().as_mut());
        coordinates_serialized.append(z_relative.to_le_bytes().to_vec().as_mut());

        colors_serialized.push((point.r >> 8) as u8);
        colors_serialized.push((point.g >> 8) as u8);
        colors_serialized.push((point.b >> 8) as u8);
    }

    let feature_table_header = FeatureTableHeader {
        points_length: points_length as u32,
        rtc_center: vec![
           quadtree.bounds.x_center as f32,
           quadtree.bounds.y_center as f32,
           quadtree.bounds.z_center as f32,
        ],
        position: AttributePosition { byte_offset: 0 },
        rgb: AttributePosition { byte_offset: coordinates_serialized.len() as u32 }
    };

    let mut feature_table_header_json = serde_json::to_string(&feature_table_header).unwrap();

    if (28 + feature_table_header_json.len()) % 8 > 0 {
        for _i in 0..(8 - (28 + feature_table_header_json.len()) % 8) {
            feature_table_header_json.push(0x20 as char);
        }
    }

    let tile_set = TileSet {
        asset: TileSetAsset { version: "1.0".to_string() },
        geometric_error: 1000_f64 / 2_f64.powf(quadtree.depth as f64),
        root: TileSetRoot::new(quadtree)
    };

    let mut feature_table_header_bytes = feature_table_header_json.into_bytes();

    let feature_table_json_byte_length = feature_table_header_bytes.len() as u32;

    let mut feature_table_binary_byte_length = coordinates_serialized.len() as u32 + colors_serialized.len() as u32;

    let mut feature_table_bytes = vec![];

    feature_table_bytes.append(&mut feature_table_header_bytes);
    feature_table_bytes.append(&mut coordinates_serialized);
    feature_table_bytes.append(&mut colors_serialized);

    if (28 + feature_table_bytes.len()) % 8 > 0 {
        for _i in 0..(8 - (28 + feature_table_bytes.len()) % 8) {
            feature_table_bytes.push(0x0);
            feature_table_binary_byte_length += 1;
        }
    }

    let mut header = Header {
        magic: "pnts",
        version: 1,
        byte_length: 28 as u32 + feature_table_bytes.len() as u32,
        feature_table_json_byte_length,
        feature_table_binary_byte_length,
        batch_table_json_byte_length: 0,
        batch_table_binary_byte_length: 0
    };

    std::fs::create_dir_all(base_dir);

    let mut tileset_json_file = std::fs::File::create(base_dir.join("tileset.json")).unwrap();
    tileset_json_file.write_all(serde_json::to_string(&tile_set).unwrap().into_bytes().as_slice());

    let mut pnts_file = std::fs::File::create(base_dir.join("root.pnts")).unwrap();
    pnts_file.write_all(header.magic.as_bytes());
    pnts_file.write_all(header.version.to_le_bytes().as_ref());
    pnts_file.write_all(header.byte_length.to_le_bytes().as_ref());
    pnts_file.write_all(header.feature_table_json_byte_length.to_le_bytes().as_ref());
    pnts_file.write_all(header.feature_table_binary_byte_length.to_le_bytes().as_ref());
    pnts_file.write_all(header.batch_table_json_byte_length.to_le_bytes().as_ref());
    pnts_file.write_all(header.batch_table_binary_byte_length.to_le_bytes().as_ref());
    pnts_file.write_all(feature_table_bytes.as_slice());

    match &quadtree.children {
        Some(children) => {
            create_tile(&*base_dir.join("Tiles").join("0"), &children.get(0).unwrap());
            create_tile(&*base_dir.join("Tiles").join("1"), &children.get(1).unwrap());
            create_tile(&*base_dir.join("Tiles").join("2"), &children.get(2).unwrap());
            create_tile(&*base_dir.join("Tiles").join("3"), &children.get(3).unwrap());
        },
        None => {}
    }

    if quadtree.depth == 1 {
        Some(quadtree.bounds.clone())
    } else {
        None
    }
}

