use bevy::{math::prelude::*, prelude::*};

mod board;

use board::{Board, BoardIndex, Cell, Form, TILE_VELOCITY, Tile};

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
        .init_resource::<Game>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_click,
                handle_selection,
                move_camera,
                (
                    move_tiles,
                    check_board_for_matching,
                    despawn_tiles.run_if(run_if_has_tiles_to_despawn),
                    spawn_tiles,
                )
                    .chain(),
            ),
        )
        .run()
}

fn move_camera(
    time: Res<Time>,
    buttons: Res<ButtonInput<KeyCode>>,
    mut query: Single<&mut Transform, With<Camera>>,
) {
    let delta = 2. * TILE_VELOCITY * time.delta_secs();

    if buttons.pressed(KeyCode::ArrowUp) {
        query.translation.y += delta;
    }
    if buttons.pressed(KeyCode::ArrowDown) {
        query.translation.y -= delta;
    }
    if buttons.pressed(KeyCode::ArrowLeft) {
        query.translation.x -= delta;
    }
    if buttons.pressed(KeyCode::ArrowRight) {
        query.translation.x += delta;
    }
}

fn setup(
    mut commands: Commands,
    mut board: ResMut<Board>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    game: Res<Game>,
    clear_color: Res<ClearColor>,
) {
    commands.spawn(Camera2d);

    let hidden_board_height = board.height() - board.visible_height();
    let hidden_board_rectangle_mesh = game.rectangle_mesh.clone();
    let background_material = materials.add(clear_color.0);
    let cell_mesh = game.rectangle_mesh.clone();
    let cell_material = materials.add(Color::srgb(0.12, 0.12, 0.18));

    let hidden_board_rectangle_pos = {
        let bottom_left = board.bottom_left();
        let top_right = board.top_right();

        Vec3::new(
            (bottom_left.x + top_right.x) / 2.,
            hidden_board_height as f32 * 0.5 * board.cell_size()
                + top_right.y
                + board.cell_size() * 0.5,
            100.,
        )
    };
    commands.spawn((
        Transform::from_translation(hidden_board_rectangle_pos).with_scale(Vec3::new(
            board.cell_size() * board.width() as f32,
            board.cell_size() * hidden_board_height as f32,
            0.,
        )),
        Mesh2d(hidden_board_rectangle_mesh),
        MeshMaterial2d(background_material),
    ));

    for i in 0..board.height() {
        let mut row = Vec::with_capacity(board.width());
        for j in 0..board.width() {
            let form = rand::random();
            let (form_mesh, form_material) = match form {
                Form::Circle => (game.circle_mesh.clone(), game.circle_material.clone()),
                Form::Square => (game.square_mesh.clone(), game.square_material.clone()),
                Form::Triangle => (game.triangle_mesh.clone(), game.triangle_material.clone()),
            };

            let Vec2 { x, y } = board.get_cell_coord((i, j));

            commands.spawn((
                Mesh2d(cell_mesh.clone()),
                MeshMaterial2d(cell_material.clone()),
                Transform::from_xyz(x, y, 0.).with_scale(Vec3::new(
                    board.cell_size() - board.border_width(),
                    board.cell_size() - board.border_width(),
                    0.,
                )),
            ));

            let select_area_entity = commands
                .spawn((
                    Mesh2d(game.rectangle_mesh.clone()),
                    MeshMaterial2d(game.select_area_material.clone()),
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
    while let Some(idx) = selection.to_unselect.pop() {
        set_selected(&mut commands, &board[idx], false);
    }

    if let Some((last_selected, selected)) = selection.last_selected.zip(selection.selected) {
        let di = (last_selected.row_id() as isize - selected.row_id() as isize).abs();
        let dj = (last_selected.col_id() as isize - selected.col_id() as isize).abs();

        if di + dj == 1
            && let Some((last_selected_tile, selected_tile)) = board[last_selected]
                .tile
                .as_ref()
                .zip(board[selected].tile.as_ref())
        {
            set_selected(&mut commands, &board[selected], false);
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

            let tmp = board[last_selected].tile;
            board[last_selected].tile = board[selected].tile;
            board[selected].tile = tmp;
        }
    } else if let Some(cell) = selection.selected.map(|idx| &board[idx]) {
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
    let upper_right_border_pos = board.top_right() + board.cell_size() / 2.;

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

    if Some((i, j).into()) == selected
        || board[i][j].tile.as_ref().is_none_or(|tile| {
            tile_query
                .get(tile.entity)
                .is_ok_and(|state| *state != TileState::Idle)
        })
    {
        selection.selected = None;
    } else {
        selection.selected = Some((i, j).into());
    }
}

fn check_board_for_matching(
    board: Res<Board>,
    mut tiles_to_despawn: ResMut<TilesToDespawn>,
    tile_query: Query<&TileState>,
) {
    let n = board.width().max(board.visible_height());

    for i in 0..n {
        let mut form_by_row = Form::Square;
        let mut form_by_column = Form::Square;
        let mut matched_by_row = vec![];
        let mut matched_by_column = vec![];

        for j in 0..n {
            if let Some(tile) = board
                .get_row(i)
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

                matched_by_row.push((i, j).into());
            }

            if let Some(tile) = board
                .get_row(j)
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

                matched_by_column.push((j, i).into());
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
    while let Some(index) = tiles_to_despawn.0.pop() {
        if let Some(tile) = board[index].tile.take() {
            commands.entity(tile.entity).despawn();
        }
    }

    for col_id in 0..board.width() {
        let last_empty = (0..board.visible_height()).find_map(|i| {
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
                        .insert(MovingTo::from((last_empty, col_id)));

                    board[last_empty][col_id].tile = Some(tile);
                    last_empty += 1;
                }
            }
        }
    }
}

fn spawn_tiles(mut board: ResMut<Board>, mut commands: Commands, game: Res<Game>) {
    for col_id in 0..board.width() {
        for row_id in board.visible_height()..board.height() {
            let form = rand::random();
            let (form_mesh, form_material) = match form {
                Form::Circle => (game.circle_mesh.clone(), game.circle_material.clone()),
                Form::Square => (game.square_mesh.clone(), game.square_material.clone()),
                Form::Triangle => (game.triangle_mesh.clone(), game.triangle_material.clone()),
            };
            let Vec2 { x, y } = board.get_cell_coord((row_id, col_id));

            if board[row_id][col_id].tile.is_none() {
                let select_area_entity = commands
                    .spawn((
                        Mesh2d(game.rectangle_mesh.clone()),
                        MeshMaterial2d(game.select_area_material.clone()),
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

                board[row_id][col_id].tile = Some(Tile {
                    form,
                    entity: tile_entity,
                    select_area_entity,
                });
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
    for (entity, mut transform, mut state, moving_to) in query {
        let target_coord = board.get_cell_coord(moving_to);
        let direction = (target_coord - transform.translation.xy()).signum();

        let diff = TILE_VELOCITY * time.delta_secs();
        transform.translation += direction.extend(0.) * diff;
        if target_coord.abs_diff_eq(transform.translation.xy(), diff) {
            transform.translation.x = target_coord.x;
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
    to_unselect: Vec<BoardIndex>,
    last_selected: Option<BoardIndex>,
    selected: Option<BoardIndex>,
}

#[derive(Resource, Default)]
struct TilesToDespawn(Vec<BoardIndex>);

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
struct MovingTo(BoardIndex);

impl From<BoardIndex> for MovingTo {
    fn from(idx: BoardIndex) -> Self {
        Self(idx)
    }
}

impl From<(usize, usize)> for MovingTo {
    fn from(idx: (usize, usize)) -> Self {
        Self(idx.into())
    }
}

impl<'a> Into<BoardIndex> for &'a MovingTo {
    fn into(self) -> BoardIndex {
        self.0
    }
}

#[derive(Resource)]
struct Game {
    rectangle_mesh: Handle<Mesh>,
    select_area_material: Handle<ColorMaterial>,
    circle_mesh: Handle<Mesh>,
    circle_material: Handle<ColorMaterial>,
    square_mesh: Handle<Mesh>,
    square_material: Handle<ColorMaterial>,
    triangle_mesh: Handle<Mesh>,
    triangle_material: Handle<ColorMaterial>,
}

impl FromWorld for Game {
    fn from_world(world: &mut World) -> Self {
        let rectangle_mesh;
        let select_area_material;
        let circle_mesh;
        let circle_material;
        let square_mesh;
        let square_material;
        let triangle_mesh;
        let triangle_material;

        {
            let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();

            rectangle_mesh = meshes.add(Rectangle::default());
            circle_mesh = meshes.add(Circle::new(0.4));
            square_mesh = meshes.add(Rectangle::from_size(Vec2::splat(0.8)));
            triangle_mesh = meshes.add(Triangle2d::new(
                Vec2::new(0., 0.4),
                Vec2::new(-0.4, -0.4),
                Vec2::new(0.4, -0.4),
            ));
        }

        {
            let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();

			// @FIXME Вернуть альфа канал 0.75
    		// С параметром альфа канала выглядит как будто область выделения находится над фигурой.
            select_area_material =
                materials.add(Color::srgba(184. / 255., 134. / 255., 11. / 255., 1.));
            circle_material = materials.add(Color::srgb(175. / 255., 43. / 255., 30. / 255.));
            square_material = materials.add(Color::srgb(71. / 255., 132. / 255., 48. / 255.));
            triangle_material = materials.add(Color::srgb(27. / 255., 85. / 255., 131. / 255.));
        }

        Self {
            rectangle_mesh,
            select_area_material,
            circle_mesh,
            circle_material,
            square_mesh,
            square_material,
            triangle_mesh,
            triangle_material,
        }
    }
}
