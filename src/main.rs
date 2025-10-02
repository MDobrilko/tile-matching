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
        .init_resource::<ScoreStorage>()
        .add_systems(Startup, (setup, setup_score))
        .add_systems(
            Update,
            (
                handle_click,
                handle_selection,
                move_camera,
                display_score,
                (
                    move_tiles,
                    check_swapped_for_matching,
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

fn setup_score(mut commands: Commands) {
    commands.spawn((
        Text::new("Score:"),
        TextFont {
            font_size: 25.,
            ..Default::default()
        },
        TextColor(Color::srgb(0.5, 0.5, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(5),
            left: px(5),
            ..Default::default()
        },
        ScoreDisplay,
    ));
}

#[derive(Component)]
struct ScoreDisplay;

#[derive(Resource, Default)]
struct ScoreStorage(usize);

fn display_score(score: Res<ScoreStorage>, mut display: Single<&mut Text, With<ScoreDisplay>>) {
    display.0 = format!("Score: {}", score.0);
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
                Form::Rhombus => (game.rhombus_mesh.clone(), game.rhombus_material.clone()),
                Form::Annulus => (game.annulus_mesh.clone(), game.annulus_material.clone()),
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
    tile_query: Query<&TileState>,
) {
    while let Some(idx) = selection.to_unselect.pop() {
        set_selected(&mut commands, &board[idx], false);
    }

    if selection
        .selected
        .and_then(|idx| board[idx].tile.as_ref())
        .is_none_or(|tile| {
            tile_query
                .get(tile.entity)
                .ok()
                .is_none_or(|state| *state != TileState::Idle)
        })
    {
        selection.selected = None;
    } else if let Some((last_selected, selected)) = selection.last_selected.zip(selection.selected)
    {
        let di = (last_selected.row_id() as isize - selected.row_id() as isize).abs();
        let dj = (last_selected.col_id() as isize - selected.col_id() as isize).abs();

        if di + dj == 1
            && board[last_selected].tile.is_some()
            && let Some(selected_tile) = board[selected].tile
        {
            set_selected(&mut commands, &board[selected], false);
            selection.last_selected = None;
            selection.selected = None;

            swap_tiles(&mut board, &mut commands, last_selected, selected);
            commands
                .entity(selected_tile.entity)
                .insert(CheckMatchesOrSwap([last_selected, selected]));
        } else {
            set_selected(&mut commands, &board[last_selected], false);
            set_selected(&mut commands, &board[selected], true);
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

    let last_selected = selection.selected;

    selection.to_unselect.extend(last_selected);
    selection.last_selected = last_selected;
    selection.selected = Some((i, j).into());
}

fn check_swapped_for_matching(
    mut commands: Commands,
    mut board: ResMut<Board>,
    swapped_tiles: Query<(Entity, &CheckMatchesOrSwap), Without<Moving>>,
) {
    for (entity, swapped) in swapped_tiles {
        let mut has_matches = false;
        'search_matches: for (idx, check_tile) in swapped
            .0
            .iter()
            .filter_map(|idx| board[*idx].tile.as_ref().map(|tile| (*idx, tile)))
        {
            let match_range = |idx: usize, max_idx: usize| {
                ((idx as isize - 2).max(0) as usize)..=((idx + 2).min(max_idx - 1))
            };
            let mut matched = 0;

            for row_id in match_range(idx.row_id(), board.visible_height()) {
                if board[row_id][idx.col_id()]
                    .tile
                    .as_ref()
                    .is_some_and(|tile| tile.form == check_tile.form)
                {
                    matched += 1;
                    has_matches = matched >= 3;

                    if has_matches {
                        break 'search_matches;
                    }
                } else {
                    matched = 0;
                }
            }

            for col_id in match_range(idx.col_id(), board.width()) {
                if board[idx.row_id()][col_id]
                    .tile
                    .as_ref()
                    .is_some_and(|tile| tile.form == check_tile.form)
                {
                    matched += 1;
                    has_matches = matched >= 3;
                    if has_matches {
                        break 'search_matches;
                    }
                } else {
                    matched = 0;
                }
            }
        }

        commands.entity(entity).remove::<CheckMatchesOrSwap>();

        if !has_matches {
            let from = swapped.0[0];
            let to = swapped.0[1];

            swap_tiles(&mut board, &mut commands, from, to);
        }
    }
}

fn swap_tiles(board: &mut Board, commands: &mut Commands, idx1: BoardIndex, idx2: BoardIndex) {
    let Some(tile1) = board[idx1].tile.as_ref() else {
        return;
    };
    let Some(tile2) = board[idx2].tile.as_ref() else {
        return;
    };

    commands
        .entity(tile1.entity)
        .entry::<TileState>()
        .and_modify(|mut state| *state = TileState::Moving);

    commands
        .entity(tile2.entity)
        .entry::<TileState>()
        .and_modify(|mut state| *state = TileState::Moving);

    commands.entity(tile1.entity).insert(Moving {
        from: idx1,
        to: idx2,
    });
    commands.entity(tile2.entity).insert(Moving {
        from: idx2,
        to: idx1,
    });

    let tmp = board[idx1].tile;
    board[idx1].tile = board[idx2].tile;
    board[idx2].tile = tmp;
}

fn check_board_for_matching(
    mut score: ResMut<ScoreStorage>,
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
                        score.0 += 10 * matched_by_row.len();
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
                        score.0 += 10 * matched_by_column.len();
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
            score.0 += 10 * matched_by_row.len();
            tiles_to_despawn.0.append(&mut matched_by_row);
        }
        if matched_by_column.len() >= 3 {
            score.0 += 10 * matched_by_column.len();
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

                    commands.entity(tile.entity).insert(Moving {
                        from: (row_id, col_id).into(),
                        to: (last_empty, col_id).into(),
                    });

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
                Form::Rhombus => (game.rhombus_mesh.clone(), game.rhombus_material.clone()),
                Form::Annulus => (game.annulus_mesh.clone(), game.annulus_material.clone()),
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
    query: Query<(Entity, &mut Transform, &mut TileState, &Moving)>,
) {
    for (entity, mut transform, mut state, moving) in query {
        let target_coord = board.get_cell_coord(moving.to);

        let dx = (moving.to.col_id() as isize - moving.from.col_id() as isize).signum() as f32;
        let dy = (moving.to.row_id() as isize - moving.from.row_id() as isize).signum() as f32;
        let direction = Vec2::new(dx, dy);

        let delta = TILE_VELOCITY * time.delta_secs();
        transform.translation += direction.extend(0.) * delta;
        if target_coord.abs_diff_eq(transform.translation.xy(), delta) {
            transform.translation.x = target_coord.x;
            transform.translation.y = target_coord.y;
            *state = TileState::Idle;
            commands.entity(entity).remove::<Moving>();
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
struct Moving {
    from: BoardIndex,
    to: BoardIndex,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct CheckMatchesOrSwap([BoardIndex; 2]);

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
    rhombus_mesh: Handle<Mesh>,
    rhombus_material: Handle<ColorMaterial>,
    annulus_mesh: Handle<Mesh>,
    annulus_material: Handle<ColorMaterial>,
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
        let rhombus_mesh;
        let rhombus_material;
        let annulus_mesh;
        let annulus_material;

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
            rhombus_mesh = meshes.add(Rhombus::new(0.8, 0.8));
            annulus_mesh = meshes.add(Annulus::new(0.3, 0.4));
        }

        {
            let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();

            // @FIXME Вернуть альфа канал 0.75
            // С параметром альфа канала выглядит как будто область выделения находится над фигурой.
            select_area_material =
                materials.add(Color::srgba(165. / 255., 187. / 255., 192. / 255., 1.));
            circle_material = materials.add(Color::srgb(175. / 255., 43. / 255., 30. / 255.));
            square_material = materials.add(Color::srgb(71. / 255., 132. / 255., 48. / 255.));
            triangle_material = materials.add(Color::srgb(27. / 255., 85. / 255., 131. / 255.));
            rhombus_material = materials.add(Color::srgb(229. / 255., 132. / 255., 38. / 255.));
            annulus_material = materials.add(Color::srgb(217. / 255., 119. / 255., 169. / 255.));
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
            rhombus_mesh,
            rhombus_material,
            annulus_mesh,
            annulus_material,
        }
    }
}
