use bevy::prelude::*;
use crate::body::*;

#[derive(Resource, Debug)]
pub struct Octree {
    pos: Vec3,
    size: f32,
    children: [OctNode; 8],
}

#[derive(Default, PartialEq, Debug)]
pub enum OctNode {
    #[default]
    Empty,
    Leaf(Entity),
    Branch { com: COM, tree: Box<Octree> },
}

impl Octree {
    pub fn new(pos: Vec3, size: f32) -> Octree {
        Octree { pos, size, children: default() }
    }
    pub fn contains(&self, p: Vec3) -> bool {
        p.x >= self.pos.x && p.x < self.pos.x + self.size &&
        p.y >= self.pos.y && p.y < self.pos.y + self.size &&
        p.z >= self.pos.z && p.z < self.pos.z + self.size
    }
    // TODO: Bounds checking.
    pub fn add(&mut self, query: &Query<(Entity, &Transform, &Body)>, new: Entity) {
        if let Ok((_, trans, body)) = query.get(new) {
            let sub_size = 0.5 * self.size;
            let offset = self.pos + Vec3::splat(sub_size);
            let bvec = trans.translation.cmpge(offset);
            let idx = bvec.bitmask() as usize;
            let node = &mut self.children[idx];
            match node {
                OctNode::Empty => *node = OctNode::Leaf(new),
                OctNode::Leaf(old) => {
                    if let Ok((_, old_trans, old_body)) = query.get(*old) {
                        let com = COM::new(old_trans.translation, old_body.mass, trans.translation, body.mass);
                        let mut tree = Box::new(Octree::new(Vec3::select(bvec, offset, self.pos), sub_size));
                        tree.add(query, *old);
                        tree.add(query, new);
                        *node = OctNode::Branch { com, tree };
                    }
                },
                OctNode::Branch {com, tree} => {
                    com.add(trans.translation, body.mass);
                    tree.add(query, new);
                }
            }
        }
    }
}

impl PartialEq for Octree {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.size == other.size && self.children == other.children
    }
}

#[derive(Debug)]
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
mod octree_tests {
    use super::*;

    #[test]
    fn tree_eq() {
        let tree1 = Octree::new(Vec3::new(-1., -1., -1.), 2.);
        let tree2 = Octree::new(Vec3::new(-1., -1., -1.), 2.);
        assert_eq!(tree1, tree2);

        let mut tree1 = Octree::new(Vec3::new(-1., -1., -1.), 2.);
        let subtree1 = Octree::new(Vec3::new(0., 0., 0.), 1.);
        tree1.children[7] = OctNode::Branch { com: COM::ZERO, tree: Box::new(subtree1) };
        let tree2 = Octree::new(Vec3::new(-1., -1., -1.), 2.);
        assert_ne!(tree1, tree2);

        let mut tree1 = Octree::new(Vec3::new(-1., -1., -1.), 2.);
        let subtree1 = Octree::new(Vec3::new(0., 0., 0.), 1.);
        tree1.children[7] = OctNode::Branch { com: COM::ZERO, tree: Box::new(subtree1) };
        let mut tree2 = Octree::new(Vec3::new(-1., -1., -1.), 2.);
        let subtree2 = Octree::new(Vec3::new(0., 0., 0.), 1.);
        tree2.children[7] = OctNode::Branch { com: COM::ZERO, tree: Box::new(subtree2) };
        assert_eq!(tree1, tree2);
    }

    fn build_body(pos: Vec3, m: f32) -> (Body, Transform) {
        (
            Body {
                name: "Test".to_owned(),
                mass: m,
                vel: Vec3::ZERO,
                angular_vel: Vec3::ZERO,
            },
            Transform::from_translation(pos)
        )
    }
    fn build_tree(query: Query<(Entity, &Transform, &Body)>, mut tree: ResMut<Octree>) {
        for body in &query {
            tree.add(&query, body.0);
        }
    }
    #[test]
    fn depth_1() {
        let mut app = App::new();
        let entities = [
            app.world.spawn(build_body(Vec3::new(0.5, 0., 0.), 1.)).id(),
            app.world.spawn(build_body(Vec3::new(0., 0.5, 0.), 1.)).id(),
            app.world.spawn(build_body(Vec3::new(0., 0., 0.5), 1.)).id(),
        ];
        let answer = Octree {
            pos: Vec3::new(0., 0., 0.),
            size: 1.,
            children: [
                OctNode::Empty,
                OctNode::Leaf(entities[0]),
                OctNode::Leaf(entities[1]),
                OctNode::Empty,
                OctNode::Leaf(entities[2]),
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
            ]
        };

        app.insert_resource(Octree::new(Vec3::new(0., 0., 0.), 1.))
        .add_systems(Startup, build_tree)
        .update();
        assert_eq!(*app.world.resource::<Octree>(), answer);
    }
    #[test]
    fn depth_2() {
        let mut app = App::new();
        let entities = [
            app.world.spawn(build_body(Vec3::new(0., 1., 0.), 1.)).id(),
            app.world.spawn(build_body(Vec3::new(0.5, 1., 0.), 1.)).id(),
        ];
        let answer = Octree {
            pos: Vec3::new(0., 0., 0.),
            size: 2.,
            children: [
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Branch {
                    com: COM { sum: Vec3::new(0.5, 2., 0.), mass: 2. },
                    tree: Box::new(Octree {
                        pos: Vec3::Y,
                        size: 1.,
                        children: [
                            OctNode::Leaf(entities[0]),
                            OctNode::Leaf(entities[1]),
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                        ]
                    }),
                },
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
            ]
        };

        app.insert_resource(Octree::new(Vec3::new(0., 0., 0.), 2.))
        .add_systems(Startup, build_tree)
        .update();
        assert_eq!(*app.world.resource::<Octree>(), answer);
    }
    #[test]
    fn depth_3() {
        let mut app = App::new();
        let entities = [
            app.world.spawn(build_body(Vec3::new(0., 1., 1.), 1.)).id(),
            app.world.spawn(build_body(Vec3::new(0.5, 1., 1.), 1.)).id(),
        ];
        let answer = Octree {
            pos: Vec3::new(0., 0., 0.),
            size: 4.,
            children: [
                OctNode::Branch {
                    com: COM { sum: Vec3::new(0.5, 2., 2.), mass: 2. },
                    tree: Box::new(Octree {
                        pos: Vec3::ZERO,
                        size: 2.,
                        children: [
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Empty,
                            OctNode::Branch {
                                com: COM { sum: Vec3::new(0.5, 2., 2.), mass: 2. },
                                tree: Box::new(Octree {
                                    pos: Vec3::new(0., 1., 1.),
                                    size: 1.,
                                    children: [
                                        OctNode::Leaf(entities[0]),
                                        OctNode::Leaf(entities[1]),
                                        OctNode::Empty,
                                        OctNode::Empty,
                                        OctNode::Empty,
                                        OctNode::Empty,
                                        OctNode::Empty,
                                        OctNode::Empty,
                                    ]
                                }),
                            },
                            OctNode::Empty,
                        ]
                    }),
                },
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
                OctNode::Empty,
            ]
        };

        app.insert_resource(Octree::new(Vec3::new(0., 0., 0.), 4.))
        .add_systems(Startup, build_tree)
        .update();
        assert_eq!(*app.world.resource::<Octree>(), answer);
    }
}
