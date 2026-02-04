// Mock Tauri API for browser/GitHub Pages demo mode
// When running outside Tauri, provides demo data instead of real IPC

const isTauri = typeof window !== 'undefined' && '__TAURI__' in window

// Demo board state
const demoNodes = [
  { id: 'probe_build', kind: 'probe', label: 'Build Check', status: { probe: 'pass' } },
  { id: 'probe_test', kind: 'probe', label: 'Test Suite', status: { probe: 'idle' } },
  { id: 'probe_hitl', kind: 'probe', label: 'HITL Audit', moduleId: 'hitl', status: { probe: 'pending' },
    reportQueue: { title: 'Weekly HITL Audit', kind: 'weekly_audit', duePolicy: 'weekly', reviewScopeGlobs: ['days/*/consilience.md'] } },
  { id: 'relay_deploy', kind: 'relay', label: 'Deploy Gate', status: { gate: 'open' } },
  { id: 'fuse_main', kind: 'fuse', label: 'Main Fuse', status: { gate: 'closed' } },
]

const demoSparks = [
  { id: 'spark_claude_ac', kind: 'ac', hp: 100, atNodeId: 'probe_build', carrying: [] },
  { id: 'spark_gemini_dc', kind: 'dc', hp: 100, atNodeId: 'probe_test', carrying: [] },
]

export async function invoke<T>(cmd: string, args?: Record<string, any>): Promise<T> {
  if (isTauri) {
    // Real Tauri environment - use actual API
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core')
    return tauriInvoke<T>(cmd, args)
  }

  // Browser demo mode - return mock data
  console.log(`[Demo Mode] Command: ${cmd}`, args)

  await new Promise(r => setTimeout(r, 300)) // Simulate latency

  switch (cmd) {
    case 'run_create':
      return {
        runId: 'demo-run-' + Date.now(),
        workspacePath: '/demo/workspace',
        board: { nodes: demoNodes, sparks: demoSparks }
      } as T

    case 'run_load':
      return {
        runId: args?.req?.runId || 'demo-run',
        board: { nodes: demoNodes, sparks: demoSparks }
      } as T

    case 'dc_run_probe':
      return {
        probeId: args?.req?.probeId || 'unknown',
        status: 'pass',
        evidenceItemIds: ['ev_001', 'ev_002'],
        artifactRefs: []
      } as T

    case 'hitl_signoff_write':
      return {
        writtenPath: '/demo/hitl_signoff.json',
        capsuleHash: 'demo_hash_' + Date.now().toString(16)
      } as T

    default:
      console.warn(`[Demo Mode] Unhandled command: ${cmd}`)
      return {} as T
  }
}

export const isBrowserDemo = !isTauri
