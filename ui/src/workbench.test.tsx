import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiFailure, api, submitMovement, type CheckpointState, type MovementChoice } from "./types";

afterEach(() => vi.unstubAllGlobals());

const movement: MovementChoice = { direction: "up", target_stage: 200 };
const checkpoint: CheckpointState = { completed_stage_count: 1, uuid: "run-1", pending: { stage_index: 1, direction: "up" } };

describe("workbench API contract", () => {
  it("submits the complete expected checkpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(submitMovement("/api/workflows/demo/movements", movement, checkpoint)).resolves.toBeUndefined();
    expect(fetchMock).toHaveBeenCalledWith("/api/workflows/demo/movements", expect.objectContaining({
      method: "POST",
      body: JSON.stringify({ ...movement, expected_checkpoint: checkpoint }),
    }));
  });

  it("keeps typed stale-checkpoint errors for the UI to report without retrying", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json({ error: { code: "stale_checkpoint", message: "state changed" } }, { status: 409 })));

    const error = await submitMovement("/api/workflows/demo/movements", movement, checkpoint).catch((value: unknown) => value);
    expect(error).toBeInstanceOf(ApiFailure);
    expect(error).toMatchObject({ code: "stale_checkpoint", status: 409, message: "state changed" });
  });

  it("returns the ordinary inventory data", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json({ name: "demo", workflows: [{ id: "scratch", name: "scratch" }] })));
    await expect(api("/api/workspace")).resolves.toEqual({ name: "demo", workflows: [{ id: "scratch", name: "scratch" }] });
  });
});
