use std::ops::{Index, IndexMut};

use bevy::prelude::{Entity, Resource, Vec2};
use rand::{
    Rng,
    distr::{Distribution, StandardUniform},
};

const BOARD_WIDTH: usize = 10;
const BOARD_VISIBLE_HEIGHT: usize = 10;
const BOARD_HEIGHT: usize = BOARD_VISIBLE_HEIGHT * 2;
const BOARD_TILE_SIZE: f32 = 75.0;
const CELL_BORDER_WIDTH: f32 = 2.0;

pub const TILE_VELOCITY: f32 = 200.;

#[derive(Resource)]
pub struct Board(Vec<Vec<Cell>>);

impl Board {
    pub fn new() -> Self {
        Self(Vec::with_capacity(BOARD_HEIGHT))
    }

    pub fn width(&self) -> usize {
        BOARD_WIDTH
    }

    pub fn height(&self) -> usize {
        BOARD_HEIGHT
    }

    pub fn visible_height(&self) -> usize {
        BOARD_VISIBLE_HEIGHT
    }

    pub fn cell_size(&self) -> f32 {
        self.tile_size() + CELL_BORDER_WIDTH
    }

    pub fn tile_size(&self) -> f32 {
        BOARD_TILE_SIZE
    }

    pub fn border_width(&self) -> f32 {
        CELL_BORDER_WIDTH
    }

    pub fn bottom_left(&self) -> Vec2 {
        Vec2::new(
            -(BOARD_WIDTH as f32 * (BOARD_TILE_SIZE + CELL_BORDER_WIDTH)) / 2.,
            -(BOARD_VISIBLE_HEIGHT as f32 * (BOARD_TILE_SIZE + CELL_BORDER_WIDTH)) / 2.,
        )
    }

    pub fn top_right(&self) -> Vec2 {
        self.bottom_left()
            + Vec2::new((BOARD_VISIBLE_HEIGHT - 1) as f32, (BOARD_WIDTH - 1) as f32)
                * self.cell_size()
    }

    pub fn get_cell_coord(&self, idx: impl Into<BoardIndex>) -> Vec2 {
        let idx = idx.into();

        self.bottom_left()
            + Vec2::new(
                idx.1 as f32 * self.cell_size(),
                idx.0 as f32 * self.cell_size(),
            )
    }

    pub fn push_row(&mut self, row: Vec<Cell>) {
        assert_eq!(
            row.len(),
            self.width(),
            "New row length is greater than board"
        );

        self.0.push(row);
    }

    pub fn get_row(&self, idx: usize) -> Option<&Vec<Cell>> {
        self.0.get(idx)
    }
}

impl<'a> IntoIterator for &'a Board {
    type Item = &'a Vec<Cell>;
    type IntoIter = std::slice::Iter<'a, Vec<Cell>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<usize> for Board {
    type Output = Vec<Cell>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Board {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Index<BoardIndex> for Board {
    type Output = Cell;

    fn index(&self, BoardIndex(row_id, col_id): BoardIndex) -> &Self::Output {
        &self.0[row_id][col_id]
    }
}

impl IndexMut<BoardIndex> for Board {
    fn index_mut(&mut self, BoardIndex(row_id, col_id): BoardIndex) -> &mut Self::Output {
        &mut self.0[row_id][col_id]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BoardIndex(usize, usize);

impl BoardIndex {
    pub fn row_id(&self) -> usize {
        self.0
    }

    pub fn col_id(&self) -> usize {
        self.1
    }
}

impl From<(usize, usize)> for BoardIndex {
    fn from((row_id, col_id): (usize, usize)) -> Self {
        Self(row_id, col_id)
    }
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub tile: Option<Tile>,
}

impl Cell {
    pub fn size(&self) -> f32 {
        self.tile_size() + CELL_BORDER_WIDTH
    }

    pub fn tile_size(&self) -> f32 {
        BOARD_TILE_SIZE
    }
}

#[derive(Clone, Copy)]
pub struct Tile {
    pub form: Form,
    pub entity: Entity,
    pub select_area_entity: Entity,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Form {
    Circle,
    Square,
    Triangle,
}

impl Distribution<Form> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Form {
        match rng.random_range(0..=2) {
            0 => Form::Circle,
            1 => Form::Square,
            _ => Form::Triangle,
        }
    }
}
