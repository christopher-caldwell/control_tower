import { mkdir, readFile, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { dirname, join } from "node:path";

function stateFile() {
  const workflow = process.env.CONTROL_TOWER_WORKFLOW;
  if (!workflow) throw new Error("CONTROL_TOWER_WORKFLOW is required");
  return join(workflow, "data", "state.json");
}

export async function saveValue(key, value) {
  const path = stateFile();
  let state = {};
  try {
    state = JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  state[key] = value;
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(state, null, 2)}\n`);
}

export async function loadValue(key) {
  const state = JSON.parse(await readFile(stateFile(), "utf8"));
  if (!Object.hasOwn(state, key)) throw new Error(`Missing saved value: ${key}`);
  return state[key];
}

export async function requireAbsentValue(key) {
  const state = JSON.parse(await readFile(stateFile(), "utf8"));
  if (Object.hasOwn(state, key)) throw new Error(`Expected saved value to be absent: ${key}`);
}

export async function deleteValue(key) {
  const path = stateFile();
  const state = JSON.parse(await readFile(path, "utf8"));
  delete state[key];
  await writeFile(path, `${JSON.stringify(state, null, 2)}\n`);
}

export function capture(command, ...args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8").on("data", (chunk) => { stdout += chunk; });
    child.stderr.setEncoding("utf8").on("data", (chunk) => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", (code) => resolve({ code, stdout, stderr }));
  });
}

export function requireSuccess(result) {
  if (result.code !== 0) throw new Error(`Operation failed (${result.code}): ${result.stderr}`);
  return result;
}
