import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getRepository: vi.fn(),
  canAccessOperation: vi.fn(),
}));

vi.mock("server-only", () => ({}));
vi.mock("../../lib/server/repositories/service", () => ({ getRepository: mocks.getRepository }));
vi.mock("../../lib/server/operations/service", () => ({ canAccessOperation: mocks.canAccessOperation }));

import { authorizeOperationOwner, authorizeRepositoryContext } from "../../lib/server/auth/authorize";
import type { AuthenticatedSession } from "../../lib/server/auth/context";

const session: AuthenticatedSession = {
  version: 1,
  sessionId: "session-a",
  userId: "user-a",
  workspaceId: "workspace-a",
  allowedScopes: ["repository"],
  teamIds: [],
  issuedAt: 1,
  expiresAt: 100,
};
const current = {
  workspaceId: "workspace-a",
  repositoryId: "repo-a",
  sourceSnapshotId: "snapshot-a",
  graphVersion: "graph-a",
};

describe("repository and snapshot isolation", () => {
  beforeEach(() => {
    mocks.getRepository.mockReset();
    mocks.canAccessOperation.mockReset();
    mocks.getRepository.mockImplementation(async (_workspaceId: string, repositoryId: string) => repositoryId === "repo-a"
      ? { repositoryId: "repo-a", sourceSnapshotId: "snapshot-a", graphVersion: "graph-a" }
      : null);
  });

  it("allows only the current authorized workspace, repository, snapshot and graph tuple", async () => {
    await expect(authorizeRepositoryContext(session, current)).resolves.toBe(true);
    await expect(authorizeRepositoryContext(session, { ...current, workspaceId: "workspace-b" })).resolves.toBe(false);
    await expect(authorizeRepositoryContext(session, { ...current, repositoryId: "repo-b" })).resolves.toBe(false);
    await expect(authorizeRepositoryContext(session, { ...current, sourceSnapshotId: "snapshot-old" })).resolves.toBe(false);
    await expect(authorizeRepositoryContext(session, { ...current, graphVersion: "graph-old" })).resolves.toBe(false);
  });

  it("denies semantic reads when the repository scope is absent or the current snapshot cannot be resolved", async () => {
    await expect(authorizeRepositoryContext({ ...session, allowedScopes: ["user"] }, current)).resolves.toBe(false);
    mocks.getRepository.mockResolvedValue(null);
    await expect(authorizeRepositoryContext(session, current)).resolves.toBe(false);
  });

  it("uses the persisted operation owner boundary instead of trusting a caller supplied ID", () => {
    mocks.canAccessOperation.mockReturnValue(false);
    expect(authorizeOperationOwner(session, "operation-a")).toBe(false);
    mocks.canAccessOperation.mockReturnValue(true);
    expect(authorizeOperationOwner(session, "operation-a")).toBe(true);
    expect(mocks.canAccessOperation).toHaveBeenLastCalledWith("operation-a", session);
  });
});
