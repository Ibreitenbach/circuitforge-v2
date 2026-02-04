export type BoardSnapshot = { runId: string; nodes: any[]; edges: any[]; sparks: any[] }
export type BoardDiff = { upsertNodes?: any[]; deleteNodeIds?: string[]; upsertEdges?: any[]; deleteEdgeIds?: string[]; upsertSparks?: any[] }

export type BoardState = {
  runId?: string
  nodes: Map<string, any>
  edges: Map<string, any>
  sparks: Map<string, any>
}

export function newBoardState(): BoardState {
  return { nodes: new Map(), edges: new Map(), sparks: new Map() }
}

export function applySnapshot(_state: BoardState, snap: BoardSnapshot): BoardState {
  const next = newBoardState()
  next.runId = snap.runId
  for (const n of snap.nodes) next.nodes.set(n.id, n)
  for (const e of snap.edges) next.edges.set(e.id, e)
  for (const s of snap.sparks) next.sparks.set(s.id, s)
  return next
}

export function applyDiff(state: BoardState, diff: BoardDiff): BoardState {
  const next: BoardState = {
    runId: state.runId,
    nodes: new Map(state.nodes),
    edges: new Map(state.edges),
    sparks: new Map(state.sparks),
  }
  if (diff.upsertNodes) for (const n of diff.upsertNodes) next.nodes.set(n.id, n)
  if (diff.deleteNodeIds) for (const id of diff.deleteNodeIds) next.nodes.delete(id)

  if (diff.upsertEdges) for (const e of diff.upsertEdges) next.edges.set(e.id, e)
  if (diff.deleteEdgeIds) for (const id of diff.deleteEdgeIds) next.edges.delete(id)

  if (diff.upsertSparks) for (const s of diff.upsertSparks) next.sparks.set(s.id, s)
  return next
}
