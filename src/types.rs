#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug)]
pub(crate) enum GameError {
    InvalidUser,
    InvalidGame,
}
