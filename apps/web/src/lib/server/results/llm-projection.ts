export function projectForLlm(value: Record<string, unknown>, maxItems = 20) {
  const projection: Record<string, unknown> = { ...value };
  for (const key of ["affectedFiles", "findings", "pathSteps", "testCandidates", "evidence"]) {
    const items = projection[key];
    if (Array.isArray(items) && items.length > maxItems) {
      projection[key] = items.slice(0, maxItems);
      projection.truncated = true;
      projection.completeness = "partial";
    }
  }
  return projection;
}