use crate::quadtree::Point;

pub struct SpatialExtent {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub z_min: f64,
    pub z_max: f64,
}

impl Default for SpatialExtent {
    fn default() -> Self {
        Self {
            x_min: f64::MAX,
            x_max: f64::MIN,
            y_min: f64::MAX,
            y_max: f64::MIN,
            z_min: f64::MAX,
            z_max: f64::MIN,
        }
    }
}

impl SpatialExtent {
    pub fn update(&mut self, point: &Point) {
        if point.x < self.x_min {
            self.x_min = point.x;
        }

        if point.x > self.x_max {
            self.x_max = point.x;
        }

        if point.y < self.y_min {
            self.y_min = point.y;
        }

        if point.y > self.y_max {
            self.y_max = point.y;
        }

        if point.z < self.z_min {
            self.z_min = point.z;
        }

        if point.z > self.z_max {
            self.z_max = point.z;
        }
    }
}
