use bevy::{math::prelude::*, prelude::*};

mod board;

use board::{Board, Cell, Form, Tile};

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tile matching".into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .init_resource::<Game>()
        // .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_click,
                draw_selected,
                (
                    check_board,
                    despawn_tiles.run_if(run_if_has_tiles_to_despawn),
                    gravity,
                )
                    .chain(),
            ),
        )
        // .add_systems(Update, handle_right_click)
        .run()
}

// #[derive(Clone, Copy, Default, PartialEq, Eq, Debug, States, Hash)]
// enum GameState {
//     #[default]
//     Playing,
//     GameOver,
// }

fn setup(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut board = Board::new();

    commands.spawn((Camera2d, Transform::default()));

    let cell_mesh = meshes.add(Rectangle::default());
    let cell_material = materials.add(Color::srgb(0.12, 0.12, 0.18));
    let select_area_mesh = meshes.add(Rectangle::default());
    // @FIXME Вернуть альфа канал 0.75
    // С параметром альфа канала выглядит как будто область выделения находится над фигурой.
    let select_area_material =
        materials.add(Color::srgba(184. / 255., 134. / 255., 11. / 255., 1.));
    let circle_mesh = meshes.add(Circle::new(0.4));
    let circle_material = materials.add(Color::srgb(175. / 255., 43. / 255., 30. / 255.));
    let square_mesh = meshes.add(Rectangle::from_size(Vec2::splat(0.8)));
    let square_material = materials.add(Color::srgb(71. / 255., 132. / 255., 48. / 255.));
    let triangle_mesh = meshes.add(Triangle2d::new(
        Vec2::new(0., 0.4),
        Vec2::new(-0.4, -0.4),
        Vec2::new(0.4, -0.4),
    ));
    let triangle_material = materials.add(Color::srgb(27. / 255., 85. / 255., 131. / 255.));

    for i in 0..board.height() {
        let mut row = Vec::with_capacity(board.width());
        for j in 0..board.width() {
            let form = rand::random();
            let (form_mesh, form_material) = match form {
                Form::Circle => (circle_mesh.clone(), circle_material.clone()),
                Form::Square => (square_mesh.clone(), square_material.clone()),
                Form::Triangle => (triangle_mesh.clone(), triangle_material.clone()),
            };

            let Vec2 { x, y } = board.get_cell_coord(i, j);
            // let x = start.x + j as f32 * board.cell_size();
            // let y = start.y + i as f32 * board.cell_size();

            let select_area_entity = commands
                .spawn((
                    Mesh2d(select_area_mesh.clone()),
                    MeshMaterial2d(select_area_material.clone()),
                    Transform::from_xyz(0., 0., 1.),
                    SelectArea,
                    Visibility::Hidden,
                ))
                .id();

            let tile_entity = commands
                .spawn((
                    Transform::from_xyz(x, y, 0.5).with_scale(Vec3::new(
                        board.cell_size() - board.border_width(),
                        board.cell_size() - board.border_width(),
                        0.,
                    )),
                    Visibility::Inherited,
                ))
                .add_child(select_area_entity)
                .with_child((
                    Mesh2d(form_mesh),
                    MeshMaterial2d(form_material),
                    Transform::from_xyz(0., 0., 100.).with_scale(Vec3::splat(0.95)),
                ))
                .id();

            commands.spawn((
                Mesh2d(cell_mesh.clone()),
                MeshMaterial2d(cell_material.clone()),
                Transform::from_xyz(x, y, 0.).with_scale(Vec3::new(
                    board.cell_size() - board.border_width(),
                    board.cell_size() - board.border_width(),
                    0.,
                )),
            ));

            row.push(Cell {
                tile: Some(Tile {
                    form,
                    entity: tile_entity,
                    select_area_entity,
                }),
            });
        }
        board.push_row(row);
    }

    // let white_color = materials.add(Color::srgb(1., 1., 1.));
    // commands.spawn((
    //     Mesh2d(circle_mesh.clone()),
    //     MeshMaterial2d(white_color.clone()),
    //     Transform::from_translation(Board::ceil_right().extend(10.)).with_scale(Vec3::splat(40.)),
    // ));
    // commands.spawn((
    //     Mesh2d(circle_mesh.clone()),
    //     MeshMaterial2d(white_color.clone()),
    //     Transform::from_translation(Board::bottom_left().extend(10.)).with_scale(Vec3::splat(40.)),
    // ));
    // commands.spawn((
    //     Mesh2d(circle_mesh.clone()),
    //     MeshMaterial2d(white_color.clone()),
    //     Transform::from_xyz(0., 0., 10.).with_scale(Vec3::splat(40.)),
    // ));

    game.board = board;
}

fn draw_selected(mut game: ResMut<Game>, mut commands: Commands) {
    while let Some((i, j)) = game.prev_selected.pop() {
        set_selected(&mut commands, &mut game.board[i][j], false);
    }

    if let Some(cell) = game.selected.map(|(i, j)| &mut game.board[i][j]) {
        set_selected(&mut commands, cell, true);
    }
}

fn set_selected(commands: &mut Commands, cell: &mut Cell, selected: bool) {
    let (new_visibility, new_scale) = if selected {
        (
            Visibility::Inherited,
            Vec3::new(cell.size() + 10., cell.size() + 10., 0.),
        )
    } else {
        (
            Visibility::Hidden,
            Vec3::new(cell.tile_size(), cell.tile_size(), 0.),
        )
    };

    // commands
    //     .entity(cell.entity)
    //     .entry::<Transform>()
    //     .and_modify(move |mut transform| transform.scale = new_scale);

    if let Some(tile) = cell.tile {
        commands
            .entity(tile.entity)
            .entry::<Transform>()
            .and_modify(move |mut transform| transform.scale = new_scale);

        commands
            .entity(tile.select_area_entity)
            .entry::<Visibility>()
            .and_modify(move |mut visibility| *visibility = new_visibility);
    }
}

fn handle_click(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    mut game: ResMut<Game>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };

    println!("Clicked on ({}, {})", mouse_pos.x, mouse_pos.y);

    let (camera, camera_transform) = *camera_query;
    let Ok(mouse_world_pos) = camera.viewport_to_world_2d(camera_transform, mouse_pos) else {
        return;
    };

    println!(
        "Mouse in world on {}, {}",
        mouse_world_pos.x, mouse_world_pos.y
    );

    let bottom_left_border_pos = game.board.bottom_left() - game.board.cell_size() / 2.;
    let upper_right_border_pos = game.board.ceil_right() + game.board.cell_size() / 2.;

    // println!("Bottom left border: {bottom_left_border_pos}");
    // println!("Upper right border: {upper_right_border_pos}");

    if !(mouse_world_pos.x > bottom_left_border_pos.x
        && mouse_world_pos.x < upper_right_border_pos.x
        && mouse_world_pos.y > bottom_left_border_pos.y
        && mouse_world_pos.y < upper_right_border_pos.y)
    {
        return;
    }

    let clicked_cell_pos =
        ((mouse_world_pos - bottom_left_border_pos) / game.board.cell_size()).floor();

    // println!("Clicked sell pos: {clicked_cell_pos}");

    let j = clicked_cell_pos.x as usize;
    let i = clicked_cell_pos.y as usize;

    println!("Defined cell: {}, {}", i, j);

    let selected = game.selected;

    game.prev_selected.extend(selected);

    if Some((i, j)) == selected {
        game.selected = None;
    } else if game.board[i][j].tile.is_none() {
		game.selected = None;
	} else {
        game.selected = Some((i, j));
    }
}

// fn handle_right_click(world: &mut World) {
// 	let buttons = world.get_resource::<ButtonInput<MouseButton>>().unwrap();
// 	if !buttons.pressed(MouseButton::Right) {
// 		return;
// 	}

// 	let system_id = world.register_system(gravity);

// 	world.run_system(system_id).unwrap();
// }

fn check_board(mut game: ResMut<Game>) {
    let tiles_to_despawn = {
        let board = &game.board;

        let mut form_by_row = Form::Square;
        let mut form_by_column = Form::Square;
        let mut matched_by_row = vec![];
        let mut matched_by_column = vec![];

        let mut tiles_to_despawn = Vec::new();

        let n = board.width().max(board.height());
        for i in 0..n {
            for j in 0..n {
                if let Some(cur_form) = board
                    .get(i)
                    .and_then(|row| row.get(j))
                    .and_then(|cell| cell.tile.as_ref())
                    .map(|tile| tile.form)
                {
                    if form_by_row != cur_form {
                        if matched_by_row.len() >= 3 {
                            tiles_to_despawn.append(&mut matched_by_row);
                        } else {
                            matched_by_row.clear();
                        }
                        form_by_row = cur_form
                    }

                    matched_by_row.push((i, j));
                }

                if let Some(cur_form) = board
                    .get(j)
                    .and_then(|row| row.get(i))
                    .and_then(|cell| cell.tile.as_ref())
                    .map(|tile| tile.form)
                {
                    if form_by_column != cur_form {
                        if matched_by_column.len() >= 3 {
                            tiles_to_despawn.append(&mut matched_by_column);
                        } else {
                            matched_by_column.clear();
                        }
                        form_by_column = cur_form;
                    }

                    matched_by_column.push((j, i));
                }
            }
        }

        if matched_by_row.len() >= 3 {
            tiles_to_despawn.append(&mut matched_by_row);
        }
        if matched_by_column.len() >= 3 {
            tiles_to_despawn.append(&mut matched_by_column);
        }

        tiles_to_despawn
    };

    game.tiles_to_despawn.extend(tiles_to_despawn);
}

fn despawn_tiles(mut game: ResMut<Game>, mut commands: Commands) {
    while let Some((i, j)) = game.tiles_to_despawn.pop() {
        if let Some(tile) = game.board[i][j].tile.take() {
            commands.entity(tile.entity).despawn();
        }
    }

    for col_id in 0..game.board.width() {
        let last_empty = (0..game.board.height()).find_map(|i| {
            if game.board[i][col_id].tile.is_none() {
                Some(i)
            } else {
                None
            }
        });

        if let Some(mut last_empty) = last_empty {
            for row_id in last_empty..game.board.height() {
                if let Some(tile) = game.board[row_id][col_id].tile.as_ref() {
                    commands
                        .entity(tile.entity)
                        .insert(Falling(last_empty, col_id));
                    last_empty += 1;
                }
            }
        }
    }
}

fn gravity(time: Res<Time>, game: Res<Game>, query: Query<(&mut Transform, &Falling)>) {
    for (mut transform, Falling(target_i, target_j)) in query {
		let target_coord = game.board.get_cell_coord(*target_i, *target_j);

		transform.translation.y -= 75. * time.delta_secs();
		if transform.translation.y <= target_coord.y {
			transform.translation.y = target_coord.y;
		}
	}
}

fn run_if_has_tiles_to_despawn(game: Res<Game>) -> bool {
    !game.tiles_to_despawn.is_empty()
}

#[derive(Resource, Default)]
struct Game {
    board: Board,
    prev_selected: Vec<(usize, usize)>,
    selected: Option<(usize, usize)>,
    tiles_to_despawn: Vec<(usize, usize)>,
}

#[derive(Component)]
struct SelectArea;

#[derive(Component)]
#[component(storage = "SparseSet")]
struct Falling(usize, usize);
