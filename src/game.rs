use crate::GameState;
use crate::{requested_direction::RequestedDirection, types::Direction, GameOverReason, Point};
use rand::Rng;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) type SharedGame = Arc<Mutex<Game>>;

struct GameStateCache {
    last_returned_game_state_version: usize,
    last_returned_game_state: Option<GameState>,
}

pub(crate) struct Game {
    height: i32,
    width: i32,
    food: Point,
    rusty: Body,
    game_over: Option<GameOverReason>,
    epoch: usize,
    requested_directions: RequestedDirection,
    users: Mutex<HashSet<String>>,
    game_state_version: usize,
    game_state_cache: GameStateCache,
}

impl Game {
    pub(crate) fn new(height: i32, width: i32) -> Self {
        Self {
            height,
            width,
            food: Point::new(width / 2, height / 2),
            rusty: Body::new(height / 2),
            game_over: None,
            epoch: 0,
            users: Mutex::new(HashSet::new()),
            requested_directions: RequestedDirection::new(),
            game_state_version: 1,
            game_state_cache: GameStateCache {
                last_returned_game_state_version: 0,
                last_returned_game_state: None,
            },
        }
    }

    pub(crate) fn get_dimensions(&self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }

    pub(crate) async fn add_user(&self, user_id: String) -> bool {
        self.users.lock().await.insert(user_id)
    }

    pub(crate) async fn user_has_joined_game(&self, user_id: String) -> bool {
        self.users.lock().await.contains(&user_id)
    }

    pub(crate) async fn add_user_direction(&self, user_id: String, direction: Direction) {
        self.requested_directions
            .add_direction(&user_id, direction)
            .await
    }

    pub(crate) async fn tick(&mut self, max_spaces: usize) -> Option<GameOverReason> {
        self.epoch += 1;
        self.game_state_version += 1;
        // Check if game previously failed
        if self.game_over.is_some() {
            return self.game_over.clone();
        }

        // Get user selected direction if available, else continue in same direction
        let direction = match self.requested_directions.calculate_direction().await {
            Some(user_selected_direction) => user_selected_direction,
            None => self.rusty.direction,
        };

        // move rusty, rusty will grow if it overlaps with food
        let did_grow = self.rusty.move_in_direction(direction, self.food);

        // Check if the player has won
        if self.rusty.body.len() == max_spaces {
            self.game_over = Some(GameOverReason::Winner);
        }

        // Check out of bounds
        let head_position = self.rusty.head();
        if head_position.x < 0
            || head_position.y < 0
            || head_position.x >= self.width
            || head_position.y >= self.height
        {
            self.game_over = Some(GameOverReason::OutOfBounds);
        }

        // Check if head overlaps the body
        if self.rusty.is_collide_with_self() {
            self.game_over = Some(GameOverReason::CollideWithSelf);
        }

        if did_grow {
            self.generate_new_food();
        }
        self.game_over.clone()
    }

    fn generate_new_food(&mut self) {
        // Pick a new food position at random that doesn't overlap rusty
        let mut new_food_point = Self::random_point(self.width, self.height);
        let mut retries = 0;

        while self.rusty.body.contains(&new_food_point) {
            new_food_point = Self::random_point(self.width, self.height);
            retries += 1;

            // Randomly selecting a new food position should be good enough but a different
            // solution should be used if it takes too many attempts
            if retries > self.height * self.width * 2 {
                panic!("Randomly selecting a new food position is taking too long!");
            }
        }

        self.food = new_food_point;
    }

    fn random_point(max_x: i32, max_y: i32) -> Point {
        Point::new(
            rand::thread_rng().gen_range(0..max_x),
            rand::thread_rng().gen_range(0..max_y),
        )
    }

    pub(crate) async fn into_game_state(&self) -> GameState {
        // If there have been no updates to the Game, return the previous GameState
        let cache = &self.game_state_cache;
        if self.game_state_version <= cache.last_returned_game_state_version {
            return cache.last_returned_game_state.as_ref().unwrap().clone();
        }

        let game_over = self.game_over.clone();

        let direction = match self.requested_directions.calculate_direction().await {
            Some(top_direction) => top_direction,
            None => self.rusty.direction,
        };

        GameState {
            tick: self.epoch,
            game_over_reason: game_over,
            direction: direction,
            body: self.rusty.body(),
            num_users: self.requested_directions.len().await.try_into().unwrap(),
            height: self.height,
            width: self.width,
            food: self.food,
        }
    }
}

struct Body {
    direction: Direction,
    body: VecDeque<Point>,
}

impl Body {
    pub fn new(starting_y: i32) -> Self {
        Self {
            direction: Direction::East,
            body: VecDeque::from([
                Point::new(2, starting_y),
                Point::new(1, starting_y),
                Point::new(0, starting_y),
            ]),
        }
    }

    /// Moves the body in the specified direction. If the new head position doesn't
    /// overlap with food, the tail is removed (doesn't grow).
    ///
    /// Returns true if the new head position overlaps with the food position.
    pub(crate) fn move_in_direction(&mut self, direction: Direction, food: Point) -> bool {
        self.direction = direction;
        let new_point = self.head().add_direction(&self.direction);
        self.body.push_front(new_point);
        let food_overlaps = new_point == food;

        // Remove the tail (don't grow) if food doesn't overlap
        if !food_overlaps {
            self.body.pop_back();
        }

        food_overlaps
    }

    pub(crate) fn head(&self) -> Point {
        self.body.front().expect("Body should not be empty").clone()
    }

    pub(crate) fn is_collide_with_self(&self) -> bool {
        let mut iterator = self.body.iter();
        let head_node = iterator.next().unwrap();

        while let Some(point) = iterator.next() {
            if point == head_node {
                return true;
            }
        }

        false
    }

    pub(crate) fn body(&self) -> Vec<Point> {
        Vec::from(self.body.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Body, Game};
    use crate::output::print_world;
    use crate::types::Direction;
    use crate::Point;
    use std::collections::{LinkedList, VecDeque};

    const HEIGHT: i32 = 4;

    #[test]
    fn rusty_new() {
        let rusty = Body::new(HEIGHT / 2);

        let expected_body = VecDeque::from([
            Point::new(2, HEIGHT / 2),
            Point::new(1, HEIGHT / 2),
            Point::new(0, HEIGHT / 2),
        ]);

        assert_eq!(rusty.body, expected_body);
    }

    #[tokio::test]
    async fn into_game_state() {
        let game = Game::new(HEIGHT, HEIGHT);
        let game_state = game.into_game_state().await;

        let expected_body = vec![
            Point::new(2, HEIGHT / 2),
            Point::new(1, HEIGHT / 2),
            Point::new(0, HEIGHT / 2),
        ];

        print_world(&game_state);
        assert_eq!(game_state.body, expected_body);
    }

    #[test]
    fn move_in_direction() {
        let mut rusty = Body::new(HEIGHT / 2);
        let food: Point = Point::new(4, HEIGHT / 2);

        let mut expected_body = VecDeque::new();
        for n in 1..4 {
            expected_body.push_front(Point::new(n, HEIGHT / 2))
        }

        let did_grow = rusty.move_in_direction(Direction::East, food);
        assert_eq!(did_grow, false);
        assert_eq!(rusty.body, expected_body);
    }

    #[test]
    fn move_in_direction_and_grow() {
        let mut rusty = Body::new(HEIGHT / 2);
        let food: Point = Point::new(3, HEIGHT / 2);

        let mut expected_body = VecDeque::new();
        for n in 0..4 {
            expected_body.push_front(Point::new(n, HEIGHT / 2))
        }

        let did_grow = rusty.move_in_direction(Direction::East, food);
        assert_eq!(did_grow, true);
        assert_eq!(rusty.body, expected_body);
    }

    #[test]
    fn collide_with_self() {
        let mut rusty = Body::new(HEIGHT / 2);
        let food: Point = Point::new(5, HEIGHT / 2);

        let mut expected_body = LinkedList::new();
        for n in 0..4 {
            expected_body.push_front(Point::new(n, HEIGHT / 2))
        }

        // Grow to a length of 5 to be large enough to hit self
        rusty.move_in_direction(Direction::East, Point::new(3, HEIGHT / 2));
        rusty.move_in_direction(Direction::East, Point::new(4, HEIGHT / 2));
        assert_eq!(rusty.is_collide_with_self(), false);

        // Move in a circle to hit self
        rusty.move_in_direction(Direction::South, food);
        rusty.move_in_direction(Direction::West, food);
        rusty.move_in_direction(Direction::North, food);
        assert_eq!(rusty.is_collide_with_self(), true);
        assert_eq!(rusty.body.len(), 5);
    }
}
