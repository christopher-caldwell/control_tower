use clap::ValueEnum;

const INDEX: &str = include_str!("../guides/index.md");
const CREATE_WORKSPACE: &str = include_str!("../guides/create_workspace.md");
const EDIT_WORKSPACE: &str = include_str!("../guides/edit_workspace.md");
const WORKSPACE_CONTRACT: &str = include_str!("../guides/workspace_contract.md");
const OPERATE_WORKSPACE: &str = include_str!("../guides/operate_workspace.md");
const RECOVER_WORKSPACE: &str = include_str!("../guides/recover_workspace.md");

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum GuideAction {
    #[value(name = "create_workspace")]
    CreateWorkspace,
    #[value(name = "edit_workspace")]
    EditWorkspace,
    #[value(name = "workspace_contract")]
    WorkspaceContract,
    #[value(name = "operate_workspace")]
    OperateWorkspace,
    #[value(name = "recover_workspace")]
    RecoverWorkspace,
}

pub(super) fn print(action: Option<GuideAction>) {
    let guide = match action {
        None => INDEX,
        Some(GuideAction::CreateWorkspace) => CREATE_WORKSPACE,
        Some(GuideAction::EditWorkspace) => EDIT_WORKSPACE,
        Some(GuideAction::WorkspaceContract) => WORKSPACE_CONTRACT,
        Some(GuideAction::OperateWorkspace) => OPERATE_WORKSPACE,
        Some(GuideAction::RecoverWorkspace) => RECOVER_WORKSPACE,
    };
    print!("{guide}");
}
