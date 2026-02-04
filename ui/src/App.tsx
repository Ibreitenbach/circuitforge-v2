import { useEffect, useState, useCallback } from 'react'
import { create } from 'zustand'
import { isTauri, runCreate, acEditFiles, acCreatePatch, dcApplyPatch, dcRunProbe, dcCloseRelay } from './ipc/commands'
import { ReportQueue } from './components/ReportQueue'
import { onBoardSnapshot, onBoardDiff, subscribeMany } from './ipc/events'
import { BoardState, newBoardState, applySnapshot, applyDiff, BoardSnapshot, BoardDiff } from './state/boardStore'

// Zustand store for board state
interface GameStore {
  board: BoardState
  runId: string | null
  selectedSparkId: string | null
  editBuffer: { path: string; contents: string }[]
  logs: string[]
  setBoard: (board: BoardState) => void
  setRunId: (runId: string | null) => void
  selectSpark: (sparkId: string | null) => void
  addEdit: (path: string, contents: string) => void
  clearEdits: () => void
  addLog: (msg: string) => void
}

const useGameStore = create<GameStore>((set) => ({
  board: newBoardState(),
  runId: null,
  selectedSparkId: null,
  editBuffer: [],
  logs: [],
  setBoard: (board) => set({ board }),
  setRunId: (runId) => set({ runId }),
  selectSpark: (sparkId) => set({ selectedSparkId: sparkId }),
  addEdit: (path, contents) => set((s) => ({ editBuffer: [...s.editBuffer, { path, contents }] })),
  clearEdits: () => set({ editBuffer: [] }),
  addLog: (msg) => set((s) => ({ logs: [...s.logs.slice(-50), `[${new Date().toLocaleTimeString()}] ${msg}`] })),
}))

// Node rendering based on kind
function NodeView({ node }: { node: any }) {
  const kindColors: Record<string, string> = {
    probe: '#4a9eff',
    fuse: '#ff6b6b',
    relay: '#51cf66',
    junction: '#868e96',
    source: '#ffd43b',
    sink: '#be4bdb',
  }
  const color = kindColors[node.kind] || '#868e96'
  const status = node.status?.probe || node.status?.gate || 'idle'

  return (
    <div style={{
      display: 'inline-flex',
      flexDirection: 'column',
      alignItems: 'center',
      padding: '8px 12px',
      margin: '4px',
      background: `${color}22`,
      border: `2px solid ${color}`,
      borderRadius: '8px',
      minWidth: '80px',
    }}>
      <span style={{ fontSize: '10px', opacity: 0.7 }}>{node.kind}</span>
      <span style={{ fontWeight: 'bold' }}>{node.id}</span>
      <span style={{ fontSize: '10px', color }}>{status}</span>
    </div>
  )
}

// Spark (player) rendering
function SparkView({ spark, selected, onClick }: { spark: any; selected: boolean; onClick: () => void }) {
  const isAC = spark.kind === 'ac'
  return (
    <div
      onClick={onClick}
      style={{
        display: 'inline-flex',
        flexDirection: 'column',
        alignItems: 'center',
        padding: '8px 12px',
        margin: '4px',
        background: selected ? '#4a9eff33' : '#2a2a4e',
        border: `2px solid ${isAC ? '#ffd43b' : '#51cf66'}`,
        borderRadius: '8px',
        cursor: 'pointer',
      }}>
      <span style={{ fontSize: '18px' }}>{isAC ? '⚡' : '🛡️'}</span>
      <span style={{ fontWeight: 'bold' }}>{spark.id}</span>
      <span style={{ fontSize: '10px' }}>HP: {spark.hp}</span>
      <span style={{ fontSize: '10px', opacity: 0.7 }}>@{spark.at_node_id || spark.atNodeId}</span>
      {spark.carrying?.length > 0 && (
        <span style={{ fontSize: '9px', color: '#4a9eff' }}>📦 {spark.carrying.length}</span>
      )}
    </div>
  )
}

// Action Panel
function ActionPanel() {
  const { runId, selectedSparkId, board, editBuffer, addEdit, clearEdits, addLog } = useGameStore()
  const [editPath, setEditPath] = useState('')
  const [editContents, setEditContents] = useState('')
  const [patchMessage, setPatchMessage] = useState('')
  const [probeId, setProbeId] = useState('')
  const [relayId, setRelayId] = useState('')
  const [patchId, setPatchId] = useState('')

  const selectedSpark = board.sparks.get(selectedSparkId || '')
  const isAC = selectedSpark?.kind === 'ac'

  const handleEditFile = async () => {
    if (!runId || !selectedSparkId || !editPath) return
    try {
      await acEditFiles({ runId, sparkId: selectedSparkId, edits: [{ path: editPath, contents: editContents }] })
      addEdit(editPath, editContents)
      addLog(`Edited: ${editPath}`)
      setEditPath('')
      setEditContents('')
    } catch (e: any) {
      addLog(`Error: ${e.message || e}`)
    }
  }

  const handleCreatePatch = async () => {
    if (!runId || !selectedSparkId || !patchMessage) return
    try {
      const res = await acCreatePatch({ runId, sparkId: selectedSparkId, message: patchMessage })
      addLog(`Patch created: ${res.patchId} (${res.touchedPaths.length} files)`)
      clearEdits()
      setPatchMessage('')
      setPatchId(res.patchId)
    } catch (e: any) {
      addLog(`Error: ${e.message || e}`)
    }
  }

  const handleApplyPatch = async () => {
    if (!runId || !selectedSparkId || !patchId) return
    try {
      await dcApplyPatch({ runId, sparkId: selectedSparkId, patchId })
      addLog(`Patch applied: ${patchId}`)
    } catch (e: any) {
      addLog(`Error: ${e.message || e}`)
    }
  }

  const handleRunProbe = async () => {
    if (!runId || !selectedSparkId || !probeId) return
    try {
      const res = await dcRunProbe({ runId, sparkId: selectedSparkId, probeId })
      addLog(`Probe ${probeId}: ${res.status} (${res.evidenceItemIds.length} evidence)`)
    } catch (e: any) {
      addLog(`Error: ${e.message || e}`)
    }
  }

  const handleCloseRelay = async () => {
    if (!runId || !selectedSparkId || !relayId) return
    try {
      await dcCloseRelay({ runId, sparkId: selectedSparkId, relayId })
      addLog(`Relay closed: ${relayId}`)
    } catch (e: any) {
      addLog(`Error: ${e.message || e}`)
    }
  }

  if (!selectedSparkId) {
    return <div style={{ padding: '16px', opacity: 0.5 }}>Select a spark to see actions</div>
  }

  return (
    <div style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
      <h3>{isAC ? '⚡ AC Actions' : '🛡️ DC Actions'}</h3>

      {isAC && (
        <>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label>Edit File</label>
            <input
              placeholder="Path (e.g., src/main.py)"
              value={editPath}
              onChange={(e) => setEditPath(e.target.value)}
              style={{ padding: '8px', background: '#2a2a4e', border: '1px solid #4a9eff', borderRadius: '4px', color: '#eee' }}
            />
            <textarea
              placeholder="Contents"
              value={editContents}
              onChange={(e) => setEditContents(e.target.value)}
              rows={4}
              style={{ padding: '8px', background: '#2a2a4e', border: '1px solid #4a9eff', borderRadius: '4px', color: '#eee', fontFamily: 'monospace' }}
            />
            <button onClick={handleEditFile} style={{ padding: '8px', background: '#4a9eff', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>
              Save Edit
            </button>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label>Create Patch ({editBuffer.length} staged)</label>
            <input
              placeholder="Commit message"
              value={patchMessage}
              onChange={(e) => setPatchMessage(e.target.value)}
              style={{ padding: '8px', background: '#2a2a4e', border: '1px solid #ffd43b', borderRadius: '4px', color: '#eee' }}
            />
            <button onClick={handleCreatePatch} disabled={editBuffer.length === 0} style={{ padding: '8px', background: '#ffd43b', color: '#1a1a2e', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>
              Create Patch
            </button>
          </div>
        </>
      )}

      {!isAC && (
        <>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label>Apply Patch</label>
            <input
              placeholder="Patch ID"
              value={patchId}
              onChange={(e) => setPatchId(e.target.value)}
              style={{ padding: '8px', background: '#2a2a4e', border: '1px solid #51cf66', borderRadius: '4px', color: '#eee' }}
            />
            <button onClick={handleApplyPatch} style={{ padding: '8px', background: '#51cf66', color: '#1a1a2e', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>
              Apply Patch
            </button>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label>Run Probe</label>
            <input
              placeholder="Probe node ID"
              value={probeId}
              onChange={(e) => setProbeId(e.target.value)}
              style={{ padding: '8px', background: '#2a2a4e', border: '1px solid #4a9eff', borderRadius: '4px', color: '#eee' }}
            />
            <button onClick={handleRunProbe} style={{ padding: '8px', background: '#4a9eff', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>
              Run Probe
            </button>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <label>Close Relay</label>
            <input
              placeholder="Relay node ID"
              value={relayId}
              onChange={(e) => setRelayId(e.target.value)}
              style={{ padding: '8px', background: '#2a2a4e', border: '1px solid #51cf66', borderRadius: '4px', color: '#eee' }}
            />
            <button onClick={handleCloseRelay} style={{ padding: '8px', background: '#51cf66', color: '#1a1a2e', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>
              Close Relay
            </button>
          </div>
        </>
      )}
    </div>
  )
}

// Log Panel
function LogPanel() {
  const logs = useGameStore((s) => s.logs)
  return (
    <div style={{
      padding: '8px',
      background: '#0a0a1e',
      borderRadius: '4px',
      height: '150px',
      overflow: 'auto',
      fontFamily: 'monospace',
      fontSize: '11px',
    }}>
      {logs.map((log, i) => (
        <div key={i} style={{ opacity: 0.8 }}>{log}</div>
      ))}
    </div>
  )
}

// Main App
export default function App() {
  const { board, runId, selectedSparkId, setBoard, setRunId, selectSpark, addLog } = useGameStore()
  const [envSpecPath, setEnvSpecPath] = useState('')
  const [loading, setLoading] = useState(false)

  // Web viewer mode: attempt to load a static snapshot if not in Tauri
  useEffect(() => {
    if (!isTauri) {
      fetch('snapshot.json')
        .then(res => {
          if (!res.ok) throw new Error("snapshot.json not found")
          return res.json()
        })
        .then(snap => {
          setBoard(applySnapshot(newBoardState(), snap))
          setRunId(snap.runId)
          addLog("Loaded static board snapshot for web viewer")
        })
        .catch(err => {
          console.warn("Web viewer active - no snapshot.json found", err)
          addLog("Web viewer active - standing by for data")
        })
    }
  }, [setBoard, setRunId, addLog])

  // Subscribe to board events (Tauri only)
  useEffect(() => {
    if (!isTauri) return

    const handleSnapshot = (env: { payload: BoardSnapshot }) => {
      const snap = env.payload
      setBoard(applySnapshot(board, snap))
      setRunId(snap.runId)
      addLog(`Snapshot received: ${snap.nodes.length} nodes, ${snap.sparks.length} sparks`)
    }

    const handleDiff = (env: { payload: BoardDiff }) => {
      setBoard(applyDiff(board, env.payload))
      addLog('Board updated (diff)')
    }

    const unsubPromise = subscribeMany([
      onBoardSnapshot(handleSnapshot as any),
      onBoardDiff(handleDiff as any),
    ])

    return () => {
      unsubPromise.then((unsub) => unsub())
    }
  }, [board, setBoard, setRunId, addLog])

  const handleCreateRun = useCallback(async () => {
    if (!envSpecPath) {
      addLog('Error: Enter environment spec path')
      return
    }
    setLoading(true)
    try {
      const res = await runCreate({ envSpecPath, seed: Date.now() })
      addLog(`Run created: ${res.runId}`)
      addLog(`Workspace: ${res.workspacePath}`)
    } catch (e: any) {
      addLog(`Error: ${e.message || e}`)
    }
    setLoading(false)
  }, [envSpecPath, addLog])

  const nodes = Array.from(board.nodes.values())
  const sparks = Array.from(board.sparks.values())

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: '16px', gap: '16px' }}>
      {/* Header */}
      <header style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
        <h1 style={{ margin: 0, fontSize: '24px' }}>⚡ CircuitForge</h1>
        {runId && <span style={{ opacity: 0.7 }}>Run: {runId.slice(0, 8)}...</span>}
      </header>

      {/* Create Run Panel */}
      {!runId && (
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <input
            placeholder="Environment spec path (e.g., /path/to/EnvironmentSpec.yaml)"
            value={envSpecPath}
            onChange={(e) => setEnvSpecPath(e.target.value)}
            style={{
              flex: 1,
              padding: '12px',
              background: '#2a2a4e',
              border: '1px solid #4a9eff',
              borderRadius: '4px',
              color: '#eee',
              fontSize: '14px',
            }}
          />
          <button
            onClick={handleCreateRun}
            disabled={loading}
            style={{
              padding: '12px 24px',
              background: '#4a9eff',
              border: 'none',
              borderRadius: '4px',
              cursor: loading ? 'wait' : 'pointer',
              fontWeight: 'bold',
            }}
          >
            {loading ? 'Creating...' : 'Create Run'}
          </button>
        </div>
      )}

      {/* Main Content */}
      <div style={{ display: 'flex', flex: 1, gap: '16px', overflow: 'hidden' }}>
        {/* Board View */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {/* Nodes */}
          <div style={{ background: '#2a2a4e', borderRadius: '8px', padding: '16px' }}>
            <h3 style={{ marginBottom: '8px' }}>Board Nodes</h3>
            <div style={{ display: 'flex', flexWrap: 'wrap' }}>
              {nodes.length === 0 && <span style={{ opacity: 0.5 }}>No nodes - create a run first</span>}
              {nodes.map((n) => <NodeView key={n.id} node={n} />)}
            </div>
          </div>

          {/* Sparks */}
          <div style={{ background: '#2a2a4e', borderRadius: '8px', padding: '16px' }}>
            <h3 style={{ marginBottom: '8px' }}>Sparks (Players)</h3>
            <div style={{ display: 'flex', flexWrap: 'wrap' }}>
              {sparks.length === 0 && <span style={{ opacity: 0.5 }}>No sparks - create a run first</span>}
              {sparks.map((s) => (
                <SparkView
                  key={s.id}
                  spark={s}
                  selected={s.id === selectedSparkId}
                  onClick={() => selectSpark(s.id)}
                />
              ))}
            </div>
          </div>

          {/* HITL Report Queue */}
          {runId && (
            <ReportQueue
              runId={runId}
              nodes={board.nodes}
              onSignoffComplete={(probeId) => addLog(`HITL signoff complete: ${probeId}`)}
            />
          )}

          {/* Logs */}
          <div>
            <h3 style={{ marginBottom: '8px' }}>Event Log</h3>
            <LogPanel />
          </div>
        </div>

        {/* Action Panel */}
        <div style={{ width: '300px', background: '#2a2a4e', borderRadius: '8px', overflow: 'auto' }}>
          <ActionPanel />
        </div>
      </div>
    </div>
  )
}
