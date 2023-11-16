use bevy::prelude::*;
use toml::Value;
use std::fs;

const G: f32 = 6.6743e-11;
const FILE: &str = "assets/bodies.toml";

#[derive(Component)]
pub struct Body {
    pub name: String,
    pub mass: f32,
    pub vel: Vec3,
    pub angular_vel: Vec3,
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

#[derive(Component)]
pub struct BodyPlugin;
impl Plugin for BodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, parse_bodies).
            add_systems(FixedUpdate, update_bodies);
    }
}
