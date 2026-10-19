import { describe, expect, it } from "vitest";
import { rustProgressState } from "./rust-progress";

describe("Rust progress mapping", () => {
  it.each([
    ["coie", "coie"],
    ["tie", "tie"],
    ["chie", "chie"],
  ] as const)("preserves the %s engine stage in operation state", (stage, status) => {
    const result = rustProgressState("operation", 4, {
      eventId: "event-9",
      sequence: 9,
      engine: "codemind",
      stage,
      stageState: "running",
      timestamp: "2026-10-18T12:00:00Z",
      correlation: null,
      context: null,
      message: `${stage} stage started`,
    });

    expect(result).toMatchObject({
      operationId: "operation",
      sequence: 4,
      status,
      engine: "codemind",
      stage,
      stageState: "running",
      progressSequence: 9,
      progressEventId: "event-9",
      message: `${stage} stage started`,
    });
  });
});
