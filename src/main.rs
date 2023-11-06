use bevy::{prelude::*, input::mouse::*, input::common_conditions::input_just_pressed};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use toml::Value;
use std::{fs, time::Duration};
use core::f32::consts::PI;

const G: f32 = 6.6743e-11;
const FILE: &str = "assets/bodies.toml";
const ACTIVATION_MAX: f32 = 120.;
const ACTIVATION_S: f32 = 0.5;
// const ACTIVATION_B: f32 = (ACTIVATION_S / ACTIVATION_MAX).exp();
const ACTIVATION_B: f32 = 1.00417535929111852966;

#[derive(Component)]
struct Body {
    name: String,
    mass: f32,
    vel: Vec3,
    angular_vel: Vec3,
}

fn parse_bodies(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let parse_vec3 = |val: &Value| -> Option<Vec3> {
        let table = val.as_table()?;
        let get_f = |key| Some(table[key].as_float()? as f32);
        Some(Vec3::new(get_f("x")?, get_f("y")?, get_f("z")?))
    };
    // TODO: Better file handling
    let texture_handle: Handle<Image> = asset_server.load("tex_DebugUVTiles.png");
    let text = fs::read_to_string(FILE).expect("Failed to open file");
    let config: Value = toml::from_str(text.as_str()).expect("Incorrect format");
    for body_cfg in config["body"].as_array().expect("Incorrect format") {
        commands.spawn((
            Body {
                name: body_cfg["name"].as_str().unwrap_or("Unnamed").to_owned(),
                mass: body_cfg["mass"].as_float().unwrap_or(1.) as f32,
                vel: parse_vec3(&body_cfg["velocity"]).unwrap_or(Vec3::ZERO),
                angular_vel: parse_vec3(&body_cfg["angular_vel"]).unwrap_or(Vec3::ZERO),
            },
            PbrBundle {
                mesh: meshes.add(shape::UVSphere{
                    radius: body_cfg["r"].as_float().unwrap_or(1.) as f32, ..default()
                }.into()),
                material: materials.add(StandardMaterial {
                    base_color_texture: Some(texture_handle.clone()),
                    ..default()
                }),
                transform: Transform::from_translation(parse_vec3(&body_cfg["position"]).unwrap_or(Vec3::ZERO)),
                ..default()
            }
        ));
    }
}

enum CamFocus {
    Entity(Entity),
    Point(Vec3),
}
#[derive(Component)]
struct CenterCam {
    focus: CamFocus,
    offset: Vec3,
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
    commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(0., 4., 0.).looking_at(Vec3::ZERO, Vec3::Z),
            ..default()
        },
        CenterCam { focus: CamFocus::Point(Vec3::ZERO), offset: Vec3::new(0., 4., 0.) }
    ));
}

const SENSITIVITY: f32 = 1e-2;
use bevy::render::camera::Projection::Perspective;
fn mouse_cam(
    buttons: Res<Input<MouseButton>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    mut cam_query: Query<(&mut Transform, &mut Projection, &mut CenterCam), With<Camera>>
) {
    let (mut trans, mut proj, mut center) = cam_query.get_single_mut().unwrap();
    let mut delta = Vec2::ZERO;
    for ev in ev_motion.read() {
        delta += ev.delta;
    }
    if buttons.pressed(MouseButton::Right) {
        match center.focus {
            CamFocus::Entity(e) => (),
            CamFocus::Point(p) => {
                // let angle = Vec3::Z.angle_between(center.offset);
                let angle = PI - trans.forward().z.acos();
                let rot_y = Quat::from_axis_angle(trans.right(), (-SENSITIVITY * delta.y).clamp(0.05 * PI - angle, 0.95 * PI - angle));
                let rot = Quat::from_rotation_z(-SENSITIVITY * delta.x) * rot_y;
                center.offset = rot * center.offset;
                trans.translation = p + center.offset;
                trans.rotation = -rot * trans.rotation;
            },
        }
    }
    if buttons.pressed(MouseButton::Middle) {
        if let CamFocus::Point(p) = &mut center.focus {
            let dolly = 0.005 * (delta.x * trans.left() + delta.y * trans.up());
            *p += dolly;
            trans.translation += dolly;
            println!("{}", p);
        }
    }
    ev_motion.clear();

    // TODO: More sophisticated zooming
    if let Perspective(p) = proj.as_mut() {
        for ev in ev_scroll.read() {
            match ev.unit {
                MouseScrollUnit::Line => p.fov = (p.fov * (1. - 0.2 * ev.y)).clamp(0.1f32.to_radians(), 180.0f32.to_radians()),
                MouseScrollUnit::Pixel => p.fov = (p.fov * (1. - 0.2 * ev.y)).clamp(0.1f32.to_radians(), 180.0f32.to_radians()),
            }
        }
    }
}

#[derive(Default, Clone)]
struct Solution {
    k: (Vec3, Vec3),
    sum: (Vec3, Vec3)
}

fn update_bodies(mut bodies: Query<(&mut Transform, &mut Body)>, delta_t: Res<Time>) {
    let dt = delta_t.delta_seconds();
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
        transform.rotation *= Quat::from_scaled_axis(dt * body.angular_vel);
        body.vel += soln.sum.1;
    }
}

const MOVE: f32 = 1.2;
const SPIN: f32 = 1.2;
fn mv_cam(mut cam: Query<&mut Transform, With<Camera>>, keys: Res<Input<KeyCode>>, time: Res<Time>) {
    let mut cam_trans = cam.get_single_mut().unwrap();
    let pos = keys.pressed(KeyCode::W);
    let neg = keys.pressed(KeyCode::S);
    if pos ^ neg {
        let forward = cam_trans.forward();
        cam_trans.translation += if pos {MOVE} else {-MOVE} * time.delta_seconds() * forward;
    }
    let pos = keys.pressed(KeyCode::D);
    let neg = keys.pressed(KeyCode::A);
    if pos ^ neg {
        let right = cam_trans.right();
        cam_trans.translation += if pos {MOVE} else {-MOVE} * time.delta_seconds() * right;
    }
    let pos = keys.pressed(KeyCode::ShiftLeft);
    let neg = keys.pressed(KeyCode::ControlLeft);
    if pos ^ neg {
        let up = cam_trans.up();
        cam_trans.translation += if pos {MOVE} else {-MOVE} * time.delta_seconds() * up;
    }

    let pos = keys.pressed(KeyCode::Q);
    let neg = keys.pressed(KeyCode::E);
    if pos ^ neg {
        let forward = cam_trans.forward();
        cam_trans.rotate_axis(forward, if pos {SPIN} else {-SPIN} * time.delta_seconds());
    }
}

fn toggle_pause(mut time: ResMut<Time<Virtual>>) {
    if time.is_paused() {
        time.unpause();
    } else {
        time.pause();
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
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_micros(15625)))
       .add_systems(Startup, (setup, parse_bodies))
       .add_systems(Update, (mouse_cam, mv_cam, toggle_pause.run_if(input_just_pressed(KeyCode::Space))))
       .add_systems(FixedUpdate, update_bodies);
    app.run();
}
