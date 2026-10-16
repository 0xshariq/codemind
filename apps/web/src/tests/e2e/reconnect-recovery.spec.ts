import { describe, expect, it } from "vitest";
import { getOperation, publishOperation } from "../../lib/server/operations/service";

describe("operation reconnect recovery", () => {
  it("returns ordered persisted operation history", () => {
    publishOperation({ operationId: "reconnect-test", status: "planning", sequence: 1 });
    publishOperation({ operationId: "reconnect-test", status: "completed", sequence: 2 });
    expect(getOperation("reconnect-test").map((item) => item.sequence)).toEqual([1, 2]);
  });

  it("ignores duplicate and out-of-order events", () => {
    publishOperation({ operationId: "ordered-test", status: "planning", sequence: 2 });
    publishOperation({ operationId: "ordered-test", status: "coie", sequence: 1 });
    publishOperation({ operationId: "ordered-test", status: "tie", sequence: 2 });
    expect(getOperation("ordered-test").map((item) => item.sequence)).toEqual([2]);
  });

  it("preserves terminal state during reconnect races", () => {
    publishOperation({ operationId: "terminal-test", status: "completed", sequence: 3 });
    publishOperation({ operationId: "terminal-test", status: "failed", sequence: 4 });
    expect(getOperation("terminal-test").map((item) => item.status)).toEqual(["completed"]);
  });
});