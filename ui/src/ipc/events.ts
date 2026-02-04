import { listen, type UnlistenFn, type Event } from '@tauri-apps/api/event'

export type EventEnvelope<T> = {
  runId: string
  seq: number
  ts: number
  type: string
  payload: T
}

// Minimal board types (replace with real ones)
export type BoardSnapshot = { runId: string; nodes: any[]; edges: any[]; sparks: any[] }
export type BoardDiff = { upsertNodes?: any[]; deleteNodeIds?: string[]; upsertEdges?: any[]; deleteEdgeIds?: string[]; upsertSparks?: any[] }

export type AppEventMap = {
  'board.snapshot': EventEnvelope<BoardSnapshot>
  'board.diff': EventEnvelope<BoardDiff>
}

export async function on<E extends keyof AppEventMap>(
  eventName: E,
  handler: (env: AppEventMap[E]) => void
): Promise<UnlistenFn> {
  return await listen<AppEventMap[E]>(eventName as string, (event: Event<AppEventMap[E]>) => {
    handler(event.payload)
  })
}

export const onBoardSnapshot = (fn: (env: AppEventMap['board.snapshot']) => void) => on('board.snapshot', fn)
export const onBoardDiff = (fn: (env: AppEventMap['board.diff']) => void) => on('board.diff', fn)

export async function subscribeMany(unsubscribers: Array<Promise<UnlistenFn>>): Promise<UnlistenFn> {
  const unlistenFns = await Promise.all(unsubscribers)
  return () => unlistenFns.forEach((u) => u())
}
