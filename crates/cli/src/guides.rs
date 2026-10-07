use clap::ValueEnum;

const INDEX: &str = include_str!("../guides/index.md");
const CREATE_WORKSPACE: &str = include_str!("../guides/create_workspace.md");
const CREATE_WORKFLOW: &str = include_str!("../guides/create_workflow.md");
const EDIT_WORKFLOW: &str = include_str!("../guides/edit_workflow.md");
const WORKFLOW_CONTRACT: &str = include_str!("../guides/workflow_contract.md");
const OPERATE_WORKFLOW: &str = include_str!("../guides/operate_workflow.md");
const RECOVER_WORKFLOW: &str = include_str!("../guides/recover_workflow.md");

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum GuideAction {
    #[value(name = "create_workspace")]
    CreateWorkspace,
    #[value(name = "create_workflow")]
    CreateWorkflow,
    #[value(name = "edit_workflow")]
    EditWorkflow,
    #[value(name = "workflow_contract")]
    WorkflowContract,
    #[value(name = "operate_workflow")]
    OperateWorkflow,
    #[value(name = "recover_workflow")]
    RecoverWorkflow,
}

pub(super) fn print(action: Option<GuideAction>) {
    let guide = match action {
        None => INDEX,
        Some(GuideAction::CreateWorkspace) => CREATE_WORKSPACE,
        Some(GuideAction::CreateWorkflow) => CREATE_WORKFLOW,
        Some(GuideAction::EditWorkflow) => EDIT_WORKFLOW,
        Some(GuideAction::WorkflowContract) => WORKFLOW_CONTRACT,
        Some(GuideAction::OperateWorkflow) => OPERATE_WORKFLOW,
        Some(GuideAction::RecoverWorkflow) => RECOVER_WORKFLOW,
    };
    print!("{guide}");
}
