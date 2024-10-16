use bevy::{
    asset::AssetMetaCheck,
    math::bounding::{Aabb2d, BoundingCircle, IntersectsVolume},
    prelude::*,
    utils::HashMap,
};

use rand::Rng;

// Config
const WINDOW_TITLE: &str = "Pickin' Sticks";
const WINDOW_WIDTH: f32 = 960.0;
const WINDOW_HEIGHT: f32 = 540.0;

// Paths
const BACKGROUND_TILE_PATH: &str = "Grass.png";
const STICK_COLLECTABLE_PATH: &str = "Stick.png";
const CHARACTER_SPRITE_SHEET_PATH: &str = "CharacterSpriteSheet.png";

// Define the resources
#[derive(Resource)]
struct Score(i32);

#[derive(Resource)]
struct Speed(f32);

#[derive(Resource)]
struct Rank {
    thresholds: HashMap<i32, String>,
    current: String,
}

// Events
#[derive(Event, Default)]
struct CollisionEvent;

// Game Objects
#[derive(Component)]
struct GrassTile;

#[derive(Component)]
struct StickCollectable;

// Stick Components
#[derive(Component)]
struct Collider;

#[derive(Component)]
struct Player;

// Player Components
#[derive(Component, PartialEq, Eq)]
enum PlayerDirection {
    Stationary,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

// UI Components
#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct SpeedText;

#[derive(Component)]
struct RankText;

fn main() {
    App::new()
        .add_event::<CollisionEvent>()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: WINDOW_TITLE.into(),
                        resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                        enabled_buttons: bevy::window::EnabledButtons {
                            maximize: false,
                            ..Default::default()
                        },
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_systems(Startup, (setup_resources, setup))
        .add_systems(
            Update,
            (
                player_input,
                player_animation,
                player_movement,
                player_screen_wrapping,
                player_collision,
            ),
        )
        .add_systems(Update, (update_score_and_speed_system, update_rank_system))
        .add_systems(
            Update,
            (
                update_score_text_system,
                update_speed_text_system,
                update_rank_text_system,
            ),
        )
        .run();
}

fn setup_resources(mut commands: Commands) {
    // Initialize Score
    commands.insert_resource(Score(0));

    // Initialize Speed
    commands.insert_resource(Speed(150.0));

    // Initialize Rank
    let mut rank_thresholds = HashMap::new();
    rank_thresholds.insert(1, "Weak".to_string());
    rank_thresholds.insert(5, "Decent".to_string());
    rank_thresholds.insert(10, "Ok".to_string());

    commands.insert_resource(Rank {
        thresholds: rank_thresholds,
        current: "Weak".to_string(),
    });
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Spawn Grass
    let grass_texture_handle: Handle<Image> = asset_server.load(BACKGROUND_TILE_PATH);

    commands.spawn((
        SpriteBundle {
            texture: grass_texture_handle,
            sprite: Sprite {
                custom_size: Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        ImageScaleMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 1.0,
        },
        GrassTile,
    ));

    // Spawn Player
    let player_texture_handle: Handle<Image> = asset_server.load(CHARACTER_SPRITE_SHEET_PATH);
    let player_layout = TextureAtlasLayout::from_grid(UVec2::splat(24), 7, 1, None, None);
    let player_texture_atlas_layout = texture_atlas_layouts.add(player_layout);
    let player_animation_indices = AnimationIndices { first: 1, last: 6 };

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(2.0)),
            texture: player_texture_handle,
            ..default()
        },
        TextureAtlas {
            layout: player_texture_atlas_layout,
            index: player_animation_indices.first,
        },
        player_animation_indices,
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        Player,
        PlayerDirection::Stationary,
    ));

    // Spawn Initial Stick
    let stick_texture_handle: Handle<Image> = asset_server.load(STICK_COLLECTABLE_PATH);

    commands.spawn((
        SpriteBundle {
            texture: stick_texture_handle,
            transform: Transform::from_xyz(48.0, 0.0, 1.0).with_scale(Vec3::splat(2.0)),
            ..default()
        },
        StickCollectable,
        Collider,
    ));

    // Initialize UI
    let ui_root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .id();

    let top_black_bar = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Px(60.),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.)),
                ..default()
            },
            background_color: Color::BLACK.into(),
            ..default()
        })
        .id();

    let bottom_black_bar = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Px(60.),
                ..default()
            },
            background_color: Color::BLACK.into(),
            ..default()
        })
        .id();

    let score_text = commands
        .spawn((TextBundle::from_section(
            "Score: 000",
            TextStyle {
                font_size: 24.0,
                color: Color::WHITE,
                ..default()
            },
        ),))
        .insert(ScoreText)
        .id();

    let speed_text = commands
        .spawn((TextBundle::from_section(
            "Current Speed: 000",
            TextStyle {
                font_size: 24.0,
                color: Color::WHITE,
                ..default()
            },
        ),))
        .insert(SpeedText)
        .id();

    let rank_text = commands
        .spawn((TextBundle::from_section(
            "Rank: Decent",
            TextStyle {
                font_size: 24.0,
                color: Color::WHITE,
                ..default()
            },
        ),))
        .insert(RankText)
        .id();

    commands
        .entity(ui_root)
        .add_child(top_black_bar)
        .add_child(bottom_black_bar);

    commands
        .entity(top_black_bar)
        .add_child(score_text)
        .add_child(speed_text)
        .add_child(rank_text);
}

fn player_animation(
    time: Res<Time>,
    mut query: Query<(
        &AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlas,
        &Player,
    )>,
) {
    for (indices, mut timer, mut atlas, _player) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}

fn player_input(
    mut player: Query<(&mut PlayerDirection, &mut Sprite), With<Player>>,
    kb_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok((mut player_direction, mut player_sprite)) = player.get_single_mut() else {
        return;
    };

    if kb_input.pressed(KeyCode::KeyW) {
        *player_direction = PlayerDirection::Up;
    }

    if kb_input.pressed(KeyCode::KeyS) {
        *player_direction = PlayerDirection::Down;
    }

    if kb_input.pressed(KeyCode::KeyA) {
        *player_direction = PlayerDirection::Left;
        player_sprite.flip_x = true;
    }

    if kb_input.pressed(KeyCode::KeyD) {
        *player_direction = PlayerDirection::Right;
        player_sprite.flip_x = false;
    }
}

fn player_movement(
    time: Res<Time>,
    speed: Res<Speed>,
    mut player_direction: Query<(&PlayerDirection, &mut Transform)>,
) {
    for (player, mut transform) in &mut player_direction {
        let cur_speed: f32 = speed.0 * time.delta_seconds();
        match *player {
            PlayerDirection::Up => transform.translation.y += cur_speed,
            PlayerDirection::Down => transform.translation.y -= cur_speed,
            PlayerDirection::Right => transform.translation.x += cur_speed,
            PlayerDirection::Left => transform.translation.x -= cur_speed,
            _ => return,
        }
    }
}

fn player_screen_wrapping(mut player_position: Query<&mut Transform, With<Player>>) {
    let Ok(mut player_pos) = player_position.get_single_mut() else {
        return;
    };

    if player_pos.translation.x < -WINDOW_WIDTH / 2.0 {
        player_pos.translation.x = (WINDOW_WIDTH / 2.0) - 24.0;
    } else if player_pos.translation.x > WINDOW_WIDTH / 2.0 {
        player_pos.translation.x = -(WINDOW_WIDTH / 2.0) + 24.0;
    }

    if player_pos.translation.y < -WINDOW_HEIGHT / 2.0 + 60.0 {
        player_pos.translation.y = (WINDOW_HEIGHT / 2.0) - 60.0 - 24.0;
    } else if player_pos.translation.y > WINDOW_HEIGHT / 2.0 - 60.0 {
        player_pos.translation.y = -(WINDOW_HEIGHT / 2.0) + 60. + 24.0;
    }
}

fn player_collision(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    stick_query: Query<(Entity, &Transform), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    for (stick_entity, stick_transform) in &stick_query {
        let collision = is_colliding(
            BoundingCircle::new(stick_transform.translation.truncate(), 24.0),
            Aabb2d::new(
                player_transform.translation.truncate(),
                player_transform.scale.truncate() / 2.0,
            ),
        );

        if collision == true {
            collision_events.send_default();
            commands.entity(stick_entity).despawn();
        }
    }
}

fn update_score_and_speed_system(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut speed: ResMut<Speed>,
    mut collision_events: EventReader<CollisionEvent>,
    asset_server: Res<AssetServer>,
) {
    if !collision_events.is_empty() {
        collision_events.clear();
        score.0 += 1;
        speed.0 += 10.0;

        spawn_new_stick(&mut commands, &asset_server);
    }
}

fn spawn_new_stick(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    let stick_texture_handle: Handle<Image> = asset_server.load(STICK_COLLECTABLE_PATH);

    let mut rng = rand::thread_rng();
    let random_x = rng.gen_range((-WINDOW_WIDTH / 2.0 + 16.)..(WINDOW_WIDTH / 2.0 - 16.));
    let random_y =
        rng.gen_range((-WINDOW_HEIGHT / 2.0 + 16. + 60.)..(WINDOW_HEIGHT / 2.0 - 16. - 60.));

    commands.spawn((
        SpriteBundle {
            texture: stick_texture_handle,
            transform: Transform::from_xyz(random_x, random_y, 1.0).with_scale(Vec3::splat(2.0)),
            ..default()
        },
        StickCollectable,
        Collider,
    ));
}

fn update_rank_system(score: Res<Score>, mut rank: ResMut<Rank>) {
    let new_rank = rank
        .thresholds
        .iter()
        .filter(|(&threshold, _)| score.0 >= threshold)
        .max_by_key(|(&threshold, _)| threshold)
        .map(|(_, rank_name)| rank_name.clone())
        .unwrap_or_else(|| "Weak".to_string());

    if new_rank != rank.current {
        rank.current = new_rank;
    }
}

fn update_score_text_system(mut query: Query<&mut Text, With<ScoreText>>, score: Res<Score>) {
    if let Ok(mut text) = query.get_single_mut() {
        text.sections[0].value = format!("Score: {:03}", score.0);
    }
}

fn update_speed_text_system(mut query: Query<&mut Text, With<SpeedText>>, speed: Res<Speed>) {
    if let Ok(mut text) = query.get_single_mut() {
        text.sections[0].value = format!("Current Speed: {:06.2}", speed.0);
    }
}

fn update_rank_text_system(mut query: Query<&mut Text, With<RankText>>, rank: Res<Rank>) {
    if let Ok(mut text) = query.get_single_mut() {
        text.sections[0].value = format!("Rank: {}", rank.current);
    }
}

fn is_colliding(stick_bounding_circle: BoundingCircle, player_bounding_box: Aabb2d) -> bool {
    if !stick_bounding_circle.intersects(&player_bounding_box) {
        return false;
    }
    return true;
}
