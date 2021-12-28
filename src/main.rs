use std::collections::LinkedList;
use rand::{Rng};

fn main() {
    let size = 10;
    let mut game = Game::new(size, size);
    
    let game_over_reason = game.start();
    match game_over_reason {
        GameOverReason::CollideWithSelf => println!("Game Over: You collided with your own body"),
        GameOverReason::OutOfBounds => println!("Game Over: You went out of bounds")
    }
    println!("Score: {}", game.rusty.body.len());
    
}

struct Game {
    height: i32,
    width: i32,
    food: Point,
    rusty: Rusty
}

impl Game {
    pub fn new(height: i32, width: i32) -> Self {
        Self {
            height,
            width,
            food: Point::new(width/2, height/2),
            rusty: Rusty::new(height/2)
        }
    }

    pub fn start(&mut self) -> GameOverReason {
        loop {
            match self.tick() {
                Some(game_over_reason) => return game_over_reason,
                None => {}
            }
        }
        
    }

    fn tick(&mut self) -> Option<GameOverReason> {

        let direction = self.get_direction();

        // move rusty, rusty will grow if it overlaps with food
        let did_grow = self.rusty.move_in_direction(direction, self.food);
        
        // Check out of bounds
        let head_position = self.rusty.head();
        if head_position.x < 0 || head_position.y < 0 ||
            head_position.x >= self.width || head_position.y >= self.height 
        {
            return Some(GameOverReason::OutOfBounds);
        }

        // Check if head overlaps the body
        if self.rusty.collide_with_self() {
            return Some(GameOverReason::CollideWithSelf)
        }
        
        if did_grow {
            self.generate_new_food();
        }
        None
    }

    fn get_direction(&self) -> Direction {
        Direction::North
    }

    fn generate_new_food(&mut self) {
        // Pick a new food position at random that doesn't overlap rusty
        let mut rand_x = rand::thread_rng().gen_range(0..self.width);
        let mut rand_y = rand::thread_rng().gen_range(0..self.height);
        let mut new_food_point = Point::new(rand_x, rand_y);
        let mut retries = 0;

        while self.rusty.body.contains(&new_food_point) {
            rand_x = rand::thread_rng().gen_range(0..self.width);
            rand_y = rand::thread_rng().gen_range(0..self.height);
            new_food_point = Point::new(rand_x, rand_y);
            retries += 1;

            // Randomly selecting a new food position should be good enough but a different
            // solution should be used if it takes too many attempts
            if retries > self.height * self.width * 2 {
                panic!("Randomly selecting a new food position is taking too many retries!");
            }
        }

        self.food = new_food_point;
    }
}

enum GameOverReason {
    OutOfBounds,
    CollideWithSelf
}

struct Rusty {
    direction: Direction,
    body: LinkedList<Point>,
}

impl Rusty {
    pub fn new(starting_y: i32) -> Self {
        Self {
            direction: Direction::East,
            body: LinkedList::from([
                Point::new(2, starting_y),
                Point::new(1, starting_y),
                Point::new(0, starting_y),
            ])
        }
    }

    pub fn move_in_direction(&mut self, direction: Direction, food: Point) -> bool {
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

    pub fn head(&self) -> Point {
        self.body.front().expect("Rusty should not be empty").clone()
    }

    pub fn collide_with_self(&self) -> bool {
        let mut iterator = self.body.iter();
        let head_node = iterator.next().unwrap();
        
        while let Some(point) = iterator.next() {
            if point == head_node {
                return true;
            }
        }

        false
    }
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y
        }
    }

    pub fn add_direction(&self, direction: &Direction) -> Point {
        match direction {
            Direction::North => Point::new(self.x, self.y - 1),
            Direction::South => Point::new(self.x, self.y + 1),
            Direction::East => Point::new(self.x + 1, self.y),
            Direction::West => Point::new(self.x - 1, self.y),
        }

    }
}

#[derive(Debug)]
enum Direction {
    North,
    South,
    East,
    West
}

#[cfg(test)]
mod tests {
    use std::collections::LinkedList;

    use crate::Direction;
    use crate::Point;
    use crate::Rusty;

    const HEIGHT: i32 = 4;

    #[test]
    fn rusty_new() {
        let rusty = Rusty::new(HEIGHT/2);

        let expected_body = LinkedList::from([
            Point::new(2, HEIGHT/2),
            Point::new(1, HEIGHT/2),
            Point::new(0, HEIGHT/2),
        ]);

        assert_eq!(rusty.body, expected_body);
    }

    #[test]
    fn move_in_direction() {
        let mut rusty = Rusty::new(HEIGHT/2);
        let food: Point = Point::new(4, HEIGHT/2);

        let mut expected_body = LinkedList::new();
        for n in 1..4 {
            expected_body.push_front(Point::new(n, HEIGHT/2))
        }

        let did_grow = rusty.move_in_direction(Direction::East, food);
        assert_eq!(did_grow, false);
        assert_eq!(rusty.body, expected_body);
    }

    #[test]
    fn move_in_direction_and_grow() {
        let mut rusty = Rusty::new(HEIGHT/2);
        let food: Point = Point::new(3, HEIGHT/2);

        let mut expected_body = LinkedList::new();
        for n in 0..4 {
            expected_body.push_front(Point::new(n, HEIGHT/2))
        }

        let did_grow = rusty.move_in_direction(Direction::East, food);
        assert_eq!(did_grow, true);
        assert_eq!(rusty.body, expected_body);
    }

    #[test]
    fn collide_with_self() {
        let mut rusty = Rusty::new(HEIGHT/2);
        let food: Point = Point::new(5, HEIGHT/2);

        let mut expected_body = LinkedList::new();
        for n in 0..4 {
            expected_body.push_front(Point::new(n, HEIGHT/2))
        }

        // Grow to a length of 5 to be large enough to hit self
        rusty.move_in_direction(Direction::East, Point::new(3, HEIGHT/2));
        rusty.move_in_direction(Direction::East, Point::new(4, HEIGHT/2));
        assert_eq!(rusty.collide_with_self(), false);

        // Move in a circle to hit self
        rusty.move_in_direction(Direction::South, food);
        rusty.move_in_direction(Direction::West, food);
        rusty.move_in_direction(Direction::North, food);
        assert_eq!(rusty.collide_with_self(), true);
        assert_eq!(rusty.body.len(), 5);
    }

    #[test]
    fn add_direction() {
        let test_values: [(Direction, Point); 4] = [
            (Direction::North, Point::new(0, -1)),
            (Direction::South, Point::new(0, 1)),
            (Direction::East, Point::new(1, 0)),
            (Direction::West, Point::new(-1, 0)),
        ];

        for test_value in test_values {
            let (direction, expected_point) = test_value;
            assert_eq!(Point::new(0, 0).add_direction(&direction), expected_point);
        }
    }
}