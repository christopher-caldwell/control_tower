import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiFailure, api, submitMovement, type CheckpointState, type MovementChoice } from "./types";

afterEach(() => vi.unstubAllGlobals());

const movement: MovementChoice = { direction: "up", target_stage: 200 };
const checkpoint: CheckpointState = { completed_stage_count: 1, uuid: "run-1", pending: { stage_index: 1, direction: "up" } };

describe("workbench API contract", () => {
  it("submits the full expected checkpoint and accepts an empty 204 completion", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(submitMovement("/api/workspaces/demo/movements", movement, checkpoint)).resolves.toBeUndefined();

    expect(fetchMock).toHaveBeenCalledWith("/api/workspaces/demo/movements", expect.objectContaining({
      method: "POST",
      credentials: "same-origin",
      body: JSON.stringify({ ...movement, expected_checkpoint: checkpoint }),
    }));
  });

  it("keeps typed stale-checkpoint errors for the UI to reconnect without retrying", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json({ error: { code: "stale_checkpoint", message: "state changed" } }, { status: 409 })));

    const error = await submitMovement("/api/workspaces/demo/movements", movement, checkpoint).catch((value: unknown) => value);
    expect(error).toBeInstanceOf(ApiFailure);
    expect(error).toMatchObject({ code: "stale_checkpoint", status: 409, message: "state changed" });
  });

  it("turns a fresh workspace read failure into the shared typed error", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json({ error: { code: "workspace_unavailable", message: "database is locked" } }, { status: 503 })));

    await expect(api("/api/workspaces/demo")).rejects.toBeInstanceOf(ApiFailure);
  });
});
