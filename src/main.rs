use std::f32::consts::PI;

use bevy::{
    input::mouse::MouseMotion,
    math::vec2,
    prelude::*,
    render::{
        render_resource::PrimitiveTopology,
        settings::settings_priority_from_env,
    },
    sprite::MaterialMesh2dBundle,
    window::CursorGrabMode,
};

// Here we create the bevy app and connect items that we need for it to work
// like we want it to
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Game>()
        .add_startup_system(create_triangle)
        .add_startup_system(create_cursor)
        .add_system(move_cursor)
        .run();
}

#[derive(Component)]
struct MainCamera;

// This data structure stores commonly used information
// about triangle segments
#[derive(Component)]
struct TriangleSegment {
    id: usize,
    start: Vec2,
    end: Vec2,
    between: Vec2,
    normal: Vec2,
}

// This system creates the triangle that is drawn on screen
fn create_triangle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut segments =
        vec![
            Mesh::new(PrimitiveTopology::TriangleStrip);
            3
        ];

    // Triangle points are positioned as follows
    //     1
    //     0
    //
    //  4     2
    // 5       3

    let points: Vec<[f32; 3]> = (0..3)
        .map(|i| i as f32 * PI / 3. * 2. + (PI / 2.))
        .map(|i| (i.cos(), i.sin()))
        .flat_map(|(cos, sin)| {
            [
                [cos * 200., sin * 200., 1.],
                [cos * 250., sin * 250., 1.],
            ]
        })
        .collect();

    for (i, (wind, color)) in [0, 1, 2, 3, 4, 5, 0, 1]
        .map(|i| points.get(i).unwrap().to_owned())
        .windows(4)
        .step_by(2)
        .zip(vec![
            ColorMaterial::from(Color::RED),
            ColorMaterial::from(Color::GREEN),
            ColorMaterial::from(Color::BLUE),
        ])
        .enumerate()
    {
        let segment = segments.get_mut(i).unwrap();
        segment.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            wind.to_vec(),
        );

        let start = wind
            .get(0)
            .map(|s| Vec2::new(s[0], s[1]))
            .unwrap();
        let end = wind
            .get(2)
            .map(|s| Vec2::new(s[0], s[1]))
            .unwrap();

        let between = end - start;

        let normal = Vec2::new(between.y, -between.x);

        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(segment.to_owned()).into(),
                material: materials.add(color),
                transform: Transform::from_translation(
                    Vec3::new(0., 0., 1.),
                ),
                ..Default::default()
            },
            TriangleSegment {
                id: i,
                start,
                end,
                between,
                normal,
            },
        ));
    }

    commands.spawn((Camera2dBundle::default(), MainCamera));
}

// This data structure stores information about the
// cursors current state
#[derive(Default)]
enum CursorState {
    #[default]
    Unlocked,
    // sliding on segment id
    Sliding(usize),
}

#[derive(Resource, Default)]
struct Game {
    cursor_state: CursorState,
}

// Some methods that were not implemented in bevy
// but i needed
trait ToVec2 {
    fn to_vec2(&self) -> Vec2;
}

impl ToVec2 for Vec3 {
    fn to_vec2(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

trait Cross {
    type Other;
    fn cross(&self, other: &Self::Other) -> f32;
}

impl Cross for Vec2 {
    type Other = Vec2;

    fn cross(&self, other: &Self::Other) -> f32 {
        (self.x * other.y) - (self.y * other.x)
    }
}

#[derive(Component)]
struct GameCursor;

// This system creates the cursor that is drawn on screen
fn create_cursor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut window: Query<&mut Window>,
) {
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes
                .add(shape::Circle::new(10.).into())
                .into(),
            material: materials
                .add(ColorMaterial::from(Color::PINK)),
            transform: Transform::from_translation(
                Vec3::new(0., 0., 0.),
            ),
            ..default()
        },
        GameCursor,
    ));

    let mut wind = window.get_single_mut().unwrap();
    wind.cursor.grab_mode = CursorGrabMode::Locked;
    wind.cursor.visible = false;
}

// This system handles cursor movement
fn move_cursor(
    mut cursor_component: Query<
        &mut Transform,
        With<GameCursor>,
    >,
    triangle_segments: Query<&TriangleSegment>,
    mut cursor_pos: EventReader<MouseMotion>,
    mut game: ResMut<Game>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    for MouseMotion { mut delta } in cursor_pos.iter() {
        let cursor =
            cursor_component.single().translation.to_vec2();
        delta.y = -delta.y;
        let mut future_pos = cursor + delta;
        match game.cursor_state {
            CursorState::Unlocked => {
                for TriangleSegment {
                    id,
                    start,
                    end,
                    between,
                    ..
                } in &triangle_segments
                {
                    if (future_pos - *start).cross(between)
                        > 0.
                    {
                        // Cursor is past the segment
                        if let Some(fp) = get_intersection(
                            (&cursor, &future_pos),
                            (&start, &end),
                        ) {
                            println!("{fp} {cursor} {future_pos} {start} {end}");
                            future_pos = fp;
                        } else {
                            break;
                        };
                        game.cursor_state =
                            CursorState::Sliding(*id);
                        audio.play(asset_server.load(
                            &format!("sounds/{}.wav", id),
                        ));
                        break;
                    }
                }
            }
            CursorState::Sliding(id) => {
                if let Some(TriangleSegment {
                    start,
                    end,
                    between,
                    ..
                }) = triangle_segments.iter().nth(id)
                {
                    if (cursor + delta - *start)
                        .cross(between)
                        < 0.
                    {
                        game.cursor_state =
                            CursorState::Unlocked;
                        let trans = &mut cursor_component
                            .single_mut()
                            .translation;
                        trans.x = future_pos.x;
                        trans.y = future_pos.y;
                        continue;
                    }

                    let axis = if between.x.abs()
                        < between.y.abs()
                    {
                        vec2(0., 1. * between.y.signum())
                    } else {
                        vec2(1. * between.x.signum(), 0.)
                    };

                    let angle =
                        axis.angle_between(*between);
                    future_pos = delta
                        .rotate(Vec2::from_angle(angle))
                        .project_onto(*between)
                        + cursor;

                    let line_dist = start.distance(*end);

                    if line_dist
                        < future_pos.distance(*start)
                    {
                        future_pos = *end;
                    } else if line_dist
                        < future_pos.distance(*end)
                    {
                        future_pos = *start;
                    }
                }
            }
        }

        let trans =
            &mut cursor_component.single_mut().translation;
        trans.x = future_pos.x;
        trans.y = future_pos.y;
    }
}

// FIXME: Sometimes it jumps around not sure if this func is the problem
fn get_intersection(
    (p1, p2): (&Vec2, &Vec2),
    (p3, p4): (&Vec2, &Vec2),
) -> Option<Vec2> {
    // y difference of points 1 and 2, 3 and 4
    // Used multiple times so saved to a var
    let yd12 = p1.y - p2.y;
    let yd34 = p3.y - p4.y;

    let denominator =
        (p1.x - p2.x) * yd34 - yd12 * (p3.x - p4.x);

    // Parallel or coincident
    if denominator == 0. {
        return None;
    }

    let det12 = p1.x * p2.y - p1.y * p2.x;
    let det34 = p3.x * p4.y - p3.y * p4.x;
    println!("{det12} {det34}");

    let xnumerator =
        det12 * (p3.x - p4.x) - (p1.x - p2.x) * det34;
    let ynumerator = det12 * yd34 - yd12 * det34;
    println!("{xnumerator} {ynumerator} {denominator}");

    Some(Vec2::new(
        xnumerator / denominator,
        ynumerator / denominator,
    ))
}
