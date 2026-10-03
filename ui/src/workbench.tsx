import { useEffect, useRef, useState } from "react";
import {
  api,
  submitMovement,
  type DefinitionView,
  type MovementChoice,
  type MovementObservation,
  type ProjectView,
  type RoleObservation,
  type RuntimeSnapshot,
  type StageView,
  type WorkspaceSummary,
  type WorkspaceView,
} from "./types";

type Preferences = { leftCollapsed: boolean; rightCollapsed: boolean; rightWidth: number };
const preferenceKey = "control-tower-workbench-layout-v1";
const defaultPreferences: Preferences = { leftCollapsed: false, rightCollapsed: false, rightWidth: 382 };
const minimumInspectorWidth = 316;
const maximumInspectorWidth = 620;
const minimumStageWidth = 520;
const expandedWorkspaceRailWidth = 252;
const resizeHandleWidth = 14;
const minimumWindowWidth = 1180;

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
  const [viewportWidth, setViewportWidth] = useState(() => window.innerWidth);
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
  const [liveStatus, setLiveStatus] = useState("Connecting");
  const [movementSubmittingFor, setMovementSubmittingFor] = useState<Record<string, number>>({});
  const [movementIssues, setMovementIssues] = useState<Record<string, string>>({});
  const detailRevision = useRef(0);
  const projectReadRevision = useRef(0);
  const definitionRevision = useRef(0);
  const previousWorkspaceId = useRef<string | null>(null);
  const serverInstance = useRef<string | null>(null);
  const selectedWorkspaceRef = useRef<string | null>(null);
  const selectedStageRef = useRef<number | null>(null);
  const deliberateStageSelection = useRef<{ workspaceId: string; stageNumber: number } | null>(null);
  const stageSelectionVersions = useRef(new Map<string, number>());
  const movementSequence = useRef(0);
  const activeMovementRequests = useRef(new Map<string, number>());
  const lostMovementResponses = useRef(new Map<string, { baselineRevision: number; selectionVersion: number }>());
  const localResultSelections = useRef(new Map<string, { serverInstanceId: string; operationId: string; stageNumber: number }>());
  const latestSnapshots = useRef(new Map<string, RuntimeSnapshot>());
  const latestWorkspaceViews = useRef(new Map<string, WorkspaceView>());
  const latestEventRevisions = useRef(new Map<string, { server_instance_id: string; revision: number }>());

  useEffect(() => {
    localStorage.setItem(preferenceKey, JSON.stringify(preferences));
  }, [preferences]);

  useEffect(() => {
    selectedWorkspaceRef.current = selectedWorkspaceId;
    selectedStageRef.current = selectedStageNumber;
  }, [selectedWorkspaceId, selectedStageNumber]);

  useEffect(() => {
    const updateViewportWidth = () => setViewportWidth(window.innerWidth);
    window.addEventListener("resize", updateViewportWidth);
    return () => window.removeEventListener("resize", updateViewportWidth);
  }, []);

  const maxInspectorWidth = Math.max(
    minimumInspectorWidth,
    Math.min(
      maximumInspectorWidth,
      Math.max(viewportWidth, minimumWindowWidth)
        - expandedWorkspaceRailWidth
        - minimumStageWidth
        - resizeHandleWidth,
    ),
  );
  const inspectorWidth = Math.max(minimumInspectorWidth, Math.min(preferences.rightWidth, maxInspectorWidth));

  async function loadProject(showRefreshing = false) {
    const requestRevision = ++projectReadRevision.current;
    if (showRefreshing) setRefreshing(true);
    setProjectIssue(null);
    try {
      await bootstrapSession();
      const view = await api<ProjectView>("/api/project");
      if (requestRevision !== projectReadRevision.current) return;
      setProject(reconcileProjectSummaries(view, latestSnapshots.current));
      if (showRefreshing) setWorkspaceRefreshVersion((version) => version + 1);
      setSelectedWorkspaceId((current) => {
        const selected = current && view.workspaces.some((item) => item.id === current)
          ? current
          : view.workspaces.find((item) => item.available)?.id ?? view.workspaces[0]?.id ?? null;
        if (selected !== current) {
          selectedStageRef.current = null;
          deliberateStageSelection.current = null;
        }
        selectedWorkspaceRef.current = selected;
        return selected;
      });
    } catch (error) {
      if (requestRevision !== projectReadRevision.current) return;
      setProjectIssue(error instanceof Error ? error.message : "Could not reach the local Control Tower host.");
    } finally {
      if (requestRevision === projectReadRevision.current) {
        setLoading(false);
        setRefreshing(false);
      }
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
      selectedStageRef.current = null;
      deliberateStageSelection.current = null;
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
        if (serverInstance.current && serverInstance.current !== view.server_instance_id) return;
        serverInstance.current ??= view.server_instance_id;
        const latest = latestSnapshots.current.get(selectedWorkspaceId);
        const reconciled = latest
          && latest.server_instance_id === view.server_instance_id
          && latest.revision >= view.observation_revision
          ? applyRuntimeSnapshot(view, latest)
          : view;
        const revisionFence = latestEventRevisions.current.get(selectedWorkspaceId);
        if (revisionFence?.server_instance_id === view.server_instance_id
          && reconciled.observation_revision < revisionFence.revision) {
          setWorkspaceRefreshVersion((version) => version + 1);
          return;
        }
        rememberWorkspaceView(latestSnapshots.current, reconciled);
        latestWorkspaceViews.current.set(selectedWorkspaceId, reconciled);
        rememberEventRevision(latestEventRevisions.current, selectedWorkspaceId, reconciled.server_instance_id, reconciled.observation_revision);
        updateProjectSummary(selectedWorkspaceId, reconciled.checkpoint);
        resolveLostMovementResponse(selectedWorkspaceId, reconciled);
        setWorkspace((current) => {
          if (current?.workspace.id === selectedWorkspaceId
            && current.server_instance_id === reconciled.server_instance_id
            && current.observation_revision > reconciled.observation_revision) return current;
          return reconciled;
        });
        reconcileInspectionSelection(selectedWorkspaceId, reconciled);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted || revision !== detailRevision.current) return;
        setWorkspaceIssue(error instanceof Error ? error.message : "Workspace details are unavailable.");
      });
    return () => controller.abort();
  }, [selectedWorkspaceId, workspaceRefreshVersion]);

  useEffect(() => {
    if (!selectedWorkspaceId) {
      setLiveStatus("Disconnected");
      return;
    }
    const workspaceId = selectedWorkspaceId;
    const source = new EventSource(`/api/workspaces/${encodeURIComponent(workspaceId)}/events`, { withCredentials: true });
    let closed = false;
    const acceptSnapshot = (event: Event) => {
      if (closed) return;
      try {
        const snapshot = JSON.parse((event as MessageEvent<string>).data) as RuntimeSnapshot;
        if (snapshot.workspace_id !== workspaceId
          || (serverInstance.current && serverInstance.current !== snapshot.server_instance_id)) return;
        serverInstance.current ??= snapshot.server_instance_id;
        const latest = latestSnapshots.current.get(workspaceId);
        if (latest && (latest.server_instance_id !== snapshot.server_instance_id
          || latest.revision > snapshot.revision)) return;
        const fence = latestEventRevisions.current.get(workspaceId);
        if (fence?.server_instance_id === snapshot.server_instance_id && fence.revision > snapshot.revision) return;
        latestSnapshots.current.set(workspaceId, snapshot);
        rememberEventRevision(latestEventRevisions.current, workspaceId, snapshot.server_instance_id, snapshot.revision);
        const cachedView = latestWorkspaceViews.current.get(workspaceId);
        if (cachedView) {
          const reconciled = applyRuntimeSnapshot(cachedView, snapshot);
          if (reconciled.observation_revision === snapshot.revision) {
            latestWorkspaceViews.current.set(workspaceId, reconciled);
            updateProjectSummary(workspaceId, reconciled.checkpoint);
            reconcileInspectionSelection(workspaceId, reconciled);
            resolveLostMovementResponse(workspaceId, reconciled);
          }
        } else {
          const checkpoint = snapshot.observation?.confirmed_checkpoint ?? snapshot.checkpoint;
          if (checkpoint) updateProjectSummary(workspaceId, checkpoint);
        }
        setWorkspace((current) => {
          if (!current || current.workspace.id !== workspaceId
            || current.server_instance_id !== snapshot.server_instance_id
            || current.observation_revision > snapshot.revision) return current;
          return applyRuntimeSnapshot(current, snapshot);
        });
      } catch {
        if (!closed) setLiveStatus("Resynchronizing");
      }
    };
    const refreshObservation = async () => {
      try {
        const view = await api<WorkspaceView>(`/api/workspaces/${encodeURIComponent(workspaceId)}`);
        if (closed || (serverInstance.current && serverInstance.current !== view.server_instance_id)) return;
        serverInstance.current ??= view.server_instance_id;
        const latest = latestSnapshots.current.get(workspaceId);
        const reconciled = latest
          && latest.server_instance_id === view.server_instance_id
          && latest.revision >= view.observation_revision
          ? applyRuntimeSnapshot(view, latest)
          : view;
        const fence = latestEventRevisions.current.get(workspaceId);
        if (fence?.server_instance_id === view.server_instance_id && reconciled.observation_revision < fence.revision) {
          void refreshObservation();
          return;
        }
        rememberWorkspaceView(latestSnapshots.current, reconciled);
        latestWorkspaceViews.current.set(workspaceId, reconciled);
        rememberEventRevision(latestEventRevisions.current, workspaceId, reconciled.server_instance_id, reconciled.observation_revision);
        updateProjectSummary(workspaceId, reconciled.checkpoint);
        reconcileInspectionSelection(workspaceId, reconciled);
        resolveLostMovementResponse(workspaceId, reconciled);
        setWorkspace((current) => {
          if (!current || current.workspace.id !== workspaceId
            || current.server_instance_id !== reconciled.server_instance_id
            || current.observation_revision > reconciled.observation_revision) return current;
          return reconciled;
        });
      } catch {
        if (!closed) setLiveStatus("Reconnecting");
      }
    };
    const refreshFromEvent = (event: Event) => {
      if (closed) return;
      try {
        const data = JSON.parse((event as MessageEvent<string>).data) as {
          workspace_id?: string;
          server_instance_id?: string;
          revision?: number;
        };
        if (data.workspace_id !== workspaceId
          || (serverInstance.current && data.server_instance_id !== serverInstance.current)) return;
        if (data.server_instance_id && typeof data.revision === "number") {
          rememberEventRevision(latestEventRevisions.current, workspaceId, data.server_instance_id, data.revision);
        }
        void refreshObservation();
      } catch {
        void refreshObservation();
      }
    };
    source.onopen = () => {
      if (!closed) setLiveStatus("Live updates connected");
    };
    source.onerror = () => {
      if (closed) return;
      setLiveStatus("Reconnecting");
      void refreshObservation();
    };
    source.addEventListener("snapshot", acceptSnapshot);
    source.addEventListener("resync", acceptSnapshot);
    source.addEventListener("movement.started", refreshFromEvent);
    source.addEventListener("role.started", refreshFromEvent);
    source.addEventListener("role.finished", refreshFromEvent);
    source.addEventListener("movement.finished", refreshFromEvent);
    return () => {
      closed = true;
      source.close();
    };
  }, [selectedWorkspaceId]);

  function updateProjectSummary(workspaceId: string, checkpoint: WorkspaceView["checkpoint"]) {
    setProject((current) => current ? updateWorkspaceSummary(current, workspaceId, checkpoint) : current);
  }

  function reconcileInspectionSelection(workspaceId: string, view: WorkspaceView) {
    if (selectedWorkspaceRef.current !== workspaceId || activeMovementRequests.current.has(workspaceId)) return;
    const stageNumber = preferredInspectionStage(
      view,
      deliberateStageSelection.current,
      localResultSelections.current.get(workspaceId) ?? null,
    );
    selectedStageRef.current = stageNumber;
    setSelectedStageNumber(stageNumber);
  }

  function resolveLostMovementResponse(workspaceId: string, view: WorkspaceView) {
    const lost = lostMovementResponses.current.get(workspaceId);
    const observation = view.observation;
    if (!lost || !observation || view.movement_busy || observation.state === "running"
      || view.observation_revision <= lost.baselineRevision) return;
    const resultStage = observation.failure?.stage?.number ?? observation.role_results.at(-1)?.stage.number;
    if (resultStage !== undefined
      && (stageSelectionVersions.current.get(workspaceId) ?? 0) === lost.selectionVersion
      && view.stages.some((stage) => stage.number === resultStage)) {
      localResultSelections.current.set(workspaceId, {
        serverInstanceId: observation.server_instance_id,
        operationId: observation.operation_id,
        stageNumber: resultStage,
      });
      if (selectedWorkspaceRef.current === workspaceId) {
        selectedStageRef.current = resultStage;
        setSelectedStageNumber(resultStage);
      }
    }
    lostMovementResponses.current.delete(workspaceId);
    setMovementIssues((current) => {
      if (!current[workspaceId]?.startsWith("Movement response lost;")) return current;
      const { [workspaceId]: _previous, ...rest } = current;
      return rest;
    });
  }

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
    const startWidth = inspectorWidth;
    const pointerId = event.pointerId;
    event.currentTarget.setPointerCapture(pointerId);
    const move = (next: PointerEvent) => {
      setPref({ rightWidth: Math.max(minimumInspectorWidth, Math.min(maxInspectorWidth, startWidth + startX - next.clientX)) });
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
    if (event.key === "ArrowLeft") setPref({ rightWidth: Math.min(maxInspectorWidth, inspectorWidth + 16) });
    if (event.key === "ArrowRight") setPref({ rightWidth: Math.max(minimumInspectorWidth, inspectorWidth - 16) });
  }

  const actions = workspace ? movementActionsFor(workspace) : [];
  const activeAction = actions[0];
  const alternateAction = actions[1];
  const controlsBusy = Boolean(workspace?.movement_busy
    || (selectedWorkspaceId && movementSubmittingFor[selectedWorkspaceId] !== undefined));
  const movementIssue = selectedWorkspaceId ? movementIssues[selectedWorkspaceId] ?? null : null;
  const finalAccepted = Boolean(workspace?.stages.length
    && !workspace.checkpoint.pending_transition
    && workspace.checkpoint.accepted_stage?.number === workspace.stages.at(-1)?.number);
  const inactiveActionLabel = !selectedWorkspaceId
    ? "Select a workspace"
    : workspaceIssue || selectedSummary?.available === false
      ? "Workspace unavailable"
      : loading
        ? "Loading workspace"
        : workspace?.stages.length === 0
          ? "No stages available"
          : "All stages applied";
  const inactiveActionDetail = !selectedWorkspaceId
    ? "Choose a project-local Workspace to inspect its stages."
    : workspaceIssue || selectedSummary?.available === false
      ? "Resolve the Workspace availability issue before moving."
      : workspace?.stages.length === 0
        ? "This Workspace has no numbered stage definitions."
        : "The current accepted position is at baseline.";
  const ambiguousEffects = Boolean(workspace?.observation?.failure
    && (workspace.observation.failure.kind === "checkpoint_save_failed"
      || (["up", "down"].includes(workspace.observation.failure.role ?? "")
        && ["process_failed", "launch_failed"].includes(workspace.observation.failure.kind))));

  async function runMovement(action: MovementAction) {
    if (!selectedWorkspaceId || !workspace || controlsBusy || !selectedSummary?.available
      || workspace.storage_issue || action.disabledReason) return;
    const workspaceId = selectedWorkspaceId;
    const requestId = ++movementSequence.current;
    activeMovementRequests.current.set(workspaceId, requestId);
    lostMovementResponses.current.delete(workspaceId);
    localResultSelections.current.delete(workspaceId);
    const baselineRevision = workspace.observation_revision;
    const submittedSelectionVersion = stageSelectionVersions.current.get(workspaceId) ?? 0;
    deliberateStageSelection.current = null;
    setMovementSubmittingFor((current) => ({ ...current, [workspaceId]: requestId }));
    setMovementIssues((current) => {
      const { [workspaceId]: _previous, ...rest } = current;
      return rest;
    });
    if (action.choice.target_stage > 0 && workspace.stages.some((stage) => stage.number === action.choice.target_stage)) {
      selectedStageRef.current = action.choice.target_stage;
      setSelectedStageNumber(action.choice.target_stage);
    } else if (workspace.checkpoint.pending_transition) {
      selectedStageRef.current = workspace.checkpoint.pending_transition.stage.number;
      setSelectedStageNumber(workspace.checkpoint.pending_transition.stage.number);
    }
    try {
      const observation = await submitMovement(
        `/api/workspaces/${encodeURIComponent(workspaceId)}/movements`,
        action.choice,
      );
      if (activeMovementRequests.current.get(workspaceId) !== requestId) return;
      lostMovementResponses.current.delete(workspaceId);
      const previousSnapshot = latestSnapshots.current.get(workspaceId);
      if (previousSnapshot && previousSnapshot.server_instance_id === observation.server_instance_id
        && previousSnapshot.revision > observation.revision) return;
      const eventFence = latestEventRevisions.current.get(workspaceId);
      if (eventFence?.server_instance_id === observation.server_instance_id
        && eventFence.revision > observation.revision) {
        void refreshWorkspace(workspaceId);
        return;
      }
      const snapshot: RuntimeSnapshot = {
        workspace_id: workspaceId,
        server_instance_id: observation.server_instance_id,
        revision: observation.revision,
        movement_busy: false,
        checkpoint: observation.confirmed_checkpoint ?? previousSnapshot?.checkpoint ?? workspace.checkpoint,
        observation,
      };
      latestSnapshots.current.set(workspaceId, snapshot);
      rememberEventRevision(latestEventRevisions.current, workspaceId, observation.server_instance_id, observation.revision);
      const cachedView = latestWorkspaceViews.current.get(workspaceId) ?? workspace;
      const reconciled = applyRuntimeSnapshot(cachedView, snapshot);
      latestWorkspaceViews.current.set(workspaceId, reconciled);
      updateProjectSummary(workspaceId, reconciled.checkpoint);
      setWorkspace((current) => {
        if (selectedWorkspaceRef.current !== workspaceId
          || !current || current.workspace.id !== workspaceId
          || current.server_instance_id !== observation.server_instance_id
          || current.observation_revision > observation.revision) return current;
        return applyRuntimeSnapshot(current, snapshot);
      });
      const relevantStage = observation.failure?.stage?.number
        ?? observation.role_results.at(-1)?.stage.number;
      if (relevantStage !== undefined
        && (stageSelectionVersions.current.get(workspaceId) ?? 0) === submittedSelectionVersion
        && reconciled.stages.some((stage) => stage.number === relevantStage)) {
        localResultSelections.current.set(workspaceId, {
          serverInstanceId: observation.server_instance_id,
          operationId: observation.operation_id,
          stageNumber: relevantStage,
        });
        if (selectedWorkspaceRef.current === workspaceId) {
          selectedStageRef.current = relevantStage;
          setSelectedStageNumber(relevantStage);
        }
      }
      if (observation.state === "stopped" || observation.state === "unavailable") {
        setMovementIssues((current) => ({
          ...current,
          [workspaceId]: `${observation.state === "stopped" ? "Movement stopped" : "Movement unavailable"}: ${observation.failure?.message ?? "Inspect the current checkpoint and authored effects."}`,
        }));
      }
    } catch (error) {
      if (activeMovementRequests.current.get(workspaceId) === requestId) {
        if (error instanceof TypeError) {
          lostMovementResponses.current.set(workspaceId, {
            baselineRevision,
            selectionVersion: submittedSelectionVersion,
          });
          setMovementIssues((current) => ({
            ...current,
            [workspaceId]: "Movement response lost; waiting for the latest workspace observation.",
          }));
        } else {
          setMovementIssues((current) => ({
            ...current,
            [workspaceId]: error instanceof Error ? error.message : "Movement could not be submitted.",
          }));
        }
      }
      void refreshWorkspace(workspaceId);
    } finally {
      if (activeMovementRequests.current.get(workspaceId) === requestId) {
        activeMovementRequests.current.delete(workspaceId);
        setMovementSubmittingFor((current) => {
          if (current[workspaceId] !== requestId) return current;
          const { [workspaceId]: _previous, ...rest } = current;
          return rest;
        });
      }
    }
  }

  async function refreshWorkspace(workspaceId: string) {
    try {
      const view = await api<WorkspaceView>(`/api/workspaces/${encodeURIComponent(workspaceId)}`);
      if (serverInstance.current && serverInstance.current !== view.server_instance_id) return;
      serverInstance.current ??= view.server_instance_id;
      const latest = latestSnapshots.current.get(workspaceId);
      const reconciled = latest
        && latest.server_instance_id === view.server_instance_id
        && latest.revision >= view.observation_revision
        ? applyRuntimeSnapshot(view, latest)
        : view;
      const fence = latestEventRevisions.current.get(workspaceId);
      if (fence?.server_instance_id === view.server_instance_id && reconciled.observation_revision < fence.revision) {
        void refreshWorkspace(workspaceId);
        return;
      }
      rememberWorkspaceView(latestSnapshots.current, reconciled);
      latestWorkspaceViews.current.set(workspaceId, reconciled);
      rememberEventRevision(latestEventRevisions.current, workspaceId, reconciled.server_instance_id, reconciled.observation_revision);
      updateProjectSummary(workspaceId, reconciled.checkpoint);
      reconcileInspectionSelection(workspaceId, reconciled);
      resolveLostMovementResponse(workspaceId, reconciled);
      setWorkspace((current) => {
        if (selectedWorkspaceRef.current !== workspaceId
          || !current || current.workspace.id !== workspaceId
          || current.server_instance_id !== reconciled.server_instance_id
          || current.observation_revision > reconciled.observation_revision) return current;
        return reconciled;
      });
    } catch {
      // SSE reconnect and the next explicit refresh remain available.
    }
  }

  function selectWorkspace(workspaceId: string) {
    if (selectedWorkspaceRef.current !== workspaceId) {
      selectedStageRef.current = null;
      deliberateStageSelection.current = null;
    }
    selectedWorkspaceRef.current = workspaceId;
    setSelectedWorkspaceId(workspaceId);
  }

  return (
    <div className="application-frame" style={{ "--inspector-width": `${inspectorWidth}px` } as React.CSSProperties}>
      <header className="topbar">
        <div className="brand-mark" aria-hidden="true"><span />CT</div>
        <div className="brand-copy"><strong>Control Tower</strong><span>LOCAL WORKBENCH</span></div>
        <div className="topbar-divider" />
        <div className="project-crumb"><span className="crumb-label">PROJECT</span><strong>{project?.name ?? "Connecting"}</strong></div>
        <div className="connection-state"><i className={projectIssue || liveStatus === "Reconnecting" ? "state-dot state-dot-error" : "state-dot"} />{projectIssue ? "Disconnected" : liveStatus}</div>
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
              {project?.workspaces.map((item) => <WorkspaceButton key={item.id} item={item} selected={item.id === selectedWorkspaceId} onClick={() => selectWorkspace(item.id)} />)}
              {projectIssue && <div className="inline-warning">{projectIssue}</div>}
            </div>
            <div className="rail-footer"><span className="footer-glyph">⌘</span><span><b>Startup discovery</b><small>Restart to add or remove</small></span></div>
          </>}
          {preferences.leftCollapsed && <div className="collapsed-workspaces">{project?.workspaces.map((item) => <button key={item.id} className={`workspace-glyph ${item.id === selectedWorkspaceId ? "selected" : ""}`} onClick={() => selectWorkspace(item.id)} title={item.name} aria-label={`Select workspace ${item.name}`}>{item.name.slice(0, 1).toUpperCase()}</button>)}</div>}
        </aside>

        <main className="stage-column" aria-label="Ordered stages">
          <div className="stage-column-scroll">
            <div className="page-heading">
              <div><div className="eyebrow">WORKSPACE / {selectedSummary?.id ?? "—"}</div><h1>{selectedSummary?.name ?? "Select a workspace"}</h1><p className="page-subtitle">An ordered view of this workspace’s stage definitions and last confirmed position.</p></div>
              {workspace?.checkpoint.workflow_started && <div className="uuid-badge"><span className="uuid-dot" />WORKFLOW UUID CREATED</div>}
            </div>

            {workspaceIssue && <div className="notice notice-error"><span className="notice-symbol">!</span><div><b>Workspace unavailable</b><p>{workspaceIssue}</p><small>{setupHint(workspaceIssue)}</small></div></div>}
            {workspace?.storage_issue && <div className="notice notice-error" role="alert"><span className="notice-symbol">!</span><div><b>Checkpoint storage unavailable</b><p>{workspace.storage_issue}</p><small>Showing the last readable checkpoint with retained movement evidence. Resolve storage before submitting another movement.</small></div></div>}
            {selectedSummary && !selectedSummary.available && !workspaceIssue && <div className="notice notice-error"><span className="notice-symbol">!</span><div><b>Workspace unavailable</b><p>{selectedSummary.issue}</p><small>{setupHint(selectedSummary.issue ?? "")}</small></div></div>}
            {!loading && !workspaceIssue && workspace && workspace.stages.length === 0 && <div className="notice"><span className="notice-symbol">i</span><div><b>No numbered stages found</b><p>This workspace does not contain any numbered directories under <code>stages/</code>.</p></div></div>}

            {workspace && <>
              <div className="checkpoint-strip">
                <div className="checkpoint-main"><span className="checkpoint-icon">✓</span><div><span className="eyebrow">LAST CONFIRMED CHECKPOINT</span><strong>{workspace.checkpoint.accepted_stage ? `${workspace.checkpoint.accepted_stage.number} · ${workspace.checkpoint.accepted_stage.name}` : "Baseline · no accepted stages"}</strong></div></div>
                {workspace.checkpoint.pending_transition && <div className="pending-banner"><span className="pending-indicator">◐</span><span><b>Pending {workspace.checkpoint.pending_transition.direction}</b><small>Stage {workspace.checkpoint.pending_transition.stage.number} · {pendingEvidenceSummary(workspace.checkpoint.pending_transition.stage.number, workspace.observation) ?? "prior outcome unavailable"}</small></span></div>}
                {workspace.movement_busy && <div className="active-banner"><span className="active-pulse" /><span><b>Movement active</b><small>{workspace.observation?.active_role ? `Stage ${workspace.observation.active_role.stage.number} · ${workspace.observation.active_role.role} attempt` : "Preparing movement"}</small></span></div>}
              </div>

              <div className="stages-heading"><div><span className="eyebrow">STAGE SEQUENCE</span><h2>Ordered progression</h2></div><span className="stage-count">{workspace.stages.length} {workspace.stages.length === 1 ? "stage" : "stages"}</span></div>
              <div className="stage-list">
                {workspace.stages.map((stage, index) => <StageCard key={stage.number} stage={stage} index={index} observation={workspace.observation} selected={stage.number === selectedStageNumber} onClick={() => {
                  selectedStageRef.current = stage.number;
                  deliberateStageSelection.current = { workspaceId: workspace.workspace.id, stageNumber: stage.number };
                  stageSelectionVersions.current.set(
                    workspace.workspace.id,
                    (stageSelectionVersions.current.get(workspace.workspace.id) ?? 0) + 1,
                  );
                  setSelectedStageNumber(stage.number);
                }} />)}
              </div>
            </>}

            {!loading && !selectedWorkspaceId && !projectIssue && <div className="welcome-panel"><div className="welcome-icon">⌁</div><span className="eyebrow">READY WHEN YOU ARE</span><h2>Choose a workspace</h2><p>Select a workspace in the left rail to inspect its checkpoint and ordered stages.</p></div>}
          </div>

          <div className="action-dock">
            <div className="action-summary"><span className="action-orbit">{workspace?.movement_busy ? "◌" : "↗"}</span><div><b>{workspace?.observation?.state === "stopped" ? "Movement stopped at the last confirmed checkpoint" : workspace?.observation?.state === "complete" ? "Movement completed" : workspace?.movement_busy ? "Movement is in progress" : finalAccepted ? "All stages applied; backout remains available" : activeAction?.label ?? inactiveActionLabel}</b><small>{workspace?.observation?.active_role ? `Attempt in progress · Stage ${workspace.observation.active_role.stage.number} ${workspace.observation.active_role.role}. Captured bytes appear when the role returns.` : activeAction?.detail ?? inactiveActionDetail}</small></div></div>
            {movementIssue && <span className="movement-issue" role="status">{movementIssue}</span>}
            {ambiguousEffects && <span className="movement-warning" role="alert">Inspect author-owned effects before continuing. The pending checkpoint alone does not make retry or reversal safe.</span>}
            <div className="movement-buttons">
              {alternateAction && <button className="secondary-action" onClick={() => void runMovement(alternateAction)} disabled={controlsBusy || Boolean(workspace?.storage_issue) || !selectedSummary?.available || Boolean(alternateAction.disabledReason)} title={workspace?.storage_issue ?? alternateAction.disabledReason}>{alternateAction.label}<span aria-hidden="true">↶</span></button>}
              {activeAction && <button className="primary-action" onClick={() => void runMovement(activeAction)} disabled={controlsBusy || Boolean(workspace?.storage_issue) || !selectedSummary?.available || Boolean(activeAction.disabledReason)} title={workspace?.storage_issue ?? activeAction.disabledReason}>{controlsBusy ? "Movement in progress" : activeAction.label}<span aria-hidden="true">{activeAction.choice.direction === "up" ? "→" : "↶"}</span></button>}
              {!activeAction && <button className="primary-action" disabled>{inactiveActionLabel}<span aria-hidden="true">{workspace?.stages.length ? "✓" : "·"}</span></button>}
            </div>
          </div>
        </main>

        <div className={`inspector-resize ${preferences.rightCollapsed ? "hidden" : ""}`}>
          <button className="resize-handle" role="separator" aria-label="Resize stage inspector" aria-orientation="vertical" aria-valuemin={minimumInspectorWidth} aria-valuemax={maxInspectorWidth} aria-valuenow={inspectorWidth} tabIndex={0} onPointerDown={resizeInspector} onKeyDown={resizeWithKeyboard}><span /></button>
        </div>

        <aside className="inspector-rail" aria-label="Selected stage inspector">
          {preferences.rightCollapsed ? <button className="inspector-reopen" onClick={() => setPref({ rightCollapsed: false })} aria-label="Expand stage inspector" title="Expand stage inspector">‹<span>INSPECTOR</span></button> : <Inspector
            selectedStage={selectedStage}
            selectedStageNumber={selectedStageNumber}
            definition={definition}
            definitionIssue={definitionIssue}
            checkpoint={workspace?.checkpoint ?? null}
            observation={workspace?.observation ?? null}
            workspaceId={selectedWorkspaceId}
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

function StageCard({ stage, index, selected, observation, onClick }: { stage: StageView; index: number; selected: boolean; observation: MovementObservation | null; onClick: () => void }) {
  const stateText = stage.state === "accepted" ? "Applied" : stage.state === "pending" ? "Pending transition" : "Not applied";
  const stateGlyph = stage.state === "accepted" ? "✓" : stage.state === "pending" ? "◐" : String(index + 1).padStart(2, "0");
  const activeRole = observation?.active_role?.stage.number === stage.number ? observation.active_role : null;
  return <button className={`stage-card stage-${stage.state} ${selected ? "selected" : ""}`} onClick={onClick} aria-pressed={selected}>
    <span className={`stage-marker marker-${stage.state}`} aria-hidden="true">{stateGlyph}</span>
    <span className="stage-card-body"><span className="stage-kicker">STAGE {String(stage.number).padStart(3, "0")} <i /> {activeRole ? `Attempt in progress · ${activeRole.role}` : stateText}{stage.is_accepted_checkpoint && stage.state === "pending" ? " · last confirmed checkpoint" : ""}</span><strong>{stage.name}</strong><span className="role-summary">{stage.definitions.map((definition) => <span key={definition.role} className="role-chip">{definition.role}</span>)}</span></span>
    <span className="stage-chevron" aria-hidden="true">›</span>
  </button>;
}

function Inspector({ selectedStage, selectedStageNumber, definition, definitionIssue, checkpoint, observation, workspaceId, loading, onCollapse }: {
  selectedStage: StageView | null;
  selectedStageNumber: number | null;
  definition: DefinitionView | null;
  definitionIssue: string | null;
  checkpoint: WorkspaceView["checkpoint"] | null;
  observation: MovementObservation | null;
  workspaceId: string | null;
  loading: boolean;
  onCollapse: () => void;
}) {
  const currentIsAccepted = selectedStage?.state === "accepted";
  const currentIsPending = selectedStage?.state === "pending";
  const stageResults = selectedStage
    ? observation?.role_results.filter((result) => result.stage.number === selectedStage.number) ?? []
    : [];
  const latestStageResult = stageResults.at(-1);
  const relevantFailure = observation?.failure && selectedStage
    && (observation.failure.stage?.number === selectedStage.number
      || latestStageResult !== undefined
      || observation.failure.stage === null && observation.attempted_checkpoint !== null)
    ? observation.failure
    : null;
  return <>
    <div className="inspector-header"><div><span className="eyebrow">DETAILS</span><h2>Stage inspector</h2></div><button className="rail-toggle" onClick={onCollapse} aria-label="Collapse stage inspector" title="Collapse stage inspector">›</button></div>
    <div className="inspector-scroll">
      {!selectedStage && <div className="inspector-empty"><span className="empty-icon">⌕</span><b>Select a stage</b><p>Choose any stage to inspect its definition. Selection never runs a script.</p></div>}
      {selectedStage && <>
        <section className="inspector-stage-title"><span className="stage-index-label">STAGE {String(selectedStage.number).padStart(3, "0")}</span><h3>{selectedStage.name}</h3><span className={`status-pill pill-${selectedStage.state}`}>{selectedStage.state === "accepted" ? "Applied" : selectedStage.state === "pending" ? `Pending · ${pendingEvidenceSummary(selectedStage.number, observation) ?? "prior outcome unavailable"}` : "Future stage"}</span></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">01</span><h4>Checkpoint state</h4></div>
          {relevantFailure && <div className="inspector-error" role="status"><b>{relevantFailure.kind === "checkpoint_save_failed" ? "Checkpoint was not confirmed" : "Observed failure"}</b><p>{relevantFailure.message}</p>{observation?.attempted_checkpoint && <small>Last confirmed position remains separate from the attempted checkpoint shown by this movement.</small>}</div>}
          {currentIsAccepted ? <div className="state-card success"><span className="state-card-icon">✓</span><span><b>Applied</b><small>{selectedStage.is_accepted_checkpoint ? "Last confirmed accepted position" : "Accepted earlier · execution evidence not retained"}</small></span></div> : currentIsPending ? <div className="state-card pending"><span className="state-card-icon">◐</span><span><b>{checkpoint?.pending_transition?.direction === "down" ? "Pending down" : "Pending up"}</b><small>{pendingEvidenceSummary(selectedStage.number, observation) ? `Observed ${pendingEvidenceSummary(selectedStage.number, observation)}` : "Prior execution result is not retained"}</small></span></div> : <div className="state-card neutral"><span className="state-card-icon">○</span><span><b>Not applied</b><small>Future stage in the ordered sequence</small></span></div>}
          {selectedStage.is_accepted_checkpoint && currentIsPending && <p className="subtle-note">This is also the last confirmed checkpoint. The pending transition has not been accepted.</p>}
          {observation?.attempted_checkpoint && <div className="attempted-checkpoint" role="note">
            <b>Attempted checkpoint · unconfirmed</b>
            <p><span>Accepted position:</span> {checkpointPosition(observation.attempted_checkpoint)}</p>
            <p><span>Pending transition:</span> {pendingPosition(observation.attempted_checkpoint)}</p>
            <small>Workbench last confirmed: {checkpointPosition(observation.confirmed_checkpoint ?? checkpoint)}. After an ambiguous save failure, actual database contents may be uncertain.</small>
          </div>}
        </section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">02</span><h4>Mutation</h4></div><RolePreview role="up" stage={selectedStage} definitions={selectedStage.definitions} results={stageResults} failure={relevantFailure} /><RolePreview role="down" stage={selectedStage} definitions={selectedStage.definitions} results={stageResults} failure={relevantFailure} /></section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">03</span><h4>Verification</h4></div><RolePreview role="verify-up" stage={selectedStage} definitions={selectedStage.definitions} results={stageResults} failure={relevantFailure} /><RolePreview role="verify-down" stage={selectedStage} definitions={selectedStage.definitions} results={stageResults} failure={relevantFailure} />{selectedStage.definitions.every((role) => !role.role.startsWith("verify-")) && <p className="subtle-note">No verifier configured. Missing verification is not a passing result.</p>}</section>
        <section className="inspector-section"><div className="section-title"><span className="section-number">04</span><h4>Captured output</h4></div>
          {stageResults.length === 0 && <div className="output-empty"><span aria-hidden="true">⌁</span><div><b>{selectedStage.state === "accepted" ? "No retained output" : selectedStage.state === "pending" ? "Prior output unavailable" : "No output yet"}</b><small>Only this process session’s latest movement results are retained.</small></div></div>}
          {stageResults.map((result) => <RoleOutput key={outputIdentity(workspaceId, observation?.operation_id, result)} workspaceId={workspaceId} operationId={observation?.operation_id ?? null} result={result} />)}
          {observation && observation.omitted_role_results > 0 && <p className="subtle-note">{observation.omitted_role_results} earlier role result(s) exceeded the in-memory summary limit.</p>}
        </section>
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
    <div className="inspector-footer"><span className="footer-lock">⌑</span><span>Output is buffered per role and rendered as inert text</span></div>
  </>;
}

function RolePreview({ role, stage, definitions, results, failure }: { role: string; stage: StageView; definitions: StageView["definitions"]; results: RoleObservation[]; failure: MovementObservation["failure"] }) {
  const configured = definitions.find((item) => item.role === role);
  const result = results.filter((item) => item.role === role).at(-1);
  const observedRoleFailure = failure?.stage?.number === stage.number && failure.role === role ? failure : null;
  const stageHasEvidence = results.length > 0 || (failure?.stage?.number === stage.number);
  const outcome = result ? roleOutcomeLabel(result) : observedRoleFailure
    ? `Observed failure · ${observedRoleFailure.message}`
    : configured
      ? stage.state === "accepted" ? "Applied · historical result unavailable" : stage.state === "pending" ? stageHasEvidence ? "No retained result for this role" : "Prior outcome unavailable" : "Not attempted"
    : role.startsWith("verify-") ? "Not configured" : "Missing · movement unavailable";
  const status = result?.state === "succeeded" ? "Process OK"
    : result?.state === "failed" ? `Exit ${result.exit_code ?? "unknown"}`
      : result?.state === "launch_failed" ? "Launch failed"
        : result?.state === "in_progress" ? "In progress"
          : observedRoleFailure ? "Failed"
            : configured ? stage.state === "accepted" ? "Applied" : stage.state === "future" ? "Not attempted" : "Unknown"
            : role.startsWith("verify-") ? "Optional" : "Missing";
  return <div className={`role-row ${configured ? "configured" : "not-configured"}`}>
    <span className="role-result-mark" aria-hidden="true">{result?.state === "succeeded" ? "✓" : result?.state === "failed" || result?.state === "launch_failed" || observedRoleFailure ? "!" : result?.state === "in_progress" ? "◌" : configured ? "·" : "—"}</span>
    <span className="role-row-copy"><b>{role}</b><small>{outcome}</small></span>
    <span className={`role-status ${result?.state === "failed" || result?.state === "launch_failed" || observedRoleFailure ? "error" : configured && !result && stage.state === "pending" ? "unknown" : ""}`}>{status}</span>
  </div>;
}

function roleOutcomeLabel(result: RoleObservation): string {
  switch (result.state) {
    case "in_progress": return "Attempt observed · process launch is not confirmed";
    case "succeeded": return `Process succeeded${result.elapsed_ms === null ? "" : ` · ${result.elapsed_ms} ms`}`;
    case "failed": return `Process failed${result.exit_code === null ? "" : ` · exit ${result.exit_code}`}${result.elapsed_ms === null ? "" : ` · ${result.elapsed_ms} ms`}`;
    case "launch_failed": return "Could not start authored executable";
  }
}

function RoleOutput({ workspaceId, operationId, result }: { workspaceId: string | null; operationId: string | null; result: RoleObservation }) {
  return <div className="captured-role">
    <div className="captured-role-heading"><b>Stage {result.stage.number} · {result.role}</b><span>{roleOutcomeLabel(result)}</span></div>
    {result.output_state === "not_returned" && <p className="subtle-note">Output is buffered until this role returns; bytes are not streamed.</p>}
    {result.output_state === "unavailable" && <p className="subtle-note">Output is unavailable because the executable could not be started.</p>}
    {result.output_state === "evicted" && <p className="subtle-note">Output was evicted to keep in-process retention bounded.</p>}
    {result.output_state === "available" && result.output_id && workspaceId && <>
      <CapturedStream key={`${outputIdentity(workspaceId, operationId, result)}:stdout`} workspaceId={workspaceId} operationId={operationId} result={result} streamName="stdout" />
      <CapturedStream key={`${outputIdentity(workspaceId, operationId, result)}:stderr`} workspaceId={workspaceId} operationId={operationId} result={result} streamName="stderr" />
      {(result.stdout_truncated || result.stderr_truncated) && <small className="truncated-note">A stream exceeds the 1 MiB capture limit. The retained prefix is marked truncated; the original byte count is shown.</small>}
    </>}
  </div>;
}

function CapturedStream({ workspaceId, operationId, result, streamName }: { workspaceId: string; operationId: string | null; result: RoleObservation; streamName: "stdout" | "stderr" }) {
  const [preview, setPreview] = useState<string | null>(null);
  const [issue, setIssue] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const requestGeneration = useRef(0);
  const length = streamName === "stdout" ? result.stdout_bytes : result.stderr_bytes;
  const truncated = streamName === "stdout" ? result.stdout_truncated : result.stderr_truncated;
  const endpoint = `/api/workspaces/${encodeURIComponent(workspaceId)}/outputs/${encodeURIComponent(result.output_id!)}/${streamName}`;

  useEffect(() => () => {
    requestGeneration.current += 1;
  }, [endpoint, operationId, result.stage.number, result.role, streamName]);

  async function showPreview() {
    const requestId = ++requestGeneration.current;
    setLoading(true);
    setIssue(null);
    try {
      const response = await fetch(endpoint, { credentials: "same-origin" });
      if (!response.ok) throw new Error(response.status === 410 ? "Output is no longer retained." : `Output read failed (${response.status}).`);
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (requestGeneration.current === requestId) setPreview(safeOutputPreview(bytes));
    } catch (error) {
      if (requestGeneration.current === requestId) setIssue(error instanceof Error ? error.message : "Output could not be read.");
    } finally {
      if (requestGeneration.current === requestId) setLoading(false);
    }
  }

  return <div className="captured-stream">
    <div className="captured-stream-heading"><span>{streamName}</span><small>{length.toLocaleString()} bytes{truncated ? " · truncated" : ""}</small></div>
    {length === 0 ? <small className="empty-stream">Empty stream</small> : <div className="captured-stream-actions">
      <button className="output-button" onClick={() => void showPreview()} disabled={loading}>{loading ? "Reading…" : preview === null ? `Preview ${streamName}` : `Reload ${streamName}`}</button>
      <a className="output-download" href={endpoint} download={`${result.stage.number}-${result.role}-${streamName}.bin`}>Download raw bytes</a>
    </div>}
    {issue && <small className="output-error" role="status">{issue}</small>}
    {preview !== null && <pre className="output-preview" aria-label={`${streamName} text preview`}>{preview || "(empty preview)"}</pre>}
  </div>;
}

function outputIdentity(workspaceId: string | null, operationId: string | null | undefined, result: RoleObservation): string {
  return [workspaceId ?? "", operationId ?? "", result.output_id ?? "", result.stage.number, result.role].join(":");
}

function safeOutputPreview(bytes: Uint8Array): string {
  const decoded = new TextDecoder("utf-8").decode(bytes);
  return decoded.replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001a\u001c-\u001f\u007f-\u009f]/g, (character) => {
    const code = character.codePointAt(0) ?? 0;
    return `⟦U+${code.toString(16).toUpperCase().padStart(4, "0")}⟧`;
  }).replace(/\u001b/g, "␛");
}

type MovementAction = {
  choice: MovementChoice;
  label: string;
  detail: string;
  disabledReason?: string;
};

function movementActionsFor(workspace: WorkspaceView): MovementAction[] {
  const stages = workspace.stages;
  const checkpoint = workspace.checkpoint;
  const pending = checkpoint.pending_transition;
  const observation = workspace.observation;
  const action = (choice: MovementChoice, label: string, detail: string, disabledReason?: string): MovementAction => ({ choice, label, detail, disabledReason });
  const stageFor = (number: number) => stages.find((stage) => stage.number === number);
  const targetLabel = (number: number) => number === 0 ? "baseline" : `Stage ${number} · ${stageFor(number)?.name ?? "unknown"}`;

  if (pending) {
    const stage = stageFor(pending.stage.number);
    const lowerNumber = stages.findIndex((candidate) => candidate.number === pending.stage.number) - 1;
    const actualLowerTarget = lowerNumber < 0 ? 0 : stages[lowerNumber]?.number ?? 0;
    const lowerName = targetLabel(actualLowerTarget);
    const verificationFailure = observation?.state === "stopped" && observation.verification_choices;
    if (verificationFailure) {
      const retry = verificationFailure.retry;
      const retryName = pending.direction === "up" ? "Retry verify-up" : "Retry verify-down";
      const reverse = verificationFailure.reverse;
      const reverseLabel = pending.direction === "up"
        ? `Back out Stage ${pending.stage.number} to ${targetLabel(reverse?.target_stage ?? actualLowerTarget)}`
        : `Reapply Stage ${pending.stage.number} upward`;
      return [
        action(retry, `${retryName} for Stage ${pending.stage.number}`, "The mutation is not repeated by this verifier retry."),
        ...(reverse ? [action(reverse, reverseLabel, pending.direction === "down" ? "Runs the upward mutation and optional verify-up for the pending down Stage." : "Runs the downward mutation and optional verify-down for the pending up Stage.")] : []),
      ];
    }

    const ambiguous = Boolean(observation?.state === "stopped" && observation.failure
      && (observation.failure.kind === "checkpoint_save_failed"
        || (["up", "down"].includes(observation.failure.role ?? "")
          && ["process_failed", "launch_failed"].includes(observation.failure.kind))));
    if (ambiguous) {
      return [
        action({ direction: "up", target_stage: pending.stage.number }, `Continue up toward Stage ${pending.stage.number}`, "Inspect author-owned effects before continuing.", stage?.definitions.some((item) => item.role === "up") ? undefined : "Stage has no up executable."),
        action({ direction: "down", target_stage: actualLowerTarget }, `Continue down toward ${lowerName}`, "Inspect author-owned effects before continuing.", stage?.definitions.some((item) => item.role === "down") ? undefined : "Stage has no down executable."),
      ];
    }

    if (pending.direction === "up") {
      return [
        action({ direction: "up", target_stage: pending.stage.number }, `Continue pending up · Stage ${pending.stage.number}`, "Prior execution result is unavailable; this resumes the pending upward transition.", stage?.definitions.some((item) => item.role === "up") ? undefined : "Stage has no up executable."),
        action({ direction: "down", target_stage: actualLowerTarget }, `Back out Stage ${pending.stage.number} to ${lowerName}`, "Uses the pending Stage’s down role.", stage?.definitions.some((item) => item.role === "down") ? undefined : "Stage has no down executable."),
      ];
    }
    return [
      action({ direction: "down", target_stage: actualLowerTarget }, `Continue pending down · Stage ${pending.stage.number}`, "Prior execution result is unavailable; this resumes the pending downward transition.", stage?.definitions.some((item) => item.role === "down") ? undefined : "Stage has no down executable."),
      action({ direction: "up", target_stage: pending.stage.number }, `Reapply Stage ${pending.stage.number} upward`, "Runs the upward mutation and optional verify-up.", stage?.definitions.some((item) => item.role === "up") ? undefined : "Stage has no up executable."),
    ];
  }

  const acceptedIndex = checkpoint.accepted_stage
    ? stages.findIndex((stage) => stage.number === checkpoint.accepted_stage?.number)
    : -1;
  const acceptedCount = acceptedIndex + 1;
  const next = stages[acceptedCount];
  const actions: MovementAction[] = [];
  if (next) {
    actions.push(action(
      { direction: "up", target_stage: next.number },
      `Advance to Stage ${next.number} · ${next.name}`,
      `Runs Stage ${next.number} up${next.definitions.some((item) => item.role === "verify-up") ? " then verify-up" : "; no verify-up is configured"}.`,
      next.definitions.some((item) => item.role === "up") ? undefined : "This Stage has no up executable.",
    ));
  }
  if (acceptedCount > 0) {
    const reversing = stages[acceptedCount - 1];
    const target = acceptedCount > 1 ? stages[acceptedCount - 2].number : 0;
    actions.push(action(
      { direction: "down", target_stage: target },
      `Back out Stage ${reversing.number} to ${targetLabel(target)}`,
      `Runs Stage ${reversing.number} down${reversing.definitions.some((item) => item.role === "verify-down") ? " then verify-down" : "; no verify-down is configured"}.`,
      reversing.definitions.some((item) => item.role === "down") ? undefined : "This Stage has no down executable.",
    ));
  }
  return actions;
}

function applyRuntimeSnapshot(view: WorkspaceView, snapshot: RuntimeSnapshot): WorkspaceView {
  if (snapshot.workspace_id !== view.workspace.id
    || snapshot.server_instance_id !== view.server_instance_id
    || snapshot.revision < view.observation_revision) return view;
  const checkpoint = snapshot.observation?.confirmed_checkpoint ?? snapshot.checkpoint;
  const updated = {
    ...view,
    server_instance_id: snapshot.server_instance_id,
    observation_revision: snapshot.revision,
    movement_busy: snapshot.movement_busy,
    observation: snapshot.observation,
  };
  return checkpoint ? applyCheckpoint(updated, checkpoint) : updated;
}

function preferredInspectionStage(
  view: WorkspaceView,
  deliberate: { workspaceId: string; stageNumber: number } | null,
  localResult: { serverInstanceId: string; operationId: string; stageNumber: number } | null,
): number | null {
  if (deliberate?.workspaceId === view.workspace.id
    && view.stages.some((stage) => stage.number === deliberate.stageNumber)) return deliberate.stageNumber;
  if (localResult?.serverInstanceId === view.server_instance_id
    && localResult.operationId === view.observation?.operation_id
    && view.stages.some((stage) => stage.number === localResult.stageNumber)) return localResult.stageNumber;
  const checkpointStages = [view.checkpoint.pending_transition?.stage.number, view.checkpoint.accepted_stage?.number];
  for (const checkpointStage of checkpointStages) {
    if (checkpointStage !== undefined && view.stages.some((stage) => stage.number === checkpointStage)) return checkpointStage;
  }
  return view.stages[0]?.number ?? null;
}

function applyCheckpoint(view: WorkspaceView, checkpoint: WorkspaceView["checkpoint"]): WorkspaceView {
  const acceptedIndex = checkpoint.accepted_stage
    ? view.stages.findIndex((stage) => stage.number === checkpoint.accepted_stage?.number)
    : -1;
  const pendingNumber = checkpoint.pending_transition?.stage.number;
  return {
    ...view,
    checkpoint,
    stages: view.stages.map((stage, index) => ({
      ...stage,
      state: stage.number === pendingNumber ? "pending" : index <= acceptedIndex ? "accepted" : "future",
      is_accepted_checkpoint: index === acceptedIndex,
    })),
  };
}

function rememberWorkspaceView(snapshots: Map<string, RuntimeSnapshot>, view: WorkspaceView) {
  const snapshot: RuntimeSnapshot = {
    workspace_id: view.workspace.id,
    server_instance_id: view.server_instance_id,
    revision: view.observation_revision,
    movement_busy: view.movement_busy,
    checkpoint: view.checkpoint,
    observation: view.observation,
  };
  const current = snapshots.get(view.workspace.id);
  if (current && (current.server_instance_id !== snapshot.server_instance_id
    || current.revision > snapshot.revision)) return;
  snapshots.set(view.workspace.id, snapshot);
}

function rememberEventRevision(
  revisions: Map<string, { server_instance_id: string; revision: number }>,
  workspaceId: string,
  serverInstanceId: string,
  revision: number,
) {
  const current = revisions.get(workspaceId);
  if (current?.server_instance_id === serverInstanceId && current.revision > revision) return;
  revisions.set(workspaceId, { server_instance_id: serverInstanceId, revision });
}

function updateWorkspaceSummary(project: ProjectView, workspaceId: string, checkpoint: WorkspaceView["checkpoint"]): ProjectView {
  return {
    ...project,
    workspaces: project.workspaces.map((item) => item.id === workspaceId ? {
      ...item,
      accepted_stage: checkpoint.accepted_stage,
      pending_transition: checkpoint.pending_transition,
    } : item),
  };
}

function reconcileProjectSummaries(project: ProjectView, snapshots: Map<string, RuntimeSnapshot>): ProjectView {
  return {
    ...project,
    workspaces: project.workspaces.map((workspace) => {
      const snapshot = snapshots.get(workspace.id);
      const checkpoint = snapshot?.observation?.confirmed_checkpoint ?? snapshot?.checkpoint;
      return checkpoint ? {
        ...workspace,
        accepted_stage: checkpoint.accepted_stage,
        pending_transition: checkpoint.pending_transition,
      } : workspace;
    }),
  };
}

function pendingEvidenceSummary(stageNumber: number, observation: MovementObservation | null): string | null {
  if (!observation) return null;
  const failure = observation.failure;
  if (failure?.kind === "checkpoint_save_failed"
    && observation.attempted_checkpoint?.pending_transition?.stage.number === stageNumber) {
    return "checkpoint publication failed";
  }
  if (failure?.stage?.number === stageNumber && failure.role) return `${failure.role} failed`;
  if (observation.role_results.some((result) => result.stage.number === stageNumber)) return "current-session role results retained";
  return null;
}

function checkpointPosition(checkpoint: WorkspaceView["checkpoint"] | null): string {
  return checkpoint?.accepted_stage
    ? `Stage ${checkpoint.accepted_stage.number} · ${checkpoint.accepted_stage.name}`
    : "Baseline · no accepted stages";
}

function pendingPosition(checkpoint: WorkspaceView["checkpoint"] | null): string {
  const pending = checkpoint?.pending_transition;
  return pending ? `${pending.direction} · Stage ${pending.stage.number} · ${pending.stage.name}` : "None";
}

function setupHint(issue: string): string {
  return issue.toLowerCase().includes("sqlite") || issue.toLowerCase().includes("checkpoint") || issue.toLowerCase().includes("schema")
    ? "Prepare storage explicitly with control-tower-db; Control Tower never initializes it during a status read."
    : "Check that this workspace has a readable stages/ directory and valid numbered stage folders.";
}
