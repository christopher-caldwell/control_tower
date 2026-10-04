import { useEffect, useRef, useState } from "react";
import { api, submitMovement, ApiFailure, type CheckpointView, type DefinitionView, type MovementChoice, type MovementObservation, type WorkspaceView, type RoleObservation, type StageView, type WorkflowIdentity, type WorkflowView } from "./types";

type Action = { choice: MovementChoice; label: string; detail: string };
function autoStage(view: WorkflowView): number | null {
  const candidates = [view.observation?.active_role?.stage.number, view.observation?.failure?.stage?.number, view.checkpoint?.pending_transition?.stage.number, view.checkpoint?.accepted_stage?.number, view.stages[0]?.number];
  return candidates.find((number) => number !== undefined && view.stages.some((stage) => stage.number === number)) ?? null;
}
export function App() {
  const [leftCollapsed, setLeftCollapsed] = useState(false);
  const [rightCollapsed, setRightCollapsed] = useState(false);
  const [workspace, setWorkspace] = useState<WorkspaceView | null>(null);
  const [workspaceIssue, setWorkspaceIssue] = useState<string | null>(null);
  const [workflowId, setWorkflowId] = useState<string | null>(null);
  const [workflow, setWorkflow] = useState<WorkflowView | null>(null);
  const [stageNumber, setStageNumber] = useState<number | null>(null);
  const [definition, setDefinition] = useState<DefinitionView | null>(null);
  const [definitionIssue, setDefinitionIssue] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [live, setLive] = useState("Connecting");
  const [submitting, setSubmitting] = useState(false);
  const [movementIssue, setMovementIssue] = useState<string | null>(null);
  const selectedWorkflow = useRef<string | null>(null);
  const manualSelection = useRef(false);
  const selectedStageRef = useRef<number | null>(null);
  const movementRequest = useRef(0);

  async function loadWorkspace() {
    try {
      const result = await api<WorkspaceView>("/api/workspace");
      setWorkspace(result);
      setWorkspaceIssue(null);
      setWorkflowId((current) => current && result.workflows.some((item) => item.id === current) ? current : result.workflows[0]?.id ?? null);
    } catch (error) { setWorkspaceIssue(error instanceof Error ? error.message : "Could not reach the local Control Tower host."); }
    finally { setLoading(false); }
  }
  useEffect(() => { void loadWorkspace(); }, []);

  useEffect(() => {
    const id = workflowId;
    selectedWorkflow.current = id;
    movementRequest.current += 1;
    setWorkflow(null); setStageNumber(null); selectedStageRef.current = null; setSubmitting(false); setMovementIssue(null);
    manualSelection.current = false;
    if (!id) { setLive("Disconnected"); return; }
  }, [workflowId]);

  useEffect(() => {
    const id = workflowId;
    if (!id) return;
    let closed = false;
    const source = new EventSource("/api/workflows/" + encodeURIComponent(id) + "/events");
    source.onopen = () => { if (!closed) setLive("Live updates connected"); };
    source.onerror = () => { if (!closed) setLive("Reconnecting"); };
    source.addEventListener("snapshot", (event) => {
      if (closed || selectedWorkflow.current !== id) return;
      try {
        const next = JSON.parse((event as MessageEvent<string>).data) as WorkflowView;
        if (next.workflow.id !== id) return;
        setWorkflow(next);
        if (!manualSelection.current) { const automatic = autoStage(next); selectedStageRef.current = automatic; setStageNumber(automatic); }
        else if (selectedStageRef.current !== null && !next.stages.some((item) => item.number === selectedStageRef.current)) {
          manualSelection.current = false; const automatic = autoStage(next); selectedStageRef.current = automatic; setStageNumber(automatic);
        }
        setMovementIssue(null);
      } catch { if (!closed) setLive("Resynchronizing"); }
    });
    return () => { closed = true; source.close(); };
  }, [workflowId]);

  useEffect(() => {
    setDefinition(null); setDefinitionIssue(null);
    if (!workflowId || stageNumber === null) return;
    const controller = new AbortController();
    api<DefinitionView>("/api/workflows/" + encodeURIComponent(workflowId) + "/stages/" + stageNumber, controller.signal)
      .then(setDefinition).catch((error: unknown) => { if (!controller.signal.aborted) setDefinitionIssue(error instanceof Error ? error.message : "Stage definitions are unavailable."); });
    return () => controller.abort();
  }, [workflowId, stageNumber]);

  const selectedSummary = workflow?.workflow.id === workflowId
    ? workflow.workflow
    : workspace?.workflows.find((item) => item.id === workflowId) ?? null;
  const selectedStage = workflow?.stages.find((stage) => stage.number === stageNumber) ?? null;
  const actions = workflow?.current_status === "available" ? movementActions(workflow) : [];
  const primary = actions[0], alternate = actions[1];
  const busy = Boolean(submitting || workflow?.movement_busy || live !== "Live updates connected");
  const finalAccepted = Boolean(workflow?.checkpoint?.accepted_stage && workflow.checkpoint.accepted_stage.number === workflow.stages.at(-1)?.number && !workflow.checkpoint.pending_transition);
  const finalLabel = workflow?.movement_choices.length ? "All stages applied; backout remains available" : "All stages applied";
  const inactive = !workflowId ? "Select a workflow" : workspaceIssue ? "Workspace unavailable" : !workflow ? "Connecting to workflow" : workflow.current_status === "unavailable" ? "Workflow unavailable" : finalAccepted ? "All stages applied" : "No immediate movement available";

  async function run(action: Action) {
    if (!workflowId || !workflow?.checkpoint || busy || workflow.current_status !== "available") return;
    const id = workflowId, requestId = ++movementRequest.current; setSubmitting(true); setMovementIssue(null);
    try { await submitMovement("/api/workflows/" + encodeURIComponent(id) + "/movements", action.choice, workflow.checkpoint.state); }
    catch (error) {
      if (error instanceof ApiFailure && error.code === "stale_checkpoint") {
        setMovementIssue("Checkpoint changed before the movement. The live view has been refreshed; submit again after it updates.");
      } else {
        setMovementIssue("Movement response was not confirmed: " + (error instanceof Error ? error.message : "delivery failed") + ". Check live status before submitting again.");
      }
    } finally { if (movementRequest.current === requestId) setSubmitting(false); }
  }
  function selectWorkflow(id: string) {
    if (selectedWorkflow.current !== id) manualSelection.current = false;
    selectedWorkflow.current = id; setWorkflowId(id);
  }
  function chooseStage(number: number) { manualSelection.current = true; selectedStageRef.current = number; setStageNumber(number); }

  return <div className="application-frame">
    <header className="topbar"><div className="brand-mark" aria-hidden="true"><span />CT</div><div className="brand-copy"><strong>Control Tower</strong><span>LOCAL WORKBENCH</span></div><div className="topbar-divider" />
      <div className="workspace-crumb"><span className="crumb-label">WORKSPACE</span><strong>{workspace?.name ?? "Connecting"}</strong></div>
      <div className="connection-state"><i className={workspaceIssue || live === "Reconnecting" ? "state-dot state-dot-error" : "state-dot"} />{workspaceIssue ? "Disconnected" : live}</div>
    </header>
    <div className={"workbench-grid " + (leftCollapsed ? "left-collapsed " : "") + (rightCollapsed ? "right-collapsed" : "")}>
      <aside className="workflow-rail" aria-label="Workspace workflows">
        <div className="rail-heading">{!leftCollapsed && <><div><span className="eyebrow">WORKSPACE WORKFLOWS</span><h2>Workflows</h2></div><span className="count-pill">{workspace?.workflows.length ?? "—"}</span></>}
          <button className="rail-toggle" onClick={() => setLeftCollapsed(!leftCollapsed)} aria-label={leftCollapsed ? "Expand workflow rail" : "Collapse workflow rail"}>{leftCollapsed ? "›" : "‹"}</button></div>
        {!leftCollapsed && <><div className="workspace-path"><span className="folder-icon">▰</span><span><b>{workspace?.name ?? "Current workspace"}</b><small>Launch directory</small></span></div>
          <div className="workflow-list">{loading && <div className="rail-message">Discovering workflows…</div>}
            {workspace?.workflows.map((item) => <WorkflowButton key={item.id} item={item} selected={item.id === workflowId} onClick={() => selectWorkflow(item.id)} />)}
            {workspace?.workflows.length === 0 && !loading && <div className="rail-message">No workflow directories were found. Add one under workflows/ and restart the UI.</div>}
            {workspaceIssue && <div className="inline-warning">{workspaceIssue}</div>}</div>
          <div className="rail-footer"><span className="footer-glyph">⌘</span><span><b>Startup discovery</b><small>Restart to add or remove</small></span></div></>}
        {leftCollapsed && <div className="collapsed-workflows">{workspace?.workflows.map((item) => <button key={item.id} className={"workflow-glyph " + (item.id === workflowId ? "selected" : "")} onClick={() => selectWorkflow(item.id)} title={item.name} aria-label={"Select workflow " + item.name}>{item.name.slice(0, 1).toUpperCase()}</button>)}</div>}
      </aside>
      <main className="stage-column" aria-label="Ordered stages">
        <div className="stage-column-scroll">
          <div className="page-heading"><div><div className="eyebrow">WORKFLOW / {selectedSummary?.id ?? "—"}</div><h1>{selectedSummary?.name ?? "Select a workflow"}</h1><p className="page-subtitle">An ordered view of this workflow’s stage definitions and current position.</p></div></div>
          {workflow?.status_issue && <div className="notice notice-error" role="alert"><span className="notice-symbol">!</span><div><b>Current checkpoint unavailable</b><p>{workflow.status_issue}</p><small>Movement is disabled until current status is readable.</small></div></div>}
          {workspaceIssue && <div className="notice notice-error" role="alert"><span className="notice-symbol">!</span><div><b>Workspace unavailable</b><p>{workspaceIssue}</p></div></div>}
          {workflow?.observation?.state === "stopped" && workflow.observation.failure && <div className="notice notice-error" role="alert"><span className="notice-symbol">!</span><div><b>Movement stopped</b><p>{workflow.observation.failure.message}</p></div></div>}
          {workflow?.checkpoint && <><div className="checkpoint-strip"><div className="checkpoint-main"><span className="checkpoint-icon">✓</span><div><span className="eyebrow">CURRENT CHECKPOINT</span><strong>{checkpointPosition(workflow.checkpoint)}</strong></div></div>
            {workflow.checkpoint.pending_transition && <div className="pending-banner"><span className="pending-indicator">◐</span><span><b>Pending {workflow.checkpoint.pending_transition.direction}</b><small>Stage {workflow.checkpoint.pending_transition.stage.number}</small></span></div>}
            {workflow.movement_busy && <div className="active-banner"><span className="active-pulse" /><span><b>Movement active</b><small>{workflow.observation?.active_role ? "Stage " + workflow.observation.active_role.stage.number + " · " + workflow.observation.active_role.role : "Preparing movement"}</small></span></div>}</div>
            <div className="stages-heading"><div><span className="eyebrow">STAGE SEQUENCE</span><h2>Ordered progression</h2></div><span className="stage-count">{workflow.stages.length} {workflow.stages.length === 1 ? "stage" : "stages"}</span></div>
            <div className="stage-list">{workflow.stages.map((stage, index) => <StageCard key={stage.number} stage={stage} index={index} observation={workflow.observation} selected={stage.number === stageNumber} onClick={() => chooseStage(stage.number)} />)}</div>
          </>}
          {!workflow && !loading && <div className="welcome-panel"><div className="welcome-icon">⌁</div><span className="eyebrow">READY WHEN YOU ARE</span><h2>{workspace?.workflows.length === 0 ? "No workflows found" : "Choose a workflow"}</h2><p>{workspace?.workflows.length === 0 ? "Add a workflow directory under workflows/ and restart the UI." : "Select a workflow in the left rail to inspect its checkpoint and ordered stages."}</p></div>}
        </div>
        <div className="action-dock"><div className="action-summary"><span className="action-orbit">{workflow?.movement_busy ? "◌" : "↗"}</span><div><b>{workflow?.observation?.state === "stopped" ? "Movement stopped" : workflow?.observation?.state === "complete" ? "Movement completed" : workflow?.movement_busy ? "Movement is in progress" : finalAccepted ? finalLabel : primary?.label ?? inactive}</b><small>{workflow?.observation?.active_role ? "Attempt in progress · " + workflow.observation.active_role.role + ". Output appears when the role returns." : primary?.detail ?? workflow?.status_issue ?? "No movement is currently available."}</small></div></div>
          {movementIssue && <span className="movement-issue" role="status">{movementIssue}</span>}
          <div className="movement-buttons">{alternate && <button className="secondary-action" onClick={() => void run(alternate)} disabled={busy || !workflow?.checkpoint}>{alternate.label}<span>↶</span></button>}
            {primary && <button className="primary-action" onClick={() => void run(primary)} disabled={busy || !workflow?.checkpoint}>{busy ? "Movement in progress" : primary.label}<span>{primary.choice.direction === "up" ? "→" : "↶"}</span></button>}
            {!primary && <button className="primary-action" disabled>{inactive}<span>{workflow?.stages.length ? "✓" : "·"}</span></button>}</div>
        </div>
      </main>
      <aside className="inspector-rail" aria-label="Selected stage inspector">{rightCollapsed ? <button className="inspector-reopen" onClick={() => setRightCollapsed(false)} aria-label="Expand stage inspector">‹<span>INSPECTOR</span></button> :
        <Inspector selectedStage={selectedStage} definition={definition} definitionIssue={definitionIssue} checkpoint={workflow?.checkpoint ?? null} observation={workflow?.observation ?? null} loading={Boolean(stageNumber !== null && !definition && !definitionIssue)} onCollapse={() => setRightCollapsed(true)} />}</aside>
    </div>
  </div>;
}

function WorkflowButton({ item, selected, onClick }: { item: WorkflowIdentity; selected: boolean; onClick: () => void }) {
  return <button className={"workflow-item " + (selected ? "active" : "")} onClick={onClick} aria-current={selected ? "page" : undefined}>
    <span className={"workflow-item-icon " + (selected ? "active" : "")}>{item.name.slice(0, 1).toUpperCase()}</span><span className="workflow-item-copy"><b>{item.name}</b><small>{selected ? "Selected for inspection" : "Select to inspect"}</small></span>
  </button>;
}
function StageCard({ stage, index, selected, observation, onClick }: { stage: StageView; index: number; selected: boolean; observation: MovementObservation | null; onClick: () => void }) {
  const stateText = stage.state === "accepted" ? "Applied" : stage.state === "pending" ? "Pending transition" : "Not applied";
  const glyph = stage.state === "accepted" ? "✓" : stage.state === "pending" ? "◐" : String(index + 1).padStart(2, "0");
  const active = observation?.active_role?.stage.number === stage.number ? observation.active_role : null;
  return <button className={"stage-card stage-" + stage.state + (selected ? " selected" : "")} onClick={onClick} aria-pressed={selected}>
    <span className={"stage-marker marker-" + stage.state}>{glyph}</span><span className="stage-card-body"><span className="stage-kicker">STAGE {String(stage.number).padStart(3, "0")} <i /> {active ? "Attempt in progress · " + active.role : stateText}</span><strong>{stage.name}</strong><span className="role-summary">{stage.definitions.map((role) => <span key={role.role} className="role-chip">{role.role}</span>)}</span></span><span className="stage-chevron">›</span>
  </button>;
}
function Inspector({ selectedStage, definition, definitionIssue, checkpoint, observation, loading, onCollapse }: {
  selectedStage: StageView | null; definition: DefinitionView | null; definitionIssue: string | null;
  checkpoint: CheckpointView | null; observation: MovementObservation | null; loading: boolean; onCollapse: () => void;
}) {
  const retained = !selectedStage && observation?.role_results.length ? observation : null;
  const results = selectedStage ? observation?.role_results.filter((result) => result.stage.number === selectedStage.number) ?? [] : [];
  const failure = observation?.failure && selectedStage && (observation.failure.stage?.number === selectedStage.number || results.length > 0) ? observation.failure : null;
  return <>
    <div className="inspector-header"><div><span className="eyebrow">DETAILS</span><h2>Stage inspector</h2></div><button className="rail-toggle" onClick={onCollapse} aria-label="Collapse stage inspector">›</button></div>
    <div className="inspector-scroll">{!selectedStage && !retained && <div className="inspector-empty"><span className="empty-icon">⌕</span><b>Select a stage</b><p>Selection only inspects a definition; it never runs a script.</p></div>}
      {retained && <section className="inspector-section"><div className="section-title"><h4>Latest observed output</h4></div><p className="subtle-note">Current stage status is unavailable. These are the latest role results this host observed.</p>{retained.role_results.map((result) => <RoleOutput key={resultIndex(retained, result)} result={result} />)}</section>}
      {selectedStage && <>
        <section className="inspector-stage-title"><span className="stage-index-label">STAGE {String(selectedStage.number).padStart(3, "0")}</span><h3>{selectedStage.name}</h3><span className={"status-pill pill-" + selectedStage.state}>{selectedStage.state === "accepted" ? "Applied" : selectedStage.state === "pending" ? "Pending" : "Future stage"}</span></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">01</span><h4>Checkpoint state</h4></div>
          {failure && <div className="inspector-error" role="status"><b>{failure.kind === "checkpoint_save_failed" ? "Checkpoint was not confirmed" : "Observed failure"}</b><p>{failure.message}</p></div>}
          <div className={"state-card " + (selectedStage.state === "accepted" ? "success" : selectedStage.state === "pending" ? "pending" : "neutral")}><span className="state-card-icon">{selectedStage.state === "accepted" ? "✓" : selectedStage.state === "pending" ? "◐" : "○"}</span><span><b>{selectedStage.state === "accepted" ? "Applied" : selectedStage.state === "pending" ? "Pending " + (checkpoint?.pending_transition?.direction ?? "") : "Not applied"}</b><small>{selectedStage.is_accepted_checkpoint ? "Last confirmed accepted position" : "Current workflow state"}</small></span></div>
        </section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">02</span><h4>Mutation</h4></div><RolePreview role="up" stage={selectedStage} results={results} failure={failure} pendingDirection={checkpoint?.pending_transition?.direction} /><RolePreview role="down" stage={selectedStage} results={results} failure={failure} pendingDirection={checkpoint?.pending_transition?.direction} /></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">03</span><h4>Verification</h4></div><RolePreview role="verify-up" stage={selectedStage} results={results} failure={failure} pendingDirection={checkpoint?.pending_transition?.direction} /><RolePreview role="verify-down" stage={selectedStage} results={results} failure={failure} pendingDirection={checkpoint?.pending_transition?.direction} /></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">04</span><h4>Captured output</h4></div>{results.map((result) => <RoleOutput key={resultIndex(observation, result)} result={result} />)}</section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">05</span><h4>Executable definitions</h4></div>{loading && <div className="definition-loading">Reading stage files…</div>}{definitionIssue && <div className="inline-warning">{definitionIssue}</div>}
          {definition?.definitions.map((role) => <div className="definition-block" key={role.role}><div className="definition-meta"><span>{role.role}</span><code>{role.path}</code></div>{role.issue ? <p className="subtle-note">{role.issue}</p> : <pre>{role.contents}</pre>}</div>)}</section>
      </>}
    </div><div className="inspector-footer"><span className="footer-lock">⌑</span><span>Output is buffered per role and rendered as inert text</span></div>
  </>;
}
function RolePreview({ role, stage, results, failure, pendingDirection }: { role: string; stage: StageView; results: RoleObservation[]; failure: MovementObservation["failure"]; pendingDirection?: "up" | "down" }) {
  const configured = stage.definitions.some((item) => item.role === role);
  const result = results.filter((item) => item.role === role).at(-1);
  const failed = failure?.stage?.number === stage.number && failure.role === role;
  const direction = role === "up" || role === "verify-up" ? "up" : "down";
  const unknownStatus = stage.state === "accepted"
    ? role === "up" ? "Applied · history unavailable" : "No recorded result"
    : stage.state === "pending"
      ? pendingDirection === direction ? "Prior outcome unavailable" : "No recorded result"
      : "Not attempted";
  const status = result ? result.state === "succeeded" ? "Process OK" : result.state === "in_progress" ? "In progress" : result.state === "failed" ? "Exit " + (result.exit_code ?? "unknown") : "Launch failed" : failed ? "Failed" : configured ? unknownStatus : role.startsWith("verify-") ? "Not configured" : "Missing";
  return <div className={"role-row " + (configured ? "configured" : "not-configured")}><span className="role-result-mark">{result?.state === "succeeded" ? "✓" : result?.state === "failed" || result?.state === "launch_failed" || failed ? "!" : result?.state === "in_progress" ? "◌" : "·"}</span><span className="role-row-copy"><b>{role}</b><small>{result?.message ?? status}</small></span><span className={"role-status " + (failed || result?.state === "failed" || result?.state === "launch_failed" ? "error" : "")}>{status}</span></div>;
}
function resultIndex(observation: MovementObservation | null, result: RoleObservation): number { return observation?.role_results.indexOf(result) ?? -1; }
function RoleOutput({ result }: { result: RoleObservation }) {
  if (result.state === "in_progress") return <div className="captured-role"><b>Stage {result.stage.number} · {result.role}</b><p className="subtle-note">Output will be available when the role returns.</p></div>;
  if (result.state === "launch_failed") return <div className="captured-role"><b>Stage {result.stage.number} · {result.role}</b><p className="subtle-note">{result.message ?? "The executable could not be started."}</p></div>;
  return <div className="captured-role"><div className="captured-role-heading"><b>Stage {result.stage.number} · {result.role}</b><span>{result.state === "succeeded" ? "Completed" : "Exit " + (result.exit_code ?? "unknown")}</span></div>
    <CapturedStream streamName="stdout" value={result.stdout ?? ""} />
    <CapturedStream streamName="stderr" value={result.stderr ?? ""} /></div>;
}
function CapturedStream({ streamName, value }: { streamName: "stdout" | "stderr"; value: string }) {
  return <div className="captured-stream"><div className="captured-stream-heading"><span>{streamName}</span></div>
    {value.length === 0 ? <small className="empty-stream">Empty stream</small> : <pre className="output-preview" aria-label={streamName}>{value}</pre>}</div>;
}
function movementActions(view: WorkflowView): Action[] {
  const checkpoint = view.checkpoint; if (!checkpoint) return [];
  const pending = checkpoint.pending_transition, observation = view.observation;
  const recovery = pending && observation?.state === "stopped" ? observation.verification_choices : null;
  const label = (number: number) => number === 0 ? "baseline" : "Stage " + number + " · " + (view.stages.find((item) => item.number === number)?.name ?? "unknown");
  if (recovery && pending) return [
    { choice: recovery.retry, label: "Retry verify-" + recovery.retry.direction + " for Stage " + pending.stage.number, detail: "The mutation is not repeated by this verifier retry." },
    ...(recovery.reverse ? [{ choice: recovery.reverse, label: recovery.reverse.direction === "down" ? "Back out Stage " + pending.stage.number + " to " + label(recovery.reverse.target_stage) : "Reapply Stage " + pending.stage.number + " upward", detail: "Runs the reverse mutation and optional verifier." }] : []),
  ];
  return view.movement_choices.map((choice) => {
    if (pending) return { choice, label: choice.direction === pending.direction ? "Continue pending " + pending.direction + " · Stage " + pending.stage.number : choice.direction === "down" ? "Back out Stage " + pending.stage.number + " to " + label(choice.target_stage) : "Reapply Stage " + pending.stage.number + " upward", detail: "Application supplied this immediate movement." };
    const stage = choice.direction === "up" ? view.stages.find((item) => item.number === choice.target_stage) : checkpoint.accepted_stage;
    return { choice, label: choice.direction === "up" ? "Advance to " + label(choice.target_stage) : "Back out Stage " + (stage?.number ?? "unknown") + " to " + label(choice.target_stage), detail: "Application supplied this immediate movement." };
  });
}
function checkpointPosition(checkpoint: CheckpointView | null): string { return checkpoint?.accepted_stage ? "Stage " + checkpoint.accepted_stage.number + " · " + checkpoint.accepted_stage.name : "Baseline · no accepted stages"; }
