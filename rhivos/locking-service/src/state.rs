//! Lock and door state data models.
//!
//! This module provides the internal state representation for door locks,
//! tracking both lock status and door open/closed status for all doors.

use std::time::SystemTime;

use crate::proto::Door;

/// State of a single door including lock and open status.
#[derive(Debug, Clone)]
pub struct DoorState {
    /// Whether the door is locked.
    pub is_locked: bool,
    /// Whether the door is physically open.
    pub is_open: bool,
    /// Timestamp of the last state update.
    pub last_updated: SystemTime,
}

impl Default for DoorState {
    fn default() -> Self {
        Self {
            is_locked: false,
            is_open: false,
            last_updated: SystemTime::now(),
        }
    }
}

impl DoorState {
    /// Creates a new DoorState with the given lock and open status.
    pub fn new(is_locked: bool, is_open: bool) -> Self {
        Self {
            is_locked,
            is_open,
            last_updated: SystemTime::now(),
        }
    }
}

/// Internal lock state for all doors in the vehicle.
#[derive(Debug, Clone, Default)]
pub struct LockState {
    /// Driver side door (Row 1, left).
    pub driver: DoorState,
    /// Passenger side door (Row 1, right).
    pub passenger: DoorState,
    /// Rear left door (Row 2, left).
    pub rear_left: DoorState,
    /// Rear right door (Row 2, right).
    pub rear_right: DoorState,
}

impl LockState {
    /// Gets a reference to the state of the specified door.
    ///
    /// Returns `None` for `Door::Unknown` or invalid door values.
    pub fn get_door(&self, door: Door) -> Option<&DoorState> {
        match door {
            Door::Driver => Some(&self.driver),
            Door::Passenger => Some(&self.passenger),
            Door::RearLeft => Some(&self.rear_left),
            Door::RearRight => Some(&self.rear_right),
            Door::Unknown | Door::All => None,
        }
    }

    /// Gets a mutable reference to the state of the specified door.
    ///
    /// Returns `None` for `Door::Unknown` or `Door::All`.
    pub fn get_door_mut(&mut self, door: Door) -> Option<&mut DoorState> {
        match door {
            Door::Driver => Some(&mut self.driver),
            Door::Passenger => Some(&mut self.passenger),
            Door::RearLeft => Some(&mut self.rear_left),
            Door::RearRight => Some(&mut self.rear_right),
            Door::Unknown | Door::All => None,
        }
    }

    /// Sets the locked state for the specified door.
    ///
    /// If `Door::All` is specified, sets the lock state for all doors.
    /// Returns `false` if the door is invalid (`Door::Unknown`).
    pub fn set_locked(&mut self, door: Door, locked: bool) -> bool {
        let now = SystemTime::now();
        match door {
            Door::Unknown => false,
            Door::All => {
                self.driver.is_locked = locked;
                self.driver.last_updated = now;
                self.passenger.is_locked = locked;
                self.passenger.last_updated = now;
                self.rear_left.is_locked = locked;
                self.rear_left.last_updated = now;
                self.rear_right.is_locked = locked;
                self.rear_right.last_updated = now;
                true
            }
            _ => {
                if let Some(state) = self.get_door_mut(door) {
                    state.is_locked = locked;
                    state.last_updated = now;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Returns an iterator over all individual doors and their states.
    pub fn iter_doors(&self) -> impl Iterator<Item = (Door, &DoorState)> {
        [
            (Door::Driver, &self.driver),
            (Door::Passenger, &self.passenger),
            (Door::RearLeft, &self.rear_left),
            (Door::RearRight, &self.rear_right),
        ]
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_door_state_default() {
        let state = DoorState::default();
        assert!(!state.is_locked);
        assert!(!state.is_open);
    }

    #[test]
    fn test_lock_state_get_door() {
        let state = LockState::default();
        assert!(state.get_door(Door::Driver).is_some());
        assert!(state.get_door(Door::Passenger).is_some());
        assert!(state.get_door(Door::RearLeft).is_some());
        assert!(state.get_door(Door::RearRight).is_some());
        assert!(state.get_door(Door::Unknown).is_none());
        assert!(state.get_door(Door::All).is_none());
    }

    #[test]
    fn test_lock_state_set_locked_single() {
        let mut state = LockState::default();

        assert!(state.set_locked(Door::Driver, true));
        assert!(state.driver.is_locked);
        assert!(!state.passenger.is_locked);
    }

    #[test]
    fn test_lock_state_set_locked_all() {
        let mut state = LockState::default();

        assert!(state.set_locked(Door::All, true));
        assert!(state.driver.is_locked);
        assert!(state.passenger.is_locked);
        assert!(state.rear_left.is_locked);
        assert!(state.rear_right.is_locked);
    }

    #[test]
    fn test_lock_state_set_locked_unknown() {
        let mut state = LockState::default();
        assert!(!state.set_locked(Door::Unknown, true));
    }

    #[test]
    fn test_iter_doors() {
        let state = LockState::default();
        let doors: Vec<_> = state.iter_doors().collect();
        assert_eq!(doors.len(), 4);
    }
}
