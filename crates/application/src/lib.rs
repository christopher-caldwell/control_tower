use std::fmt;
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
    /// Number of entries in the discovered ordered stage list that have completed.
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvent {
    pub stage_number: u32,
    pub role: ExecutableRole,
    pub result: Result<ProcessOutput, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
        message: String,
    },
    ConflictingPendingTransition {
        stage_number: u32,
        direction: Direction,
    },
    DirectionDoesNotReachTarget {
        direction: Direction,
    },
    StateCouldNotBeSaved(String),
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
                message,
            } => {
                write!(
                    formatter,
                    "could not start stage {stage_number} {role}: {message}"
                )
            }
            Self::ConflictingPendingTransition {
                stage_number,
                direction,
            } => write!(
                formatter,
                "stage {stage_number} has an unfinished {direction} transition; retry that direction first"
            ),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveStatus {
    Complete(WorkbenchState),
    Stopped {
        state: WorkbenchState,
        failure: TransitionFailure,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveOutcome {
    pub executions: Vec<ExecutionEvent>,
    pub status: MoveStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchStatus {
    pub state: WorkbenchState,
    pub stages: Vec<Stage>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WorkbenchError {
    StageDiscovery(String),
    StateStore(String),
    InvalidState(String),
    InvalidTarget(String),
}

impl fmt::Display for WorkbenchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StageDiscovery(message) => write!(formatter, "stage discovery failed: {message}"),
            Self::StateStore(message) => write!(formatter, "workbench state failed: {message}"),
            Self::InvalidState(message) => {
                write!(formatter, "invalid stored workbench state: {message}")
            }
            Self::InvalidTarget(message) => write!(formatter, "invalid target: {message}"),
        }
    }
}

impl std::error::Error for WorkbenchError {}

pub trait StageDiscovery {
    fn discover(&self, workspace_root: &Path) -> Result<Vec<Stage>, String>;
}

pub trait WorkbenchStateStore {
    fn load(&self) -> Result<Option<WorkbenchState>, String>;
    fn save(&self, state: &WorkbenchState) -> Result<(), String>;
}

pub trait ExecutableRunner {
    fn run(&self, invocation: &Invocation) -> Result<ProcessOutput, String>;
}

pub struct Workbench {
    stage_discovery: Box<dyn StageDiscovery>,
    state_store: Box<dyn WorkbenchStateStore>,
    executable_runner: Box<dyn ExecutableRunner>,
}

impl Workbench {
    pub fn new(
        stage_discovery: Box<dyn StageDiscovery>,
        state_store: Box<dyn WorkbenchStateStore>,
        executable_runner: Box<dyn ExecutableRunner>,
    ) -> Self {
        Self {
            stage_discovery,
            state_store,
            executable_runner,
        }
    }

    pub fn move_to(
        &self,
        workspace_root: &Path,
        direction: Direction,
        target_stage: u32,
    ) -> Result<MoveOutcome, WorkbenchError> {
        let stages = self.load_stages(workspace_root)?;
        let target_count = target_count(&stages, target_stage)?;
        let mut state = self.load_state(stages.len())?;
        let mut executions = Vec::new();

        if let Some(pending) = state.pending {
            let target_matches_pending = match pending.direction {
                Direction::Up => direction == Direction::Up && target_count > pending.stage_index,
                Direction::Down => {
                    direction == Direction::Down && target_count <= pending.stage_index
                }
            };
            if !target_matches_pending {
                let stage_number = stages[pending.stage_index].number;
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped {
                        state,
                        failure: TransitionFailure::ConflictingPendingTransition {
                            stage_number,
                            direction: pending.direction,
                        },
                    },
                });
            }

            if !self.run_role(
                workspace_root,
                &stages[pending.stage_index],
                pending.direction,
                true,
                &state,
                &mut executions,
            ) {
                let failure = failure_for_last_execution(&executions);
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped { state, failure },
                });
            }

            finish_transition(&mut state, pending);
            if let Err(message) = self.state_store.save(&state) {
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped {
                        state,
                        failure: TransitionFailure::StateCouldNotBeSaved(message),
                    },
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
            let stage = &stages[stage_index];
            let mutation_role = ExecutableRole::for_direction(direction, false);
            if stage.executable(mutation_role).is_none() {
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped {
                        state,
                        failure: TransitionFailure::MissingExecutable {
                            stage_number: stage.number,
                            role: mutation_role,
                        },
                    },
                });
            }

            if state.uuid.is_none() {
                state.uuid = Some(Uuid::new_v4().to_string());
                if let Err(message) = self.state_store.save(&state) {
                    return Ok(MoveOutcome {
                        executions,
                        status: MoveStatus::Stopped {
                            state,
                            failure: TransitionFailure::StateCouldNotBeSaved(message),
                        },
                    });
                }
            }

            if !self.run_role(
                workspace_root,
                stage,
                direction,
                false,
                &state,
                &mut executions,
            ) {
                let failure = failure_for_last_execution(&executions);
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped { state, failure },
                });
            }

            let verifier_role = ExecutableRole::for_direction(direction, true);
            let transition = PendingTransition {
                stage_index,
                direction,
            };
            if stage.executable(verifier_role).is_some() {
                state.pending = Some(transition);
                if let Err(message) = self.state_store.save(&state) {
                    return Ok(MoveOutcome {
                        executions,
                        status: MoveStatus::Stopped {
                            state,
                            failure: TransitionFailure::StateCouldNotBeSaved(message),
                        },
                    });
                }

                if !self.run_role(
                    workspace_root,
                    stage,
                    direction,
                    true,
                    &state,
                    &mut executions,
                ) {
                    let failure = failure_for_last_execution(&executions);
                    return Ok(MoveOutcome {
                        executions,
                        status: MoveStatus::Stopped { state, failure },
                    });
                }
            }

            finish_transition(&mut state, transition);
            if let Err(message) = self.state_store.save(&state) {
                return Ok(MoveOutcome {
                    executions,
                    status: MoveStatus::Stopped {
                        state,
                        failure: TransitionFailure::StateCouldNotBeSaved(message),
                    },
                });
            }
        }

        Ok(MoveOutcome {
            executions,
            status: MoveStatus::Complete(state),
        })
    }

    pub fn status(&self, workspace_root: &Path) -> Result<WorkbenchStatus, WorkbenchError> {
        let stages = self.load_stages(workspace_root)?;
        let state = self.load_state(stages.len())?;
        Ok(WorkbenchStatus { state, stages })
    }

    fn load_stages(&self, workspace_root: &Path) -> Result<Vec<Stage>, WorkbenchError> {
        let stages = self
            .stage_discovery
            .discover(workspace_root)
            .map_err(WorkbenchError::StageDiscovery)?;
        if stages.is_empty() {
            return Err(WorkbenchError::StageDiscovery(
                "no numbered stage directories were found under `stages/`".to_owned(),
            ));
        }
        Ok(stages)
    }

    fn load_state(&self, stage_count: usize) -> Result<WorkbenchState, WorkbenchError> {
        let state = self
            .state_store
            .load()
            .map_err(WorkbenchError::StateStore)?
            .unwrap_or_default();
        if state.completed_stage_count > stage_count {
            return Err(WorkbenchError::InvalidState(format!(
                "{} completed stages exceed the {} discovered stages",
                state.completed_stage_count, stage_count
            )));
        }
        if let Some(pending) = state.pending {
            let consistent = pending.stage_index < stage_count
                && match pending.direction {
                    Direction::Up => pending.stage_index == state.completed_stage_count,
                    Direction::Down => {
                        state.completed_stage_count > 0
                            && pending.stage_index + 1 == state.completed_stage_count
                    }
                };
            if !consistent {
                return Err(WorkbenchError::InvalidState(
                    "pending transition does not match the completed stage position".to_owned(),
                ));
            }
        }
        if (state.completed_stage_count > 0 || state.pending.is_some()) && state.uuid.is_none() {
            return Err(WorkbenchError::InvalidState(
                "the workbench has stage state but no UUID".to_owned(),
            ));
        }
        Ok(state)
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
                result: Err("executable is missing".to_owned()),
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
                    result: Err(message),
                });
                false
            }
        }
    }
}

fn target_count(stages: &[Stage], target_stage: u32) -> Result<usize, WorkbenchError> {
    if target_stage == 0 {
        return Ok(0);
    }
    stages
        .iter()
        .position(|stage| stage.number == target_stage)
        .map(|index| index + 1)
        .ok_or_else(|| WorkbenchError::InvalidTarget(format!("stage {target_stage} was not found")))
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
        return TransitionFailure::StateCouldNotBeSaved(
            "execution stopped without a process result".to_owned(),
        );
    };
    match &event.result {
        Err(message) => TransitionFailure::ExecutableCouldNotStart {
            stage_number: event.stage_number,
            role: event.role,
            message: message.clone(),
        },
        Ok(output) => TransitionFailure::ExecutableFailed {
            stage_number: event.stage_number,
            role: event.role,
            exit_code: output.exit_code,
        },
    }
}
