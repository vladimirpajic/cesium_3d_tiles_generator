use serde::Serialize;

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

#[derive(Serialize)]
pub struct AttributePosition {
    #[serde(rename = "byteOffset")]
    byte_offset: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct FeatureTableHeader {
    pub points_length: u32,
    pub rtc_center: Vec<f32>,
    pub position: AttributePosition,
    pub rgb: AttributePosition,
}
