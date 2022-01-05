use serde::Serialize;
use las::{Point, Color};
use crate::quadtree::QuadTree;
use std::path::Path;
use std::borrow::BorrowMut;

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

pub fn create_tile(quadtree: &QuadTree) {
    let points_length = quadtree.points.len();

    let mut coordinates_serialized: Vec<u8> = Vec::with_capacity(points_length);

    let mut colors_serialized: Vec<u8> = Vec::with_capacity(points_length);

    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    let mut z_sum = 0.0;

    for point in &quadtree.points {
        let (x, y, z) = geodetic_to_geocentric(point.x as f32, point.y as f32, point.z as f32);

        x_sum += x;
        y_sum += y;
        z_sum += z;

        coordinates_serialized.append(x.to_le_bytes().to_vec().borrow_mut());
        coordinates_serialized.append(y.to_le_bytes().to_vec().borrow_mut());
        coordinates_serialized.append(z.to_le_bytes().to_vec().borrow_mut());

        let color = point.color.unwrap_or(Color::default());

        colors_serialized.append(color.red.to_le_bytes().to_vec().as_mut());
        colors_serialized.append(color.green.to_le_bytes().to_vec().as_mut());
        colors_serialized.append(color.blue.to_le_bytes().to_vec().as_mut())
    }

    let feature_table_header = FeatureTableHeader {
        points_length: points_length as u32,
        rtc_center: vec![
            x_sum / points_length as f32,
            y_sum / points_length as f32,
            z_sum / points_length as f32,
        ],
        position: AttributePosition { byte_offset: 0 },
        rgb: AttributePosition { byte_offset: 0 }
    };

    println!("{:?}", feature_table_header);
}

fn geodetic_to_geocentric(lat: f32, lon: f32, h: f32) -> (f32, f32, f32) {
    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians();
    let t: f32 = 1.0 - 1.0 / 298.257223563;
    let nn = 6378137.0 / (1.0 - (1.0 - t.powf(2.0)) * lat_rad.sin().powf(2.0)).sqrt();
    let x = (nn + h) * lat_rad.cos() * lon_rad.cos();
    let y = (nn + h) * lat_rad.cos() * lon_rad.sin();
    let z = (t.powf(2.0) * nn + h) * lat_rad.sin();

    (x, y, z)
}