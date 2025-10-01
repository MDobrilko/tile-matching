use bevy::{math::prelude::*, prelude::*};

mod board;

use board::{Board, Cell, Form, Tile, TILE_VELOCITY};

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tile matching".into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .init_resource::<Board>()
        .init_resource::<Selection>()
        .init_resource::<TilesToDespawn>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_click,
                handle_selection,
                (
					move_tiles,
                    check_board,
                    despawn_tiles.run_if(run_if_has_tiles_to_despawn),
                )
                    .chain(),
            ),
        )
        .run()
}

fn setup(
    mut commands: Commands,
    mut board: ResMut<Board>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
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
                .spawn(TileBundle {
                    transform: Transform::from_xyz(x, y, 0.5).with_scale(Vec3::new(
                        board.cell_size() - board.border_width(),
                        board.cell_size() - board.border_width(),
                        0.,
                    )),
                    visibility: Visibility::Inherited,
                    state: TileState::Idle,
                })
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
}

fn handle_selection(
    mut board: ResMut<Board>,
    mut selection: ResMut<Selection>,
    mut commands: Commands,
) {
    while let Some((i, j)) = selection.to_unselect.pop() {
        set_selected(&mut commands, &board[i][j], false);
    }

    if let Some((last_selected, selected)) = selection.last_selected.zip(selection.selected) {
        let di = (last_selected.0 as isize - selected.0 as isize).abs();
        let dj = (last_selected.1 as isize - selected.1 as isize).abs();

        if di + dj == 1
            && let Some((last_selected_tile, selected_tile)) = board[last_selected.0][last_selected.1]
                .tile
                .as_ref()
                .zip(board[selected.0][selected.1].tile.as_ref())
        {
			set_selected(&mut commands, &board[selected.0][selected.1], false);
			selection.last_selected = None;
			selection.selected = None;

            commands
                .entity(last_selected_tile.entity)
                .entry::<TileState>()
                .and_modify(|mut state| *state = TileState::Moving);

            commands
                .entity(last_selected_tile.entity)
                .insert(MovingTo::from(selected));

            commands
                .entity(selected_tile.entity)
                .entry::<TileState>()
                .and_modify(|mut state| *state = TileState::Moving);
            commands
                .entity(selected_tile.entity)
                .insert(MovingTo::from(last_selected));

			let tmp = board[last_selected.0][last_selected.1].tile;
			board[last_selected.0][last_selected.1].tile = board[selected.0][selected.1].tile;
			board[selected.0][selected.1].tile = tmp;
        }
    } else if let Some(cell) = selection.selected.map(|(i, j)| &board[i][j]) {
        set_selected(&mut commands, cell, true);
    }
}

fn set_selected(commands: &mut Commands, cell: &Cell, selected: bool) {
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

    if let Some(tile) = cell.tile.as_ref() {
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
    board: Res<Board>,
    mut selection: ResMut<Selection>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    tile_query: Query<&TileState>,
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

    let bottom_left_border_pos = board.bottom_left() - board.cell_size() / 2.;
    let upper_right_border_pos = board.ceil_right() + board.cell_size() / 2.;

    // println!("Bottom left border: {bottom_left_border_pos}");
    // println!("Upper right border: {upper_right_border_pos}");

    if !(mouse_world_pos.x > bottom_left_border_pos.x
        && mouse_world_pos.x < upper_right_border_pos.x
        && mouse_world_pos.y > bottom_left_border_pos.y
        && mouse_world_pos.y < upper_right_border_pos.y)
    {
        return;
    }

    let clicked_cell_pos = ((mouse_world_pos - bottom_left_border_pos) / board.cell_size()).floor();

    // println!("Clicked sell pos: {clicked_cell_pos}");

    let j = clicked_cell_pos.x as usize;
    let i = clicked_cell_pos.y as usize;

    println!("Defined cell: {}, {}", i, j);

    let selected = selection.selected;

    selection.to_unselect.extend(selected);
    selection.last_selected = selected;

    if Some((i, j)) == selected
        || board[i][j].tile.as_ref().is_none_or(|tile| {
            tile_query
                .get(tile.entity)
                .is_ok_and(|state| *state != TileState::Idle)
        })
    {
        selection.selected = None;
    } else {
        selection.selected = Some((i, j));
    }
}

fn check_board(
    board: Res<Board>,
    mut tiles_to_despawn: ResMut<TilesToDespawn>,
    tile_query: Query<&TileState>,
) {
    let n = board.width().max(board.height());

    for i in 0..n {
        let mut form_by_row = Form::Square;
        let mut form_by_column = Form::Square;
        let mut matched_by_row = vec![];
        let mut matched_by_column = vec![];

        for j in 0..n {
            if let Some(tile) = board
                .get(i)
                .and_then(|row| row.get(j))
                .and_then(|cell| cell.tile.as_ref())
            {
                if tile_query
                    .get(tile.entity)
                    .is_ok_and(|state| *state != TileState::Idle)
                {
                    break;
                }

                if form_by_row != tile.form {
                    if matched_by_row.len() >= 3 {
                        tiles_to_despawn.0.append(&mut matched_by_row);
                    } else {
                        matched_by_row.clear();
                    }
                    form_by_row = tile.form;
                }

                matched_by_row.push((i, j));
            }

            if let Some(tile) = board
                .get(j)
                .and_then(|row| row.get(i))
                .and_then(|cell| cell.tile.as_ref())
            {
                if tile_query
                    .get(tile.entity)
                    .is_ok_and(|state| *state != TileState::Idle)
                {
                    break;
                }

                if form_by_column != tile.form {
                    if matched_by_column.len() >= 3 {
                        tiles_to_despawn.0.append(&mut matched_by_column);
                    } else {
                        matched_by_column.clear();
                    }
                    form_by_column = tile.form;
                }

                matched_by_column.push((j, i));
            }
        }

        if matched_by_row.len() >= 3 {
            tiles_to_despawn.0.append(&mut matched_by_row);
        }
        if matched_by_column.len() >= 3 {
            tiles_to_despawn.0.append(&mut matched_by_column);
        }
    }
}

fn despawn_tiles(
    mut board: ResMut<Board>,
    mut tiles_to_despawn: ResMut<TilesToDespawn>,
    mut commands: Commands,
) {
    while let Some((i, j)) = tiles_to_despawn.0.pop() {
        if let Some(tile) = board[i][j].tile.take() {
            commands.entity(tile.entity).despawn();
        }
    }

    for col_id in 0..board.width() {
        let last_empty = (0..board.height()).find_map(|i| {
            if board[i][col_id].tile.is_none() {
                Some(i)
            } else {
                None
            }
        });

        if let Some(mut last_empty) = last_empty {
            for row_id in last_empty..board.height() {
                if let Some(tile) = board[row_id][col_id].tile.take() {
                    commands
                        .entity(tile.entity)
                        .entry::<TileState>()
                        .and_modify(|mut state| *state = TileState::Moving);

                    commands
                        .entity(tile.entity)
                        .insert(MovingTo(last_empty, col_id));

                    board[last_empty][col_id].tile = Some(tile);
                    last_empty += 1;
                }
            }
        }
    }
}

fn move_tiles(
    time: Res<Time>,
    mut commands: Commands,
    board: Res<Board>,
    query: Query<(Entity, &mut Transform, &mut TileState, &MovingTo)>,
) {
    for (entity, mut transform, mut state, moving) in query {
        let target_coord = board.get_cell_coord(moving.0, moving.1);
		let direction = (target_coord - transform.translation.xy()).signum();

		let diff = TILE_VELOCITY * time.delta_secs();
        transform.translation += direction.extend(0.) * diff;
        if target_coord.abs_diff_eq(transform.translation.xy(), diff) {
            transform.translation.y = target_coord.y;
            *state = TileState::Idle;
            commands.entity(entity).remove::<MovingTo>();
        }
    }
}

fn run_if_has_tiles_to_despawn(tiles_to_despawn: Res<TilesToDespawn>) -> bool {
    !tiles_to_despawn.0.is_empty()
}

#[derive(Resource, Default)]
struct Selection {
    to_unselect: Vec<(usize, usize)>,
    last_selected: Option<(usize, usize)>,
    selected: Option<(usize, usize)>,
}

#[derive(Resource, Default)]
struct TilesToDespawn(Vec<(usize, usize)>);

#[derive(Component)]
struct SelectArea;

#[derive(Component, Default, Eq, PartialEq)]
enum TileState {
    #[default]
    Idle,
    Moving,
}

#[derive(Bundle)]
struct TileBundle {
    transform: Transform,
    visibility: Visibility,
    state: TileState,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct MovingTo(usize, usize);

impl From<(usize, usize)> for MovingTo {
    fn from((i, j): (usize, usize)) -> Self {
        Self(i, j)
    }
}
