use bevy::{prelude::*, input::mouse::MouseMotion};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use toml::Value;
use rand::Rng;
use std::fs;

const G: f32 = 6.6743e-11;
const FILE: &str = "assets/bodies.toml";

#[derive(Component)]
struct Body {
    mass: f32,
    vel: Vec3,
}

fn parse_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    let parse_vec3 = |val: &Value| -> Option<Vec3> {
        let table = val.as_table()?;
        let get_f = |key| Some(table[key].as_float()? as f32);
        Some(Vec3::new(get_f("x")?, get_f("y")?, get_f("z")?))
    };
    let text = fs::read_to_string(FILE).expect("Failed to open file");
    let config: Value = toml::from_str(text.as_str()).expect("Incorrect format");
    for body_cfg in config["body"].as_array().expect("Incorrect format") {
        commands.spawn((
            Body {
                mass: body_cfg["mass"].as_float().unwrap_or(1.) as f32,
                vel: parse_vec3(&body_cfg["velocity"]).unwrap_or(Vec3::ZERO),
            },
            PbrBundle {
                mesh: meshes.add(shape::UVSphere{
                    radius: body_cfg["r"].as_float().unwrap_or(1.) as f32, ..default()
                }.into()),
                material: materials.add(Color::hsl(360. * rng.gen::<f32>(), 0.4, 0.4).into()),
                transform: Transform::from_translation(parse_vec3(&body_cfg["position"]).unwrap_or(Vec3::ZERO)),
                ..default()
            }
        ));
    }
}

fn setup(mut commands: Commands) {
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
            soln.k = (this_y.1, Vec3::ZERO);
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
    app.add_systems(Startup, (setup, parse_bodies)).add_systems(Update, (mouse_cam, mv_cam)).add_systems(FixedUpdate, update_bodies);
    app.insert_resource(FixedTime::new_from_secs(1. / 128.));
    app.run();
}
