import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ChevronDown, AlertCircle, CheckCircle } from 'lucide-react'

interface ProvenanceEntry {
  id: string
  title: string
  object_type: string
  status: string
  created_by: string
  created_at: string
  extraction_span?: string
}

interface ProvenanceChainData {
  root_id: string
  root_title: string
  chain: ProvenanceEntry[]
}

interface ProvChainProps {
  blockId: string
}

const statusConfig = {
  UserAuthored: { bg: 'bg-blue-50', border: 'border-blue-200', text: 'text-blue-900', icon: '✍️', label: 'User-Created' },
  RawSource: { bg: 'bg-gray-50', border: 'border-gray-200', text: 'text-gray-900', icon: '📄', label: 'Raw Source' },
  AiSummary: { bg: 'bg-amber-50', border: 'border-amber-200', text: 'text-amber-900', icon: '🤖', label: 'AI Summary' },
  UnreviewedSuggestion: { bg: 'bg-yellow-50', border: 'border-yellow-200', text: 'text-yellow-900', icon: '⚠️', label: 'Unreviewed' },
  Reviewed: { bg: 'bg-green-50', border: 'border-green-200', text: 'text-green-900', icon: '✓', label: 'Reviewed' },
  ExtractedMetadata: { bg: 'bg-purple-50', border: 'border-purple-200', text: 'text-purple-900', icon: '🏷️', label: 'Extracted' },
  InferredLink: { bg: 'bg-emerald-50', border: 'border-emerald-200', text: 'text-emerald-900', icon: '🔗', label: 'Inferred' },
}

export default function ProvChain({ blockId }: ProvChainProps) {
  const [chain, setChain] = useState<ProvenanceChainData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [selectedEntry, setSelectedEntry] = useState<ProvenanceEntry | null>(null)

  useEffect(() => {
    const loadChain = async () => {
      try {
        setLoading(true)
        setError(null)
        const data = await invoke<ProvenanceChainData>('get_provenance_chain', {
          blockId,
        })
        setChain(data)
        if (data.chain.length > 0) {
          setSelectedEntry(data.chain[0])
        }
      } catch (err) {
        setError(String(err))
      } finally {
        setLoading(false)
      }
    }

    if (blockId.trim()) {
      loadChain()
    }
  }, [blockId])

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center bg-gray-50" role="status" aria-live="polite" aria-label="Loading provenance chain">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading provenance chain...</p>
          <p className="text-xs text-gray-500 mt-2">This may take a moment...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center bg-gray-50 p-4" role="alert">
        <div className="bg-red-50 border-2 border-red-200 rounded-lg p-4 max-w-md shadow-sm">
          <div className="flex gap-3">
            <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" aria-hidden="true" />
            <div>
              <p className="font-semibold text-red-900">Failed to Load Provenance Chain</p>
              <p className="text-red-800 text-sm mt-2">{error}</p>
              <p className="text-red-700 text-xs mt-3">Try entering a different block ID or contact support.</p>
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (!chain || chain.chain.length === 0) {
    return (
      <div className="h-full flex items-center justify-center bg-gray-50" role="status">
        <div className="text-center text-gray-600 max-w-md">
          <p className="font-medium">No provenance data available</p>
          <p className="text-sm mt-2">The block <code className="bg-gray-100 px-2 py-1 rounded text-xs">{blockId}</code> has no recorded provenance chain.</p>
        </div>
      </div>
    )
  }

  const getStatusConfig = (status: string) => {
    return statusConfig[status as keyof typeof statusConfig] || statusConfig.RawSource
  }

  const getObjectTypeLabel = (type: string) => {
    const labels = {
      block: '📦 Block',
      source: '📄 Source',
      note: '📝 Note',
      entity: '🏷️ Entity',
    }
    return labels[type as keyof typeof labels] || type
  }

  return (
    <div className="h-full flex gap-0 bg-gray-50">
      {/* Left pane: Provenance chain tree */}
      <nav className="w-80 border-r border-gray-300 bg-white overflow-y-auto flex flex-col" aria-label="Provenance chain">
        {/* Sticky header */}
        <div className="sticky top-0 bg-gradient-to-b from-white to-gray-50 px-4 py-4 border-b-2 border-gray-200 z-10">
          <h2 className="text-lg font-bold text-gray-900">Supply Chain of Truth</h2>
          <p className="text-sm text-gray-600 mt-2 leading-snug">
            Trace <strong>{chain.root_title}</strong> back through its derivation history
          </p>
        </div>

        {/* Chain entries */}
        <div className="p-4 space-y-3 flex-1 min-h-0">
          {chain.chain.map((entry, idx) => {
            const config = getStatusConfig(entry.status)
            const isSelected = selectedEntry?.id === entry.id

            return (
              <div key={entry.id} className="relative">
                {/* Connector line */}
                {idx < chain.chain.length - 1 && (
                  <div className="absolute left-6 top-12 h-3 w-0.5 bg-gradient-to-b from-gray-400 to-transparent"></div>
                )}

                {/* Entry button */}
                <button
                  onClick={() => setSelectedEntry(entry)}
                  className={`w-full text-left p-3 rounded-lg border-2 transition-all duration-150 ${
                    isSelected
                      ? `${config.bg} ${config.border} shadow-md ring-2 ring-blue-400 ring-offset-1`
                      : `bg-white border-gray-300 hover:border-gray-400 hover:shadow-sm`
                  }`}
                  aria-pressed={isSelected}
                  aria-label={`${config.label}: ${entry.title}`}
                >
                  <div className="flex gap-2 items-start">
                    <span className="text-lg mt-0.5 flex-shrink-0" aria-hidden="true">{config.icon}</span>
                    <div className="flex-1 min-w-0">
                      <p className="font-semibold text-gray-900 text-sm truncate">{entry.title}</p>
                      <p className="text-xs text-gray-500 mt-1.5">{getObjectTypeLabel(entry.object_type)}</p>
                      <p className="text-xs text-gray-600 mt-1 truncate">by {entry.created_by.split('@')[0] || 'System'}</p>
                    </div>
                  </div>

                  {/* Status badge */}
                  <div className="mt-2.5 ml-8">
                    <span className={`inline-block px-2 py-1 rounded text-xs font-semibold ${config.bg} ${config.border} border`}>
                      {config.label}
                    </span>
                  </div>
                </button>
              </div>
            )
          })}
        </div>
      </nav>

      {/* Right pane: Selected entry details and source preview */}
      {selectedEntry ? (
        <article className="flex-1 flex flex-col bg-white overflow-hidden">
          {/* Details header */}
          <header className="border-b-2 border-gray-300 p-6 bg-gradient-to-br from-white to-gray-50">
            <div className="flex items-start justify-between gap-4 mb-4">
              <div className="flex-1">
                <h2 className="text-2xl font-bold text-gray-900 break-words">{selectedEntry.title}</h2>
                <p className="text-gray-700 mt-2 text-sm">
                  <span className="font-medium">{getObjectTypeLabel(selectedEntry.object_type).replace('🧩 ', '').replace('📄 ', '').replace('📝 ', '').replace('🏷️ ', '')}</span>
                  <span className="mx-1 text-gray-400">•</span>
                  <code className="bg-gray-200 px-2 py-0.5 rounded text-xs text-gray-800">{selectedEntry.id.slice(0, 12)}...</code>
                </p>
              </div>
              <div className={`px-3 py-2 rounded-lg font-semibold text-sm flex-shrink-0 ${getStatusConfig(selectedEntry.status).bg} ${getStatusConfig(selectedEntry.status).text} border ${getStatusConfig(selectedEntry.status).border}`}>
                {getStatusConfig(selectedEntry.status).icon} {getStatusConfig(selectedEntry.status).label}
              </div>
            </div>

            {/* Metadata grid */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 mt-5">
              <div className="bg-white rounded-lg p-3 border border-gray-200 shadow-sm">
                <p className="text-xs font-semibold text-gray-600 uppercase tracking-wide mb-1">Created By</p>
                <p className="text-sm text-gray-900 font-medium break-all">{selectedEntry.created_by}</p>
              </div>
              <div className="bg-white rounded-lg p-3 border border-gray-200 shadow-sm">
                <p className="text-xs font-semibold text-gray-600 uppercase tracking-wide mb-1">Date Created</p>
                <time className="text-sm text-gray-900 font-medium">
                  {new Date(parseInt(selectedEntry.created_at) * 1000).toLocaleDateString('en-US', {
                    year: 'numeric',
                    month: 'short',
                    day: 'numeric',
                  })}
                </time>
              </div>
              {selectedEntry.extraction_span && (
                <div className="bg-blue-50 rounded-lg p-3 border border-blue-200 shadow-sm">
                  <p className="text-xs font-semibold text-blue-700 uppercase tracking-wide mb-1">Extraction Span</p>
                  <p className="text-sm text-blue-900 font-mono font-medium">{selectedEntry.extraction_span}</p>
                </div>
              )}
            </div>
          </header>

          {/* Source preview area */}
          <section className="flex-1 overflow-auto p-6 space-y-4" aria-label="Source document preview">
            <div className="bg-amber-50 border-l-4 border-amber-400 p-4 rounded-lg">
              <p className="text-sm text-amber-900">
                <strong>📋 Document Preview:</strong> This shows where the AI extracted information from. In a production implementation, the exact byte range would be highlighted in the original PDF or web document.
              </p>
            </div>

            {selectedEntry.extraction_span ? (
              <div className="bg-gray-50 border border-gray-300 rounded-lg overflow-hidden">
                <div className="px-4 py-3 bg-gray-100 border-b border-gray-300">
                  <p className="text-xs font-semibold text-gray-700 uppercase tracking-wide">
                    Extracted bytes: {selectedEntry.extraction_span}
                  </p>
                </div>
                <pre className="p-4 text-sm leading-relaxed text-gray-800 overflow-x-auto">
{`"This is a sample extracted passage from the source document. In a real implementation,
the exact byte range [${selectedEntry.extraction_span}] would be highlighted in the original
PDF or web page with a blue background color.

The highlighting would show exactly which sentences the AI system used to generate the
summary or extract the claim. This creates complete transparency about the AI's reasoning
and sources."

... [rest of document continues] ...`}
                </pre>
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center py-12 text-gray-500">
                <p className="text-lg font-medium">No source document available</p>
                <p className="text-sm mt-2">This entry has no extraction span recorded</p>
              </div>
            )}
          </section>
        </article>
      ) : (
        <div className="flex-1 flex items-center justify-center bg-gray-50" role="status">
          <div className="text-center text-gray-500">
            <p className="text-lg font-medium">Select an entry</p>
            <p className="text-sm mt-2">Click any entry in the chain to view its details and source</p>
          </div>
        </div>
      )}
    </div>
  )
}
