mod quadtree;
mod tiles;

use las::{Read, Reader};
use std::path::Path;

const MAX_POINTS_PER_TILE: u64 = 4096;

fn main() {
    let mut reader =
        Reader::from_path(Path::new("C:\\tmp\\test.las")).expect("Can't read LAS file.");

    let header = reader.header();

    let number_of_points = header.number_of_points();

    let bounds_min = header.bounds().min;

    let bounds_max = header.bounds().max;

    for (i, point) in reader.points().enumerate() {
        if let Ok(point) = point {}
    }
}
