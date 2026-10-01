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
    StateCouldNotBeSaved(PersistenceError),
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
            Self::StateCouldNotBeSaved(message) => {
                write!(formatter, "could not save workbench state: {message}")
            }
        }
    }
}

impl std::error::Error for TransitionFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StateCouldNotBeSaved(error) => Some(error),
            Self::ExecutableCouldNotStart { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum MoveStatus {
    Complete(WorkbenchState),
    Stopped {
        state: WorkbenchState,
        failure: TransitionFailure,
    },
}

#[derive(Debug)]
pub struct MoveOutcome {
    pub executions: Vec<ExecutionEvent>,
    pub status: MoveStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchStatus {
    pub state: WorkbenchState,
    pub stages: Vec<Stage>,
}

pub trait StageDiscovery {
    fn discover(&self, workspace_root: &Path) -> Result<Vec<Stage>, StageDiscoveryError>;
}

pub trait WorkbenchQueries {
    fn read_checkpoint(&self) -> Result<Option<WorkbenchState>, PersistenceError>;
}

pub trait WorkbenchWrites {
    /// Record one orchestration checkpoint as an independently atomic mutation.
    fn record_checkpoint(&self, checkpoint: &WorkbenchState) -> Result<(), PersistenceError>;
}

pub struct MoveToInput<'a> {
    pub workspace_root: &'a Path,
    pub direction: Direction,
    pub target_stage: u32,
}

pub trait ExecutableRunner {
    fn run(&self, invocation: &Invocation) -> Result<ProcessOutput, ExecutableRunError>;
}

pub struct Workbench {
    stage_discovery: Box<dyn StageDiscovery>,
    queries: Box<dyn WorkbenchQueries>,
    writes: Box<dyn WorkbenchWrites>,
    executable_runner: Box<dyn ExecutableRunner>,
}

impl Workbench {
    pub fn new(
        stage_discovery: Box<dyn StageDiscovery>,
        queries: Box<dyn WorkbenchQueries>,
        writes: Box<dyn WorkbenchWrites>,
        executable_runner: Box<dyn ExecutableRunner>,
    ) -> Self {
        Self {
            stage_discovery,
            queries,
            writes,
            executable_runner,
        }
    }

    pub fn move_to(&self, input: MoveToInput<'_>) -> Result<MoveOutcome, MoveToError> {
        let MoveToInput {
            workspace_root,
            direction,
            target_stage,
        } = input;
        let stages = self.load_stages(workspace_root)?;
        let target_count = target_count(&stages, target_stage)?;
        let mut state = self.load_state(stages.len())?;
        let mut executions = Vec::new();

        if let Some(pending) = state.pending {
            // Resolve the active stage toward the requested side before walking
            // farther. The last completed position alone cannot select this action.
            let target_is_reachable = match direction {
                Direction::Up => target_count > pending.stage_index,
                Direction::Down => target_count <= pending.stage_index,
            };
            if !target_is_reachable {
                return Ok(MoveOutcome {
                    executions,
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
                &mut executions,
            ) {
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped { state, failure },
                });
            }
        }

        if target_count == state.completed_stage_count {
            return Ok(MoveOutcome {
                executions,
                status: MoveStatus::Complete(state),
            });
        }

        let walk_is_valid = match direction {
            Direction::Up => target_count > state.completed_stage_count,
            Direction::Down => target_count < state.completed_stage_count,
        };
        if !walk_is_valid {
            return Ok(MoveOutcome {
                executions,
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
                &mut executions,
            ) {
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped { state, failure },
                });
            }
        }

        Ok(MoveOutcome {
            executions,
            status: MoveStatus::Complete(state),
        })
    }

    pub fn status(&self, workspace_root: &Path) -> Result<WorkbenchStatus, StatusError> {
        let stages = self.load_stages(workspace_root)?;
        let state = self.load_state(stages.len())?;
        Ok(WorkbenchStatus { state, stages })
    }

    fn load_stages(&self, workspace_root: &Path) -> Result<Vec<Stage>, StatusError> {
        let stages = self
            .stage_discovery
            .discover(workspace_root)
            .map_err(StatusError::StageDiscovery)?;
        if stages.is_empty() {
            return Err(StatusError::StageDiscovery(StageDiscoveryError::message(
                "no numbered stage directories were found under `stages/`",
            )));
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
        executions: &mut Vec<ExecutionEvent>,
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
                state.uuid = Some(Uuid::new_v4().to_string());
                self.writes
                    .record_checkpoint(state)
                    .map_err(TransitionFailure::StateCouldNotBeSaved)?;
            }
            if !self.run_role(workspace_root, stage, direction, false, state, executions) {
                // A failed reverse mutation does not replace the previously
                // accepted checkpoint or infer any external recovery state.
                return Err(failure_for_last_execution(executions));
            }
        }

        let verifier_role = ExecutableRole::for_direction(direction, true);
        if stage.executable(verifier_role).is_some() {
            if run_mutation {
                state.pending = Some(transition);
                self.writes
                    .record_checkpoint(state)
                    .map_err(TransitionFailure::StateCouldNotBeSaved)?;
            }
            if !self.run_role(workspace_root, stage, direction, true, state, executions) {
                return Err(failure_for_last_execution(executions));
            }
        }

        finish_transition(state, transition);
        self.writes
            .record_checkpoint(state)
            .map_err(TransitionFailure::StateCouldNotBeSaved)
    }

    fn run_role(
        &self,
        workspace_root: &Path,
        stage: &Stage,
        direction: Direction,
        verify: bool,
        state: &WorkbenchState,
        executions: &mut Vec<ExecutionEvent>,
    ) -> bool {
        let role = ExecutableRole::for_direction(direction, verify);
        let Some(executable) = stage.executable(role) else {
            executions.push(ExecutionEvent {
                stage_number: stage.number,
                role,
                result: Err(Rc::new(ExecutableRunError::message(
                    "executable is missing",
                ))),
            });
            return false;
        };
        let invocation = Invocation {
            executable: executable.to_path_buf(),
            working_directory: stage.directory.clone(),
            workspace_root: workspace_root.to_path_buf(),
            stage_number: stage.number,
            direction,
            role,
            uuid: state.uuid.clone().unwrap_or_default(),
        };
        match self.executable_runner.run(&invocation) {
            Ok(output) => {
                let succeeded = output.success;
                executions.push(ExecutionEvent {
                    stage_number: stage.number,
                    role,
                    result: Ok(output),
                });
                succeeded
            }
            Err(message) => {
                executions.push(ExecutionEvent {
                    stage_number: stage.number,
                    role,
                    result: Err(Rc::new(message)),
                });
                false
            }
        }
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
    let Some(event) = executions.last() else {
        return TransitionFailure::StateCouldNotBeSaved(PersistenceError::message(
            "execution stopped without a process result",
        ));
    };
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
