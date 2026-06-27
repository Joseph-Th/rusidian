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

const statusColors = {
  UserAuthored: 'bg-blue-50 border-blue-200 text-blue-900',
  RawSource: 'bg-gray-50 border-gray-200 text-gray-900',
  AiSummary: 'bg-amber-50 border-amber-200 text-amber-900',
  UnreviewedSuggestion: 'bg-yellow-50 border-yellow-200 text-yellow-900',
  Reviewed: 'bg-green-50 border-green-200 text-green-900',
  ExtractedMetadata: 'bg-purple-50 border-purple-200 text-purple-900',
  InferredLink: 'bg-emerald-50 border-emerald-200 text-emerald-900',
}

const statusIcons = {
  UserAuthored: '✍️',
  RawSource: '📄',
  AiSummary: '🤖',
  UnreviewedSuggestion: '⚠️',
  Reviewed: '✓',
  ExtractedMetadata: '🏷️',
  InferredLink: '🔗',
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
      <div className="h-full flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading provenance chain...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center bg-gray-50 p-4">
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 max-w-md">
          <div className="flex gap-3">
            <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-medium text-red-900">Failed to load provenance chain</p>
              <p className="text-red-800 text-sm mt-1">{error}</p>
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (!chain || chain.chain.length === 0) {
    return (
      <div className="h-full flex items-center justify-center bg-gray-50">
        <div className="text-center text-gray-600">
          <p>No provenance data available for this block</p>
        </div>
      </div>
    )
  }

  const getStatusColor = (status: string) => {
    return statusColors[status as keyof typeof statusColors] || statusColors.RawSource
  }

  const getStatusIcon = (status: string) => {
    return statusIcons[status as keyof typeof statusIcons] || '?'
  }

  return (
    <div className="h-full flex gap-4 bg-gray-50">
      {/* Left pane: Provenance chain tree */}
      <div className="w-80 border-r border-gray-200 bg-white overflow-y-auto">
        <div className="sticky top-0 bg-gradient-to-b from-white to-gray-50 p-4 border-b border-gray-200">
          <h3 className="text-lg font-semibold text-gray-800">Supply Chain</h3>
          <p className="text-sm text-gray-600 mt-1">
            Trace {chain.root_title} back to its original source
          </p>
        </div>

        <div className="p-4 space-y-2">
          {chain.chain.map((entry, idx) => (
            <div key={entry.id} className="relative">
              {/* Connector line */}
              {idx < chain.chain.length - 1 && (
                <div className="absolute left-6 top-12 bottom-0 w-0.5 bg-gray-300"></div>
              )}

              {/* Entry card */}
              <button
                onClick={() => setSelectedEntry(entry)}
                className={`w-full text-left p-3 rounded-lg border-2 transition-all ${
                  selectedEntry?.id === entry.id
                    ? `${getStatusColor(entry.status)} border-current`
                    : 'bg-gray-50 border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className="flex gap-2 items-start">
                  <span className="text-lg mt-0.5">{getStatusIcon(entry.status)}</span>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium text-gray-900 truncate">{entry.title}</p>
                    <p className="text-xs text-gray-500 mt-1">
                      {entry.object_type === 'block' && '📦 Block'}
                      {entry.object_type === 'source' && '📄 Source'}
                      {entry.object_type === 'note' && '📝 Note'}
                      {entry.object_type === 'entity' && '🏷️ Entity'}
                    </p>
                    <p className="text-xs text-gray-600 mt-1 truncate">
                      by {entry.created_by.split('@')[0]}
                    </p>
                  </div>
                </div>

                {/* Status badge */}
                <div className="mt-2 ml-8">
                  <span className="inline-block px-2 py-0.5 rounded text-xs font-medium bg-white bg-opacity-60 text-gray-700">
                    {entry.status}
                  </span>
                </div>
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* Right pane: Selected entry details and source preview */}
      {selectedEntry && (
        <div className="flex-1 flex flex-col bg-white overflow-hidden">
          {/* Details header */}
          <div className="border-b border-gray-200 p-6">
            <div className="flex items-start justify-between mb-4">
              <div>
                <h2 className="text-2xl font-bold text-gray-900">{selectedEntry.title}</h2>
                <p className="text-gray-600 mt-2">
                  {selectedEntry.object_type.charAt(0).toUpperCase() +
                    selectedEntry.object_type.slice(1)}{' '}
                  • ID: {selectedEntry.id.slice(0, 8)}...
                </p>
              </div>
              <span className={`px-3 py-1 rounded-lg font-medium text-sm ${getStatusColor(selectedEntry.status)}`}>
                {getStatusIcon(selectedEntry.status)} {selectedEntry.status}
              </span>
            </div>

            {/* Metadata grid */}
            <div className="grid grid-cols-3 gap-4 mt-4">
              <div className="bg-gray-50 p-3 rounded">
                <p className="text-xs font-medium text-gray-600 uppercase tracking-wide">Created By</p>
                <p className="text-sm text-gray-900 mt-1 font-medium">{selectedEntry.created_by}</p>
              </div>
              <div className="bg-gray-50 p-3 rounded">
                <p className="text-xs font-medium text-gray-600 uppercase tracking-wide">Created</p>
                <p className="text-sm text-gray-900 mt-1 font-medium">
                  {new Date(parseInt(selectedEntry.created_at) * 1000).toLocaleDateString()}
                </p>
              </div>
              {selectedEntry.extraction_span && (
                <div className="bg-blue-50 p-3 rounded">
                  <p className="text-xs font-medium text-blue-600 uppercase tracking-wide">
                    Extraction Span
                  </p>
                  <p className="text-sm text-blue-900 mt-1 font-medium">{selectedEntry.extraction_span}</p>
                </div>
              )}
            </div>
          </div>

          {/* Source preview area */}
          <div className="flex-1 overflow-auto p-6">
            <div className="bg-amber-50 border-l-4 border-amber-400 p-4 rounded mb-6">
              <p className="text-sm text-amber-900">
                <strong>Source Preview:</strong> In a full implementation, the extracted byte range would be
                highlighted in the original PDF or web document.
              </p>
            </div>

            {selectedEntry.extraction_span ? (
              <div className="bg-gray-50 p-4 rounded-lg font-mono text-sm text-gray-700 space-y-2">
                <div className="text-gray-500 text-xs uppercase tracking-wide mb-2">
                  Extracted from: {selectedEntry.extraction_span}
                </div>
                <p className="leading-relaxed">
                  "This is a sample extracted passage from the source document. In a real implementation,
                  the exact byte range [{selectedEntry.extraction_span}] would be highlighted in the
                  original PDF or web page. The blue highlighting would show exactly which sentences the AI
                  system used to generate the summary or claim."
                </p>
                <div className="pt-2 text-gray-400 border-t border-gray-200">
                  ... [rest of document] ...
                </div>
              </div>
            ) : (
              <div className="text-center text-gray-500 py-8">
                <p>No source document available for this entry</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
