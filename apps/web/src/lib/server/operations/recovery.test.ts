import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

vi.mock("server-only", () => ({}));

let storageRoot = "";
let previousRoot: string | undefined;

beforeEach(() => {
  previousRoot = process.env.CODEMIND_REPOSITORY_STORAGE_ROOT;
  storageRoot = mkdtempSync(path.join(tmpdir(), "codemind-operation-recovery-"));
  process.env.CODEMIND_REPOSITORY_STORAGE_ROOT = storageRoot;
});

afterEach(() => {
  if (previousRoot === undefined) delete process.env.CODEMIND_REPOSITORY_STORAGE_ROOT;
  else process.env.CODEMIND_REPOSITORY_STORAGE_ROOT = previousRoot;
  rmSync(storageRoot, { recursive: true, force: true });
});

describe("durable operation recovery", () => {
  it("returns ordered persisted history after the service module is reloaded", async () => {
    const service = await import("./service");
    service.bindOperationOwner("reconnect-test", { userId: "user", workspaceId: "workspace" });
    service.publishOperation({ operationId: "reconnect-test", status: "planning", sequence: 1 });
    service.publishOperation({ operationId: "reconnect-test", status: "completed", sequence: 2 });
    vi.resetModules();
    const restartedService = await import("./service");
    expect(restartedService.getOperation("reconnect-test").map((item) => item.sequence)).toEqual([1, 2]);
    expect(restartedService.canAccessOperation("reconnect-test", { userId: "user", workspaceId: "workspace" })).toBe(true);
  });

  it("ignores duplicate and out-of-order events without rewinding state", async () => {
    const { publishOperation, getOperation } = await import("./service");
    publishOperation({ operationId: "ordered-test", status: "planning", sequence: 2 });
    publishOperation({ operationId: "ordered-test", status: "coie", sequence: 1 });
    publishOperation({ operationId: "ordered-test", status: "tie", sequence: 2 });
    expect(getOperation("ordered-test").map((item) => item.sequence)).toEqual([2]);
  });

  it("keeps terminal state during reconnect races", async () => {
    const { publishOperation, getOperation } = await import("./service");
    publishOperation({ operationId: "terminal-test", status: "completed", sequence: 3 });
    publishOperation({ operationId: "terminal-test", status: "failed", sequence: 4 });
    expect(getOperation("terminal-test").map((item) => item.status)).toEqual(["completed"]);
  });
});
