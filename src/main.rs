use bevy::{prelude::*, input::mouse::MouseMotion};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

// const G: f32 = 6.6743e-11;
const G: f32 = 1.;

#[derive(Component)]
struct Body {
    mass: f32,
    vel: Vec3,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // bodies
    const P:Vec3 = Vec3 { x: -1.0024277970, y: 0.0041695061, z: 0. };
    const V:Vec3 = Vec3 { x: 0.3489048974, y: 0.5306305100, z: 0. };
    commands.spawn((
        Body {
            mass: 1.,
            vel: V,
        },
        PbrBundle {
        mesh: meshes.add(shape::UVSphere{radius: 0.1, ..default()}.into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        transform: Transform::from_translation(P),
        ..default()
    }));
    commands.spawn((
        Body {
            mass: 1.,
            vel: V,
        },
        PbrBundle {
        mesh: meshes.add(shape::UVSphere{radius: 0.1, ..default()}.into()),
        material: materials.add(Color::rgb(0.5, 0.3, 0.3).into()),
        transform: Transform::from_translation(-P),
        ..default()
    }));
    commands.spawn((
        Body {
            mass: 1.,
            vel: -2. * V,
        },
        PbrBundle {
        mesh: meshes.add(shape::UVSphere{radius: 0.1, ..default()}.into()),
        material: materials.add(Color::rgb(0.3, 0.3, 0.5).into()),
        transform: Transform::from_translation(Vec3::ZERO),
        ..default()
    }));
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4., 8., 4.),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2., 2.5, 7.5).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Default, Clone)]
struct Solution {
    k: (Vec3, Vec3),
    sum: (Vec3, Vec3)
}

fn update_bodies(mut bodies: Query<(&mut Transform, &mut Body)>, delta_t: Res<FixedTime>) {
    let dt = delta_t.period.as_secs_f32();
    let nbodies = bodies.iter().len();
    let mut y_vec: Vec<(Vec3, Vec3)> = Vec::new();
    y_vec.resize(nbodies, (Vec3::ZERO, Vec3::ZERO));
    let mut soln_vec: Vec<Solution> = Vec::new();
    soln_vec.resize(nbodies, default());

    let mut bodies_solve = |weight: f32, k_coefficient: f32| {
        for ((trans, body), (y, soln)) in bodies.iter().zip(y_vec.iter_mut().zip(&soln_vec)) {
            *y = (trans.translation + k_coefficient * soln.k.0, body.vel + k_coefficient * soln.k.1);
        }
        for (this_y, soln) in y_vec.iter().zip(&mut soln_vec) {
            soln.k.0 = this_y.1;
            soln.k.1 = Vec3::ZERO;
            for ((_, body), other_y) in bodies.iter().zip(&y_vec) {
                let d = other_y.0 - this_y.0;
                let len_squared = d.length_squared();
                if len_squared <= 0. {
                    continue;
                }
                soln.k.1 += (body.mass / (len_squared * len_squared.sqrt())) * d;
            }
            soln.k.1 *= G;
            soln.sum.0 += weight * soln.k.0;
            soln.sum.1 += weight * soln.k.1;
        }
    };

    bodies_solve(dt / 6., 0.);
    bodies_solve(dt / 3., 0.5 * dt);
    bodies_solve(dt / 3., 0.5 * dt);
    bodies_solve(dt / 6., dt);

    for ((mut transform, mut body), soln) in bodies.iter_mut().zip(&soln_vec) {
        transform.translation += soln.sum.0;
        body.vel += soln.sum.1;
    }
}

const SENSITIVITY: f32 = 0.001;
const MOVE: f32 = 1.2;

fn mouse_cam(mut ev_motion: EventReader<MouseMotion>, keys: Res<Input<KeyCode>>, mut cam: Query<&mut Transform, With<Camera>>) {
    if keys.pressed(KeyCode::AltLeft) {
        return;
    }
    let mut cam_trans = cam.get_single_mut().unwrap();
    let mut delta = Vec2::ZERO;
    for ev in ev_motion.iter() {
        delta += ev.delta;
    }
    let yaw = Quat::from_rotation_y(-SENSITIVITY * delta.x);
    let pitch = Quat::from_rotation_x(-SENSITIVITY * delta.y);
    cam_trans.rotation = yaw * cam_trans.rotation * pitch;
    ev_motion.clear();
}

#[inline]
fn combine(b0: bool, b1: bool) -> f32 {
    return if b0 {MOVE} else {0.} - if b1 {MOVE} else {0.};
}

fn mv_cam(mut cam: Query<&mut Transform, With<Camera>>, keys: Res<Input<KeyCode>>, time: Res<Time>) {
    let mut cam_trans = cam.get_single_mut().unwrap();
    let pos = keys.pressed(KeyCode::W);
    let neg = keys.pressed(KeyCode::S);
    if pos || neg {
        let forward = cam_trans.forward();
        cam_trans.translation += combine(pos, neg) * time.delta_seconds() * forward;
    }
    let pos = keys.pressed(KeyCode::D);
    let neg = keys.pressed(KeyCode::A);
    if pos || neg {
        let right = cam_trans.right();
        cam_trans.translation += combine(pos, neg) * time.delta_seconds() * right;
    }
    let pos = keys.pressed(KeyCode::ShiftLeft);
    let neg = keys.pressed(KeyCode::ControlLeft);
    if pos || neg {
        let up = cam_trans.up();
        cam_trans.translation += combine(pos, neg) * time.delta_seconds() * up;
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Iso Diamond Example"),
                    ..default()
                }),
                ..default()
            })
        .set(ImagePlugin::default_nearest()),
    );
    #[cfg(feature = "inspector")]
    app.add_plugins(WorldInspectorPlugin::new());
    app.add_systems(Startup, setup).add_systems(Update, (mouse_cam, mv_cam)).add_systems(FixedUpdate, update_bodies);
    app.insert_resource(FixedTime::new_from_secs(1. / 128.));
    app.run();
}
