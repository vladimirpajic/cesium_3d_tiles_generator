mod quadtree;
mod tiles;

use las::{Read, Reader};
use std::path::Path;
use crate::quadtree::{QuadTree, Aabb};
use crate::tiles::create_tile;

const MAX_POINTS_PER_TILE: u64 = 4096;

fn main() {
    let mut reader =
        Reader::from_path(Path::new("C:\\temp\\test.las")).expect("Can't read LAS file.");

    let header = reader.header();

    let number_of_points = header.number_of_points();

    let bounds_min = header.bounds().min;

    let bounds_max = header.bounds().max;

    let half_width = (bounds_max.x - bounds_min.x) / 2.0;

    let half_height = (bounds_max.y - bounds_min.y) / 2.0;

    let mut quadtree = QuadTree::new(Aabb {
        x_center: bounds_min.x + half_width,
        y_center: bounds_min.y + half_height,
        half_width,
        half_height
    }, 1, 4096);

    for point in reader.points() {
        if let Ok(point) = point {
            quadtree.insert(&point);
        }
    }

    create_tile(&quadtree);
}
