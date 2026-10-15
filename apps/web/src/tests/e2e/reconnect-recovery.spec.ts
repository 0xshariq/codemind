import { describe, expect, it } from "vitest";
import { getOperation, publishOperation } from "../../lib/server/operations/service";

describe("operation reconnect recovery", () => {
  it("returns ordered persisted operation history", () => {
    publishOperation({ operationId: "reconnect-test", status: "planning", sequence: 1 });
    publishOperation({ operationId: "reconnect-test", status: "completed", sequence: 2 });
    expect(getOperation("reconnect-test").map((item) => item.sequence)).toEqual([1, 2]);
  });
});