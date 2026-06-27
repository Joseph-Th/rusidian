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
    <div className="flex flex-col h-full">
      {/* Filter toolbar */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <h3 className="text-lg font-semibold text-gray-900 mb-3">Visualization Options</h3>
        <div className="flex gap-4 flex-wrap">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={showGapsOnly}
              onChange={(e) => {
                setShowGapsOnly(e.target.checked)
                if (e.target.checked) setShowResolvedOnly(false)
              }}
              className="w-4 h-4 rounded border-gray-300"
            />
            <span className="text-sm text-gray-700">Show gaps only</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={showResolvedOnly}
              onChange={(e) => {
                setShowResolvedOnly(e.target.checked)
                if (e.target.checked) setShowGapsOnly(false)
              }}
              className="w-4 h-4 rounded border-gray-300"
            />
            <span className="text-sm text-gray-700">Show resolved only</span>
          </label>
        </div>

        {/* Legend */}
        <div className="mt-4 p-3 bg-blue-50 rounded-lg border border-blue-200">
          <p className="text-xs font-medium text-blue-900 mb-2">Legend:</p>
          <div className="grid grid-cols-2 gap-3 text-xs text-blue-900">
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 bg-blue-500 rounded-full border-2 border-blue-600"></div>
              <span>Known entity (solid, bright)</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 bg-gray-200 rounded-full border-2 border-dashed border-gray-400 pulse-soft"></div>
              <span>Unknown/gap (dashed, pulsing)</span>
            </div>
          </div>
        </div>
      </div>

      {/* Graph visualization */}
      <div className="flex-1 overflow-hidden relative">
        <ProgressiveGraph initialNodeId={initialNodeId} initialNodeName={initialNodeName} />

        {/* Overlay info box */}
        <div className="absolute bottom-4 left-4 bg-white rounded-lg shadow-lg p-4 max-w-xs z-10">
          <p className="text-sm text-gray-700">
            <strong>Fog of War Visualization:</strong> Solid circles represent known entities. Dashed circles
            represent knowledge gaps where the AI identified questions but hasn't found answers yet.
          </p>
          <p className="text-xs text-gray-500 mt-2">
            Double-click nodes to expand and discover connections. Unresolved gaps are shown with a pulsing
            dashed outline.
          </p>
        </div>
      </div>
    </div>
  )
}
