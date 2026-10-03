import { useEffect, useRef, useState } from "react";
import { api, type DefinitionView, type ProjectView, type StageView, type WorkspaceSummary, type WorkspaceView } from "./types";

type Preferences = { leftCollapsed: boolean; rightCollapsed: boolean; rightWidth: number };
const preferenceKey = "control-tower-workbench-layout-v1";
const defaultPreferences: Preferences = { leftCollapsed: false, rightCollapsed: false, rightWidth: 382 };

function loadPreferences(): Preferences {
  try {
    const stored = localStorage.getItem(preferenceKey);
    return stored ? { ...defaultPreferences, ...JSON.parse(stored) as Partial<Preferences> } : defaultPreferences;
  } catch {
    return defaultPreferences;
  }
}

function bootstrapSession(): Promise<void> {
  const params = new URLSearchParams(location.hash.slice(1));
  const token = params.get("session");
  if (!token) return Promise.resolve();
  history.replaceState(null, "", `${location.pathname}${location.search}`);
  return fetch("/api/session", {
    method: "POST",
    credentials: "same-origin",
    headers: { Authorization: `Bearer ${token}` },
  }).then((response) => {
    if (!response.ok) throw new Error("Could not establish the terminal session. Relaunch the UI from the terminal.");
  });
}

export function App() {
  const [preferences, setPreferences] = useState(loadPreferences);
  const [project, setProject] = useState<ProjectView | null>(null);
  const [projectIssue, setProjectIssue] = useState<string | null>(null);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(null);
  const [workspace, setWorkspace] = useState<WorkspaceView | null>(null);
  const [workspaceIssue, setWorkspaceIssue] = useState<string | null>(null);
  const [selectedStageNumber, setSelectedStageNumber] = useState<number | null>(null);
  const [definition, setDefinition] = useState<DefinitionView | null>(null);
  const [definitionIssue, setDefinitionIssue] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [workspaceRefreshVersion, setWorkspaceRefreshVersion] = useState(0);
  const detailRevision = useRef(0);
  const definitionRevision = useRef(0);
  const previousWorkspaceId = useRef<string | null>(null);

  useEffect(() => {
    localStorage.setItem(preferenceKey, JSON.stringify(preferences));
  }, [preferences]);

  async function loadProject(showRefreshing = false) {
    if (showRefreshing) setRefreshing(true);
    setProjectIssue(null);
    try {
      await bootstrapSession();
      const view = await api<ProjectView>("/api/project");
      setProject(view);
      if (showRefreshing) setWorkspaceRefreshVersion((version) => version + 1);
      setSelectedWorkspaceId((current) => {
        if (current && view.workspaces.some((item) => item.id === current)) return current;
        return view.workspaces.find((item) => item.available)?.id ?? view.workspaces[0]?.id ?? null;
      });
    } catch (error) {
      setProjectIssue(error instanceof Error ? error.message : "Could not reach the local Control Tower host.");
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }

  useEffect(() => {
    void loadProject();
  }, []);

  useEffect(() => {
    const revision = ++detailRevision.current;
    const workspaceChanged = previousWorkspaceId.current !== selectedWorkspaceId;
    previousWorkspaceId.current = selectedWorkspaceId;
    if (workspaceChanged) {
      setWorkspace(null);
      setSelectedStageNumber(null);
      setDefinition(null);
      setDefinitionIssue(null);
    }
    setWorkspaceIssue(null);
    if (!selectedWorkspaceId) return;
    const controller = new AbortController();
    api<WorkspaceView>(`/api/workspaces/${encodeURIComponent(selectedWorkspaceId)}`, controller.signal)
      .then((view) => {
        if (revision !== detailRevision.current) return;
        setWorkspace(view);
        setSelectedStageNumber((current) => current !== null && view.stages.some((stage) => stage.number === current)
          ? current
          : view.selected_stage_number);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted || revision !== detailRevision.current) return;
        setWorkspaceIssue(error instanceof Error ? error.message : "Workspace details are unavailable.");
      });
    return () => controller.abort();
  }, [selectedWorkspaceId, workspaceRefreshVersion]);

  useEffect(() => {
    const revision = ++definitionRevision.current;
    setDefinition(null);
    setDefinitionIssue(null);
    if (!selectedWorkspaceId || selectedStageNumber === null) return;
    const controller = new AbortController();
    api<DefinitionView>(
      `/api/workspaces/${encodeURIComponent(selectedWorkspaceId)}/stages/${selectedStageNumber}`,
      controller.signal,
    )
      .then((view) => {
        if (revision === definitionRevision.current) setDefinition(view);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted || revision !== definitionRevision.current) return;
        setDefinitionIssue(error instanceof Error ? error.message : "Stage definitions are unavailable.");
      });
    return () => controller.abort();
  }, [selectedWorkspaceId, selectedStageNumber, workspaceRefreshVersion]);

  const selectedSummary = project?.workspaces.find((item) => item.id === selectedWorkspaceId) ?? null;
  const selectedStage = workspace?.stages.find((stage) => stage.number === selectedStageNumber) ?? null;
  const setPref = (patch: Partial<Preferences>) => setPreferences((current) => ({ ...current, ...patch }));

  function resizeInspector(event: React.PointerEvent<HTMLButtonElement>) {
    const startX = event.clientX;
    const startWidth = preferences.rightWidth;
    const pointerId = event.pointerId;
    event.currentTarget.setPointerCapture(pointerId);
    const move = (next: PointerEvent) => {
      setPref({ rightWidth: Math.max(316, Math.min(620, startWidth + startX - next.clientX)) });
    };
    const done = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", done);
      window.removeEventListener("pointercancel", done);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", done, { once: true });
    window.addEventListener("pointercancel", done, { once: true });
  }

  function resizeWithKeyboard(event: React.KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "ArrowLeft") setPref({ rightWidth: Math.min(620, preferences.rightWidth + 16) });
    if (event.key === "ArrowRight") setPref({ rightWidth: Math.max(316, preferences.rightWidth - 16) });
  }

  return (
    <div className="application-frame" style={{ "--inspector-width": `${preferences.rightWidth}px` } as React.CSSProperties}>
      <header className="topbar">
        <div className="brand-mark" aria-hidden="true"><span />CT</div>
        <div className="brand-copy"><strong>Control Tower</strong><span>LOCAL WORKBENCH</span></div>
        <div className="topbar-divider" />
        <div className="project-crumb"><span className="crumb-label">PROJECT</span><strong>{project?.name ?? "Connecting"}</strong></div>
        <div className="connection-state"><i className={projectIssue ? "state-dot state-dot-error" : "state-dot"} />{projectIssue ? "Disconnected" : "Local session"}</div>
        <button className="icon-button refresh-button" onClick={() => void loadProject(true)} disabled={loading || refreshing} aria-label="Refresh workspace status" title="Refresh status">
          <span aria-hidden="true">↻</span>
        </button>
      </header>

      <div className={`workbench-grid ${preferences.leftCollapsed ? "left-collapsed" : ""} ${preferences.rightCollapsed ? "right-collapsed" : ""}`}>
        <aside className="workspace-rail" aria-label="Project workspaces">
          <div className="rail-heading">
            {!preferences.leftCollapsed && <><div><span className="eyebrow">PROJECT WORKSPACES</span><h2>Workspaces</h2></div><span className="count-pill">{project?.workspaces.length ?? "—"}</span></>}
            <button className="rail-toggle" onClick={() => setPref({ leftCollapsed: !preferences.leftCollapsed })} aria-label={preferences.leftCollapsed ? "Expand workspace rail" : "Collapse workspace rail"} title={preferences.leftCollapsed ? "Expand workspace rail" : "Collapse workspace rail"}>{preferences.leftCollapsed ? "›" : "‹"}</button>
          </div>
          {!preferences.leftCollapsed && <>
            <div className="project-path"><span className="folder-icon">▰</span><span><b>{project?.name ?? "Current project"}</b><small>Launch directory</small></span></div>
            <div className="workspace-list">
              {loading && <div className="rail-message">Discovering workspaces…</div>}
              {!loading && project?.workspaces.length === 0 && <div className="empty-note"><span className="empty-icon">⌘</span><b>No workspaces found</b><p>Add scratch workspaces under <code>workspaces/</code>, then restart Control Tower.</p></div>}
              {project?.discovery_error && <div className="inline-warning">{project.discovery_error}</div>}
              {project?.workspaces.map((item) => <WorkspaceButton key={item.id} item={item} selected={item.id === selectedWorkspaceId} onClick={() => setSelectedWorkspaceId(item.id)} />)}
              {projectIssue && <div className="inline-warning">{projectIssue}</div>}
            </div>
            <div className="rail-footer"><span className="footer-glyph">⌘</span><span><b>Startup discovery</b><small>Restart to add or remove</small></span></div>
          </>}
          {preferences.leftCollapsed && <div className="collapsed-workspaces">{project?.workspaces.map((item) => <button key={item.id} className={`workspace-glyph ${item.id === selectedWorkspaceId ? "selected" : ""}`} onClick={() => setSelectedWorkspaceId(item.id)} title={item.name} aria-label={`Select workspace ${item.name}`}>{item.name.slice(0, 1).toUpperCase()}</button>)}</div>}
        </aside>

        <main className="stage-column" aria-label="Ordered stages">
          <div className="stage-column-scroll">
            <div className="page-heading">
              <div><div className="eyebrow">WORKSPACE / {selectedSummary?.id ?? "—"}</div><h1>{selectedSummary?.name ?? "Select a workspace"}</h1><p className="page-subtitle">An ordered view of this workspace’s stage definitions and last confirmed position.</p></div>
              {workspace?.checkpoint.workflow_started && <div className="uuid-badge"><span className="uuid-dot" />WORKFLOW UUID CREATED</div>}
            </div>

            {workspaceIssue && <div className="notice notice-error"><span className="notice-symbol">!</span><div><b>Workspace unavailable</b><p>{workspaceIssue}</p><small>{setupHint(workspaceIssue)}</small></div></div>}
            {selectedSummary && !selectedSummary.available && !workspaceIssue && <div className="notice notice-error"><span className="notice-symbol">!</span><div><b>Workspace unavailable</b><p>{selectedSummary.issue}</p><small>{setupHint(selectedSummary.issue ?? "")}</small></div></div>}
            {!loading && !workspaceIssue && workspace && workspace.stages.length === 0 && <div className="notice"><span className="notice-symbol">i</span><div><b>No numbered stages found</b><p>This workspace does not contain any numbered directories under <code>stages/</code>.</p></div></div>}

            {workspace && <>
              <div className="checkpoint-strip">
                <div className="checkpoint-main"><span className="checkpoint-icon">✓</span><div><span className="eyebrow">LAST CONFIRMED CHECKPOINT</span><strong>{workspace.checkpoint.accepted_stage ? `${workspace.checkpoint.accepted_stage.number} · ${workspace.checkpoint.accepted_stage.name}` : "Baseline · no accepted stages"}</strong></div></div>
                {workspace.checkpoint.pending_transition && <div className="pending-banner"><span className="pending-indicator">◐</span><span><b>Pending {workspace.checkpoint.pending_transition.direction}</b><small>Stage {workspace.checkpoint.pending_transition.stage.number} · prior outcome unavailable</small></span></div>}
              </div>

              <div className="stages-heading"><div><span className="eyebrow">STAGE SEQUENCE</span><h2>Ordered progression</h2></div><span className="stage-count">{workspace.stages.length} {workspace.stages.length === 1 ? "stage" : "stages"}</span></div>
              <div className="stage-list">
                {workspace.stages.map((stage, index) => <StageCard key={stage.number} stage={stage} index={index} selected={stage.number === selectedStageNumber} onClick={() => setSelectedStageNumber(stage.number)} />)}
              </div>
            </>}

            {!loading && !selectedWorkspaceId && !projectIssue && <div className="welcome-panel"><div className="welcome-icon">⌁</div><span className="eyebrow">READY WHEN YOU ARE</span><h2>Choose a workspace</h2><p>Select a workspace in the left rail to inspect its checkpoint and ordered stages.</p></div>}
          </div>

          <div className="action-dock">
            <div className="action-summary"><span className="action-orbit">↗</span><div><b>Stage movement is unavailable in this preview</b><small>This first slice is read-only. Your scripts have not been run.</small></div></div>
            <button className="primary-action" disabled>Advance to next stage <span aria-hidden="true">→</span></button>
          </div>
        </main>

        <div className={`inspector-resize ${preferences.rightCollapsed ? "hidden" : ""}`}>
          <button className="resize-handle" role="separator" aria-label="Resize stage inspector" aria-orientation="vertical" aria-valuemin={316} aria-valuemax={620} aria-valuenow={preferences.rightWidth} tabIndex={0} onPointerDown={resizeInspector} onKeyDown={resizeWithKeyboard}><span /></button>
        </div>

        <aside className="inspector-rail" aria-label="Selected stage inspector">
          {preferences.rightCollapsed ? <button className="inspector-reopen" onClick={() => setPref({ rightCollapsed: false })} aria-label="Expand stage inspector" title="Expand stage inspector">‹<span>INSPECTOR</span></button> : <Inspector
            selectedStage={selectedStage}
            selectedStageNumber={selectedStageNumber}
            definition={definition}
            definitionIssue={definitionIssue}
            checkpoint={workspace?.checkpoint ?? null}
            loading={Boolean(selectedStageNumber !== null && !definition && !definitionIssue)}
            onCollapse={() => setPref({ rightCollapsed: true })}
          />}
        </aside>
      </div>
    </div>
  );
}

function WorkspaceButton({ item, selected, onClick }: { item: WorkspaceSummary; selected: boolean; onClick: () => void }) {
  const state = item.available ? item.pending_transition ? "Pending" : item.accepted_stage ? `Applied · ${item.accepted_stage.number}` : "At baseline" : "Unavailable";
  return <button className={`workspace-item ${selected ? "active" : ""}`} onClick={onClick} aria-current={selected ? "page" : undefined}>
    <span className={`workspace-item-icon ${selected ? "active" : ""}`} aria-hidden="true">{item.name.slice(0, 1).toUpperCase()}</span>
    <span className="workspace-item-copy"><b>{item.name}</b><small>{state}</small></span>
    <span className={`availability-dot ${item.available ? "" : "unavailable"}`} aria-label={item.available ? "Available" : "Unavailable"} />
  </button>;
}

function StageCard({ stage, index, selected, onClick }: { stage: StageView; index: number; selected: boolean; onClick: () => void }) {
  const stateText = stage.state === "accepted" ? "Applied" : stage.state === "pending" ? "Pending transition" : "Not applied";
  const stateGlyph = stage.state === "accepted" ? "✓" : stage.state === "pending" ? "◐" : String(index + 1).padStart(2, "0");
  return <button className={`stage-card stage-${stage.state} ${selected ? "selected" : ""}`} onClick={onClick} aria-pressed={selected}>
    <span className={`stage-marker marker-${stage.state}`} aria-hidden="true">{stateGlyph}</span>
    <span className="stage-card-body"><span className="stage-kicker">STAGE {String(stage.number).padStart(3, "0")} <i /> {stateText}{stage.is_accepted_checkpoint && stage.state === "pending" ? " · last confirmed checkpoint" : ""}</span><strong>{stage.name}</strong><span className="role-summary">{stage.definitions.map((definition) => <span key={definition.role} className="role-chip">{definition.role}</span>)}</span></span>
    <span className="stage-chevron" aria-hidden="true">›</span>
  </button>;
}

function Inspector({ selectedStage, selectedStageNumber, definition, definitionIssue, checkpoint, loading, onCollapse }: {
  selectedStage: StageView | null;
  selectedStageNumber: number | null;
  definition: DefinitionView | null;
  definitionIssue: string | null;
  checkpoint: WorkspaceView["checkpoint"] | null;
  loading: boolean;
  onCollapse: () => void;
}) {
  const currentIsAccepted = selectedStage?.state === "accepted";
  const currentIsPending = selectedStage?.state === "pending";
  return <>
    <div className="inspector-header"><div><span className="eyebrow">DETAILS</span><h2>Stage inspector</h2></div><button className="rail-toggle" onClick={onCollapse} aria-label="Collapse stage inspector" title="Collapse stage inspector">›</button></div>
    <div className="inspector-scroll">
      {!selectedStage && <div className="inspector-empty"><span className="empty-icon">⌕</span><b>Select a stage</b><p>Choose any stage to inspect its definition. Selection never runs a script.</p></div>}
      {selectedStage && <>
        <section className="inspector-stage-title"><span className="stage-index-label">STAGE {String(selectedStage.number).padStart(3, "0")}</span><h3>{selectedStage.name}</h3><span className={`status-pill pill-${selectedStage.state}`}>{selectedStage.state === "accepted" ? "Applied" : selectedStage.state === "pending" ? "Pending · outcome unknown" : "Future stage"}</span></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">01</span><h4>Checkpoint state</h4></div>
          {currentIsAccepted ? <div className="state-card success"><span className="state-card-icon">✓</span><span><b>Applied</b><small>{selectedStage.is_accepted_checkpoint ? "Last confirmed accepted position" : "Accepted earlier · execution evidence not retained"}</small></span></div> : currentIsPending ? <div className="state-card pending"><span className="state-card-icon">◐</span><span><b>{checkpoint?.pending_transition?.direction === "down" ? "Pending down" : "Pending up"}</b><small>Prior execution result is not retained</small></span></div> : <div className="state-card neutral"><span className="state-card-icon">○</span><span><b>Not applied</b><small>Future stage in the ordered sequence</small></span></div>}
          {selectedStage.is_accepted_checkpoint && currentIsPending && <p className="subtle-note">This is also the last confirmed checkpoint. The pending transition has not been accepted.</p>}
        </section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">02</span><h4>Mutation</h4></div><RolePreview role="up" definitions={selectedStage.definitions} definition={definition} /><RolePreview role="down" definitions={selectedStage.definitions} definition={definition} /></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">03</span><h4>Verification</h4></div><RolePreview role="verify-up" definitions={selectedStage.definitions} definition={definition} /><RolePreview role="verify-down" definitions={selectedStage.definitions} definition={definition} />{selectedStage.definitions.every((role) => !role.role.startsWith("verify-")) && <p className="subtle-note">No verifier configured. Missing verification is not a passing result.</p>}</section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">04</span><h4>Captured output</h4></div><div className="output-empty"><span aria-hidden="true">⌁</span><div><b>No retained output</b><small>This read-only phase does not run roles or collect output.</small></div></div></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">05</span><h4>Definition preview</h4></div>
          {loading && <div className="definition-loading">Reading stage files…</div>}
          {definitionIssue && <div className="inline-warning">{definitionIssue}</div>}
          {definition?.definitions.length === 0 && <p className="subtle-note">No role definitions are available for this stage.</p>}
          {definition?.definitions.map((role) => <div className="definition-block" key={role.role}><div className="definition-meta"><span>{role.role}</span><code>{role.path}</code></div>{role.issue ? <p className="subtle-note">{role.issue}</p> : <pre>{role.contents}</pre>}{role.truncated && <small className="truncated-note">Preview limited to 128 KiB.</small>}</div>)}
          {!definition && !loading && selectedStageNumber === null && <p className="subtle-note">Choose a stage to read its role definitions.</p>}
        </section>
        <section className="inspector-section technical-section"><div className="section-title"><span className="section-number">06</span><h4>Technical definition</h4></div><div className="technical-row"><span>Stage directory</span><code>stages/{String(selectedStage.number).padStart(3, "0")}-…</code></div><div className="technical-row"><span>Stage number</span><code>{selectedStage.number}</code></div><div className="technical-row"><span>Configured roles</span><code>{selectedStage.definitions.length}</code></div></section>
      </>}
    </div>
    <div className="inspector-footer"><span className="footer-lock">⌑</span><span>Inspection only · scripts are never run here</span></div>
  </>;
}

function RolePreview({ role, definitions, definition }: { role: string; definitions: StageView["definitions"]; definition: DefinitionView | null }) {
  const configured = definitions.find((item) => item.role === role);
  const result = definition?.definitions.find((item) => item.role === role);
  return <div className={`role-row ${configured ? "configured" : "not-configured"}`}><span className="role-result-mark">{configured ? "·" : "—"}</span><span className="role-row-copy"><b>{role}</b><small>{configured ? result?.issue ? "Definition unavailable" : configured.path : role.startsWith("verify-") ? "Not configured" : "Missing · movement unavailable"}</small></span><span className={`role-status ${configured ? "configured" : ""}`}>{configured ? "Not run" : role.startsWith("verify-") ? "Optional" : "Missing"}</span></div>;
}

function setupHint(issue: string): string {
  return issue.toLowerCase().includes("sqlite") || issue.toLowerCase().includes("checkpoint") || issue.toLowerCase().includes("schema")
    ? "Prepare storage explicitly with control-tower-db; Control Tower never initializes it during a status read."
    : "Check that this workspace has a readable stages/ directory and valid numbered stage folders.";
}
