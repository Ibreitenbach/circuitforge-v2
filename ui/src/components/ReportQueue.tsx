import { useState } from 'react'
import { HitlSignoffModal } from './HitlSignoffModal'

export interface HitlItem {
  probeId: string
  title: string
  kind: string
  status: 'pending' | 'pass' | 'fail'
  duePolicy: string
  reviewScopeGlobs: string[]
  constructRoot: string
  cycleDay: string
}

interface ReportQueueProps {
  runId: string | null
  nodes: Map<string, any>
  onSignoffComplete: (probeId: string) => void
}

/**
 * Report Queue Panel
 *
 * Displays outstanding HITL items derived from the board state.
 * Items are identified by moduleId === 'hitl' or probe ID containing 'hitl'.
 */
export function ReportQueue({ runId, nodes, onSignoffComplete }: ReportQueueProps) {
  const [selectedItem, setSelectedItem] = useState<HitlItem | null>(null)
  const [isModalOpen, setIsModalOpen] = useState(false)

  // Derive HITL items from nodes
  const hitlItems: HitlItem[] = []

  nodes.forEach((node) => {
    if (node.kind === 'probe') {
      // Check if this is an HITL probe
      const isHitl =
        node.moduleId === 'hitl' ||
        node.id?.toLowerCase().includes('hitl') ||
        node.label?.toLowerCase().includes('hitl')

      if (isHitl) {
        const status = node.status?.probe || 'pending'
        hitlItems.push({
          probeId: node.id,
          title: node.reportQueue?.title || node.label || node.id,
          kind: node.reportQueue?.kind || 'weekly_audit',
          status: status === 'pass' ? 'pass' : status === 'fail' ? 'fail' : 'pending',
          duePolicy: node.reportQueue?.duePolicy || 'weekly',
          reviewScopeGlobs: node.reportQueue?.reviewScopeGlobs || [],
          constructRoot: node.constructRoot || '',
          cycleDay: node.cycleDay || '',
        })
      }
    }
  })

  // Filter to only show pending/required items
  const pendingItems = hitlItems.filter((item) => item.status !== 'pass')

  const handleReview = (item: HitlItem) => {
    setSelectedItem(item)
    setIsModalOpen(true)
  }

  const handleModalClose = () => {
    setIsModalOpen(false)
    setSelectedItem(null)
  }

  const handleSignoffComplete = (probeId: string) => {
    setIsModalOpen(false)
    setSelectedItem(null)
    onSignoffComplete(probeId)
  }

  return (
    <div
      style={{
        background: '#1a1a2e',
        borderRadius: '8px',
        padding: '16px',
        marginBottom: '16px',
      }}
    >
      <h3 style={{ margin: '0 0 12px 0', display: 'flex', alignItems: 'center', gap: '8px' }}>
        📋 HITL Report Queue
        {pendingItems.length > 0 && (
          <span
            style={{
              background: '#ff6b6b',
              color: 'white',
              padding: '2px 8px',
              borderRadius: '12px',
              fontSize: '12px',
            }}
          >
            {pendingItems.length}
          </span>
        )}
      </h3>

      {pendingItems.length === 0 ? (
        <div style={{ opacity: 0.6, fontStyle: 'italic' }}>
          No pending HITL items. All audits complete.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {pendingItems.map((item) => (
            <div
              key={item.probeId}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                background: '#2a2a4e',
                padding: '12px',
                borderRadius: '6px',
                border: item.status === 'fail' ? '1px solid #ff6b6b' : '1px solid #4a9eff33',
              }}
            >
              <div>
                <div style={{ fontWeight: 'bold' }}>{item.title}</div>
                <div style={{ fontSize: '12px', opacity: 0.7 }}>
                  {item.kind} • {item.duePolicy}
                </div>
              </div>
              <button
                onClick={() => handleReview(item)}
                style={{
                  padding: '8px 16px',
                  background: '#4a9eff',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontWeight: 'bold',
                }}
              >
                Review
              </button>
            </div>
          ))}
        </div>
      )}

      {/* All items (including passed) for reference */}
      {hitlItems.length > pendingItems.length && (
        <details style={{ marginTop: '12px' }}>
          <summary style={{ cursor: 'pointer', opacity: 0.7 }}>
            Show all HITL items ({hitlItems.length})
          </summary>
          <div style={{ marginTop: '8px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
            {hitlItems.map((item) => (
              <div
                key={item.probeId}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  padding: '4px 8px',
                  fontSize: '12px',
                }}
              >
                <span>
                  {item.status === 'pass' ? '✅' : item.status === 'fail' ? '❌' : '⏳'}
                </span>
                <span>{item.title}</span>
              </div>
            ))}
          </div>
        </details>
      )}

      {/* Signoff Modal */}
      {isModalOpen && selectedItem && (
        <HitlSignoffModal
          runId={runId}
          item={selectedItem}
          onClose={handleModalClose}
          onSignoffComplete={handleSignoffComplete}
        />
      )}
    </div>
  )
}
