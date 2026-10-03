import { useEffect, useRef, useState } from "react";
import { api, submitMovement, ApiFailure, type CheckpointView, type DefinitionView, type MovementChoice, type MovementObservation, type ProjectView, type RoleObservation, type StageView, type WorkspaceIdentity, type WorkspaceView } from "./types";

type Action = { choice: MovementChoice; label: string; detail: string };
function autoStage(view: WorkspaceView): number | null {
  const candidates = [view.observation?.active_role?.stage.number, view.observation?.failure?.stage?.number, view.checkpoint?.pending_transition?.stage.number, view.checkpoint?.accepted_stage?.number, view.stages[0]?.number];
  return candidates.find((number) => number !== undefined && view.stages.some((stage) => stage.number === number)) ?? null;
}
export function App() {
  const [leftCollapsed, setLeftCollapsed] = useState(false);
  const [rightCollapsed, setRightCollapsed] = useState(false);
  const [project, setProject] = useState<ProjectView | null>(null);
  const [projectIssue, setProjectIssue] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [workspace, setWorkspace] = useState<WorkspaceView | null>(null);
  const [stageNumber, setStageNumber] = useState<number | null>(null);
  const [definition, setDefinition] = useState<DefinitionView | null>(null);
  const [definitionIssue, setDefinitionIssue] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [live, setLive] = useState("Connecting");
  const [submitting, setSubmitting] = useState(false);
  const [movementIssue, setMovementIssue] = useState<string | null>(null);
  const selectedWorkspace = useRef<string | null>(null);
  const manualSelection = useRef(false);
  const selectedStageRef = useRef<number | null>(null);
  const movementRequest = useRef(0);

  async function loadProject() {
    try {
      const result = await api<ProjectView>("/api/project");
      setProject(result);
      setProjectIssue(null);
      setWorkspaceId((current) => current && result.workspaces.some((item) => item.id === current) ? current : result.workspaces[0]?.id ?? null);
    } catch (error) { setProjectIssue(error instanceof Error ? error.message : "Could not reach the local Control Tower host."); }
    finally { setLoading(false); }
  }
  useEffect(() => { void loadProject(); }, []);

  useEffect(() => {
    const id = workspaceId;
    selectedWorkspace.current = id;
    movementRequest.current += 1;
    setWorkspace(null); setStageNumber(null); selectedStageRef.current = null; setSubmitting(false); setMovementIssue(null);
    manualSelection.current = false;
    if (!id) { setLive("Disconnected"); return; }
  }, [workspaceId]);

  useEffect(() => {
    const id = workspaceId;
    if (!id) return;
    let closed = false;
    const source = new EventSource("/api/workspaces/" + encodeURIComponent(id) + "/events");
    source.onopen = () => { if (!closed) setLive("Live updates connected"); };
    source.onerror = () => { if (!closed) setLive("Reconnecting"); };
    source.addEventListener("snapshot", (event) => {
      if (closed || selectedWorkspace.current !== id) return;
      try {
        const next = JSON.parse((event as MessageEvent<string>).data) as WorkspaceView;
        if (next.workspace.id !== id) return;
        setWorkspace(next);
        if (!manualSelection.current) { const automatic = autoStage(next); selectedStageRef.current = automatic; setStageNumber(automatic); }
        else if (selectedStageRef.current !== null && !next.stages.some((item) => item.number === selectedStageRef.current)) {
          manualSelection.current = false; const automatic = autoStage(next); selectedStageRef.current = automatic; setStageNumber(automatic);
        }
        setMovementIssue(null);
      } catch { if (!closed) setLive("Resynchronizing"); }
    });
    return () => { closed = true; source.close(); };
  }, [workspaceId]);

  useEffect(() => {
    setDefinition(null); setDefinitionIssue(null);
    if (!workspaceId || stageNumber === null) return;
    const controller = new AbortController();
    api<DefinitionView>("/api/workspaces/" + encodeURIComponent(workspaceId) + "/stages/" + stageNumber, controller.signal)
      .then(setDefinition).catch((error: unknown) => { if (!controller.signal.aborted) setDefinitionIssue(error instanceof Error ? error.message : "Stage definitions are unavailable."); });
    return () => controller.abort();
  }, [workspaceId, stageNumber]);

  const selectedSummary = workspace?.workspace.id === workspaceId
    ? workspace.workspace
    : project?.workspaces.find((item) => item.id === workspaceId) ?? null;
  const selectedStage = workspace?.stages.find((stage) => stage.number === stageNumber) ?? null;
  const actions = workspace?.current_status === "available" ? movementActions(workspace) : [];
  const primary = actions[0], alternate = actions[1];
  const busy = Boolean(submitting || workspace?.movement_busy || live !== "Live updates connected");
  const finalAccepted = Boolean(workspace?.checkpoint?.accepted_stage && workspace.checkpoint.accepted_stage.number === workspace.stages.at(-1)?.number && !workspace.checkpoint.pending_transition);
  const finalLabel = workspace?.movement_choices.length ? "All stages applied; backout remains available" : "All stages applied";
  const inactive = !workspaceId ? "Select a workspace" : projectIssue ? "Project unavailable" : !workspace ? "Connecting to workspace" : workspace.current_status === "unavailable" ? "Workspace unavailable" : finalAccepted ? "All stages applied" : "No immediate movement available";

  async function run(action: Action) {
    if (!workspaceId || !workspace?.checkpoint || busy || workspace.current_status !== "available") return;
    const id = workspaceId, requestId = ++movementRequest.current; setSubmitting(true); setMovementIssue(null);
    try { await submitMovement("/api/workspaces/" + encodeURIComponent(id) + "/movements", action.choice, workspace.checkpoint.state); }
    catch (error) {
      if (error instanceof ApiFailure && error.code === "stale_checkpoint") {
        setMovementIssue("Checkpoint changed before the movement. The live view has been refreshed; submit again after it updates.");
      } else {
        setMovementIssue("Movement response was not confirmed: " + (error instanceof Error ? error.message : "delivery failed") + ". Check live status before submitting again.");
      }
    } finally { if (movementRequest.current === requestId) setSubmitting(false); }
  }
  function selectWorkspace(id: string) {
    if (selectedWorkspace.current !== id) manualSelection.current = false;
    selectedWorkspace.current = id; setWorkspaceId(id);
  }
  function chooseStage(number: number) { manualSelection.current = true; selectedStageRef.current = number; setStageNumber(number); }

  return <div className="application-frame">
    <header className="topbar"><div className="brand-mark" aria-hidden="true"><span />CT</div><div className="brand-copy"><strong>Control Tower</strong><span>LOCAL WORKBENCH</span></div><div className="topbar-divider" />
      <div className="project-crumb"><span className="crumb-label">PROJECT</span><strong>{project?.name ?? "Connecting"}</strong></div>
      <div className="connection-state"><i className={projectIssue || live === "Reconnecting" ? "state-dot state-dot-error" : "state-dot"} />{projectIssue ? "Disconnected" : live}</div>
    </header>
    <div className={"workbench-grid " + (leftCollapsed ? "left-collapsed " : "") + (rightCollapsed ? "right-collapsed" : "")}>
      <aside className="workspace-rail" aria-label="Project workspaces">
        <div className="rail-heading">{!leftCollapsed && <><div><span className="eyebrow">PROJECT WORKSPACES</span><h2>Workspaces</h2></div><span className="count-pill">{project?.workspaces.length ?? "—"}</span></>}
          <button className="rail-toggle" onClick={() => setLeftCollapsed(!leftCollapsed)} aria-label={leftCollapsed ? "Expand workspace rail" : "Collapse workspace rail"}>{leftCollapsed ? "›" : "‹"}</button></div>
        {!leftCollapsed && <><div className="project-path"><span className="folder-icon">▰</span><span><b>{project?.name ?? "Current project"}</b><small>Launch directory</small></span></div>
          <div className="workspace-list">{loading && <div className="rail-message">Discovering workspaces…</div>}
            {project?.workspaces.map((item) => <WorkspaceButton key={item.id} item={item} selected={item.id === workspaceId} onClick={() => selectWorkspace(item.id)} />)}
            {project?.workspaces.length === 0 && !loading && <div className="rail-message">No workspace directories were found. Add one under workspaces/ and restart the UI.</div>}
            {projectIssue && <div className="inline-warning">{projectIssue}</div>}</div>
          <div className="rail-footer"><span className="footer-glyph">⌘</span><span><b>Startup discovery</b><small>Restart to add or remove</small></span></div></>}
        {leftCollapsed && <div className="collapsed-workspaces">{project?.workspaces.map((item) => <button key={item.id} className={"workspace-glyph " + (item.id === workspaceId ? "selected" : "")} onClick={() => selectWorkspace(item.id)} title={item.name} aria-label={"Select workspace " + item.name}>{item.name.slice(0, 1).toUpperCase()}</button>)}</div>}
      </aside>
      <main className="stage-column" aria-label="Ordered stages">
        <div className="stage-column-scroll">
          <div className="page-heading"><div><div className="eyebrow">WORKSPACE / {selectedSummary?.id ?? "—"}</div><h1>{selectedSummary?.name ?? "Select a workspace"}</h1><p className="page-subtitle">An ordered view of this workspace’s stage definitions and current position.</p></div></div>
          {workspace?.status_issue && <div className="notice notice-error" role="alert"><span className="notice-symbol">!</span><div><b>Current checkpoint unavailable</b><p>{workspace.status_issue}</p><small>Movement is disabled until current status is readable.</small></div></div>}
          {projectIssue && <div className="notice notice-error" role="alert"><span className="notice-symbol">!</span><div><b>Project unavailable</b><p>{projectIssue}</p></div></div>}
          {workspace?.observation?.state === "stopped" && workspace.observation.failure && <div className="notice notice-error" role="alert"><span className="notice-symbol">!</span><div><b>Movement stopped</b><p>{workspace.observation.failure.message}</p></div></div>}
          {workspace?.checkpoint && <><div className="checkpoint-strip"><div className="checkpoint-main"><span className="checkpoint-icon">✓</span><div><span className="eyebrow">CURRENT CHECKPOINT</span><strong>{checkpointPosition(workspace.checkpoint)}</strong></div></div>
            {workspace.checkpoint.pending_transition && <div className="pending-banner"><span className="pending-indicator">◐</span><span><b>Pending {workspace.checkpoint.pending_transition.direction}</b><small>Stage {workspace.checkpoint.pending_transition.stage.number}</small></span></div>}
            {workspace.movement_busy && <div className="active-banner"><span className="active-pulse" /><span><b>Movement active</b><small>{workspace.observation?.active_role ? "Stage " + workspace.observation.active_role.stage.number + " · " + workspace.observation.active_role.role : "Preparing movement"}</small></span></div>}</div>
            <div className="stages-heading"><div><span className="eyebrow">STAGE SEQUENCE</span><h2>Ordered progression</h2></div><span className="stage-count">{workspace.stages.length} {workspace.stages.length === 1 ? "stage" : "stages"}</span></div>
            <div className="stage-list">{workspace.stages.map((stage, index) => <StageCard key={stage.number} stage={stage} index={index} observation={workspace.observation} selected={stage.number === stageNumber} onClick={() => chooseStage(stage.number)} />)}</div>
          </>}
          {!workspace && !loading && <div className="welcome-panel"><div className="welcome-icon">⌁</div><span className="eyebrow">READY WHEN YOU ARE</span><h2>{project?.workspaces.length === 0 ? "No workspaces found" : "Choose a workspace"}</h2><p>{project?.workspaces.length === 0 ? "Add a workspace directory under workspaces/ and restart the UI." : "Select a workspace in the left rail to inspect its checkpoint and ordered stages."}</p></div>}
        </div>
        <div className="action-dock"><div className="action-summary"><span className="action-orbit">{workspace?.movement_busy ? "◌" : "↗"}</span><div><b>{workspace?.observation?.state === "stopped" ? "Movement stopped" : workspace?.observation?.state === "complete" ? "Movement completed" : workspace?.movement_busy ? "Movement is in progress" : finalAccepted ? finalLabel : primary?.label ?? inactive}</b><small>{workspace?.observation?.active_role ? "Attempt in progress · " + workspace.observation.active_role.role + ". Output appears when the role returns." : primary?.detail ?? workspace?.status_issue ?? "No movement is currently available."}</small></div></div>
          {movementIssue && <span className="movement-issue" role="status">{movementIssue}</span>}
          <div className="movement-buttons">{alternate && <button className="secondary-action" onClick={() => void run(alternate)} disabled={busy || !workspace?.checkpoint}>{alternate.label}<span>↶</span></button>}
            {primary && <button className="primary-action" onClick={() => void run(primary)} disabled={busy || !workspace?.checkpoint}>{busy ? "Movement in progress" : primary.label}<span>{primary.choice.direction === "up" ? "→" : "↶"}</span></button>}
            {!primary && <button className="primary-action" disabled>{inactive}<span>{workspace?.stages.length ? "✓" : "·"}</span></button>}</div>
        </div>
      </main>
      <aside className="inspector-rail" aria-label="Selected stage inspector">{rightCollapsed ? <button className="inspector-reopen" onClick={() => setRightCollapsed(false)} aria-label="Expand stage inspector">‹<span>INSPECTOR</span></button> :
        <Inspector selectedStage={selectedStage} definition={definition} definitionIssue={definitionIssue} checkpoint={workspace?.checkpoint ?? null} observation={workspace?.observation ?? null} loading={Boolean(stageNumber !== null && !definition && !definitionIssue)} onCollapse={() => setRightCollapsed(true)} />}</aside>
    </div>
  </div>;
}

function WorkspaceButton({ item, selected, onClick }: { item: WorkspaceIdentity; selected: boolean; onClick: () => void }) {
  return <button className={"workspace-item " + (selected ? "active" : "")} onClick={onClick} aria-current={selected ? "page" : undefined}>
    <span className={"workspace-item-icon " + (selected ? "active" : "")}>{item.name.slice(0, 1).toUpperCase()}</span><span className="workspace-item-copy"><b>{item.name}</b><small>{selected ? "Selected for inspection" : "Select to inspect"}</small></span>
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
  const results = selectedStage ? observation?.role_results.filter((result) => result.stage.number === selectedStage.number) ?? [] : [];
  const failure = observation?.failure && selectedStage && (observation.failure.stage?.number === selectedStage.number || results.length > 0) ? observation.failure : null;
  return <>
    <div className="inspector-header"><div><span className="eyebrow">DETAILS</span><h2>Stage inspector</h2></div><button className="rail-toggle" onClick={onCollapse} aria-label="Collapse stage inspector">›</button></div>
    <div className="inspector-scroll">{!selectedStage && <div className="inspector-empty"><span className="empty-icon">⌕</span><b>Select a stage</b><p>Selection only inspects a definition; it never runs a script.</p></div>}
      {selectedStage && <>
        <section className="inspector-stage-title"><span className="stage-index-label">STAGE {String(selectedStage.number).padStart(3, "0")}</span><h3>{selectedStage.name}</h3><span className={"status-pill pill-" + selectedStage.state}>{selectedStage.state === "accepted" ? "Applied" : selectedStage.state === "pending" ? "Pending" : "Future stage"}</span></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">01</span><h4>Checkpoint state</h4></div>
          {failure && <div className="inspector-error" role="status"><b>{failure.kind === "checkpoint_save_failed" ? "Checkpoint was not confirmed" : "Observed failure"}</b><p>{failure.message}</p></div>}
          <div className={"state-card " + (selectedStage.state === "accepted" ? "success" : selectedStage.state === "pending" ? "pending" : "neutral")}><span className="state-card-icon">{selectedStage.state === "accepted" ? "✓" : selectedStage.state === "pending" ? "◐" : "○"}</span><span><b>{selectedStage.state === "accepted" ? "Applied" : selectedStage.state === "pending" ? "Pending " + (checkpoint?.pending_transition?.direction ?? "") : "Not applied"}</b><small>{selectedStage.is_accepted_checkpoint ? "Last confirmed accepted position" : "Current workspace state"}</small></span></div>
        </section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">02</span><h4>Mutation</h4></div><RolePreview role="up" stage={selectedStage} results={results} failure={failure} /><RolePreview role="down" stage={selectedStage} results={results} failure={failure} /></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">03</span><h4>Verification</h4></div><RolePreview role="verify-up" stage={selectedStage} results={results} failure={failure} /><RolePreview role="verify-down" stage={selectedStage} results={results} failure={failure} /></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">04</span><h4>Captured output</h4></div>{results.map((result) => <RoleOutput key={resultIndex(observation, result)} result={result} />)}</section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">05</span><h4>Executable definitions</h4></div>{loading && <div className="definition-loading">Reading stage files…</div>}{definitionIssue && <div className="inline-warning">{definitionIssue}</div>}
          {definition?.definitions.map((role) => <div className="definition-block" key={role.role}><div className="definition-meta"><span>{role.role}</span><code>{role.path}</code></div>{role.issue ? <p className="subtle-note">{role.issue}</p> : <pre>{role.contents}</pre>}</div>)}</section>
      </>}
    </div><div className="inspector-footer"><span className="footer-lock">⌑</span><span>Output is buffered per role and rendered as inert text</span></div>
  </>;
}
function RolePreview({ role, stage, results, failure }: { role: string; stage: StageView; results: RoleObservation[]; failure: MovementObservation["failure"] }) {
  const configured = stage.definitions.some((item) => item.role === role);
  const result = results.filter((item) => item.role === role).at(-1);
  const failed = failure?.stage?.number === stage.number && failure.role === role;
  const status = result ? result.state === "succeeded" ? "Process OK" : result.state === "in_progress" ? "In progress" : result.state === "failed" ? "Exit " + (result.exit_code ?? "unknown") : "Launch failed" : failed ? "Failed" : configured ? stage.state === "accepted" ? "Applied · history unavailable" : stage.state === "pending" ? "Prior outcome unavailable" : "Not attempted" : role.startsWith("verify-") ? "Not configured" : "Missing";
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
function movementActions(view: WorkspaceView): Action[] {
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
