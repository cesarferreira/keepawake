pub trait AwakeControl {
    fn is_active(&self) -> bool;
    fn set_active(&mut self, active: bool) -> Result<(), String>;
    fn requires_periodic_refresh(&self) -> bool;
    fn refresh(&mut self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileOutcome {
    StateChanged,
    Refreshed,
    Unchanged,
}

pub fn reconcile_awake(
    controller: &mut impl AwakeControl,
    desired_active: bool,
    refresh_due: bool,
) -> Result<ReconcileOutcome, String> {
    if controller.is_active() != desired_active {
        controller.set_active(desired_active)?;
        return Ok(ReconcileOutcome::StateChanged);
    }

    if desired_active && refresh_due && controller.requires_periodic_refresh() {
        controller.refresh()?;
        return Ok(ReconcileOutcome::Refreshed);
    }

    Ok(ReconcileOutcome::Unchanged)
}

pub fn minimum_delay(delays: &[Option<std::time::Duration>]) -> Option<std::time::Duration> {
    delays.iter().flatten().copied().min()
}

#[cfg(test)]
mod tests {
    use super::{AwakeControl, ReconcileOutcome, minimum_delay, reconcile_awake};
    use std::time::Duration;

    #[derive(Default)]
    struct FakeController {
        active: bool,
        periodic: bool,
        activations: usize,
        deactivations: usize,
        refreshes: usize,
    }

    impl AwakeControl for FakeController {
        fn is_active(&self) -> bool {
            self.active
        }

        fn set_active(&mut self, active: bool) -> Result<(), String> {
            self.active = active;
            if active {
                self.activations += 1;
            } else {
                self.deactivations += 1;
            }
            Ok(())
        }

        fn requires_periodic_refresh(&self) -> bool {
            self.periodic
        }

        fn refresh(&mut self) -> Result<(), String> {
            self.refreshes += 1;
            Ok(())
        }
    }

    #[test]
    fn activates_only_on_transition() {
        let mut controller = FakeController::default();

        assert_eq!(
            reconcile_awake(&mut controller, true, false).unwrap(),
            ReconcileOutcome::StateChanged
        );
        assert_eq!(
            reconcile_awake(&mut controller, true, false).unwrap(),
            ReconcileOutcome::Unchanged
        );
        assert_eq!(controller.activations, 1);
    }

    #[test]
    fn deactivates_only_on_transition() {
        let mut controller = FakeController {
            active: true,
            ..FakeController::default()
        };

        assert_eq!(
            reconcile_awake(&mut controller, false, false).unwrap(),
            ReconcileOutcome::StateChanged
        );
        assert_eq!(
            reconcile_awake(&mut controller, false, false).unwrap(),
            ReconcileOutcome::Unchanged
        );
        assert_eq!(controller.deactivations, 1);
    }

    #[test]
    fn refreshes_only_when_due_active_and_periodic() {
        let mut controller = FakeController {
            active: true,
            periodic: true,
            ..FakeController::default()
        };

        assert_eq!(
            reconcile_awake(&mut controller, true, false).unwrap(),
            ReconcileOutcome::Unchanged
        );
        assert_eq!(
            reconcile_awake(&mut controller, true, true).unwrap(),
            ReconcileOutcome::Refreshed
        );
        assert_eq!(controller.refreshes, 1);

        controller.active = false;
        assert_eq!(
            reconcile_awake(&mut controller, false, true).unwrap(),
            ReconcileOutcome::Unchanged
        );
        assert_eq!(controller.refreshes, 1);
    }

    #[test]
    fn minimum_delay_ignores_missing_deadlines() {
        assert_eq!(
            minimum_delay(&[
                None,
                Some(Duration::from_secs(30)),
                Some(Duration::from_secs(5)),
            ]),
            Some(Duration::from_secs(5))
        );
    }

    #[test]
    fn minimum_delay_returns_none_without_deadlines() {
        assert_eq!(minimum_delay(&[None, None]), None);
    }
}
