use std::fmt;
use std::rc::Rc;

mod errors;
pub use errors::*;
use std::path::{Path, PathBuf};

use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutableRole {
    Up,
    Down,
    VerifyUp,
    VerifyDown,
}

impl ExecutableRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::VerifyUp => "verify-up",
            Self::VerifyDown => "verify-down",
        }
    }

    fn for_direction(direction: Direction, verify: bool) -> Self {
        match (direction, verify) {
            (Direction::Up, false) => Self::Up,
            (Direction::Down, false) => Self::Down,
            (Direction::Up, true) => Self::VerifyUp,
            (Direction::Down, true) => Self::VerifyDown,
        }
    }
}

impl fmt::Display for ExecutableRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    pub number: u32,
    pub name: String,
    pub directory: PathBuf,
    pub up: Option<PathBuf>,
    pub down: Option<PathBuf>,
    pub verify_up: Option<PathBuf>,
    pub verify_down: Option<PathBuf>,
}

impl Stage {
    fn executable(&self, role: ExecutableRole) -> Option<&Path> {
        match role {
            ExecutableRole::Up => self.up.as_deref(),
            ExecutableRole::Down => self.down.as_deref(),
            ExecutableRole::VerifyUp => self.verify_up.as_deref(),
            ExecutableRole::VerifyDown => self.verify_down.as_deref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WorkbenchState {
    /// Last accepted position in the discovered ordered stage list.
    /// This remains unchanged while a directional verifier is pending.
    pub completed_stage_count: usize,
    /// The one opaque identifier currently handed to stage executables.
    pub uuid: Option<String>,
    /// A mutation succeeded but its verifier has not yet succeeded.
    pub pending: Option<PendingTransition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingTransition {
    /// Zero-based position in the ordered stage list.
    pub stage_index: usize,
    /// Most recently successful mutation; the completed position may be on
    /// either side of this stage while verification or reversal is outstanding.
    pub direction: Direction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub executable: PathBuf,
    pub working_directory: PathBuf,
    pub workspace_root: PathBuf,
    pub stage_number: u32,
    pub direction: Direction,
    pub role: ExecutableRole,
    pub uuid: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug)]
pub struct ExecutionEvent {
    pub stage_number: u32,
    pub role: ExecutableRole,
    /// The event and stopped outcome share this diagnostic in this synchronous
    /// result. Dependencies themselves remain exclusively owned Boxes.
    pub result: Result<ProcessOutput, Rc<ExecutableRunError>>,
}

/// Synchronous observations surrounding an actual role attempt. Captured
/// results remain in the outcome; observers do not decide navigation.
pub enum ExecutionProgress<'a> {
    Admitted {
        state: &'a WorkbenchState,
        stages: &'a [Stage],
    },
    Starting {
        stage: &'a Stage,
        role: ExecutableRole,
    },
    Finished {
        stage: &'a Stage,
        execution: &'a ExecutionEvent,
    },
}

struct ExecutionLog<'a> {
    executions: Vec<ExecutionEvent>,
    observe: &'a mut dyn FnMut(ExecutionProgress<'_>),
}

#[derive(Debug)]
pub enum TransitionFailure {
    MissingExecutable {
        stage_number: u32,
        role: ExecutableRole,
    },
    ExecutableFailed {
        stage_number: u32,
        role: ExecutableRole,
        exit_code: Option<i32>,
    },
    ExecutableCouldNotStart {
        stage_number: u32,
        role: ExecutableRole,
        error: Rc<ExecutableRunError>,
    },
    DirectionDoesNotReachTarget {
        direction: Direction,
    },
    StateCouldNotBeSaved {
        error: PersistenceError,
        attempted: WorkbenchState,
    },
}

impl fmt::Display for TransitionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingExecutable { stage_number, role } => {
                write!(formatter, "stage {stage_number} has no {role} executable")
            }
            Self::ExecutableFailed {
                stage_number,
                role,
                exit_code,
            } => {
                write!(formatter, "stage {stage_number} {role} exited with status ")?;
                match exit_code {
                    Some(code) => write!(formatter, "{code}"),
                    None => formatter.write_str("a signal or unknown status"),
                }
            }
            Self::ExecutableCouldNotStart {
                stage_number,
                role,
                error,
            } => {
                write!(
                    formatter,
                    "could not start stage {stage_number} {role}: {error}"
                )
            }
            Self::DirectionDoesNotReachTarget { direction } => write!(
                formatter,
                "the requested target is not reachable in the {direction} direction"
            ),
            Self::StateCouldNotBeSaved { error, .. } => {
                write!(formatter, "could not save workbench state: {error}")
            }
        }
    }
}

impl std::error::Error for TransitionFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StateCouldNotBeSaved { error, .. } => Some(error),
            Self::ExecutableCouldNotStart { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum MoveStatus {
    Complete(WorkbenchState),
    Stopped {
        /// Latest checkpoint confirmed by a successful read or write. A failed
        /// write's proposed update is carried separately by the failure.
        state: WorkbenchState,
        failure: TransitionFailure,
    },
}

#[derive(Debug)]
pub struct MoveOutcome {
    pub executions: Vec<ExecutionEvent>,
    pub stages: Vec<Stage>,
    pub status: MoveStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovementChoice {
    pub direction: Direction,
    pub target_stage: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerificationChoices {
    pub retry: MovementChoice,
    pub reverse: Option<MovementChoice>,
}

impl MoveOutcome {
    /// Immediate mechanically available movements from the last confirmed
    /// checkpoint. These are not recommendations about author-owned effects.
    pub fn movement_choices(&self) -> Vec<MovementChoice> {
        let state = match &self.status {
            MoveStatus::Complete(state) | MoveStatus::Stopped { state, .. } => state,
        };
        movement_choices(state, &self.stages)
    }

    /// Only this invocation's failed verifier earns verifier-specific choices.
    /// An old pending checkpoint after a failed mutation/save is insufficient.
    pub fn verification_choices(&self) -> Option<VerificationChoices> {
        let MoveStatus::Stopped { state, failure } = &self.status else {
            return None;
        };
        let role = match failure {
            TransitionFailure::ExecutableFailed { role, .. }
            | TransitionFailure::ExecutableCouldNotStart { role, .. }
                if matches!(role, ExecutableRole::VerifyUp | ExecutableRole::VerifyDown) =>
            {
                *role
            }
            _ => return None,
        };
        let pending = state.pending?;
        if role != ExecutableRole::for_direction(pending.direction, true) {
            return None;
        }
        let choices = self.movement_choices();
        Some(VerificationChoices {
            retry: *choices
                .iter()
                .find(|choice| choice.direction == pending.direction)?,
            reverse: choices
                .into_iter()
                .find(|choice| choice.direction != pending.direction),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchStatus {
    pub state: WorkbenchState,
    pub stages: Vec<Stage>,
}

impl WorkbenchStatus {
    /// Immediate mechanical choices from this authoritative status. Ordering is
    /// up then down, not recovery preference. No processes or storage are touched.
    pub fn movement_choices(&self) -> Vec<MovementChoice> {
        movement_choices(&self.state, &self.stages)
    }
}

fn movement_choices(state: &WorkbenchState, stages: &[Stage]) -> Vec<MovementChoice> {
    let mut choices = Vec::with_capacity(2);
    for direction in [Direction::Up, Direction::Down] {
        let (index, run_mutation) = if let Some(pending) = state.pending {
            (Some(pending.stage_index), direction != pending.direction)
        } else {
            (
                match direction {
                    Direction::Up => Some(state.completed_stage_count),
                    Direction::Down => state.completed_stage_count.checked_sub(1),
                },
                true,
            )
        };
        let Some(index) = index else { continue };
        let Some(stage) = stages.get(index) else {
            continue;
        };
        if run_mutation
            && stage
                .executable(ExecutableRole::for_direction(direction, false))
                .is_none()
        {
            continue;
        }
        choices.push(MovementChoice {
            direction,
            target_stage: match direction {
                Direction::Up => stage.number,
                Direction::Down => index.checked_sub(1).map_or(0, |i| stages[i].number),
            },
        });
    }
    choices
}

pub trait StageDiscovery: Send + Sync {
    fn discover(&self, workspace_root: &Path) -> Result<Vec<Stage>, StageDiscoveryError>;
}

pub trait StageDefinitionReader: Send + Sync {
    fn read(
        &self,
        path: &Path,
        maximum_bytes: usize,
    ) -> Result<StageDefinitionContents, StageDefinitionReadError>;
}

pub trait WorkbenchQueries: Send + Sync {
    fn read_checkpoint(&self) -> Result<Option<WorkbenchState>, PersistenceError>;
}

pub trait WorkbenchWrites: Send + Sync {
    /// Record one orchestration checkpoint as an independently atomic mutation.
    fn record_checkpoint(&self, checkpoint: &WorkbenchState) -> Result<(), PersistenceError>;
}

pub struct MoveToInput<'a> {
    pub workspace_root: &'a Path,
    pub direction: Direction,
    pub target_stage: u32,
    pub expected_checkpoint: Option<&'a WorkbenchState>,
}

pub struct StageDefinitionsInput<'a> {
    pub workspace_root: &'a Path,
    pub stage_number: u32,
    pub maximum_bytes: usize,
}

pub trait ExecutableRunner: Send + Sync {
    fn run(&self, invocation: &Invocation) -> Result<ProcessOutput, ExecutableRunError>;
}

pub struct StageDefinitionContents {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

pub struct StageDefinition {
    pub role: ExecutableRole,
    pub path: PathBuf,
    pub contents: Result<StageDefinitionContents, StageDefinitionReadError>,
}

pub struct StageDefinitions {
    pub stage: Stage,
    pub definitions: Vec<StageDefinition>,
}

pub struct StageDefinitionReadError {
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl StageDefinitionReadError {
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

impl fmt::Display for StageDefinitionReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl fmt::Debug for StageDefinitionReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("StageDefinitionReadError")
            .field(&self.source)
            .finish()
    }
}

impl std::error::Error for StageDefinitionReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.source)
    }
}

pub struct Workbench {
    stage_discovery: Box<dyn StageDiscovery>,
    queries: Box<dyn WorkbenchQueries>,
    writes: Box<dyn WorkbenchWrites>,
    executable_runner: Box<dyn ExecutableRunner>,
    definition_reader: Box<dyn StageDefinitionReader>,
}

impl Workbench {
    pub fn new(
        stage_discovery: Box<dyn StageDiscovery>,
        queries: Box<dyn WorkbenchQueries>,
        writes: Box<dyn WorkbenchWrites>,
        executable_runner: Box<dyn ExecutableRunner>,
        definition_reader: Box<dyn StageDefinitionReader>,
    ) -> Self {
        Self {
            stage_discovery,
            queries,
            writes,
            executable_runner,
            definition_reader,
        }
    }

    pub fn move_to(&self, input: MoveToInput<'_>) -> Result<MoveOutcome, MoveToError> {
        self.move_to_observed(input, &mut |_| {})
    }

    pub fn move_to_observed(
        &self,
        input: MoveToInput<'_>,
        observe: &mut dyn FnMut(ExecutionProgress<'_>),
    ) -> Result<MoveOutcome, MoveToError> {
        let MoveToInput {
            workspace_root,
            direction,
            target_stage,
            expected_checkpoint,
        } = input;
        let stages = self.load_stages(workspace_root)?;
        let mut state = self.load_state(stages.len())?;
        if expected_checkpoint.is_some_and(|expected| expected != &state) {
            return Err(MoveToError::StaleCheckpoint);
        }
        let target_count = target_count(&stages, target_stage)?;
        observe(ExecutionProgress::Admitted {
            state: &state,
            stages: &stages,
        });
        let mut log = ExecutionLog {
            executions: Vec::new(),
            observe,
        };

        if let Some(pending) = state.pending {
            // Resolve the active stage toward the requested side before walking
            // farther. The last completed position alone cannot select this action.
            let target_is_reachable = match direction {
                Direction::Up => target_count > pending.stage_index,
                Direction::Down => target_count <= pending.stage_index,
            };
            if !target_is_reachable {
                return Ok(MoveOutcome {
                    executions: log.executions,
                    stages,
                    status: MoveStatus::Stopped {
                        state,
                        failure: TransitionFailure::DirectionDoesNotReachTarget { direction },
                    },
                });
            }

            let transition = PendingTransition {
                stage_index: pending.stage_index,
                direction,
            };
            if let Err(failure) = self.run_transition(
                workspace_root,
                &stages[pending.stage_index],
                transition,
                direction != pending.direction,
                &mut state,
                &mut log,
            ) {
                return Ok(MoveOutcome {
                    executions: log.executions,
                    stages,
                    status: MoveStatus::Stopped { state, failure },
                });
            }
        }

        if target_count == state.completed_stage_count {
            return Ok(MoveOutcome {
                executions: log.executions,
                stages,
                status: MoveStatus::Complete(state),
            });
        }

        let walk_is_valid = match direction {
            Direction::Up => target_count > state.completed_stage_count,
            Direction::Down => target_count < state.completed_stage_count,
        };
        if !walk_is_valid {
            return Ok(MoveOutcome {
                executions: log.executions,
                stages,
                status: MoveStatus::Stopped {
                    state,
                    failure: TransitionFailure::DirectionDoesNotReachTarget { direction },
                },
            });
        }

        while state.completed_stage_count != target_count {
            let stage_index = match direction {
                Direction::Up => state.completed_stage_count,
                Direction::Down => state.completed_stage_count - 1,
            };
            if let Err(failure) = self.run_transition(
                workspace_root,
                &stages[stage_index],
                PendingTransition {
                    stage_index,
                    direction,
                },
                true,
                &mut state,
                &mut log,
            ) {
                return Ok(MoveOutcome {
                    executions: log.executions,
                    stages,
                    status: MoveStatus::Stopped { state, failure },
                });
            }
        }

        Ok(MoveOutcome {
            executions: log.executions,
            stages,
            status: MoveStatus::Complete(state),
        })
    }

    pub fn status(&self, workspace_root: &Path) -> Result<WorkbenchStatus, StatusError> {
        let stages = self.load_stages(workspace_root)?;
        let state = self.load_state(stages.len())?;
        Ok(WorkbenchStatus { state, stages })
    }

    pub fn stage_definitions(
        &self,
        input: StageDefinitionsInput<'_>,
    ) -> Result<StageDefinitions, StageDefinitionsError> {
        let StageDefinitionsInput {
            workspace_root,
            stage_number,
            maximum_bytes,
        } = input;
        let stages = self.load_stages(workspace_root)?;
        let stage = stages
            .into_iter()
            .find(|stage| stage.number == stage_number)
            .ok_or(StageDefinitionsError::UnknownStage(stage_number))?;
        let definitions = [
            (ExecutableRole::Up, stage.up.as_deref()),
            (ExecutableRole::Down, stage.down.as_deref()),
            (ExecutableRole::VerifyUp, stage.verify_up.as_deref()),
            (ExecutableRole::VerifyDown, stage.verify_down.as_deref()),
        ]
        .into_iter()
        .filter_map(|(role, path)| {
            path.map(|path| StageDefinition {
                role,
                path: path.to_path_buf(),
                contents: self.definition_reader.read(path, maximum_bytes),
            })
        })
        .collect();
        Ok(StageDefinitions { stage, definitions })
    }

    fn load_stages(&self, workspace_root: &Path) -> Result<Vec<Stage>, StageDiscoveryError> {
        let stages = self.stage_discovery.discover(workspace_root)?;
        if stages.is_empty() {
            return Err(StageDiscoveryError::message(
                "no numbered stage directories were found under `stages/`",
            ));
        }
        Ok(stages)
    }

    fn load_state(&self, stage_count: usize) -> Result<WorkbenchState, StatusError> {
        let state = self
            .queries
            .read_checkpoint()
            .map_err(StatusError::Persistence)?
            .unwrap_or_default();
        if state.completed_stage_count > stage_count {
            return Err(StatusError::InvalidState(format!(
                "{} completed stages exceed the {} discovered stages",
                state.completed_stage_count, stage_count
            )));
        }
        if let Some(pending) = state.pending {
            let consistent = pending.stage_index < stage_count
                && (pending.stage_index == state.completed_stage_count
                    || pending.stage_index + 1 == state.completed_stage_count);
            if !consistent {
                return Err(StatusError::InvalidState(
                    "pending transition does not match the completed stage position".to_owned(),
                ));
            }
        }
        if (state.completed_stage_count > 0 || state.pending.is_some()) && state.uuid.is_none() {
            return Err(StatusError::InvalidState(
                "the workbench has stage state but no UUID".to_owned(),
            ));
        }
        Ok(state)
    }

    fn run_transition(
        &self,
        workspace_root: &Path,
        stage: &Stage,
        transition: PendingTransition,
        run_mutation: bool,
        state: &mut WorkbenchState,
        log: &mut ExecutionLog<'_>,
    ) -> Result<(), TransitionFailure> {
        let direction = transition.direction;
        if run_mutation {
            let role = ExecutableRole::for_direction(direction, false);
            if stage.executable(role).is_none() {
                return Err(TransitionFailure::MissingExecutable {
                    stage_number: stage.number,
                    role,
                });
            }
            if state.uuid.is_none() {
                let mut proposed = state.clone();
                proposed.uuid = Some(Uuid::new_v4().to_string());
                self.publish_checkpoint(state, proposed)?;
            }
            if !self.run_role(workspace_root, stage, direction, false, state, log) {
                return Err(failure_for_last_execution(&log.executions));
            }
        }

        let verifier_role = ExecutableRole::for_direction(direction, true);
        if stage.executable(verifier_role).is_some() {
            if run_mutation {
                let mut proposed = state.clone();
                proposed.pending = Some(transition);
                self.publish_checkpoint(state, proposed)?;
            }
            if !self.run_role(workspace_root, stage, direction, true, state, log) {
                return Err(failure_for_last_execution(&log.executions));
            }
        }

        let mut proposed = state.clone();
        finish_transition(&mut proposed, transition);
        self.publish_checkpoint(state, proposed)
    }

    fn publish_checkpoint(
        &self,
        confirmed: &mut WorkbenchState,
        proposed: WorkbenchState,
    ) -> Result<(), TransitionFailure> {
        match self.writes.record_checkpoint(&proposed) {
            Ok(()) => {
                *confirmed = proposed;
                Ok(())
            }
            Err(error) => Err(TransitionFailure::StateCouldNotBeSaved {
                error,
                attempted: proposed,
            }),
        }
    }

    fn run_role(
        &self,
        workspace_root: &Path,
        stage: &Stage,
        direction: Direction,
        verify: bool,
        state: &WorkbenchState,
        log: &mut ExecutionLog<'_>,
    ) -> bool {
        let role = ExecutableRole::for_direction(direction, verify);
        let executable = stage
            .executable(role)
            .expect("transition checked role availability");
        let invocation = Invocation {
            executable: executable.to_path_buf(),
            working_directory: stage.directory.clone(),
            workspace_root: workspace_root.to_path_buf(),
            stage_number: stage.number,
            direction,
            role,
            uuid: state.uuid.clone().unwrap_or_default(),
        };
        (log.observe)(ExecutionProgress::Starting { stage, role });
        let result = self.executable_runner.run(&invocation).map_err(Rc::new);
        let succeeded = result.as_ref().is_ok_and(|output| output.success);
        log.executions.push(ExecutionEvent {
            stage_number: stage.number,
            role,
            result,
        });
        (log.observe)(ExecutionProgress::Finished {
            stage,
            execution: log.executions.last().expect("just recorded role"),
        });
        succeeded
    }
}

fn target_count(stages: &[Stage], target_stage: u32) -> Result<usize, MoveToError> {
    if target_stage == 0 {
        return Ok(0);
    }
    stages
        .iter()
        .position(|stage| stage.number == target_stage)
        .map(|index| index + 1)
        .ok_or_else(|| MoveToError::InvalidTarget(format!("stage {target_stage} was not found")))
}

fn finish_transition(state: &mut WorkbenchState, transition: PendingTransition) {
    state.completed_stage_count = match transition.direction {
        Direction::Up => transition.stage_index + 1,
        Direction::Down => transition.stage_index,
    };
    state.pending = None;
    if state.completed_stage_count == 0 {
        state.uuid = None;
    }
}

fn failure_for_last_execution(executions: &[ExecutionEvent]) -> TransitionFailure {
    let event = executions.last().expect("called only after a failed role");
    match &event.result {
        Err(message) => TransitionFailure::ExecutableCouldNotStart {
            stage_number: event.stage_number,
            role: event.role,
            error: Rc::clone(message),
        },
        Ok(output) => TransitionFailure::ExecutableFailed {
            stage_number: event.stage_number,
            role: event.role,
            exit_code: output.exit_code,
        },
    }
}

#[cfg(test)]
mod stage_definition_contract_tests {
    use super::*;
    use std::{error::Error, io};

    struct Discovery(Vec<Stage>);

    impl StageDiscovery for Discovery {
        fn discover(&self, _: &Path) -> Result<Vec<Stage>, StageDiscoveryError> {
            Ok(self.0.clone())
        }
    }

    struct FailingDiscovery;

    impl StageDiscovery for FailingDiscovery {
        fn discover(&self, _: &Path) -> Result<Vec<Stage>, StageDiscoveryError> {
            Err(StageDiscoveryError::new(io::Error::other(
                "discovery source retained",
            )))
        }
    }

    struct UnusedQueries;

    impl WorkbenchQueries for UnusedQueries {
        fn read_checkpoint(&self) -> Result<Option<WorkbenchState>, PersistenceError> {
            panic!("stage definition lookup must not query checkpoint storage")
        }
    }

    struct UnusedWrites;

    impl WorkbenchWrites for UnusedWrites {
        fn record_checkpoint(&self, _: &WorkbenchState) -> Result<(), PersistenceError> {
            panic!("stage definition lookup must not write checkpoint storage")
        }
    }

    struct UnusedRunner;

    impl ExecutableRunner for UnusedRunner {
        fn run(&self, _: &Invocation) -> Result<ProcessOutput, ExecutableRunError> {
            panic!("stage definition lookup must not run a role")
        }
    }

    struct FailingReader;

    impl StageDefinitionReader for FailingReader {
        fn read(
            &self,
            _: &Path,
            maximum_bytes: usize,
        ) -> Result<StageDefinitionContents, StageDefinitionReadError> {
            assert_eq!(maximum_bytes, 64);
            Err(StageDefinitionReadError::new(io::Error::other(
                "definition read denied",
            )))
        }
    }

    fn build_workbench(discovery: Box<dyn StageDiscovery>) -> Workbench {
        Workbench::new(
            discovery,
            Box::new(UnusedQueries),
            Box::new(UnusedWrites),
            Box::new(UnusedRunner),
            Box::new(FailingReader),
        )
    }

    fn stage() -> Stage {
        let directory = PathBuf::from("stages/007-sample");
        Stage {
            number: 7,
            name: "sample".to_owned(),
            up: Some(directory.join("up")),
            down: None,
            verify_up: None,
            verify_down: None,
            directory,
        }
    }

    #[test]
    fn definition_input_has_narrow_errors_and_keeps_checkpoint_storage_out_of_lookup() {
        let workspace_root = Path::new("workspace");
        let workbench = build_workbench(Box::new(Discovery(vec![stage()])));
        let definitions = workbench
            .stage_definitions(StageDefinitionsInput {
                workspace_root,
                stage_number: 7,
                maximum_bytes: 64,
            })
            .unwrap();
        assert_eq!(definitions.stage.number, 7);
        assert_eq!(definitions.definitions.len(), 1);
        let read_error = definitions.definitions[0].contents.as_ref().err().unwrap();
        assert!(read_error.to_string().contains("definition read denied"));

        let unknown = match workbench.stage_definitions(StageDefinitionsInput {
            workspace_root,
            stage_number: 99,
            maximum_bytes: 64,
        }) {
            Ok(_) => panic!("unknown stage unexpectedly resolved"),
            Err(error) => error,
        };
        assert!(matches!(unknown, StageDefinitionsError::UnknownStage(99)));

        let empty = build_workbench(Box::new(Discovery(Vec::new())));
        let empty_error = match empty.stage_definitions(StageDefinitionsInput {
            workspace_root,
            stage_number: 7,
            maximum_bytes: 64,
        }) {
            Ok(_) => panic!("empty stage discovery unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(matches!(
            &empty_error,
            StageDefinitionsError::StageDiscovery(_)
        ));
        assert!(
            empty_error
                .to_string()
                .contains("no numbered stage directories")
        );

        let failed = build_workbench(Box::new(FailingDiscovery));
        let failure = match failed.stage_definitions(StageDefinitionsInput {
            workspace_root,
            stage_number: 7,
            maximum_bytes: 64,
        }) {
            Ok(_) => panic!("failing stage discovery unexpectedly succeeded"),
            Err(error) => error,
        };
        let capability = failure.source().unwrap();
        assert!(capability.is::<StageDiscoveryError>());
        assert!(capability.source().unwrap().is::<io::Error>());
    }
}
