/* Implement me to replace octree */
use bevy::prelude::*;
use crate::body::*;

#[derive(Resource, Debug/*, PartialEq*/)]
pub enum BVTree {
    Leaf(Entity),
    Branch {
        com: COM,
        bound: shape::Box,
        children: Box<[BVTree; 2]>
    }
}

impl BVTree {
    fn build_tree
}

#[derive(Debug, Default, Clone, Copy)]
pub struct COM {
    sum: Vec3,
    mass: f32,
}
impl COM {
    const ZERO: Self = Self { sum: Vec3::ZERO, mass: 0. };
    fn new(p1: Vec3, m1: f32, p2: Vec3, m2: f32) -> COM {
        COM { sum: m1 * p1 + m2 * p2, mass: m1 + m2 }
    }
    fn com(&self) -> (Vec3, f32) {
        (self.sum / self.mass, self.mass)
    }
    fn add(&mut self, pos: Vec3, mass: f32) {
        self.sum += mass * pos;
        self.mass += mass;
    }
}
impl PartialEq for COM {
    fn eq(&self, other: &Self) -> bool {
        self.sum == other.sum && self.mass == other.mass
    }
}

#[cfg(test)]
mod bvh_tests {
    use super::*;
    #[test]
    fn tree_eq() {
        assert_eq!(1, 1);
    }
}
