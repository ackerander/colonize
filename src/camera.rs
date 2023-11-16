use bevy::{prelude::*, input::mouse::*};
use core::f32::consts::PI;

enum CamFocus {
    Entity(Entity),
    Point(Vec3),
}
#[derive(Component)]
struct CenterCam {
    focus: CamFocus,
    offset: Vec3,
}

fn calc_fov(x: f32) -> f32 {
    ACTIVATION_MAX * (1. - ACTIVATION_B.powf(-x))
}

fn setup_cam(mut commands: Commands) {
    let center = CenterCam { focus: CamFocus::Point(Vec3::ZERO), offset: Vec3::new(0., 3., 0.) };
    commands.spawn((Camera3dBundle {
            transform: Transform::from_translation(center.offset).looking_at(Vec3::ZERO, Vec3::Z),
            projection: Projection::Perspective(PerspectiveProjection {
                fov: calc_fov(center.offset.length()),
                ..default()
            }),
            ..default()
        },
        center
    ));
}

const ACTIVATION_MAX: f32 = 105.;
// const ACTIVATION_S: f32 = 0.5;
// const ACTIVATION_B: f32 = (ACTIVATION_S / ACTIVATION_MAX).exp();
const ACTIVATION_B: f32 = 1.00477326064844708774;
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
            CamFocus::Entity(_) => (),
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
            let left = trans.left();
            let forward = Vec2::new(left.y, -left.x).normalize().extend(0.);
            let dolly = 0.005 * (delta.x * left + delta.y * forward);
            *p += dolly;
            trans.translation += dolly;
        }
    }
    ev_motion.clear();

    // TODO: More sophisticated zooming
    if let Perspective(p) = proj.as_mut() {
        for ev in ev_scroll.read() {
            match ev.unit {
                MouseScrollUnit::Line => {
                    let delta = -0.2 * ev.y;
                    let len = center.offset.length();
                    let delta_vec = (delta / len) * center.offset;
                    center.offset += delta_vec;
                    trans.translation += delta_vec;
                    p.fov = calc_fov(len + delta);
                },
                MouseScrollUnit::Pixel => (),
            }
        }
    }
}

#[derive(Component)]
pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_cam).
            add_systems(Update, (mouse_cam, mv_cam));
    }
}
