import { useCallback, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export interface LlmConfigInput {
  api_base: string
  api_key: string
  model: string
}

export interface LlmConfigStatus {
  configured: boolean
  model: string
  api_base: string
}

export interface ExtractedEntity {
  node_type: string
  label: string
  description: string
  confidence: number
  properties: Record<string, unknown>
}

export interface ExtractedRelation {
  source_index: number
  target_index: number
  relation_type: string
  label: string
  confidence: number
}

export interface Suggestion {
  action: 'add_node' | 'add_relation' | 'query_external' | 'investigate'
  description: string
  entity_type?: string | null
  relation_type?: string | null
  confidence: number
}

export interface AgentAction {
  action: 'AddNode' | 'AddRelation' | 'QueryExternal'
  params: Record<string, unknown>
}

export interface AgentPlan {
  reasoning: string
  actions: AgentAction[]
}

export interface ActionApproval {
  action_index: number
  approved: boolean
  modifications?: string | null
}

export interface ConfigureLlmParams {
  apiBase: string
  apiKey: string
  model: string
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function useLLM() {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const configureLlm = useCallback(
    async ({ apiBase, apiKey, model }: ConfigureLlmParams): Promise<LlmConfigStatus> => {
      setError(null)
      try {
        const config: LlmConfigInput = {
          api_base: apiBase,
          api_key: apiKey,
          model,
        }
        return await invoke<LlmConfigStatus>('configure_llm', { config })
      } catch (e) {
        const message = toErrorMessage(e)
        setError(message)
        throw e
      }
    },
    [],
  )

  const getLlmConfig = useCallback(async (): Promise<LlmConfigStatus> => {
    setError(null)
    try {
      return await invoke<LlmConfigStatus>('get_llm_config')
    } catch (e) {
      const message = toErrorMessage(e)
      setError(message)
      throw e
    }
  }, [])

  const extractFromText = useCallback(async (text: string): Promise<ExtractedEntity[]> => {
    setIsLoading(true)
    setError(null)
    try {
      return await invoke<ExtractedEntity[]>('extract_from_text', { text })
    } catch (e) {
      const message = toErrorMessage(e)
      setError(message)
      throw e
    } finally {
      setIsLoading(false)
    }
  }, [])

  const extractRelations = useCallback(
    async (text: string, entities: ExtractedEntity[]): Promise<ExtractedRelation[]> => {
      setIsLoading(true)
      setError(null)
      try {
        return await invoke<ExtractedRelation[]>('extract_relations_from_text', { entities, text })
      } catch (e) {
        const message = toErrorMessage(e)
        setError(message)
        throw e
      } finally {
        setIsLoading(false)
      }
    },
    [],
  )

  const getSuggestions = useCallback(async (nodeInfo: string): Promise<Suggestion[]> => {
    setError(null)
    try {
      return await invoke<Suggestion[]>('suggest_next_steps', { node_info: nodeInfo })
    } catch (e) {
      const message = toErrorMessage(e)
      setError(message)
      throw e
    }
  }, [])

  const analyzeGraph = useCallback(
    async (nodeSummaries: string[], relationSummaries: string[]): Promise<AgentPlan> => {
      setIsLoading(true)
      setError(null)
      try {
        return await invoke<AgentPlan>('agent_analyze', {
          node_summaries: nodeSummaries,
          relation_summaries: relationSummaries,
        })
      } catch (e) {
        const message = toErrorMessage(e)
        setError(message)
        throw e
      } finally {
        setIsLoading(false)
      }
    },
    [],
  )

  const applyApprovals = useCallback(async (plan: AgentPlan, approvals: ActionApproval[]): Promise<AgentAction[]> => {
    setError(null)
    try {
      return await invoke<AgentAction[]>('agent_apply_approvals', { plan, approvals })
    } catch (e) {
      const message = toErrorMessage(e)
      setError(message)
      throw e
    }
  }, [])

  return {
    isLoading,
    error,
    configureLlm,
    getLlmConfig,
    extractFromText,
    extractRelations,
    getSuggestions,
    analyzeGraph,
    applyApprovals,
  }
}
