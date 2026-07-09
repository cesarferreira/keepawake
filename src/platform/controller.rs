use crate::runtime::AwakeControl;

pub(crate) trait Backend {
    fn activate(&mut self) -> Result<(), String>;
    fn deactivate(&mut self) -> Result<(), String>;
    fn refresh(&mut self) -> Result<(), String>;
    fn requires_periodic_refresh(&self) -> bool;
}

pub(crate) struct StatefulController<B: Backend> {
    backend: B,
    active: bool,
}

impl<B: Backend> StatefulController<B> {
    pub(crate) fn new(backend: B) -> Self {
        Self {
            backend,
            active: false,
        }
    }
}

impl<B: Backend> AwakeControl for StatefulController<B> {
    fn is_active(&self) -> bool {
        self.active
    }

    fn set_active(&mut self, active: bool) -> Result<(), String> {
        if self.active == active {
            return Ok(());
        }

        if active {
            self.backend.activate()?;
        } else {
            self.backend.deactivate()?;
        }
        self.active = active;
        Ok(())
    }

    fn requires_periodic_refresh(&self) -> bool {
        self.active && self.backend.requires_periodic_refresh()
    }

    fn refresh(&mut self) -> Result<(), String> {
        if self.active {
            self.backend.refresh()?;
        }
        Ok(())
    }
}

impl<B: Backend> Drop for StatefulController<B> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.backend.deactivate();
            self.active = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, StatefulController};
    use crate::runtime::AwakeControl;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Default)]
    struct Calls {
        activations: usize,
        deactivations: usize,
        refreshes: usize,
    }

    struct FakeBackend {
        calls: Rc<RefCell<Calls>>,
        fail_activation: bool,
        periodic: bool,
    }

    impl Backend for FakeBackend {
        fn activate(&mut self) -> Result<(), String> {
            self.calls.borrow_mut().activations += 1;
            if self.fail_activation {
                Err("activation failed".to_string())
            } else {
                Ok(())
            }
        }

        fn deactivate(&mut self) -> Result<(), String> {
            self.calls.borrow_mut().deactivations += 1;
            Ok(())
        }

        fn refresh(&mut self) -> Result<(), String> {
            self.calls.borrow_mut().refreshes += 1;
            Ok(())
        }

        fn requires_periodic_refresh(&self) -> bool {
            self.periodic
        }
    }

    fn controller(
        calls: Rc<RefCell<Calls>>,
        fail_activation: bool,
        periodic: bool,
    ) -> StatefulController<FakeBackend> {
        StatefulController::new(FakeBackend {
            calls,
            fail_activation,
            periodic,
        })
    }

    #[test]
    fn backend_changes_only_on_state_transitions() {
        let calls = Rc::new(RefCell::new(Calls::default()));
        let mut controller = controller(Rc::clone(&calls), false, false);

        controller.set_active(true).unwrap();
        controller.set_active(true).unwrap();
        controller.set_active(false).unwrap();
        controller.set_active(false).unwrap();

        assert_eq!(calls.borrow().activations, 1);
        assert_eq!(calls.borrow().deactivations, 1);
    }

    #[test]
    fn failed_activation_does_not_change_controller_state() {
        let calls = Rc::new(RefCell::new(Calls::default()));
        let mut controller = controller(calls, true, false);

        assert_eq!(
            controller.set_active(true),
            Err("activation failed".to_string())
        );
        assert!(!controller.is_active());
    }

    #[test]
    fn active_backend_is_deactivated_on_drop() {
        let calls = Rc::new(RefCell::new(Calls::default()));
        {
            let mut controller = controller(Rc::clone(&calls), false, false);
            controller.set_active(true).unwrap();
        }

        assert_eq!(calls.borrow().deactivations, 1);
    }

    #[test]
    fn refresh_and_periodic_requirement_delegate_to_backend() {
        let calls = Rc::new(RefCell::new(Calls::default()));
        let mut controller = controller(Rc::clone(&calls), false, true);
        controller.set_active(true).unwrap();

        assert!(controller.requires_periodic_refresh());
        controller.refresh().unwrap();
        assert_eq!(calls.borrow().refreshes, 1);
    }
}
