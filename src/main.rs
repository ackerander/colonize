use bevy::{prelude::*, input::common_conditions::input_just_pressed};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use std::time::Duration;
mod body;
mod camera;
mod octree;

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
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Iso Diamond Example"),
                    ..default()
                }),
                ..default()
            })
        .set(ImagePlugin::default_nearest()),
        camera::CameraPlugin,
        body::BodyPlugin,
    ));
    #[cfg(feature = "inspector")]
    app.add_plugins(WorldInspectorPlugin::new());
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_micros(15625)))
       .add_systems(Startup, setup)
       .add_systems(Update, toggle_pause.run_if(input_just_pressed(KeyCode::Space)));
    app.run();
}
