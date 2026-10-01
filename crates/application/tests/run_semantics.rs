use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use Direction::{Down, Up};
use ExecutableRole::{Down as DownRole, Up as UpRole, VerifyDown, VerifyUp};
use control_tower_application::*;

#[derive(Default)]
struct Memory {
    checkpoint: Option<WorkbenchState>,
    writes: Vec<WorkbenchState>,
    calls: Vec<(Invocation, WorkbenchState)>,
    failures: VecDeque<(u32, ExecutableRole)>,
}
struct Queries(Rc<RefCell<Memory>>);
struct Writes(Rc<RefCell<Memory>>);
struct Runner(Rc<RefCell<Memory>>);
struct Discovery(Vec<Stage>);
impl WorkbenchQueries for Queries {
    fn read_checkpoint(&self) -> Result<Option<WorkbenchState>, PersistenceError> {
        Ok(self.0.borrow().checkpoint.clone())
    }
}
impl WorkbenchWrites for Writes {
    fn record_checkpoint(&self, state: &WorkbenchState) -> Result<(), PersistenceError> {
        let mut memory = self.0.borrow_mut();
        memory.checkpoint = Some(state.clone());
        memory.writes.push(state.clone());
        Ok(())
    }
}
impl ExecutableRunner for Runner {
    fn run(&self, invocation: &Invocation) -> Result<ProcessOutput, ExecutableRunError> {
        let mut memory = self.0.borrow_mut();
        let checkpoint = memory.checkpoint.clone().unwrap_or_default();
        memory.calls.push((invocation.clone(), checkpoint));
        let success = memory.failures.front() != Some(&(invocation.stage_number, invocation.role));
        if !success {
            memory.failures.pop_front();
        }
        Ok(ProcessOutput {
            success,
            exit_code: Some(if success { 0 } else { 23 }),
            stdout: vec![],
            stderr: vec![],
        })
    }
}
impl StageDiscovery for Discovery {
    fn discover(&self, _: &Path) -> Result<Vec<Stage>, StageDiscoveryError> {
        Ok(self.0.clone())
    }
}
struct Fixture {
    memory: Rc<RefCell<Memory>>,
    stages: Vec<Stage>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            memory: Rc::default(),
            stages: (1..=3)
                .map(|number| Stage {
                    number,
                    name: format!("stage-{number}"),
                    directory: PathBuf::from(format!("/fixture/{number}")),
                    up: Some("up".into()),
                    down: Some("down".into()),
                    verify_up: Some("verify-up".into()),
                    verify_down: Some("verify-down".into()),
                })
                .collect(),
        }
    }
    fn workbench(&self) -> Workbench {
        Workbench::new(
            Box::new(Discovery(self.stages.clone())),
            Box::new(Queries(self.memory.clone())),
            Box::new(Writes(self.memory.clone())),
            Box::new(Runner(self.memory.clone())),
        )
    }
    fn move_to(&self, direction: Direction, target_stage: u32) -> MoveOutcome {
        self.workbench()
            .move_to(MoveToInput {
                workspace_root: Path::new("/fixture"),
                direction,
                target_stage,
            })
            .unwrap()
    }
    fn complete(&self, direction: Direction, target: u32, completed: usize) {
        let outcome = self.move_to(direction, target);
        match outcome.status {
            MoveStatus::Complete(state) => {
                assert_eq!(state.completed_stage_count, completed);
                assert_eq!(state.pending, None);
                assert_eq!(state, self.state());
            }
            other => panic!("expected completion, got {other:?}"),
        }
    }
    fn state(&self) -> WorkbenchState {
        self.workbench()
            .status(Path::new("/fixture"))
            .unwrap()
            .state
    }
    fn fail(&self, number: u32, role: ExecutableRole) {
        self.memory.borrow_mut().failures.push_back((number, role));
    }
    fn calls(&self) -> Vec<(u32, ExecutableRole)> {
        self.memory
            .borrow()
            .calls
            .iter()
            .map(|(invocation, _)| (invocation.stage_number, invocation.role))
            .collect()
    }
    fn clear_calls(&self) {
        self.memory.borrow_mut().calls.clear();
    }
    fn pending(&self, completed: usize, number: u32, direction: Direction) {
        let state = self.state();
        assert_eq!(state.completed_stage_count, completed);
        assert_eq!(
            state.pending,
            Some(PendingTransition {
                stage_index: number as usize - 1,
                direction
            })
        );
        assert!(state.uuid.is_some());
    }
    fn pending_up(&self) {
        self.complete(Up, 2, 2);
        self.clear_calls();
        self.fail(3, VerifyUp);
        assert!(matches!(
            self.move_to(Up, 3).status,
            MoveStatus::Stopped { .. }
        ));
        self.pending(2, 3, Up);
    }
    fn pending_down(&self) {
        self.complete(Up, 3, 3);
        self.clear_calls();
        self.fail(3, VerifyDown);
        assert!(matches!(
            self.move_to(Down, 2).status,
            MoveStatus::Stopped { .. }
        ));
        self.pending(3, 3, Down);
    }
}

#[test]
fn verified_walks_commit_each_stage_and_share_one_uuid() {
    let fixture = Fixture::new();
    fixture.complete(Up, 3, 3);
    let uuid = fixture.state().uuid.unwrap();
    fixture.complete(Down, 0, 0);
    assert_eq!(fixture.state(), WorkbenchState::default());
    assert_eq!(
        fixture.calls(),
        vec![
            (1, UpRole),
            (1, VerifyUp),
            (2, UpRole),
            (2, VerifyUp),
            (3, UpRole),
            (3, VerifyUp),
            (3, DownRole),
            (3, VerifyDown),
            (2, DownRole),
            (2, VerifyDown),
            (1, DownRole),
            (1, VerifyDown),
        ]
    );
    let memory = fixture.memory.borrow();
    for (invocation, checkpoint) in &memory.calls {
        assert_eq!(invocation.uuid, uuid);
        let expected_completed = match invocation.direction {
            Up => invocation.stage_number as usize - 1,
            Down => invocation.stage_number as usize,
        };
        assert_eq!(checkpoint.completed_stage_count, expected_completed);
        if matches!(invocation.role, VerifyUp | VerifyDown) {
            assert_eq!(
                checkpoint.pending,
                Some(PendingTransition {
                    stage_index: invocation.stage_number as usize - 1,
                    direction: invocation.direction,
                })
            );
        }
    }
    let settled: Vec<_> = memory
        .writes
        .iter()
        .filter(|s| s.pending.is_none())
        .map(|s| s.completed_stage_count)
        .collect();
    assert_eq!(settled, vec![0, 1, 2, 3, 2, 1, 0]);
    drop(memory);
    fixture.complete(Up, 1, 1);
    assert_ne!(fixture.state().uuid.unwrap(), uuid);
}

#[test]
fn absent_verifiers_complete_both_directions() {
    let mut fixture = Fixture::new();
    for stage in &mut fixture.stages {
        stage.verify_up = None;
        stage.verify_down = None;
    }
    fixture.complete(Up, 3, 3);
    fixture.complete(Down, 0, 0);
    assert_eq!(
        fixture.calls(),
        vec![
            (1, UpRole),
            (2, UpRole),
            (3, UpRole),
            (3, DownRole),
            (2, DownRole),
            (1, DownRole)
        ]
    );
    assert!(
        fixture
            .memory
            .borrow()
            .writes
            .iter()
            .all(|s| s.pending.is_none())
    );
}

#[test]
fn failed_verify_up_retries_only_verifier() {
    let fixture = Fixture::new();
    fixture.pending_up();
    fixture.complete(Up, 3, 3);
    assert_eq!(
        fixture.calls(),
        vec![(3, UpRole), (3, VerifyUp), (3, VerifyUp)]
    );
}

#[test]
fn failed_verify_up_backs_out_same_stage_and_can_walk_farther() {
    for target in [2, 1, 0] {
        let fixture = Fixture::new();
        fixture.pending_up();
        let uuid = fixture.state().uuid.clone();
        fixture.complete(Down, target, target as usize);
        let mut expected = vec![(3, UpRole), (3, VerifyUp), (3, DownRole), (3, VerifyDown)];
        for number in (target + 1..=2).rev() {
            expected.extend([(number, DownRole), (number, VerifyDown)]);
        }
        assert_eq!(fixture.calls(), expected);
        if target > 0 {
            assert_eq!(fixture.state().uuid, uuid);
        }
    }
}

#[test]
fn failed_reverse_verifier_retries_without_mutation_replay_on_either_side() {
    for initial_direction in [Up, Down] {
        let fixture = Fixture::new();
        let (reverse, target, role, completed) = match initial_direction {
            Up => {
                fixture.pending_up();
                (Down, 2, VerifyDown, 2)
            }
            Down => {
                fixture.pending_down();
                (Up, 3, VerifyUp, 3)
            }
        };
        let uuid = fixture.state().uuid.clone();
        fixture.fail(3, role);
        assert!(matches!(
            fixture.move_to(reverse, target).status,
            MoveStatus::Stopped { .. }
        ));
        fixture.pending(completed, 3, reverse);
        assert_eq!(fixture.state().uuid, uuid);
        fixture.complete(reverse, target, completed);
        let expected = match initial_direction {
            Up => vec![
                (3, UpRole),
                (3, VerifyUp),
                (3, DownRole),
                (3, VerifyDown),
                (3, VerifyDown),
            ],
            Down => vec![
                (3, DownRole),
                (3, VerifyDown),
                (3, UpRole),
                (3, VerifyUp),
                (3, VerifyUp),
            ],
        };
        assert_eq!(fixture.calls(), expected);
    }
}

#[test]
fn failed_normal_verify_down_keeps_completed_position_until_retry() {
    let fixture = Fixture::new();
    fixture.pending_down();
    fixture.complete(Down, 1, 1);
    assert_eq!(
        fixture.calls(),
        vec![
            (3, DownRole),
            (3, VerifyDown),
            (3, VerifyDown),
            (2, DownRole),
            (2, VerifyDown)
        ]
    );
}

#[test]
fn pending_down_reverses_same_stage_and_can_walk_farther_up() {
    for target in [3, 4] {
        let mut fixture = Fixture::new();
        let mut fourth = fixture.stages[2].clone();
        fourth.number = 4;
        fixture.stages.push(fourth);
        fixture.pending_down();
        fixture.complete(Up, target, target as usize);
        let mut expected = vec![(3, DownRole), (3, VerifyDown), (3, UpRole), (3, VerifyUp)];
        if target == 4 {
            expected.extend([(4, UpRole), (4, VerifyUp)]);
        }
        assert_eq!(fixture.calls(), expected);
    }
}

#[test]
fn reverse_without_verifier_completes_immediately_in_both_directions() {
    for direction in [Up, Down] {
        let mut fixture = Fixture::new();
        match direction {
            Up => {
                fixture.pending_up();
                fixture.stages[2].verify_down = None;
                fixture.complete(Down, 2, 2);
            }
            Down => {
                fixture.pending_down();
                fixture.stages[2].verify_up = None;
                fixture.complete(Up, 3, 3);
            }
        }
        assert_eq!(fixture.calls().len(), 3);
    }
}

#[test]
fn baseline_rollback_verifier_failure_retains_uuid_until_acceptance() {
    let fixture = Fixture::new();
    fixture.fail(1, VerifyUp);
    fixture.move_to(Up, 1);
    fixture.pending(0, 1, Up);
    let uuid = fixture.state().uuid.clone();
    fixture.fail(1, VerifyDown);
    fixture.move_to(Down, 0);
    fixture.pending(0, 1, Down);
    assert_eq!(fixture.state().uuid, uuid);
    fixture.complete(Down, 0, 0);
    assert_eq!(fixture.state().uuid, None);
    assert_eq!(
        fixture.calls(),
        vec![
            (1, UpRole),
            (1, VerifyUp),
            (1, DownRole),
            (1, VerifyDown),
            (1, VerifyDown)
        ]
    );
}

#[test]
fn mutation_failure_stops_without_verifying_or_advancing_in_both_directions() {
    for direction in [Up, Down] {
        let fixture = Fixture::new();
        fixture.complete(Up, 2, 2);
        fixture.clear_calls();
        let (number, role, target) = match direction {
            Up => (3, UpRole, 3),
            Down => (2, DownRole, 0),
        };
        let before = fixture.state();
        fixture.fail(number, role);
        let outcome = fixture.move_to(direction, target);
        assert!(matches!(
            outcome.status,
            MoveStatus::Stopped {
                failure: TransitionFailure::ExecutableFailed { .. },
                ..
            }
        ));
        assert_eq!(fixture.state(), before);
        assert_eq!(fixture.calls(), vec![(number, role)]);
    }
}

#[test]
fn missing_mutations_stop_normal_and_reverse_walks_without_fallback() {
    for direction in [Up, Down] {
        for active in [false, true] {
            let mut fixture = Fixture::new();
            if active {
                match direction {
                    Up => fixture.pending_down(),
                    Down => fixture.pending_up(),
                }
            } else {
                fixture.complete(Up, 2, 2);
            }
            fixture.clear_calls();
            let (index, target) = match (direction, active) {
                (Up, _) => (2, 3),
                (Down, true) => (2, 2),
                (Down, false) => (1, 0),
            };
            match direction {
                Up => fixture.stages[index].up = None,
                Down => fixture.stages[index].down = None,
            }
            let before = fixture.state();
            assert!(matches!(
                fixture.move_to(direction, target).status,
                MoveStatus::Stopped {
                    failure: TransitionFailure::MissingExecutable { .. },
                    ..
                }
            ));
            assert_eq!(fixture.state(), before);
            assert!(fixture.calls().is_empty());
        }
    }
}

#[test]
fn failed_reverse_mutation_stops_and_preserves_previous_active_checkpoint() {
    for direction in [Up, Down] {
        let fixture = Fixture::new();
        let (reverse, role, target) = match direction {
            Up => {
                fixture.pending_up();
                (Down, DownRole, 1)
            }
            Down => {
                fixture.pending_down();
                (Up, UpRole, 3)
            }
        };
        fixture.clear_calls();
        let before = fixture.state();
        fixture.fail(3, role);
        assert!(matches!(
            fixture.move_to(reverse, target).status,
            MoveStatus::Stopped { .. }
        ));
        assert_eq!(fixture.state(), before);
        assert_eq!(fixture.calls(), vec![(3, role)]);
    }
}

#[test]
fn settled_target_is_noop_and_unreachable_targets_do_not_execute() {
    let fixture = Fixture::new();
    fixture.complete(Up, 2, 2);
    fixture.clear_calls();
    for direction in [Up, Down] {
        fixture.complete(direction, 2, 2);
    }
    for (direction, target) in [(Up, 1), (Down, 3)] {
        assert!(matches!(
            fixture.move_to(direction, target).status,
            MoveStatus::Stopped {
                failure: TransitionFailure::DirectionDoesNotReachTarget { .. },
                ..
            }
        ));
    }
    assert!(fixture.calls().is_empty());
    assert!(matches!(
        fixture.workbench().move_to(MoveToInput {
            workspace_root: Path::new("/fixture"),
            direction: Up,
            target_stage: 9
        }),
        Err(MoveToError::InvalidTarget(_))
    ));
}

#[test]
fn active_unreachable_targets_do_not_execute_or_change_state() {
    let fixture = Fixture::new();
    fixture.pending_up();
    fixture.clear_calls();
    let before = fixture.state();
    for (direction, target) in [(Up, 2), (Down, 3)] {
        assert!(matches!(
            fixture.move_to(direction, target).status,
            MoveStatus::Stopped {
                failure: TransitionFailure::DirectionDoesNotReachTarget { .. },
                ..
            }
        ));
        assert_eq!(fixture.state(), before);
    }
    assert!(fixture.calls().is_empty());
}

#[test]
fn validates_both_adjacent_completed_positions_independently_of_direction() {
    let fixture = Fixture::new();
    for direction in [Up, Down] {
        for completed_stage_count in [2, 3] {
            fixture.memory.borrow_mut().checkpoint = Some(WorkbenchState {
                completed_stage_count,
                uuid: Some("opaque".into()),
                pending: Some(PendingTransition {
                    stage_index: 2,
                    direction,
                }),
            });
            fixture.pending(completed_stage_count, 3, direction);
        }
        for (completed_stage_count, stage_index, uuid) in [
            (1, 2, Some("opaque")),
            (2, 3, Some("opaque")),
            (2, usize::MAX, Some("opaque")),
            (2, 2, None),
        ] {
            fixture.memory.borrow_mut().checkpoint = Some(WorkbenchState {
                completed_stage_count,
                uuid: uuid.map(str::to_owned),
                pending: Some(PendingTransition {
                    stage_index,
                    direction,
                }),
            });
            assert!(matches!(
                fixture.workbench().status(Path::new("/fixture")),
                Err(StatusError::InvalidState(_))
            ));
        }
    }
}

#[test]
fn targets_are_stage_numbers_not_contiguous_numeric_counts() {
    let mut fixture = Fixture::new();
    fixture.stages[1].number = 20;
    fixture.stages[2].number = 30;
    fixture.complete(Up, 30, 3);
    fixture.complete(Down, 1, 1);
    assert_eq!(
        fixture.calls(),
        vec![
            (1, UpRole),
            (1, VerifyUp),
            (20, UpRole),
            (20, VerifyUp),
            (30, UpRole),
            (30, VerifyUp),
            (30, DownRole),
            (30, VerifyDown),
            (20, DownRole),
            (20, VerifyDown)
        ]
    );
}
