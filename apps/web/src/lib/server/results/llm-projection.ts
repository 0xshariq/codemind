export function projectForLlm(value: Record<string, unknown>, maxItems = 20) {
  if (Array.isArray(value) || value === null || typeof value !== "object") return value;
  const projection: Record<string, unknown> = { ...value };
  for (const key of ["affectedFiles", "findings", "pathSteps", "testCandidates", "evidence"]) {
    const items = projection[key];
    if (Array.isArray(items) && items.length > maxItems) {
      projection[key] = items.slice(0, maxItems);
      projection.truncated = true;
      projection.completeness = "partial";
    }
  }
  for (const [key, item] of Object.entries(projection)) {
    if (item && typeof item === "object" && !Array.isArray(item)) {
      projection[key] = projectForLlm(item as Record<string, unknown>, maxItems);
    }
  }
  return projection;
}