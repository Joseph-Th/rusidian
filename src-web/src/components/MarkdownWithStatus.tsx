import { useState } from 'react'
import { AlertCircle, CheckCircle, HelpCircle } from 'lucide-react'

export interface Block {
  id: string
  content: string
  status: 'UserAuthored' | 'RawSource' | 'AiSummary' | 'ExtractedMetadata' | 'InferredLink' | 'Reviewed' | 'UnreviewedSuggestion'
  created_by?: string
}

interface MarkdownWithStatusProps {
  blocks: Block[]
  onReviewBlock?: (blockId: string, accepted: boolean) => void
}

const statusConfig = {
  UserAuthored: { label: 'User', color: 'bg-blue-50', borderColor: 'border-blue-200', textColor: 'text-blue-900' },
  RawSource: { label: 'Raw Source', color: 'bg-gray-50', borderColor: 'border-gray-200', textColor: 'text-gray-900' },
  AiSummary: { label: 'AI Summary', color: 'bg-amber-50', borderColor: 'border-amber-200', textColor: 'text-amber-900' },
  ExtractedMetadata: { label: 'Extracted', color: 'bg-purple-50', borderColor: 'border-purple-200', textColor: 'text-purple-900' },
  InferredLink: { label: 'Inferred', color: 'bg-teal-50', borderColor: 'border-teal-200', textColor: 'text-teal-900' },
  Reviewed: { label: 'Reviewed', color: 'bg-green-50', borderColor: 'border-green-200', textColor: 'text-green-900' },
  UnreviewedSuggestion: { label: 'Unreviewed', color: 'bg-yellow-50', borderColor: 'border-yellow-200', textColor: 'text-yellow-900' },
}

function getStatusIcon(status: Block['status']) {
  const iconProps = 'w-4 h-4 flex-shrink-0'
  switch (status) {
    case 'UserAuthored': return <span className={`${iconProps} text-blue-600`}>✍️</span>
    case 'RawSource': return <span className={`${iconProps} text-gray-600`}>📄</span>
    case 'AiSummary': return <span className={`${iconProps} text-amber-600`}>🤖</span>
    case 'ExtractedMetadata': return <span className={`${iconProps} text-purple-600`}>🏷️</span>
    case 'InferredLink': return <span className={`${iconProps} text-teal-600`}>🔗</span>
    case 'Reviewed': return <CheckCircle className={`${iconProps} text-green-600`} aria-label="Reviewed" />
    case 'UnreviewedSuggestion': return <AlertCircle className={`${iconProps} text-yellow-600`} aria-label="Needs review" />
  }
}

export default function MarkdownWithStatus({ blocks, onReviewBlock }: MarkdownWithStatusProps) {
  const [expandedBlockId, setExpandedBlockId] = useState<string | null>(null)
  const [hoveredBlockId, setHoveredBlockId] = useState<string | null>(null)

  if (blocks.length === 0) {
    return (
      <div className="text-center py-8 text-gray-500">
        <p>No blocks to display</p>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {blocks.map((block) => {
        const config = statusConfig[block.status]
        const isUnreviewed = block.status === 'UnreviewedSuggestion'
        const isAiSummary = block.status === 'AiSummary'
        const isExpanded = expandedBlockId === block.id

        return (
          <article
            key={block.id}
            className={`rounded-lg border transition-all duration-200 ${
              isExpanded
                ? `${config.color} ${config.borderColor} border-2 shadow-md p-4`
                : `${config.color} ${config.borderColor} border p-4 hover:shadow-sm cursor-pointer`
            }`}
            onMouseEnter={() => !isExpanded && setHoveredBlockId(block.id)}
            onMouseLeave={() => setHoveredBlockId(null)}
            role="region"
            aria-expanded={isExpanded}
            aria-labelledby={`block-${block.id}-title`}
          >
            {/* Header with icon and content */}
            <button
              onClick={() => setExpandedBlockId(isExpanded ? null : block.id)}
              className="w-full text-left flex gap-3 items-start group"
              aria-label={`${isExpanded ? 'Collapse' : 'Expand'} block: ${block.content.slice(0, 50)}...`}
            >
              <div className="flex-shrink-0 flex items-start pt-1 group-hover:scale-110 transition-transform duration-150">
                {getStatusIcon(block.status)}
              </div>

              <div className="flex-1 min-w-0">
                {/* Content with conditional styling */}
                <p
                  id={`block-${block.id}-title`}
                  className={`text-gray-900 leading-relaxed font-medium ${
                    isUnreviewed
                      ? 'ai-unverified'
                      : isAiSummary
                        ? 'ai-summary'
                        : ''
                  }`}
                >
                  {block.content}
                </p>

                {/* Status badge and metadata */}
                <div className="flex items-center gap-2 mt-2 flex-wrap">
                  <span
                    className={`inline-block px-2 py-1 rounded text-xs font-semibold ${config.color} ${config.borderColor} border`}
                    role="status"
                    aria-label={`Status: ${config.label}`}
                  >
                    {config.label}
                  </span>
                  {block.created_by && (
                    <span className="text-xs text-gray-600" title={`Created by ${block.created_by}`}>
                      by {block.created_by.split('@')[0]}
                    </span>
                  )}
                  {isUnreviewed && hoveredBlockId === block.id && (
                    <HelpCircle className="w-3 h-3 text-yellow-600 flex-shrink-0" aria-label="This block needs review" />
                  )}
                </div>
              </div>
            </button>

            {/* Expanded details section */}
            {isExpanded && (
              <div className="mt-4 pt-4 border-t border-current border-opacity-20 space-y-4" role="region" aria-label="Block details">
                {isUnreviewed && (
                  <>
                    <div className="bg-white bg-opacity-50 rounded p-3 space-y-2 border-l-4 border-yellow-400">
                      <p className="font-semibold text-sm text-gray-900">⚠️ AI-generated Content</p>
                      <p className="text-sm text-gray-700">
                        This block was generated by an AI and hasn't been reviewed yet. Review it below to accept or dismiss.
                      </p>
                      <p className="text-xs text-gray-600 mt-2">
                        <strong>Created by:</strong> {block.created_by || 'Unknown'}
                      </p>
                    </div>

                    <div className="flex gap-2 justify-end pt-2">
                      <button
                        onClick={() => {
                          onReviewBlock?.(block.id, false)
                          setExpandedBlockId(null)
                        }}
                        className="px-4 py-2 text-sm font-medium bg-white border border-gray-300 text-gray-800 rounded-lg hover:bg-gray-50 active:bg-gray-100 transition-colors duration-150"
                        aria-label="Dismiss this block"
                      >
                        Dismiss
                      </button>
                      <button
                        onClick={() => {
                          onReviewBlock?.(block.id, true)
                          setExpandedBlockId(null)
                        }}
                        className="px-4 py-2 text-sm font-semibold bg-green-600 text-white rounded-lg hover:bg-green-700 active:bg-green-800 transition-colors duration-150 shadow-sm"
                        aria-label="Accept and mark as reviewed"
                      >
                        ✓ Accept
                      </button>
                    </div>
                  </>
                )}

                {isAiSummary && (
                  <div className="bg-white bg-opacity-50 rounded p-3 border-l-4 border-amber-400">
                    <p className="font-semibold text-sm text-gray-900">🤖 AI Summary</p>
                    <p className="text-sm text-gray-700 mt-2">
                      This content was automatically summarized from source material by an AI system.
                    </p>
                  </div>
                )}

                {!isUnreviewed && !isAiSummary && (
                  <div className="bg-white bg-opacity-50 rounded p-3 space-y-1 text-xs text-gray-700">
                    <p><strong>Block ID:</strong> <code className="bg-gray-100 px-1 rounded">{block.id.slice(0, 12)}...</code></p>
                    <p><strong>Status:</strong> {config.label}</p>
                    {block.created_by && <p><strong>Created by:</strong> {block.created_by}</p>}
                  </div>
                )}
              </div>
            )}
          </article>
        )
      })}
    </div>
  )
}
