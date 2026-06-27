import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ProgressiveGraph from './ProgressiveGraph'

interface FogOfWarProps {
  initialNodeId: string
  initialNodeName: string
}

export default function FogOfWar({ initialNodeId, initialNodeName }: FogOfWarProps) {
  const [showGapsOnly, setShowGapsOnly] = useState(false)
  const [showResolvedOnly, setShowResolvedOnly] = useState(false)

  return (
    <div className="flex flex-col h-full bg-gray-50">
      {/* Header */}
      <header className="bg-white border-b-2 border-gray-300 px-6 py-4 shadow-sm">
        <h2 className="text-xl font-bold text-gray-900 mb-3">Knowledge Gap Visualization (Fog of War)</h2>
        <fieldset className="space-y-3">
          <legend className="text-sm font-semibold text-gray-700 mb-2">Display Options</legend>
          <div className="flex flex-wrap gap-4">
            <label className="flex items-center gap-2 cursor-pointer hover:text-blue-600 transition-colors">
              <input
                type="radio"
                name="filter"
                checked={!showGapsOnly && !showResolvedOnly}
                onChange={() => {
                  setShowGapsOnly(false)
                  setShowResolvedOnly(false)
                }}
                className="w-4 h-4 text-blue-600"
                aria-label="Show all entities"
              />
              <span className="text-sm text-gray-700">All entities</span>
            </label>
            <label className="flex items-center gap-2 cursor-pointer hover:text-blue-600 transition-colors">
              <input
                type="radio"
                name="filter"
                checked={showGapsOnly}
                onChange={() => {
                  setShowGapsOnly(true)
                  setShowResolvedOnly(false)
                }}
                className="w-4 h-4 text-blue-600"
                aria-label="Show only knowledge gaps"
              />
              <span className="text-sm text-gray-700">Gaps only</span>
            </label>
            <label className="flex items-center gap-2 cursor-pointer hover:text-blue-600 transition-colors">
              <input
                type="radio"
                name="filter"
                checked={showResolvedOnly}
                onChange={() => {
                  setShowResolvedOnly(true)
                  setShowGapsOnly(false)
                }}
                className="w-4 h-4 text-blue-600"
                aria-label="Show only resolved entities"
              />
              <span className="text-sm text-gray-700">Resolved only</span>
            </label>
          </div>
        </fieldset>

        {/* Legend */}
        <div className="mt-4 p-3 bg-blue-50 rounded-lg border-2 border-blue-200">
          <p className="text-xs font-bold text-blue-900 uppercase tracking-wide mb-2.5">Legend:</p>
          <div className="grid grid-cols-2 gap-4 text-xs">
            <div className="flex items-center gap-2">
              <div className="w-5 h-5 bg-blue-500 rounded-full border-2 border-blue-700 flex-shrink-0"></div>
              <span className="text-blue-900">Known entity</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-5 h-5 bg-gray-100 rounded-full border-2 border-dashed border-gray-500 flex-shrink-0 pulse-soft"></div>
              <span className="text-blue-900">Knowledge gap</span>
            </div>
          </div>
        </div>
      </header>

      {/* Graph visualization */}
      <div className="flex-1 overflow-hidden relative">
        <ProgressiveGraph initialNodeId={initialNodeId} initialNodeName={initialNodeName} />

        {/* Contextual info box */}
        <article className="absolute bottom-4 left-4 bg-white rounded-lg shadow-lg p-4 max-w-xs border-l-4 border-blue-500 z-10" role="complementary" aria-label="Fog of War explanation">
          <p className="text-sm font-medium text-gray-900 mb-2">💡 How it works:</p>
          <ul className="text-xs text-gray-700 space-y-1.5 list-disc list-inside">
            <li><strong>Solid circles:</strong> Entities the AI knows about</li>
            <li><strong>Dashed circles:</strong> Questions raised but unanswered</li>
            <li><strong>Pulsing animation:</strong> Unresolved knowledge gaps</li>
          </ul>
          <p className="text-xs text-gray-500 mt-3 italic">Double-click any node to expand and explore connections.</p>
        </article>
      </div>
    </div>
  )
}
