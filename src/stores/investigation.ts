import { create } from 'zustand'
import type { Editor } from 'tldraw'

interface Investigation {
  id: string
  name: string
}

interface InvestigationState {
  // 当前调查
  currentId: string | null
  investigations: Investigation[]
  setCurrentId: (id: string) => void
  addInvestigation: (inv: Investigation) => void

  // tldraw Editor 实例（供外部 toolbar 使用）
  editor: Editor | null
  setEditor: (editor: Editor | null) => void
}

export const useInvestigationStore = create<InvestigationState>((set) => ({
  currentId: null,
  investigations: [],

  setCurrentId: (id) => set({ currentId: id }),

  addInvestigation: (inv) =>
    set((state) => ({ investigations: [...state.investigations, inv] })),

  editor: null,
  setEditor: (editor) => set({ editor }),
}))
