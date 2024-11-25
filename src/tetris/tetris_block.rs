use rand::Rng;
use ratatui::{
    style::Color,
    widgets::canvas::{Painter, Shape},
};

#[derive(Debug, Clone)]
pub struct TetrisBlock {
    pub color: Color,
    pub pos: (i32, i32),
    pub pattern: Vec<Vec<bool>>,
}

impl TetrisBlock {
    pub fn new_random() -> Self {
        let mut rng: rand::prelude::ThreadRng = rand::thread_rng();
        let mut pattern: Vec<Vec<bool>> = match rng.gen_range(0..7) as i32 {
            1 => "XX\nXX",
            2 => "XXX\nOXO",
            3 => "OXX\nXXO",
            4 => "XXO\nOXX",
            5 => "XXX\nOOX",
            6 => "OOX\nXXX",
            _ => "XXXX",
        }
        .lines()
        .map(|l| l.chars().map(|c| c == 'X').collect())
        .collect();

        let color = Color::Indexed(rng.gen_range(9..=14));

        for _ in 0..rng.gen_range(0..4) {
            pattern = TetrisBlock::rotate90(&pattern);
        }

        Self {
            color,
            pattern,
            pos: (0, 0),
        }
    }

    pub fn rotate90(pattern: &Vec<Vec<bool>>) -> Vec<Vec<bool>> {
        let width = 1.max(pattern.len());
        let height = 1.max(pattern.iter().map(|x| x.len()).max().unwrap_or(1));

        let mut new_pattern = vec![vec![false; width]; height];
        for w in 0..width {
            for h in 0..height {
                new_pattern[height - h - 1][w] = pattern[w][h];
            }
        }

        new_pattern
    }
}

impl Shape for TetrisBlock {
    fn draw(&self, painter: &mut Painter) {
        for (i, col) in self.pattern.iter().enumerate() {
            for (j, draw) in col.iter().enumerate() {
                if *draw {
                    let x = i as f64 + self.pos.0 as f64;
                    let y = j as f64 + self.pos.1 as f64;
                    if let Some((x, y)) = painter.get_point(x, y) {
                        painter.paint(x, y, self.color);
                    }
                }
            }
        }
    }
}
