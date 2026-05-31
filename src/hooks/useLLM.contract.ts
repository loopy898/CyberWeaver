import {
  useLLM,
  type ActionApproval,
  type AgentAction,
  type AgentPlan,
  type ConfigureLlmParams,
  type ExtractedEntity,
  type ExtractedRelation,
  type LlmConfigStatus,
  type Suggestion,
} from './useLLM'

// Compile-time contract for the frontend LLM hook surface.
export function verifyUseLlmContract(hook: ReturnType<typeof useLLM>) {
  const configureLlm: (params: ConfigureLlmParams) => Promise<LlmConfigStatus> =
    hook.configureLlm
  const getLlmConfig: () => Promise<LlmConfigStatus> = hook.getLlmConfig
  const extractFromText: (text: string) => Promise<ExtractedEntity[]> = hook.extractFromText
  const extractRelations: (text: string, entities: ExtractedEntity[]) => Promise<ExtractedRelation[]> =
    hook.extractRelations
  const getSuggestions: (nodeInfo: string) => Promise<Suggestion[]> = hook.getSuggestions
  const analyzeGraph: (nodeSummaries: string[], relationSummaries: string[]) => Promise<AgentPlan> =
    hook.analyzeGraph
  const applyApprovals: (plan: AgentPlan, approvals: ActionApproval[]) => Promise<AgentAction[]> =
    hook.applyApprovals

  return {
    isLoading: hook.isLoading,
    error: hook.error,
    configureLlm,
    getLlmConfig,
    extractFromText,
    extractRelations,
    getSuggestions,
    analyzeGraph,
    applyApprovals,
  }
}
