use std::ops::{Index, IndexMut};

use bevy::prelude::{Entity, Vec2};
use rand::{
    Rng,
    distr::{Distribution, StandardUniform},
};

const BOARD_WIDTH: usize = 10;
const BOARD_HEIGHT: usize = 10;
const BOARD_TILE_SIZE: f32 = 75.0;
const CELL_BORDER_WIDTH: f32 = 2.0;

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
            -(BOARD_HEIGHT as f32 * (BOARD_TILE_SIZE + CELL_BORDER_WIDTH)) / 2.,
        )
    }

    pub fn ceil_right(&self) -> Vec2 {
        self.bottom_left()
            + Vec2::new((BOARD_HEIGHT - 1) as f32, (BOARD_WIDTH - 1) as f32)
                * Vec2::splat(BOARD_TILE_SIZE as f32)
    }

    pub fn push_row(&mut self, row: Vec<Cell>) {
        assert_eq!(
            row.len(),
            self.width(),
            "New row length is greater than board"
        );

        self.0.push(row);
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

#[derive(Clone, Copy)]
pub struct Cell {
    pub form: Form,
    pub entity: Entity,
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

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Form {
    Circle,
    Square,
    Triangle,
}

#[derive(Clone, Copy)]
pub struct Tile {
    pub entity: Entity,
    pub select_area_entity: Entity,
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