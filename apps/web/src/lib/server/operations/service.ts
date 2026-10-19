import "server-only";

import { createHash, randomUUID } from "node:crypto";
import { closeSync, mkdirSync, openSync, readFileSync, renameSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { OperationEvent } from "./events";
import type { OperationState } from "./state";
import { event } from "./events";

const operations = new Map<string, OperationEvent[]>();
const results = new Map<string, { answer?: string; canonical?: unknown; error?: string; handoffId?: string; handoffStatus?: string; targetStage?: string }>();
const operationOwners = new Map<string, { userId: string; workspaceId: string }>();
const TERMINAL_TTL_MS = 5 * 60 * 1000;

type StoredOperation = {
  owner?: { userId: string; workspaceId: string };
  events: OperationEvent[];
  result?: { answer?: string; canonical?: unknown; error?: string; handoffId?: string; handoffStatus?: string; targetStage?: string };
};

function operationFile(operationId: string) {
  const root = path.resolve(process.env.CODEMIND_REPOSITORY_STORAGE_ROOT ?? path.join(process.cwd(), ".codemind", "repositories"));
  return path.join(root, "operations", `${createHash("sha256").update(operationId).digest("hex")}.json`);
}

function load(operationId: string, fresh = false): StoredOperation {
  const cachedOwner = operationOwners.get(operationId);
  const cachedEvents = operations.get(operationId);
  const cachedResult = results.get(operationId);
  if (!fresh && (cachedOwner || cachedEvents || cachedResult)) {
    return { owner: cachedOwner, events: cachedEvents ?? [], result: cachedResult };
  }
  try {
    const file = operationFile(operationId);
    const stored = JSON.parse(readFileSync(file, "utf8")) as StoredOperation;
    if (!stored || !Array.isArray(stored.events)) return { events: [] };
    if (stored.owner) operationOwners.set(operationId, stored.owner);
    if (stored.events.length) operations.set(operationId, stored.events);
    if (stored.result) results.set(operationId, stored.result);
    return stored;
  } catch { return { events: [] }; }
}

function withOperationLock<T>(operationId: string, action: () => T): T {
  const file = operationFile(operationId);
  mkdirSync(path.dirname(file), { recursive: true, mode: 0o700 });
  const lock = `${file}.lock`;
  const wait = new Int32Array(new SharedArrayBuffer(4));
  const deadline = Date.now() + 5_000;
  let descriptor: number | undefined;
  while (descriptor === undefined) {
    try { descriptor = openSync(lock, "wx", 0o600); }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
      try {
        if (Date.now() - statSync(lock).mtimeMs > 30_000) unlinkSync(lock);
      } catch { /* Another process released or reclaimed the lock. */ }
      if (Date.now() >= deadline) throw new Error("operation state is busy; retry the update");
      Atomics.wait(wait, 0, 0, 10);
    }
  }
  try { return action(); }
  finally {
    closeSync(descriptor);
    try { unlinkSync(lock); } catch { /* The next caller will detect a released lock file. */ }
  }
}

function refreshCache(operationId: string, stored: StoredOperation) {
  if (stored.owner) operationOwners.set(operationId, stored.owner); else operationOwners.delete(operationId);
  if (stored.events.length) operations.set(operationId, stored.events); else operations.delete(operationId);
  if (stored.result) results.set(operationId, stored.result); else results.delete(operationId);
}

function persist(operationId: string) {
  const stored: StoredOperation = {
    owner: operationOwners.get(operationId),
    events: operations.get(operationId) ?? [],
    result: results.get(operationId),
  };
  const serialized = JSON.stringify(stored);
  if (serialized.length > 4_000_000) throw new Error("operation state exceeds persistent storage limit");
  const file = operationFile(operationId);
  mkdirSync(path.dirname(file), { recursive: true, mode: 0o700 });
  const temporary = `${file}.${randomUUID()}.tmp`;
  writeFileSync(temporary, serialized, { encoding: "utf8", mode: 0o600, flag: "wx" });
  renameSync(temporary, file);
}

function scheduleEviction(operationId: string) {
  setTimeout(() => {
    withOperationLock(operationId, () => {
      const stored = load(operationId, true);
      const last = stored.events.at(-1);
      if (!last || !isTerminal(last.status) || Date.now() - Date.parse(last.timestamp) < TERMINAL_TTL_MS) return;
      operations.delete(operationId);
      results.delete(operationId);
      operationOwners.delete(operationId);
      try { unlinkSync(operationFile(operationId)); } catch { /* Already evicted. */ }
    });
  }, TERMINAL_TTL_MS).unref?.();
}

export function bindOperationOwner(operationId: string, owner: { userId: string; workspaceId: string }) {
  if (!operationId.trim() || !owner.userId.trim() || !owner.workspaceId.trim()) {
    throw new Error("operation ownership requires an ID, user and workspace");
  }
  withOperationLock(operationId, () => {
    const stored = load(operationId, true);
    refreshCache(operationId, stored);
    if (stored.owner && (stored.owner.userId !== owner.userId || stored.owner.workspaceId !== owner.workspaceId)) {
      throw new Error("operation ownership cannot be reassigned");
    }
    operationOwners.set(operationId, { ...owner });
    persist(operationId);
  });
}

export function canAccessOperation(operationId: string, owner: { userId: string; workspaceId: string }) {
  const stored = load(operationId, true).owner;
  return !!stored && stored.userId === owner.userId && stored.workspaceId === owner.workspaceId;
}

export function isWebOperation(operationId: string) {
  return !!load(operationId, true).owner;
}

export function publishOperation(state: OperationState): OperationEvent {
  const published = withOperationLock(state.operationId, () => {
    const stored = load(state.operationId, true);
    refreshCache(state.operationId, stored);
    const history = [...stored.events];
    const last = history.at(-1);
    if (last && (state.sequence <= last.sequence || isTerminal(last.status))) return last;
    const next = event(state);
    history.push(next);
    operations.set(state.operationId, history);
    persist(state.operationId);
    if (isTerminal(state.status)) scheduleEviction(state.operationId);
    return next;
  });
  return published;
}

function isTerminal(status: OperationState["status"]) {
  return status === "completed" || status === "partial" || status === "failed";
}

export function getOperation(operationId: string): OperationEvent[] {
  return [...load(operationId, true).events];
}

export function setOperationResult(operationId: string, result: { answer?: string; canonical?: unknown; error?: string; handoffId?: string; handoffStatus?: string; targetStage?: string }) {
  withOperationLock(operationId, () => {
    const stored = load(operationId, true);
    refreshCache(operationId, stored);
    results.set(operationId, result);
    persist(operationId);
  });
}

export function getOperationResult(operationId: string) {
  return load(operationId, true).result;
}
