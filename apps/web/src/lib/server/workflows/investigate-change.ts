import "server-only";
import { runAgent } from "../agent/runtime";
import type { AgentProvider } from "../agent/provider";
import type { AuthenticatedSession } from "../auth/context";
import { authorizeRepositoryContext } from "../auth/authorize";
import { retrieveAuthorizedMemory } from "../memory/retrieval";
import { toolInputSchema } from "../tools/schemas";
import { APPROVED_TOOL_NAMES } from "../tools/catalog";

type RepositoryContext = { workspaceId: string; repositoryId: string; sourceSnapshotId: string; graphVersion: string };

const SEARCH_STOP_WORDS = new Set([
  "and", "are", "for", "the", "about", "after", "also", "change", "could", "does", "from", "happen",
  "have", "into", "make", "new", "that", "them", "then", "this", "what", "when", "where", "which",
  "will", "with", "would", "your",
]);

function searchQueries(question: string): string[] {
  const terms = question.toLowerCase().match(/[a-z][a-z0-9_]{2,}/g) ?? [];
  const normalized = terms.map((term) => term.endsWith("ing") && term.length > 5
    ? term.slice(0, -3)
    : term.endsWith("s") && term.length > 4 ? term.slice(0, -1) : term);
  const useful = [...new Set(normalized.filter((term) => !SEARCH_STOP_WORDS.has(term)))].slice(0, 3);
  return useful.length ? useful : [question.trim().slice(0, 500)];
}

export function initialEvidenceToolCalls(
  question: string,
  symbolId: string | undefined,
  gatherAllTools: boolean,
): Array<{ name: string; input: Record<string, unknown> }> {
  if (gatherAllTools) {
    if (!symbolId?.trim()) throw new Error("a selected graph entity is required for initial evidence gathering");
    return APPROVED_TOOL_NAMES.flatMap<{ name: string; input: Record<string, unknown> }>((name) => {
      if (name === "search_code") return searchQueries(question).map((query) => ({ name, input: { query, limit: 20 } }));
      const input = name === "get_repository_context" ? {}
        : name === "get_historical_changes" ? { limit: 20 }
          : name === "get_symbol" ? { symbolId }
            : name === "query_code_relationships" || name === "get_test_intelligence" || name === "get_test_impact" || name === "get_evidence"
              ? { symbolId, limit: 20 }
              : { symbolId, maxDepth: 3 };
      return [{ name, input }];
    });
  }
  return [
    ...searchQueries(question).map((query) => ({ name: "search_code", input: { query, limit: 20 } })),
    ...(symbolId ? [{ name: "analyze_change", input: { symbolId } }] : []),
  ];
}

export async function investigateChange(input: {
  provider: AgentProvider;
  question: string;
  session: AuthenticatedSession;
  context: RepositoryContext;
  symbolId?: string;
  gatherAllTools?: boolean;
  conversationHistory?: readonly { role: "user" | "assistant"; content: string }[];
  onToolStart?: (name: string) => void;
  onReasoningStart?: () => void;
  onPlanningStart?: () => void;
  onPlanningIssue?: (reason: string) => void;
  onCanonicalResult?: (result: unknown) => void;
}) {
  if (input.session.workspaceId !== input.context.workspaceId || (input.symbolId !== undefined && !input.symbolId.trim())) {
    throw new Error("investigation identity or repository context is invalid");
  }
  if (!await authorizeRepositoryContext(input.session, input.context)) {
    throw new Error("investigation repository snapshot is not authorized or current");
  }
  const memoryCandidates = await retrieveAuthorizedMemory({
    context: input.context,
    scope: "repository",
    kind: "semantic",
    session: input.session,
  }).catch(() => []);
  const initialToolCalls = initialEvidenceToolCalls(input.question, input.symbolId, input.gatherAllTools === true);
  return runAgent(
    input.provider,
    input.question,
    initialToolCalls,
    input.onToolStart,
    input.onReasoningStart,
    input.onCanonicalResult,
    {
      context: input.context,
      symbolId: input.symbolId,
      conversationHistory: input.conversationHistory,
      memoryCandidates,
      preplannedOnly: true,
      authorizeTool: async (_name, value) => {
        const parsed = toolInputSchema.safeParse(value);
        if (!parsed.success || !await authorizeRepositoryContext(input.session, parsed.data.context)) {
          throw new Error("semantic tool context is unauthorized");
        }
        const context = parsed.data.context;
        if (context.repositoryId !== input.context.repositoryId || context.workspaceId !== input.context.workspaceId
          || context.sourceSnapshotId !== input.context.sourceSnapshotId || context.graphVersion !== input.context.graphVersion) {
          throw new Error("semantic tool context does not match the authorized snapshot");
        }
      },
    },
    input.onPlanningStart,
    input.onPlanningIssue,
  );
}
