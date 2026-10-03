use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use Direction::{Down, Up};
use ExecutableRole::{Down as DownRole, Up as UpRole, VerifyDown, VerifyUp};
use control_tower_application::*;

#[derive(Default)]
struct Memory {
    checkpoint: Option<WorkbenchState>,
    writes: Vec<WorkbenchState>,
    calls: Vec<(Invocation, WorkbenchState)>,
    failures: VecDeque<(u32, ExecutableRole)>,
    write_attempts: usize,
    fail_write_at: Option<usize>,
    trace: Vec<String>,
}
struct Queries(Arc<Mutex<Memory>>);
struct Writes(Arc<Mutex<Memory>>);
struct Runner(Arc<Mutex<Memory>>);
struct Discovery(Vec<Stage>);
struct Reader;
impl StageDefinitionReader for Reader {
    fn read(
        &self,
        path: &Path,
        maximum_bytes: usize,
    ) -> Result<StageDefinitionContents, StageDefinitionReadError> {
        let bytes = path
            .to_string_lossy()
            .as_bytes()
            .iter()
            .copied()
            .take(maximum_bytes)
            .collect();
        Ok(StageDefinitionContents {
            bytes,
            truncated: false,
        })
    }
}
impl WorkbenchQueries for Queries {
    fn read_checkpoint(&self) -> Result<Option<WorkbenchState>, PersistenceError> {
        Ok(self.0.lock().unwrap().checkpoint.clone())
    }
}
impl WorkbenchWrites for Writes {
    fn record_checkpoint(&self, state: &WorkbenchState) -> Result<(), PersistenceError> {
        let mut memory = self.0.lock().unwrap();
        memory.write_attempts += 1;
        if memory.fail_write_at == Some(memory.write_attempts) {
            return Err(PersistenceError::new(std::io::Error::other(
                "injected checkpoint failure",
            )));
        }
        memory.checkpoint = Some(state.clone());
        memory.writes.push(state.clone());
        Ok(())
    }
}
impl ExecutableRunner for Runner {
    fn run(&self, invocation: &Invocation) -> Result<ProcessOutput, ExecutableRunError> {
        let mut memory = self.0.lock().unwrap();
        let checkpoint = memory.checkpoint.clone().unwrap_or_default();
        memory.trace.push(format!(
            "run {} {}",
            invocation.stage_number, invocation.role
        ));
        memory.calls.push((invocation.clone(), checkpoint));
        let success = memory.failures.front() != Some(&(invocation.stage_number, invocation.role));
        if !success {
            memory.failures.pop_front();
        }
        Ok(ProcessOutput {
            success,
            exit_code: Some(if success { 0 } else { 23 }),
            stdout: b"raw\xff".to_vec(),
            stderr: b"err\0".to_vec(),
        })
    }
}
impl StageDiscovery for Discovery {
    fn discover(&self, _: &Path) -> Result<Vec<Stage>, StageDiscoveryError> {
        Ok(self.0.clone())
    }
}
struct Fixture {
    memory: Arc<Mutex<Memory>>,
    stages: Vec<Stage>,
}
impl Fixture {
    fn new() -> Self {
        Self {
            memory: Arc::default(),
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
            Box::new(Reader),
        )
    }
    fn move_to(&self, direction: Direction, target_stage: u32) -> MoveOutcome {
        self.workbench()
            .move_to(MoveToInput {
                workspace_root: Path::new("/fixture"),
                direction,
                target_stage,
                expected_checkpoint: None,
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
        self.memory
            .lock()
            .unwrap()
            .failures
            .push_back((number, role));
    }
    fn calls(&self) -> Vec<(u32, ExecutableRole)> {
        self.memory
            .lock()
            .unwrap()
            .calls
            .iter()
            .map(|(invocation, _)| (invocation.stage_number, invocation.role))
            .collect()
    }
    fn clear_calls(&self) {
        self.memory.lock().unwrap().calls.clear();
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
    let memory = fixture.memory.lock().unwrap();
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
            .lock()
            .unwrap()
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
            target_stage: 9,
            expected_checkpoint: None,
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
            fixture.memory.lock().unwrap().checkpoint = Some(WorkbenchState {
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
            fixture.memory.lock().unwrap().checkpoint = Some(WorkbenchState {
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

#[test]
fn synchronous_observations_surround_each_role_before_the_next_invocation() {
    let fixture = Fixture::new();
    let memory = fixture.memory.clone();
    let outcome = fixture
        .workbench()
        .move_to_observed(
            MoveToInput {
                workspace_root: Path::new("/fixture"),
                direction: Up,
                target_stage: 2,
                expected_checkpoint: None,
            },
            &mut |progress| match progress {
                ExecutionProgress::Starting { stage, role } => memory
                    .lock()
                    .unwrap()
                    .trace
                    .push(format!("starting {} {role}", stage.number)),
                ExecutionProgress::Admitted { .. } => {}
                ExecutionProgress::Finished { stage, execution } => {
                    let output = execution.result.as_ref().unwrap();
                    assert_eq!(output.stdout, b"raw\xff");
                    assert_eq!(output.stderr, b"err\0");
                    memory
                        .lock()
                        .unwrap()
                        .trace
                        .push(format!("finished {} {}", stage.number, execution.role));
                }
            },
        )
        .unwrap();
    assert_eq!(
        fixture.memory.lock().unwrap().trace,
        [
            "starting 1 up",
            "run 1 up",
            "finished 1 up",
            "starting 1 verify-up",
            "run 1 verify-up",
            "finished 1 verify-up",
            "starting 2 up",
            "run 2 up",
            "finished 2 up",
            "starting 2 verify-up",
            "run 2 verify-up",
            "finished 2 verify-up",
        ]
    );
    assert_eq!(outcome.executions.len(), 4);
    for event in outcome.executions {
        assert_eq!(event.result.unwrap().stdout, b"raw\xff");
    }
}

#[test]
fn failed_checkpoint_publication_returns_latest_confirmed_state_and_attempted_update() {
    use std::error::Error;
    struct Case {
        name: &'static str,
        initial: Option<(usize, Option<(usize, Direction)>)>,
        no_verifier: bool,
        direction: Direction,
        target: u32,
        fail_at: usize,
        calls: Vec<(u32, ExecutableRole)>,
        confirmed: (usize, Option<(usize, Direction)>),
        attempted: (usize, Option<(usize, Direction)>),
    }
    let cases = [
        Case {
            name: "initial UUID",
            initial: None,
            no_verifier: false,
            direction: Up,
            target: 3,
            fail_at: 1,
            calls: vec![],
            confirmed: (0, None),
            attempted: (0, None),
        },
        Case {
            name: "pending after up",
            initial: None,
            no_verifier: false,
            direction: Up,
            target: 3,
            fail_at: 2,
            calls: vec![(1, UpRole)],
            confirmed: (0, None),
            attempted: (0, Some((0, Up))),
        },
        Case {
            name: "accepted up",
            initial: None,
            no_verifier: false,
            direction: Up,
            target: 3,
            fail_at: 3,
            calls: vec![(1, UpRole), (1, VerifyUp)],
            confirmed: (0, Some((0, Up))),
            attempted: (1, None),
        },
        Case {
            name: "no verifier",
            initial: None,
            no_verifier: true,
            direction: Up,
            target: 3,
            fail_at: 2,
            calls: vec![(1, UpRole)],
            confirmed: (0, None),
            attempted: (1, None),
        },
        Case {
            name: "baseline UUID clearing",
            initial: Some((1, None)),
            no_verifier: false,
            direction: Down,
            target: 0,
            fail_at: 2,
            calls: vec![(1, DownRole), (1, VerifyDown)],
            confirmed: (1, Some((0, Down))),
            attempted: (0, None),
        },
        Case {
            name: "earlier accepted stage",
            initial: None,
            no_verifier: false,
            direction: Up,
            target: 3,
            fail_at: 5,
            calls: vec![(1, UpRole), (1, VerifyUp), (2, UpRole), (2, VerifyUp)],
            confirmed: (1, Some((1, Up))),
            attempted: (2, None),
        },
        Case {
            name: "reverse pending save",
            initial: Some((0, Some((0, Up)))),
            no_verifier: false,
            direction: Down,
            target: 0,
            fail_at: 1,
            calls: vec![(1, DownRole)],
            confirmed: (0, Some((0, Up))),
            attempted: (0, Some((0, Down))),
        },
        Case {
            name: "reverse baseline acceptance",
            initial: Some((0, Some((0, Up)))),
            no_verifier: false,
            direction: Down,
            target: 0,
            fail_at: 2,
            calls: vec![(1, DownRole), (1, VerifyDown)],
            confirmed: (0, Some((0, Down))),
            attempted: (0, None),
        },
        Case {
            name: "verifier-only acceptance",
            initial: Some((0, Some((0, Up)))),
            no_verifier: false,
            direction: Up,
            target: 1,
            fail_at: 1,
            calls: vec![(1, VerifyUp)],
            confirmed: (0, Some((0, Up))),
            attempted: (1, None),
        },
    ];
    fn position(state: &WorkbenchState) -> (usize, Option<(usize, Direction)>) {
        (
            state.completed_stage_count,
            state.pending.map(|p| (p.stage_index, p.direction)),
        )
    }
    for case in cases {
        let mut fixture = Fixture::new();
        if case.no_verifier {
            fixture.stages[0].verify_up = None;
        }
        let initial = case
            .initial
            .map(|(completed_stage_count, pending)| WorkbenchState {
                completed_stage_count,
                uuid: Some("existing-run".into()),
                pending: pending.map(|(stage_index, direction)| PendingTransition {
                    stage_index,
                    direction,
                }),
            });
        fixture.memory.lock().unwrap().checkpoint = initial.clone();
        fixture.memory.lock().unwrap().fail_write_at = Some(case.fail_at);
        let mut observations = 0;
        let outcome = fixture
            .workbench()
            .move_to_observed(
                MoveToInput {
                    workspace_root: Path::new("/fixture"),
                    direction: case.direction,
                    target_stage: case.target,
                    expected_checkpoint: None,
                },
                &mut |_| observations += 1,
            )
            .unwrap();
        assert!(outcome.verification_choices().is_none(), "{}", case.name);
        let MoveStatus::Stopped { state, failure } = outcome.status else {
            panic!("{} did not stop", case.name)
        };
        assert_eq!(position(&state), case.confirmed, "{}", case.name);
        let TransitionFailure::StateCouldNotBeSaved {
            ref error,
            ref attempted,
        } = failure
        else {
            panic!("expected save failure")
        };
        assert!(error.source().unwrap().is::<std::io::Error>());
        assert!(failure.source().unwrap().is::<PersistenceError>());
        assert_eq!(position(attempted), case.attempted, "{}", case.name);
        if case.name == "initial UUID" {
            assert_eq!(state.uuid, None);
            assert!(attempted.uuid.is_some());
            assert_eq!(fixture.memory.lock().unwrap().checkpoint, None);
        } else {
            assert!(state.uuid.is_some());
            assert_eq!(
                attempted.uuid.is_none(),
                case.direction == Down && case.attempted.0 == 0 && case.attempted.1.is_none()
            );
        }
        let memory = fixture.memory.lock().unwrap();
        assert_eq!(
            memory.checkpoint.clone().unwrap_or_default(),
            state,
            "{}",
            case.name
        );
        assert_eq!(
            memory.write_attempts, case.fail_at,
            "no writes after failure"
        );
        assert_eq!(
            observations,
            1 + case.calls.len() * 2,
            "one admission observation plus attempted role observations only"
        );
        assert_eq!(outcome.executions.len(), case.calls.len());
        for event in &outcome.executions {
            assert_eq!(event.result.as_ref().unwrap().stdout, b"raw\xff");
        }
        drop(memory);
        assert_eq!(fixture.calls(), case.calls, "{}", case.name);
    }
}

#[test]
fn verifier_choices_resolve_only_active_sparse_stage_and_exclude_other_failures() {
    for direction in [Up, Down] {
        for first_stage in [false, true] {
            let mut fixture = Fixture::new();
            fixture.stages[0].number = 10;
            fixture.stages[1].number = 200;
            fixture.stages[2].number = 900;
            let index = if first_stage { 0 } else { 1 };
            fixture.memory.lock().unwrap().checkpoint = Some(WorkbenchState {
                completed_stage_count: if direction == Up { index } else { index + 1 },
                uuid: Some("run".into()),
                pending: None,
            });
            fixture.fail(
                fixture.stages[index].number,
                if direction == Up {
                    VerifyUp
                } else {
                    VerifyDown
                },
            );
            let outcome = fixture.move_to(direction, if direction == Up { 900 } else { 0 });
            let choices = outcome.verification_choices().unwrap();
            let mechanical = outcome.movement_choices();
            assert!(mechanical.contains(&choices.retry));
            assert!(
                choices
                    .reverse
                    .is_none_or(|reverse| mechanical.contains(&reverse))
            );
            let upper = fixture.stages[index].number;
            let lower = if first_stage { 0 } else { 10 };
            assert_eq!(
                choices.retry,
                MovementChoice {
                    direction,
                    target_stage: if direction == Up { upper } else { lower }
                }
            );
            assert_eq!(
                choices.reverse,
                Some(MovementChoice {
                    direction: if direction == Up { Down } else { Up },
                    target_stage: if direction == Up { lower } else { upper }
                })
            );
            fixture.clear_calls();
            fixture.move_to(choices.retry.direction, choices.retry.target_stage);
            assert_eq!(
                fixture.calls(),
                vec![(
                    upper,
                    if direction == Up {
                        VerifyUp
                    } else {
                        VerifyDown
                    }
                )]
            );
        }
    }
    let mut fixture = Fixture::new();
    fixture.stages[0].down = None;
    fixture.fail(1, VerifyUp);
    assert!(
        fixture
            .move_to(Up, 1)
            .verification_choices()
            .unwrap()
            .reverse
            .is_none()
    );
    let fixture = Fixture::new();
    fixture.pending_up();
    fixture.fail(3, DownRole);
    assert!(fixture.move_to(Down, 2).verification_choices().is_none());
    let fixture = Fixture::new();
    fixture.fail(1, UpRole);
    assert!(fixture.move_to(Up, 1).verification_choices().is_none());
}

#[test]
fn immediate_settled_choices_use_adjacent_sparse_identities_and_execute_only_that_stage() {
    let expected = [
        vec![(Up, 10)],
        vec![(Up, 200), (Down, 0)],
        vec![(Up, 900), (Down, 10)],
        vec![(Down, 200)],
    ];
    for (completed, expected) in expected.into_iter().enumerate() {
        let mut fixture = Fixture::new();
        for (stage, number) in fixture.stages.iter_mut().zip([10, 200, 900]) {
            stage.number = number;
        }
        let state = WorkbenchState {
            completed_stage_count: completed,
            uuid: (completed > 0).then(|| "run".to_owned()),
            pending: None,
        };
        fixture.memory.lock().unwrap().checkpoint = Some(state.clone());
        let status = fixture.workbench().status(Path::new("/fixture")).unwrap();
        let choices = status.movement_choices();
        assert_eq!(
            choices
                .iter()
                .map(|c| (c.direction, c.target_stage))
                .collect::<Vec<_>>(),
            expected
        );
        assert!(fixture.calls().is_empty());
        assert!(fixture.memory.lock().unwrap().writes.is_empty());
        assert_eq!(fixture.state(), state);
        for choice in choices {
            fixture.memory.lock().unwrap().checkpoint = Some(state.clone());
            fixture.clear_calls();
            assert!(matches!(
                fixture
                    .move_to(choice.direction, choice.target_stage)
                    .status,
                MoveStatus::Complete(_)
            ));
            let index = if choice.direction == Up {
                completed
            } else {
                completed - 1
            };
            let number = fixture.stages[index].number;
            let roles = if choice.direction == Up {
                [UpRole, VerifyUp]
            } else {
                [DownRole, VerifyDown]
            };
            assert_eq!(fixture.calls(), roles.map(|role| (number, role)));
        }
    }
}

#[test]
fn immediate_pending_choices_resolve_the_active_stage_from_either_accepted_side() {
    for index in 0..3 {
        for completed in [index, index + 1] {
            for direction in [Up, Down] {
                let mut fixture = Fixture::new();
                for (stage, number) in fixture.stages.iter_mut().zip([10, 200, 900]) {
                    stage.number = number;
                }
                let state = WorkbenchState {
                    completed_stage_count: completed,
                    uuid: Some("run".into()),
                    pending: Some(PendingTransition {
                        stage_index: index,
                        direction,
                    }),
                };
                fixture.memory.lock().unwrap().checkpoint = Some(state.clone());
                let choices = fixture
                    .workbench()
                    .status(Path::new("/fixture"))
                    .unwrap()
                    .movement_choices();
                let number = fixture.stages[index].number;
                let lower = if index == 0 {
                    0
                } else {
                    fixture.stages[index - 1].number
                };
                assert_eq!(
                    choices,
                    vec![
                        MovementChoice {
                            direction: Up,
                            target_stage: number
                        },
                        MovementChoice {
                            direction: Down,
                            target_stage: lower
                        }
                    ]
                );
                assert!(fixture.calls().is_empty());
                assert!(fixture.memory.lock().unwrap().writes.is_empty());
                for choice in choices {
                    fixture.memory.lock().unwrap().checkpoint = Some(state.clone());
                    fixture.clear_calls();
                    let outcome = fixture.move_to(choice.direction, choice.target_stage);
                    assert!(matches!(outcome.status, MoveStatus::Complete(_)));
                    let mutation = if choice.direction == Up {
                        UpRole
                    } else {
                        DownRole
                    };
                    let verify = if choice.direction == Up {
                        VerifyUp
                    } else {
                        VerifyDown
                    };
                    let expected = if choice.direction == direction {
                        vec![(number, verify)]
                    } else {
                        vec![(number, mutation), (number, verify)]
                    };
                    assert_eq!(fixture.calls(), expected);
                }
            }
        }
    }
}

#[test]
fn immediate_choices_exclude_missing_mutations_without_blocking_pending_continuation() {
    let mut fixture = Fixture::new();
    fixture.stages[0].up = None;
    assert!(
        fixture
            .workbench()
            .status(Path::new("/fixture"))
            .unwrap()
            .movement_choices()
            .is_empty()
    );
    fixture.stages[0].up = Some("up".into());
    fixture.memory.lock().unwrap().checkpoint = Some(WorkbenchState {
        completed_stage_count: 3,
        uuid: Some("run".into()),
        pending: None,
    });
    fixture.stages[2].down = None;
    assert!(
        fixture
            .workbench()
            .status(Path::new("/fixture"))
            .unwrap()
            .movement_choices()
            .is_empty()
    );
    for direction in [Up, Down] {
        for missing_matching in [false, true] {
            let mut fixture = Fixture::new();
            let missing = if missing_matching {
                direction
            } else if direction == Up {
                Down
            } else {
                Up
            };
            match missing {
                Up => {
                    fixture.stages[0].up = None;
                    fixture.stages[0].verify_up = None;
                }
                Down => {
                    fixture.stages[0].down = None;
                    fixture.stages[0].verify_down = None;
                }
            }
            fixture.memory.lock().unwrap().checkpoint = Some(WorkbenchState {
                completed_stage_count: usize::from(direction == Down),
                uuid: Some("run".into()),
                pending: Some(PendingTransition {
                    stage_index: 0,
                    direction,
                }),
            });
            let choices = fixture
                .workbench()
                .status(Path::new("/fixture"))
                .unwrap()
                .movement_choices();
            assert_eq!(choices.len(), if missing_matching { 2 } else { 1 });
            let continuation = choices
                .iter()
                .find(|choice| choice.direction == direction)
                .unwrap();
            assert!(matches!(
                fixture.move_to(direction, continuation.target_stage).status,
                MoveStatus::Complete(_)
            ));
            assert_eq!(
                fixture.calls(),
                if missing_matching {
                    vec![]
                } else {
                    vec![(
                        1,
                        if direction == Up {
                            VerifyUp
                        } else {
                            VerifyDown
                        },
                    )]
                }
            );
        }
    }
}

#[test]
fn outcome_choices_use_confirmed_state_without_claiming_recovery_after_a_failed_save() {
    let fixture = Fixture::new();
    fixture.memory.lock().unwrap().fail_write_at = Some(3);
    let outcome = fixture.move_to(Up, 3);
    assert!(outcome.verification_choices().is_none());
    assert_eq!(
        outcome.movement_choices(),
        vec![
            MovementChoice {
                direction: Up,
                target_stage: 1
            },
            MovementChoice {
                direction: Down,
                target_stage: 0
            }
        ]
    );
    let MoveStatus::Stopped {
        state,
        failure: TransitionFailure::StateCouldNotBeSaved { attempted, .. },
    } = outcome.status
    else {
        panic!("expected final checkpoint failure")
    };
    assert_eq!(state.completed_stage_count, 0);
    assert!(state.pending.is_some());
    assert_eq!(attempted.completed_stage_count, 1);
    assert!(attempted.pending.is_none());
}

#[test]
fn stale_expected_checkpoints_reject_before_uuid_writes_or_executable_calls() {
    let cases = [
        WorkbenchState {
            completed_stage_count: 1,
            uuid: None,
            pending: None,
        },
        WorkbenchState {
            completed_stage_count: 0,
            uuid: Some("another-run".into()),
            pending: None,
        },
        WorkbenchState {
            completed_stage_count: 0,
            uuid: None,
            pending: Some(PendingTransition {
                stage_index: 0,
                direction: Up,
            }),
        },
    ];
    for expected in cases {
        let fixture = Fixture::new();
        let result = fixture.workbench().move_to(MoveToInput {
            workspace_root: Path::new("/fixture"),
            direction: Up,
            target_stage: 1,
            expected_checkpoint: Some(&expected),
        });
        assert!(matches!(result, Err(MoveToError::StaleCheckpoint)));
        let memory = fixture.memory.lock().unwrap();
        assert!(memory.calls.is_empty());
        assert!(memory.writes.is_empty());
        assert_eq!(memory.write_attempts, 0);
        assert_eq!(memory.checkpoint, None);
    }
}
