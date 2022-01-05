use las::Point;

pub struct Aabb {
    pub x_center: f64,
    pub y_center: f64,
    pub half_width: f64,
    pub half_height: f64,
}

pub struct QuadTree {
    pub capacity: usize,
    pub bounds: Aabb,
    pub points: Vec<Point>,
    pub children: Option<[Box<QuadTree>; 4]>,
    pub depth: u8,
}

impl QuadTree {
    pub fn new(bounds: Aabb, depth: u8, capacity: usize) -> QuadTree {
        QuadTree {
            capacity,
            bounds,
            points: vec![],
            children: None,
            depth,
        }
    }

    pub fn insert(&mut self, point: &Point) {
        if point.x >= self.bounds.x_center - self.bounds.half_width
            && point.x < self.bounds.x_center + self.bounds.half_width
            && point.y >= self.bounds.y_center - self.bounds.half_height
            && point.y < self.bounds.y_center + self.bounds.half_height
        {
            if self.points.len() < self.capacity {
                self.points.push(point.to_owned());
            } else {
                if let Some(children) = &mut self.children {
                    for child in children {
                        child.insert(point);
                    }
                } else {
                    self.split();

                    if let Some(children) = &mut self.children {
                        for child in children {
                            child.insert(point);
                        }
                    }
                }
            }
        }
    }

    fn split(&mut self) {
        let half_width = self.bounds.half_width / 2.0;

        let half_height = self.bounds.half_height / 2.0;

        let depth = self.depth + 1;

        let tl = QuadTree::new(
            Aabb {
                x_center: self.bounds.x_center - half_width,
                y_center: self.bounds.y_center + half_height,
                half_width,
                half_height,
            },
            depth,
            self.capacity,
        );

        let tr = QuadTree::new(
            Aabb {
                x_center: self.bounds.x_center + half_width,
                y_center: self.bounds.y_center + half_height,
                half_width,
                half_height,
            },
            depth,
            self.capacity,
        );

        let bl = QuadTree::new(
            Aabb {
                x_center: self.bounds.x_center - half_width,
                y_center: self.bounds.y_center - half_height,
                half_width,
                half_height,
            },
            depth,
            self.capacity,
        );

        let br = QuadTree::new(
            Aabb {
                x_center: self.bounds.x_center + half_width,
                y_center: self.bounds.y_center - half_height,
                half_width,
                half_height,
            },
            depth,
            self.capacity,
        );

        self.children = Some([Box::new(tl), Box::new(tr), Box::new(bl), Box::new(br)]);
    }
}

#[cfg(test)]
mod tests {
    use crate::quadtree::{Aabb, QuadTree};

    #[test]
    fn create_quad_tree_test() {
        let points: Vec<Vec<f64>> = vec![
            vec![0.5, 0.5],
            vec![2.0, 0.5],
            vec![0.5, 2.0],
            vec![2.0, 2.5],
            vec![1.0, 0.5],
            vec![2.5, 0.5],
            vec![0.5, 1.0],
            vec![1.0, 1.0],
            vec![2.0, 1.0],
            vec![2.5, 1.0],
            vec![1.0, 2.0],
            vec![2.0, 2.0],
            vec![2.5, 2.0],
            vec![0.5, 2.5],
            vec![1.0, 2.5],
            vec![2.5, 2.5],
        ];

        let mut quad_tree = QuadTree::new(
            Aabb {
                x_center: 1.5,
                y_center: 1.5,
                half_width: 1.5,
                half_height: 1.5,
            },
            1,
            4,
        );

        for (index, point) in points.iter().enumerate() {
            quad_tree.insert(point[0], point[1], index);
        }

        assert_eq!(quad_tree.points.len(), 4);

        assert_eq!(quad_tree.children.is_some(), true);

        let [tl, tr, bl, br] = quad_tree.children.unwrap();

        assert_eq!(tl.points.len(), 3);

        assert_eq!(tr.points.len(), 3);

        assert_eq!(bl.points.len(), 3);

        assert_eq!(br.points.len(), 3);
    }
}
