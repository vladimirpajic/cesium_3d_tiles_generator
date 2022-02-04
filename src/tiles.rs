use serde::Serialize;
use las::{Point, Color};
use crate::quadtree::{QuadTree, Aabb};
use std::path::{Path, PathBuf};
use std::borrow::BorrowMut;
use std::io::Write;

const MAGIC: &str = "pnts";

const VERSION: u32 = 1;

#[derive(Debug)]
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

#[derive(Serialize, Debug, Clone)]
pub struct TileSetAsset {
    pub version: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct TileSetRootContent {
    pub uri: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct TileSetRootBoundingVolume {
    #[serde(rename = "box")]
    pub bbox: [f64; 12]
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TileSetRootChild {
    pub content: TileSetRootContent,
    pub bounding_volume: TileSetRootBoundingVolume,
    pub geometric_error: f64,
    pub refine: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TileSetRoot {
    pub content: TileSetRootContent,
    pub bounding_volume: TileSetRootBoundingVolume,
    pub geometric_error: f64,
    pub refine: String,
    pub children: Option<Vec<TileSetRootChild>>
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TileSet {
    pub(crate) asset: TileSetAsset,
    pub(crate) geometric_error: f64,
    pub(crate) root: TileSetRoot,
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
                let child_0 = &children.get(0).unwrap();
                let child_1 = &children.get(1).unwrap();
                let child_2 = &children.get(2).unwrap();
                let child_3 = &children.get(3).unwrap();

                let geometric_error_0 = (child_0.bounds.half_width.powf(2.0_f64) + child_0.bounds.half_length.powf(2.0_f64)).sqrt();
                let geometric_error_1 = (child_1.bounds.half_width.powf(2.0_f64)+ child_1.bounds.half_length.powf(2.0_f64)).sqrt();
                let geometric_error_2 = (child_2.bounds.half_width.powf(2.0_f64) + child_2.bounds.half_length.powf(2.0_f64)).sqrt();
                let geometric_error_3 = (child_3.bounds.half_width.powf(2.0_f64) + child_3.bounds.half_length.powf(2.0_f64)).sqrt();

                Some(vec![
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "0/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&child_0),
                        geometric_error: geometric_error_0,
                        refine: "ADD".to_string()
                    },
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "1/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&child_1),
                        geometric_error: geometric_error_1,
                        refine: "ADD".to_string()
                    },
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "2/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&child_2),
                        geometric_error: geometric_error_2,
                        refine: "ADD".to_string()
                    },
                    TileSetRootChild {
                        content: TileSetRootContent {
                            uri: "3/tileset.json".to_string()
                        },
                        bounding_volume: TileSetRootBoundingVolume::new(&child_3),
                        geometric_error: geometric_error_3,
                        refine: "ADD".to_string()
                    }
                ])
            },
            None => None
        };

        let geometric_error = (quadtree.bounds.half_width.powf(2.0_f64) + quadtree.bounds.half_length.powf(2.0_f64)).sqrt();

        TileSetRoot {
            content,
            bounding_volume: TileSetRootBoundingVolume::new(&quadtree),
            geometric_error,
            refine: "ADD".to_string(),
            children
        }
    }
}

pub fn create_tile(base_dir: &Path, quadtree: &QuadTree) -> TileSet {
    let geometric_error = match quadtree.children {
        Some(_) => (quadtree.bounds.half_width.powf(2.0_f64) + quadtree.bounds.half_length.powf(2.0_f64)).sqrt(),
        None => 0.0
    };

    let tile_set = TileSet {
        asset: TileSetAsset { version: "1.0".to_string() },
        geometric_error,
        root: TileSetRoot::new(quadtree)
    };

    if quadtree.points.len() == 0 {
        return tile_set;
    }

    let tile_content_binary_inner = package_points(&quadtree);

    std::fs::create_dir_all(base_dir);

    let mut tileset_json_file = std::fs::File::create(base_dir.join("tileset.json")).unwrap();
    tileset_json_file.write_all(serde_json::to_string(&tile_set).unwrap().into_bytes().as_slice());

    let mut pnts_file = std::fs::File::create(base_dir.join("root.pnts")).unwrap();
    pnts_file.write_all(tile_content_binary_inner.as_slice());

    match &quadtree.children {
        Some(children) => {
            create_tile(&*base_dir.join("0"), &children.get(0).unwrap());
            create_tile(&*base_dir.join("1"), &children.get(1).unwrap());
            create_tile(&*base_dir.join("2"), &children.get(2).unwrap());
            create_tile(&*base_dir.join("3"), &children.get(3).unwrap());
        },
        None => {}
    }

    tile_set
}

pub fn package_points(quadtree: &&QuadTree) -> Vec<u8> {
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

    let feature_table_header_json = serde_json::to_string(&feature_table_header).unwrap();

    let mut feature_table_header_json_bytes = feature_table_header_json.into_bytes();
    feature_table_header_json_bytes.resize(feature_table_header_json_bytes.len() + (8 - (28 + feature_table_header_json_bytes.len()) % 8) % 8, 0x20);

    let feature_table_json_byte_length = feature_table_header_json_bytes.len();

    let mut feature_table_bytes = vec![];

    feature_table_bytes.append(&mut feature_table_header_json_bytes);
    feature_table_bytes.append(&mut coordinates_serialized);
    feature_table_bytes.append(&mut colors_serialized);

    feature_table_bytes.resize(feature_table_bytes.len() + (8 - (28 + feature_table_bytes.len()) % 8) % 8, 0);

    let mut header = Header {
        magic: "pnts",
        version: 1,
        byte_length: 28 as u32 + feature_table_bytes.len() as u32,
        feature_table_json_byte_length: feature_table_json_byte_length as u32,
        feature_table_binary_byte_length: (feature_table_bytes.len() - feature_table_json_byte_length) as u32,
        batch_table_json_byte_length: 0,
        batch_table_binary_byte_length: 0
    };

    let mut tile_content_binary_inner = vec![];
    tile_content_binary_inner.append(&mut header.magic.as_bytes().to_vec());
    tile_content_binary_inner.append(&mut header.version.to_le_bytes().to_vec());
    tile_content_binary_inner.append(&mut header.byte_length.to_le_bytes().to_vec());
    tile_content_binary_inner.append(&mut header.feature_table_json_byte_length.to_le_bytes().to_vec());
    tile_content_binary_inner.append(&mut header.feature_table_binary_byte_length.to_le_bytes().to_vec());
    tile_content_binary_inner.append(&mut header.batch_table_json_byte_length.to_le_bytes().to_vec());
    tile_content_binary_inner.append(&mut header.batch_table_binary_byte_length.to_le_bytes().to_vec());
    tile_content_binary_inner.append(&mut feature_table_bytes);
    tile_content_binary_inner
}

