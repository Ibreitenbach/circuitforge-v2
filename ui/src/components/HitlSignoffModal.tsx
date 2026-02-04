import { useState } from 'react'
import { hitlSignoffWrite, dcRunProbe } from '../ipc/commands'
import type { HitlItem } from './ReportQueue'

interface HitlSignoffModalProps {
  runId: string | null
  item: HitlItem
  onClose: () => void
  onSignoffComplete: (probeId: string) => void
}

interface SignoffChecks {
  realityContact: boolean
  distanceReduction: boolean
  antiGoodhart: boolean
}

/**
 * HITL Signoff Modal
 *
 * Provides a structured review interface with:
 * - 3 check controls (Reality contact, Distance reduction, Anti-Goodhart)
 * - Optional notes field
 * - Typed interlock: user must type "SIGNOFF" to confirm
 * - One-button approve/reject
 *
 * On approval:
 * 1. Writes HITL evidence file
 * 2. Runs the HITL probe automatically
 * 3. Updates board state
 */
export function HitlSignoffModal({
  runId,
  item,
  onClose,
  onSignoffComplete,
}: HitlSignoffModalProps) {
  const [checks, setChecks] = useState<SignoffChecks>({
    realityContact: false,
    distanceReduction: false,
    antiGoodhart: false,
  })
  const [notes, setNotes] = useState('')
  const [interlock, setInterlock] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [probeResult, setProbeResult] = useState<{ status: string; message: string } | null>(null)

  const allChecked = checks.realityContact && checks.distanceReduction && checks.antiGoodhart
  const interlockValid = interlock === 'SIGNOFF'
  const canApprove = allChecked && interlockValid && runId

  const handleCheckChange = (check: keyof SignoffChecks) => {
    setChecks((prev) => ({ ...prev, [check]: !prev[check] }))
  }

  const handleApprove = async () => {
    if (!canApprove) return

    setIsSubmitting(true)
    setError(null)

    try {
      // Step 1: Write HITL evidence
      await hitlSignoffWrite({
        runId: runId!,
        constructRoot: item.constructRoot,
        cycleDay: item.cycleDay,
        decision: 'approved',
        checks: {
          reality_contact: checks.realityContact,
          distance_reduction: checks.distanceReduction,
          anti_goodhart: checks.antiGoodhart,
        },
        notes: notes || undefined,
        artifactsReviewed: item.reviewScopeGlobs,
      })

      // Step 2: Run the HITL probe
      const probeResult = await dcRunProbe({
        runId: runId!,
        sparkId: 'spark_gemini_dc', // DC spark runs probes
        probeId: item.probeId,
      })

      setProbeResult({
        status: probeResult.status,
        message: `Probe ${probeResult.status}: ${probeResult.evidenceItemIds.length} evidence items`,
      })

      if (probeResult.status === 'pass') {
        // Success - close modal after short delay
        setTimeout(() => {
          onSignoffComplete(item.probeId)
        }, 1500)
      }
    } catch (e: any) {
      setError(e.message || String(e))
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleReject = async () => {
    if (!runId) return

    setIsSubmitting(true)
    setError(null)

    try {
      await hitlSignoffWrite({
        runId: runId!,
        constructRoot: item.constructRoot,
        cycleDay: item.cycleDay,
        decision: 'rejected',
        checks: {
          reality_contact: checks.realityContact,
          distance_reduction: checks.distanceReduction,
          anti_goodhart: checks.antiGoodhart,
        },
        notes: notes || 'Rejected by reviewer',
        artifactsReviewed: item.reviewScopeGlobs,
      })

      onClose()
    } catch (e: any) {
      setError(e.message || String(e))
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0, 0, 0, 0.8)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={onClose}
    >
      <div
        style={{
          background: '#1a1a2e',
          borderRadius: '12px',
          padding: '24px',
          width: '500px',
          maxHeight: '80vh',
          overflow: 'auto',
          border: '2px solid #4a9eff',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{ marginBottom: '20px' }}>
          <h2 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: '8px' }}>
            📋 HITL Review
          </h2>
          <div style={{ opacity: 0.7, marginTop: '4px' }}>{item.title}</div>
        </div>

        {/* Review Scope */}
        <div style={{ marginBottom: '20px' }}>
          <h4 style={{ margin: '0 0 8px 0' }}>Review Scope</h4>
          <div
            style={{
              background: '#2a2a4e',
              padding: '12px',
              borderRadius: '6px',
              fontFamily: 'monospace',
              fontSize: '12px',
            }}
          >
            {item.reviewScopeGlobs.length > 0 ? (
              item.reviewScopeGlobs.map((glob, i) => <div key={i}>{glob}</div>)
            ) : (
              <div style={{ opacity: 0.5 }}>No specific scope defined</div>
            )}
          </div>
        </div>

        {/* Checklist */}
        <div style={{ marginBottom: '20px' }}>
          <h4 style={{ margin: '0 0 12px 0' }}>Review Checklist</h4>

          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '12px',
              padding: '12px',
              background: checks.realityContact ? '#4a9eff22' : '#2a2a4e',
              borderRadius: '6px',
              marginBottom: '8px',
              cursor: 'pointer',
            }}
          >
            <input
              type="checkbox"
              checked={checks.realityContact}
              onChange={() => handleCheckChange('realityContact')}
              style={{ width: '20px', height: '20px' }}
            />
            <div>
              <div style={{ fontWeight: 'bold' }}>Reality Contact</div>
              <div style={{ fontSize: '12px', opacity: 0.7 }}>
                Claims are grounded in evidence, not rhetoric
              </div>
            </div>
          </label>

          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '12px',
              padding: '12px',
              background: checks.distanceReduction ? '#4a9eff22' : '#2a2a4e',
              borderRadius: '6px',
              marginBottom: '8px',
              cursor: 'pointer',
            }}
          >
            <input
              type="checkbox"
              checked={checks.distanceReduction}
              onChange={() => handleCheckChange('distanceReduction')}
              style={{ width: '20px', height: '20px' }}
            />
            <div>
              <div style={{ fontWeight: 'bold' }}>Distance Reduction</div>
              <div style={{ fontSize: '12px', opacity: 0.7 }}>
                Gap between current and target state is measurably reduced
              </div>
            </div>
          </label>

          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '12px',
              padding: '12px',
              background: checks.antiGoodhart ? '#4a9eff22' : '#2a2a4e',
              borderRadius: '6px',
              cursor: 'pointer',
            }}
          >
            <input
              type="checkbox"
              checked={checks.antiGoodhart}
              onChange={() => handleCheckChange('antiGoodhart')}
              style={{ width: '20px', height: '20px' }}
            />
            <div>
              <div style={{ fontWeight: 'bold' }}>Anti-Goodhart</div>
              <div style={{ fontSize: '12px', opacity: 0.7 }}>
                Improvements are real, not metric gaming
              </div>
            </div>
          </label>
        </div>

        {/* Notes */}
        <div style={{ marginBottom: '20px' }}>
          <h4 style={{ margin: '0 0 8px 0' }}>Notes (optional)</h4>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder="Add any observations or concerns..."
            rows={3}
            style={{
              width: '100%',
              padding: '12px',
              background: '#2a2a4e',
              border: '1px solid #4a9eff33',
              borderRadius: '6px',
              color: '#eee',
              fontFamily: 'inherit',
              resize: 'vertical',
            }}
          />
        </div>

        {/* Typed Interlock */}
        <div style={{ marginBottom: '20px' }}>
          <h4 style={{ margin: '0 0 8px 0' }}>
            Type <code style={{ background: '#2a2a4e', padding: '2px 6px' }}>SIGNOFF</code> to
            confirm
          </h4>
          <input
            type="text"
            value={interlock}
            onChange={(e) => setInterlock(e.target.value.toUpperCase())}
            placeholder="Type SIGNOFF"
            style={{
              width: '100%',
              padding: '12px',
              background: '#2a2a4e',
              border: interlockValid ? '2px solid #51cf66' : '1px solid #4a9eff33',
              borderRadius: '6px',
              color: '#eee',
              fontFamily: 'monospace',
              fontSize: '16px',
              textAlign: 'center',
            }}
          />
        </div>

        {/* Error Display */}
        {error && (
          <div
            style={{
              background: '#ff6b6b22',
              border: '1px solid #ff6b6b',
              padding: '12px',
              borderRadius: '6px',
              marginBottom: '20px',
              color: '#ff6b6b',
            }}
          >
            {error}
          </div>
        )}

        {/* Probe Result Display */}
        {probeResult && (
          <div
            style={{
              background: probeResult.status === 'pass' ? '#51cf6622' : '#ff6b6b22',
              border: `1px solid ${probeResult.status === 'pass' ? '#51cf66' : '#ff6b6b'}`,
              padding: '12px',
              borderRadius: '6px',
              marginBottom: '20px',
              color: probeResult.status === 'pass' ? '#51cf66' : '#ff6b6b',
            }}
          >
            {probeResult.status === 'pass' ? '✅' : '❌'} {probeResult.message}
          </div>
        )}

        {/* Action Buttons */}
        <div style={{ display: 'flex', gap: '12px' }}>
          <button
            onClick={handleReject}
            disabled={isSubmitting}
            style={{
              flex: 1,
              padding: '12px',
              background: '#ff6b6b',
              border: 'none',
              borderRadius: '6px',
              cursor: isSubmitting ? 'wait' : 'pointer',
              fontWeight: 'bold',
              color: 'white',
            }}
          >
            Reject
          </button>
          <button
            onClick={handleApprove}
            disabled={!canApprove || isSubmitting}
            style={{
              flex: 1,
              padding: '12px',
              background: canApprove ? '#51cf66' : '#51cf6666',
              border: 'none',
              borderRadius: '6px',
              cursor: canApprove && !isSubmitting ? 'pointer' : 'not-allowed',
              fontWeight: 'bold',
              color: 'white',
            }}
          >
            {isSubmitting ? 'Processing...' : 'Approve'}
          </button>
        </div>

        {/* Cancel link */}
        <div style={{ textAlign: 'center', marginTop: '16px' }}>
          <button
            onClick={onClose}
            style={{
              background: 'none',
              border: 'none',
              color: '#4a9eff',
              cursor: 'pointer',
              textDecoration: 'underline',
            }}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  )
}
