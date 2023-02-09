use std::collections::HashMap;

use crate::types::Direction;
use tokio::sync::Mutex;

pub(crate) struct RequestedDirection {
    directions: Mutex<HashMap<String, Direction>>,
}

impl RequestedDirection {
    pub fn new() -> Self {
        Self {
            directions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn add_direction(&self, user_id: &str, direction: Direction) {
        self.directions
            .lock()
            .await
            .insert(user_id.to_string(), direction);
    }

    pub async fn clear(&self) {
        self.directions.lock().await.clear();
    }

    pub async fn calculate_direction(&self) -> Option<Direction> {
        let directions_guard = self.directions.lock().await;
        let mut directions_count: HashMap<Direction, usize> = HashMap::new();

        for (_, direction) in &*directions_guard {
            *directions_count.entry(*direction).or_insert(1) += 1;
        }

        directions_count
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(k, _v)| k.clone())
    }

    pub async fn len(&self) -> usize {
        self.directions.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::{requested_direction::RequestedDirection, types::Direction};

    #[tokio::test]
    async fn add_direction() {
        let requested_direction = RequestedDirection::new();
        let user_id = "test_user_id";

        // Add direction
        let _ = requested_direction
            .add_direction(user_id, Direction::South)
            .await;
        let _max_direction = requested_direction.calculate_direction().await;

        assert!(matches!(Some(Direction::South), _max_direction));
    }

    #[tokio::test]
    async fn clear() {
        let requested_direction = RequestedDirection::new();
        requested_direction.clear().await;
    }

    #[tokio::test]
    async fn max_direction_selected() {
        let requested_direction = RequestedDirection::new();

        let users = generate_user_ids(10);
        let majority_index = (users.len() / 2) + 1;

        // Add direction
        for user_index in 0..users.len() {
            let user_id = users.get(user_index).unwrap();
            let direction = match user_index.cmp(&majority_index) {
                std::cmp::Ordering::Greater => Direction::North,
                _ => Direction::South,
            };
            let _ = requested_direction.add_direction(user_id, direction).await;
        }

        let _max_direction = requested_direction.calculate_direction().await;
        assert!(matches!(Some(Direction::South), _max_direction));
    }

    #[tokio::test]
    async fn user_counted_once() {
        let requested_direction = RequestedDirection::new();

        let south_user_1 = "user_south";
        let north_user_1 = "north_user_1";
        let north_user_2 = "north_user_2";
        let _ = requested_direction
            .add_direction(north_user_1, Direction::North)
            .await;
        let _ = requested_direction
            .add_direction(north_user_2, Direction::North)
            .await;

        // Same users selects the same location multiple times
        for _i in 0..10 {
            let _ = requested_direction
                .add_direction(south_user_1, Direction::South)
                .await;
        }

        // Others users direction is the winner
        let _max_direction = requested_direction.calculate_direction().await;
        assert!(matches!(Some(Direction::North), _max_direction));

        // User's selection is still selected if more users agree
        let south_user_2 = "user_south_2";
        let south_user_3 = "user_south_3";
        let _ = requested_direction
            .add_direction(south_user_2, Direction::South)
            .await;
        let _ = requested_direction
            .add_direction(south_user_3, Direction::South)
            .await;

        let _max_direction = requested_direction.calculate_direction().await;
        assert!(matches!(Some(Direction::South), _max_direction));
    }

    fn generate_user_ids(num_users: usize) -> Vec<String> {
        let mut users: Vec<String> = Vec::new();
        for i in 0..num_users {
            let user_id = format!("user-{}", i);
            users.push(user_id);
        }
        users
    }
}
