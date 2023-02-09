use crate::GameState;

pub fn print_world(game_state: &GameState) {
    let food = game_state.food;
    let mut sorted_body = game_state.body.clone();
    // sort by row, then by column
    sorted_body.sort_by(|a, b| {
        if a.y == b.y {
            a.x.cmp(&b.x)
        } else {
            a.y.cmp(&b.y)
        }
    });

    let empty_space_separator = "-".to_string();

    let mut point_inter = sorted_body.iter();
    let mut current_point = point_inter.next();
    println!(
        "Head Point: {:?} Direction: {:?}",
        game_state.body.first().unwrap(),
        game_state.direction
    );
    println!("Game Over Reason: {:?}", game_state.game_over_reason);
    println!("Food: {:?}", game_state.food);
    for row_index in 0..game_state.height {
        print!("{}\t|", row_index);
        // ignore points that are out of bounds.
        // skips points when y < 0 (starting row.)
        // Skips if y >= current row and x < 0
        // If x > width, then it will be skipped on the next row iteration (then y < row)
        while let Some(point) = current_point {
            if point.y < row_index || point.x < 0 {
                current_point = point_inter.next();
            } else {
                break;
            }
        }
        for column_index in 0..game_state.width {
            let mut grid_point_output = empty_space_separator.clone();
            if food.y == row_index && food.x == column_index {
                grid_point_output = "*".to_string();
            }
            if let Some(point) = current_point {
                if point.y == row_index && point.x == column_index {
                    grid_point_output = column_index.to_string();
                    // Advance to the next point for the next loop to handle
                    current_point = point_inter.next();
                }
            }
            print!("{}", grid_point_output);
        }
        print!("|\n");
    }
    println!("{:?}", game_state.body);
}

#[cfg(test)]
mod tests {
    use crate::output::print_world;
    use crate::types::Direction;
    use crate::{GameOverReason, GameState, Point};

    #[tokio::test]
    async fn output_missing_food() {
        let size = 10;
        let test_body = vec![
            Point { x: 1, y: 1 },
            Point { x: 2, y: 1 },
            Point { x: 3, y: 1 },
            Point { x: 4, y: 1 },
            Point { x: 5, y: 1 },
        ];

        print_world(&GameState {
            height: size,
            width: size,
            tick: 1000,
            game_over_reason: Some(GameOverReason::OutOfBounds),
            direction: Direction::North,
            num_users: 1,
            body: test_body,
            food: Point { x: 0, y: 2 },
        });
    }

    #[tokio::test]
    async fn output() {
        let size = 20;
        let test_body = vec![
            Point::new(2, (size / 2) - 1),
            Point::new(2, size / 2),
            Point::new(1, size / 2),
            Point::new(0, size / 2),
        ];

        print_world(&GameState {
            height: size,
            width: size,
            tick: 1000,
            game_over_reason: Some(GameOverReason::OutOfBounds),
            direction: Direction::North,
            num_users: 1,
            body: test_body,
            food: Point::new(0, 0),
        });
    }

    #[tokio::test]
    async fn output_with_out_of_bounds_points() {
        let size = 20;
        let test_body = vec![
            Point::new(2, (size / 2) - 1),
            Point::new(2, size / 2),
            Point::new(1, size / 2),
            Point::new(0, size / 2),
            Point::new(-1, size / 2),
            Point::new(size + 1, size / 2),
            Point::new(0, -1),
            Point::new(0, size + 1),
        ];

        print_world(&GameState {
            height: size,
            width: size,
            tick: 1000,
            game_over_reason: Some(GameOverReason::OutOfBounds),
            direction: Direction::North,
            num_users: 1,
            body: test_body,
            food: Point::new(0, 0),
        });
    }

    #[tokio::test]
    async fn output_body_overlaps_food() {
        let size = 20;
        let head = Point::new(2, (size / 2) - 1);
        let test_body = vec![
            head,
            Point::new(2, size / 2),
            Point::new(1, size / 2),
            Point::new(0, size / 2),
        ];

        print_world(&GameState {
            height: size,
            width: size,
            tick: 1000,
            game_over_reason: Some(GameOverReason::OutOfBounds),
            direction: Direction::North,
            num_users: 1,
            body: test_body,
            food: head,
        });
    }
}
