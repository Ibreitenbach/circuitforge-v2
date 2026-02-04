import { invoke } from '@tauri-apps/api/core'

export type ApiError = { code: string; message: string; detail?: any }

// Check if running in Tauri
export const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__ !== undefined

// Demo data for browser mode
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

async function call<T>(cmd: string, args: Record<string, any>): Promise<T> {
  if (!isTauri) {
    console.log(`[Demo Mode] Command: ${cmd}`, args)
    await new Promise(r => setTimeout(r, 200)) // Simulate latency

    // Demo mode returns mock data
    switch (cmd) {
      case 'run_create':
        return { runId: 'demo-run-' + Date.now(), workspacePath: '/demo/workspace', board: { nodes: demoNodes, sparks: demoSparks } } as T
      case 'run_load':
        return { runId: args?.req?.runId || 'demo-run', board: { nodes: demoNodes, sparks: demoSparks } } as T
      case 'dc_run_probe':
        return { probeId: args?.req?.probeId || 'unknown', status: 'pass', evidenceItemIds: ['ev_001'], artifactRefs: [] } as T
      case 'hitl_signoff_write':
        return { writtenPath: '/demo/hitl_signoff.json', capsuleHash: 'demo_' + Date.now().toString(16) } as T
      case 'run_list':
      case 'construct_list':
        return [] as T
      default:
        console.warn(`[Demo Mode] Unhandled: ${cmd}`)
        return {} as T
    }
  }
  return await invoke<T>(cmd, args)
}

export async function callSafe<T>(cmd: string, args: Record<string, any>)
  : Promise<{ ok: true; data: T } | { ok: false; error: ApiError }> {
  try {
    const data = await call<T>(cmd, args)
    return { ok: true, data }
  } catch (e: any) {
    const err: ApiError = (e && typeof e === 'object' && 'code' in e)
      ? (e as ApiError)
      : { code: 'UNKNOWN', message: String(e), detail: e }
    return { ok: false, error: err }
  }
}

export type RunCreateReq = { envSpecPath: string; seed?: number }
export type RunCreateRes = { runId: string; workspacePath: string; board: any }

export type RunLoadReq = { runId: string }
export type RunLoadRes = { runId: string; board: any }

export type RunResetReq = { runId: string }

export type FileEdit = { path: string; contents: string }

export type EditFilesReq = { runId: string; sparkId: string; edits: FileEdit[] }

export type CreatePatchReq = { runId: string; sparkId: string; message: string; intent?: string; risk?: 'low'|'med'|'high' }
export type CreatePatchRes = { patchId: string; itemId: string; touchedPaths: string[] }

export type ApplyPatchReq = { runId: string; sparkId: string; patchId: string }

export type RunProbeReq = { runId: string; sparkId: string; probeId: string }
export type RunProbeRes = { probeId: string; status: 'pass'|'fail'; evidenceItemIds: string[]; artifactRefs: string[]; failureSignature?: string }

export type CloseRelayReq = { runId: string; sparkId: string; relayId: string }

export type EditConfigReq = { runId: string; sparkId: string; reason: string; edits: FileEdit[] }
export type EditConfigRes = { editedPaths: string[] }

export const runCreate = (req: RunCreateReq) => call<RunCreateRes>('run_create', { req })
export const runLoad = (req: RunLoadReq) => call<RunLoadRes>('run_load', { req })
export const runResetToCheckpoint = (req: RunResetReq) => call<void>('run_reset_to_checkpoint', { req })

export const acEditFiles = (req: EditFilesReq) => call<void>('ac_edit_files', { req })
export const acCreatePatch = (req: CreatePatchReq) => call<CreatePatchRes>('ac_create_patch', { req })

export const dcApplyPatch = (req: ApplyPatchReq) => call<void>('dc_apply_patch', { req })
export const dcRunProbe = (req: RunProbeReq) => call<RunProbeRes>('dc_run_probe', { req })
export const dcCloseRelay = (req: CloseRelayReq) => call<void>('dc_close_relay', { req })
export const dcEditConfig = (req: EditConfigReq) => call<EditConfigRes>('dc_edit_config', { req })

// ---------- Constructs ----------
export type ConstructKind = 'cli' | 'service' | 'library' | 'schema' | 'pipeline'
export type ExportType = 'command' | 'http_endpoint' | 'library_api' | 'file_artifact'

export interface InterfaceExport {
  id: string
  type: ExportType
  signature: string
  notes?: string
}

export interface Invariant {
  id: string
  statement: string
  severity: 'must' | 'should'
  probeRef: string
}

export interface ConstructSpec {
  id: string
  kind: ConstructKind
  name: string
  description: string
  root: string
  interface: { exports: InterfaceExport[] }
  invariants: Invariant[]
  acceptance: { requiredProbes: string[]; passPolicy: { mode: 'all' } }
  dependencies?: any
  metadata?: any
}

export type ConstructStatus = 'draft' | 'active' | 'certified' | 'regressed' | 'archived'

export interface EvaluateResult {
  status: ConstructStatus
  missingProbes: string[]
  failingProbes: string[]
  satisfiedInvariants: string[]
}

export type ConstructDefineReq = { runId: string; spec: ConstructSpec }
export type ConstructDefineRes = { constructId: string }

export type ConstructUpdateSpecReq = { constructId: string; spec: ConstructSpec; runId: string }

export type ConstructGetReq = { constructId: string }
export type ConstructGetRes = { spec: ConstructSpec }

export type ConstructEvaluateReq = { runId: string; constructId: string }

export type ConstructListReq = { runId: string }

export const constructDefine = (req: ConstructDefineReq) => call<ConstructDefineRes>('construct_define', { req })
export const constructUpdateSpec = (req: ConstructUpdateSpecReq) => call<number>('construct_update_spec', { req })
export const constructGet = (req: ConstructGetReq) => call<ConstructGetRes>('construct_get', { req })
export const constructEvaluate = (req: ConstructEvaluateReq) => call<EvaluateResult>('construct_evaluate', { req })
export const constructList = (req: ConstructListReq) => call<string[]>('construct_list', { req })

// ---------- HITL Signoff ----------
export interface HitlChecks {
  reality_contact: boolean
  distance_reduction: boolean
  anti_goodhart: boolean
}

export type HitlSignoffWriteReq = {
  runId: string
  constructRoot: string
  cycleDay: string
  decision: 'approved' | 'rejected'
  checks: HitlChecks
  notes?: string
  artifactsReviewed: string[]
}

export type HitlSignoffWriteRes = {
  writtenPath: string
  capsuleHash: string
}

export const hitlSignoffWrite = (req: HitlSignoffWriteReq) =>
  call<HitlSignoffWriteRes>('hitl_signoff_write', { req })
