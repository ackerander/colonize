use bevy::prelude::*;
use crate::body::*;

#[derive(Resource, Default, Debug, PartialEq)]
pub struct Octree {
    pos: Vec3,
    size: f32,
    node: OctNode,
}
#[derive(Default, Debug, PartialEq)]
pub enum OctNode {
    #[default]
    Empty,
    Leaf(Entity),
    Branch {
        com: COM,
        children: Box<[Octree; 8]>
    },
}

impl Octree {
    fn idx_offset(a: Vec3, b: Vec3, i: u8) -> Vec3 {
        Vec3::select(
            BVec3::new(i & 1 != 0, i >> 1 & 1 != 0, i >> 2 & 1 != 0),
            a, b
        )
    }
    pub fn leaf_unchecked(pos: Vec3, size: f32, new: Entity) -> Octree {
        Octree { pos, size, node: OctNode::Leaf(new) }
    }
    pub fn empty(pos: Vec3, size: f32) -> Octree {
        Octree { pos, size, ..default() }
    }
    pub fn contains(&self, p: Vec3) -> bool {
        (self.pos.cmple(p) & self.pos.cmpgt(p - self.size)).all()
    }
    fn add_super(&mut self, query: &Query<(Entity, &Transform, &Body)>, new: Entity) {
        if let Ok((_, Transform { translation, .. }, Body { mass, .. })) = query.get(new) {
            let bvec = translation.cmpge(self.pos);
            let origin = self.pos - Vec3::select(bvec, Vec3::ZERO, Vec3::splat(self.size));
            let middle = origin + Vec3::splat(self.size);
            // New node replacing root
            let mut tree = Octree {
                pos: origin,
                size: 2. * self.size,
                node: OctNode::Branch {
                    com: match self.node {
                        OctNode::Branch { com, .. } => com,
                        OctNode::Leaf(e) =>
                            if let Ok((_, trans, body)) = query.get(e) {
                                COM { sum: body.mass * trans.translation, mass: body.mass }
                            } else { COM::ZERO },
                        OctNode::Empty => COM::ZERO
                    },
                    children: Box::new([
                            Octree { pos: origin, size: self.size, ..default() },
                            Octree { pos: Self::idx_offset(middle, origin, 1), size: self.size, ..default() },
                            Octree { pos: Self::idx_offset(middle, origin, 2), size: self.size, ..default() },
                            Octree { pos: Self::idx_offset(middle, origin, 3), size: self.size, ..default() },
                            Octree { pos: Self::idx_offset(middle, origin, 4), size: self.size, ..default() },
                            Octree { pos: Self::idx_offset(middle, origin, 5), size: self.size, ..default() },
                            Octree { pos: Self::idx_offset(middle, origin, 6), size: self.size, ..default() },
                            Octree { pos: middle, size: self.size, ..default() },
                    ]),
                },
            };
            std::mem::swap(self, &mut tree);
            let contained = self.contains(*translation);
            if let OctNode::Branch { ref mut com, ref mut children } = self.node {
                let tree_idx = tree.pos.cmpge(middle).bitmask();
                children[tree_idx as usize] = tree;
                if contained {
                    let body_idx = translation.cmpge(middle).bitmask();
                    // assert_ne!(tree_idx, body_idx);
                    children[body_idx as usize].node = OctNode::Leaf(new);
                    com.add(*translation, *mass);
                } else {
                    self.add_super(query, new);
                }
            // } else {
            //     panic!("Should be Branch!");
            }
        }
    }
    fn add_sub(&mut self, query: &Query<(Entity, &Transform, &Body)>, new: Entity) {
        if let Ok((_, Transform { translation, .. }, Body { mass, .. })) = query.get(new) {
            match &mut self.node {
                OctNode::Empty => self.node = OctNode::Leaf(new),
                OctNode::Leaf(old) => {
                    if let Ok((_, trans, body)) = query.get(*old) {
                        let com = COM::new(trans.translation, body.mass, *translation, *mass);
                        let size = 0.5 * self.size;
                        let offset = self.pos + Vec3::splat(size);
                        let old_idx = trans.translation.cmpge(offset).bitmask();
                        let new_idx = translation.cmpge(offset).bitmask();
                        let mut children = Box::new([
                            Octree { pos: self.pos, size, ..default() },
                            Octree { pos: Self::idx_offset(offset, self.pos, 1), size, ..default() },
                            Octree { pos: Self::idx_offset(offset, self.pos, 2), size, ..default() },
                            Octree { pos: Self::idx_offset(offset, self.pos, 3), size, ..default() },
                            Octree { pos: Self::idx_offset(offset, self.pos, 4), size, ..default() },
                            Octree { pos: Self::idx_offset(offset, self.pos, 5), size, ..default() },
                            Octree { pos: Self::idx_offset(offset, self.pos, 6), size, ..default() },
                            Octree { pos: offset, size, ..default() },
                        ]);
                        children[old_idx as usize].node = OctNode::Leaf(*old);
                        children[new_idx as usize].add(query, new);
                        self.node = OctNode::Branch { com, children };
                    }
                },
                OctNode::Branch { ref mut com, children } => {
                    com.add(*translation, *mass);
                    let offset = self.pos + Vec3::splat(0.5 * self.size);
                    let idx = translation.cmpge(offset).bitmask();
                    children[idx as usize].add(query, new);
                },
            }
        }
    }
    pub fn add(&mut self, query: &Query<(Entity, &Transform, &Body)>, new: Entity) {
        if let Ok((_, trans, _)) = query.get(new) {
            if self.contains(trans.translation) {
                self.add_sub(query, new);
            } else {
                self.add_super(query, new);
            }
        }
    }
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
mod octree_tests {
    use super::*;
    #[test]
    fn tree_eq() {
        let tree1 = Octree::empty(Vec3::new(-1., -1., -1.), 2.);
        let tree2 = Octree::empty(Vec3::new(-1., -1., -1.), 2.);
        assert_eq!(tree1, tree2);

        let tree1 = Octree { pos: Vec3::new(-1., -1., -1.), size: 2., node: OctNode::Branch {
            com: COM::ZERO,
            children: Box::new([
                Octree::empty(Vec3::new(-1., -1., -1.), 1.),
                Octree::empty(Vec3::new( 0., -1., -1.), 1.),
                Octree::empty(Vec3::new(-1.,  0., -1.), 1.),
                Octree::empty(Vec3::new( 0.,  0., -1.), 1.),
                Octree::empty(Vec3::new(-1., -1.,  0.), 1.),
                Octree::empty(Vec3::new( 0., -1.,  0.), 1.),
                Octree::empty(Vec3::new(-1.,  0.,  0.), 1.),
                Octree::empty(Vec3::new( 0.,  0.,  0.), 1.),
            ]),
        }};
        let tree2 = Octree::empty(Vec3::new(-1., -1., -1.), 2.);
        assert_ne!(tree1, tree2);

        let tree2 = Octree { pos: Vec3::new(-1., -1., -1.), size: 2., node: OctNode::Branch {
            com: COM::ZERO,
            children: Box::new([
                Octree::empty(Vec3::new(-1., -1., -1.), 1.),
                Octree::empty(Vec3::new( 0., -1., -1.), 1.),
                Octree::empty(Vec3::new(-1.,  0., -1.), 1.),
                Octree::empty(Vec3::new( 0.,  0., -1.), 1.),
                Octree::empty(Vec3::new(-1., -1.,  0.), 1.),
                Octree::empty(Vec3::new( 0., -1.,  0.), 1.),
                Octree::empty(Vec3::new(-1.,  0.,  0.), 1.),
                Octree::empty(Vec3::new( 0.,  0.,  0.), 1.),
            ]),
        }};
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
            pos: Vec3::ZERO, size: 1.,
            node: OctNode::Branch {
                com: COM { sum: Vec3::new(0.5, 0.5, 0.5), mass: 3. },
                children: Box::new([
                    Octree::empty(Vec3::ZERO, 0.5),
                    Octree::leaf_unchecked(Vec3::new(0.5, 0., 0.), 0.5, entities[0]),
                    Octree::leaf_unchecked(Vec3::new(0., 0.5, 0.), 0.5, entities[1]),
                    Octree::empty(Vec3::new(0.5, 0.5, 0.), 0.5),
                    Octree::leaf_unchecked(Vec3::new(0., 0., 0.5), 0.5, entities[2]),
                    Octree::empty(Vec3::new(0.5, 0., 0.5), 0.5),
                    Octree::empty(Vec3::new(0., 0.5, 0.5), 0.5),
                    Octree::empty(Vec3::new(0.5, 0.5, 0.5), 0.5),
                ]),
            },
        };
        app.insert_resource(Octree::empty(Vec3::ZERO, 1.))
        .add_systems(Startup, build_tree).update();
        assert_eq!(*app.world.resource::<Octree>(), answer);
    }
    #[test]
    fn depth_2() {
        let mut app = App::new();
        let entities = [
            app.world.spawn(build_body(Vec3::Y, 1.)).id(),
            app.world.spawn(build_body(Vec3::new(0.5, 1., 0.), 1.)).id(),
        ];
        let answer = Octree {
            pos: Vec3::ZERO, size: 2.,
            node: OctNode::Branch {
                com: COM { sum: Vec3::new(0.5, 2., 0.), mass: 2. },
                children: Box::new([
                    Octree::empty(Vec3::ZERO, 1.),
                    Octree::empty(Vec3::X, 1.),
                    Octree {
                        pos: Vec3::Y, size: 1.,
                        node: OctNode::Branch {
                            com: COM { sum: Vec3::new(0.5, 2., 0.), mass: 2. },
                            children: Box::new([
                                Octree::leaf_unchecked(Vec3::new(0., 1., 0.), 0.5, entities[0]),
                                Octree::leaf_unchecked(Vec3::new(0.5, 1., 0.), 0.5, entities[1]),
                                Octree::empty(Vec3::new(0., 1.5, 0.), 0.5),
                                Octree::empty(Vec3::new(0.5, 1.5, 0.), 0.5),
                                Octree::empty(Vec3::new(0., 1., 0.5), 0.5),
                                Octree::empty(Vec3::new(0.5, 1., 0.5), 0.5),
                                Octree::empty(Vec3::new(0., 1.5, 0.5), 0.5),
                                Octree::empty(Vec3::new(0.5, 1.5, 0.5), 0.5),
                            ]),
                        }
                    },
                    Octree::empty(Vec3::new(1., 1., 0.), 1.),
                    Octree::empty(Vec3::Z, 1.),
                    Octree::empty(Vec3::new(1., 0., 1.), 1.),
                    Octree::empty(Vec3::new(0., 1., 1.), 1.),
                    Octree::empty(Vec3::ONE, 1.),
                ]),
            },
        };
        app.insert_resource(Octree::empty(Vec3::new(0., 0., 0.), 2.))
        .add_systems(Startup, build_tree).update();
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
            pos: Vec3::ZERO, size: 4.,
            node: OctNode::Branch {
                com: COM { sum: Vec3::new(0.5, 2., 2.), mass: 2. },
                children: Box::new ([
                    Octree {
                        pos: Vec3::ZERO, size: 2.,
                        node: OctNode::Branch {
                            com: COM { sum: Vec3::new(0.5, 2., 2.), mass: 2. },
                            children: Box::new([
                                Octree::empty(Vec3::ZERO, 1.),
                                Octree::empty(Vec3::X, 1.),
                                Octree::empty(Vec3::Y, 1.),
                                Octree::empty(Vec3::new(1., 1., 0.), 1.),
                                Octree::empty(Vec3::Z, 1.),
                                Octree::empty(Vec3::new(1., 0., 1.), 1.),
                                Octree {
                                    pos: Vec3::new(0., 1., 1.), size: 1.,
                                    node: OctNode::Branch {
                                        com: COM { sum: Vec3::new(0.5, 2., 2.), mass: 2. },
                                        children: Box::new([
                                            Octree::leaf_unchecked(Vec3::new(0., 1., 1.), 0.5, entities[0]),
                                            Octree::leaf_unchecked(Vec3::new(0.5, 1., 1.), 0.5, entities[1]),
                                            Octree::empty(Vec3::new(0., 1.5, 1.), 0.5),
                                            Octree::empty(Vec3::new(0.5, 1.5, 1.), 0.5),
                                            Octree::empty(Vec3::new(0., 1., 1.5), 0.5),
                                            Octree::empty(Vec3::new(0.5, 1., 1.5), 0.5),
                                            Octree::empty(Vec3::new(0., 1.5, 1.5), 0.5),
                                            Octree::empty(Vec3::new(0.5, 1.5, 1.5), 0.5),
                                        ]),
                                    },
                                },
                                Octree::empty(Vec3::ONE, 1.),
                            ])
                        }
                    },
                    Octree::empty(Vec3::new(2., 0., 0.), 2.),
                    Octree::empty(Vec3::new(0., 2., 0.), 2.),
                    Octree::empty(Vec3::new(2., 2., 0.), 2.),
                    Octree::empty(Vec3::new(0., 0., 2.), 2.),
                    Octree::empty(Vec3::new(2., 0., 2.), 2.),
                    Octree::empty(Vec3::new(0., 2., 2.), 2.),
                    Octree::empty(Vec3::splat(2.), 2.),
                ]),
            },
        };
        app.insert_resource(Octree::empty(Vec3::ZERO, 4.))
        .add_systems(Startup, build_tree)
        .update();
        assert_eq!(*app.world.resource::<Octree>(), answer);
    }
    #[test]
    fn add_super() {
        let mut app = App::new();
        let entities = [
            app.world.spawn(build_body(Vec3::new(1., 0., 0.), 1.)).id(),
            app.world.spawn(build_body(Vec3::new(0., 1., 0.), 1.)).id(),
            app.world.spawn(build_body(Vec3::new(0., 0., 1.), 1.)).id(),
        ];
        app.insert_resource(Octree::empty(Vec3::ZERO, 1.))
            .add_systems(Startup, build_tree).update();
        let answer = Octree {
            pos: Vec3::ZERO, size: 2.,
            node: OctNode::Branch {
                com: COM { sum: Vec3::new(1., 1., 1.), mass: 3. },
                children: Box::new([
                    Octree::empty(Vec3::ZERO, 1.),
                    Octree::leaf_unchecked(Vec3::X, 1., entities[0]),
                    Octree::leaf_unchecked(Vec3::Y, 1., entities[1]),
                    Octree::empty(Vec3::new(1., 1., 0.), 1.),
                    Octree::leaf_unchecked(Vec3::Z, 1., entities[2]),
                    Octree::empty(Vec3::new(1., 0., 1.), 1.),
                    Octree::empty(Vec3::new(0., 1., 1.), 1.),
                    Octree::empty(Vec3::splat(1.), 1.),
                ])
            }
        };
        assert_eq!(*app.world.resource::<Octree>(), answer);
    }
    #[test]
    fn add_super2() {
        let mut app = App::new();
        let entities = [
            app.world.spawn(build_body(Vec3::new(1., 0., 0.), 1.)).id(),
            app.world.spawn(build_body(Vec3::new(-1., 0., 0.), 1.)).id(),
        ];
        app.insert_resource(Octree::empty(Vec3::ZERO, 1.))
            .add_systems(Startup, build_tree).update();
        let answer = Octree {
            pos: Vec3::new(-2., 0., 0.), size: 4.,
            node: OctNode::Branch {
                com: COM { sum: Vec3::ZERO, mass: 2. },
                children: Box::new([
                    Octree::leaf_unchecked(Vec3::new(-2., 0., 0.), 2., entities[1]),
                    // Octree::leaf_unchecked(Vec3::ZERO, 2., entities[0]),
                    Octree {
                        pos: Vec3::ZERO, size: 2.,
                        node: OctNode::Branch {
                            com: COM { sum: Vec3::new(1., 0., 0.), mass: 1. },
                            children: Box::new([
                                Octree::empty(Vec3::ZERO, 1.),
                                Octree::leaf_unchecked(Vec3::X, 1., entities[0]),
                                Octree::empty(Vec3::Y, 1.),
                                Octree::empty(Vec3::new(1., 1., 0.), 1.),
                                Octree::empty(Vec3::Z, 1.),
                                Octree::empty(Vec3::new(1., 0., 1.), 1.),
                                Octree::empty(Vec3::new(0., 1., 1.), 1.),
                                Octree::empty(Vec3::splat(1.), 1.),
                            ])
                        }
                    },
                    Octree::empty(Vec3::new(-2., 2., 0.), 2.),
                    Octree::empty(Vec3::new(0., 2., 0.), 2.),
                    Octree::empty(Vec3::new(-2., 0., 2.), 2.),
                    Octree::empty(Vec3::new(0., 0., 2.), 2.),
                    Octree::empty(Vec3::new(-2., 2., 2.), 2.),
                    Octree::empty(Vec3::new(0., 2., 2.), 2.),
                ])
            }
        };
        assert_eq!(*app.world.resource::<Octree>(), answer);
    }
}
