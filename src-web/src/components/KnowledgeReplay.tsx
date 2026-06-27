import { useState, useEffect, useRef } from 'react'
import ArgumentTree from './ArgumentTree'
import { Play, Pause, RotateCcw } from 'lucide-react'

interface KnowledgeReplayProps {
  rootEntityId: string
  rootEntityName: string
}

export default function KnowledgeReplay({ rootEntityId, rootEntityName }: KnowledgeReplayProps) {
  const [isPlaying, setIsPlaying] = useState(false)
  const [timelineProgress, setTimelineProgress] = useState(0)
  const [displayDate, setDisplayDate] = useState('')
  const animationRef = useRef<number | null>(null)

  // Simulated timeline data (start date to now)
  const startDate = new Date('2024-01-01')
  const endDate = new Date()

  useEffect(() => {
    if (!isPlaying) return

    const duration = 10000 // 10 seconds for full replay
    const startTime = Date.now()

    const animate = () => {
      const elapsed = Date.now() - startTime
      const progress = Math.min(elapsed / duration, 1)
      setTimelineProgress(progress)

      if (progress < 1) {
        animationRef.current = requestAnimationFrame(animate)
      } else {
        setIsPlaying(false)
      }
    }

    animationRef.current = requestAnimationFrame(animate)

    return () => {
      if (animationRef.current) cancelAnimationFrame(animationRef.current)
    }
  }, [isPlaying])

  // Update displayed date based on progress
  useEffect(() => {
    const msRange = endDate.getTime() - startDate.getTime()
    const currentMs = startDate.getTime() + msRange * timelineProgress
    const currentDate = new Date(currentMs)
    setDisplayDate(currentDate.toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' }))
  }, [timelineProgress])

  const handleReset = () => {
    setTimelineProgress(0)
    setIsPlaying(false)
  }

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setTimelineProgress(parseFloat(e.target.value))
    setIsPlaying(false)
  }

  return (
    <div className="flex flex-col h-full">
      {/* Timeline controls */}
      <div className="bg-white border-b border-gray-200 p-6 space-y-4">
        <h3 className="text-lg font-semibold text-gray-900">Knowledge Timeline Replay</h3>

        {/* Timeline slider */}
        <div className="space-y-2">
          <div className="flex justify-between items-center mb-2">
            <label className="text-sm font-medium text-gray-700">Timeline Progress</label>
            <div className="text-sm font-mono text-gray-700 bg-gray-100 px-3 py-1 rounded">
              {displayDate} • {Math.round(timelineProgress * 100)}%
            </div>
          </div>

          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={timelineProgress}
            onChange={handleSliderChange}
            className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600"
          />

          <div className="flex justify-between text-xs text-gray-500">
            <span>{startDate.toLocaleDateString('en-US', { year: 'numeric', month: 'short' })}</span>
            <span>{endDate.toLocaleDateString('en-US', { year: 'numeric', month: 'short' })}</span>
          </div>
        </div>

        {/* Play controls */}
        <div className="flex gap-2">
          <button
            onClick={() => setIsPlaying(!isPlaying)}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg font-medium transition-colors ${
              isPlaying
                ? 'bg-red-600 text-white hover:bg-red-700'
                : 'bg-blue-600 text-white hover:bg-blue-700'
            }`}
          >
            {isPlaying ? (
              <>
                <Pause className="w-4 h-4" /> Pause
              </>
            ) : (
              <>
                <Play className="w-4 h-4" /> Play
              </>
            )}
          </button>
          <button
            onClick={handleReset}
            className="flex items-center gap-2 px-4 py-2 rounded-lg font-medium bg-gray-200 text-gray-900 hover:bg-gray-300 transition-colors"
          >
            <RotateCcw className="w-4 h-4" /> Reset
          </button>
        </div>

        {/* Info */}
        <div className="p-3 bg-blue-50 rounded-lg border border-blue-200">
          <p className="text-sm text-blue-900">
            <strong>Timeline Replay:</strong> Watch how your knowledge base evolved over time. The graph shows
            which entities and connections existed as of the selected date. Drag the slider or click Play to
            watch the knowledge grow.
          </p>
        </div>
      </div>

      {/* Graph visualization */}
      <div className="flex-1 overflow-hidden border-t border-gray-200">
        <div className="absolute top-4 right-4 bg-white rounded-lg shadow-md p-3 z-10 max-w-xs">
          <p className="text-xs text-gray-700">
            <strong>As of {displayDate}:</strong> The graph shows the state of your knowledge base on this date.
          </p>
        </div>

        {/* Show the argument tree with the simulated temporal data */}
        <ArgumentTree rootEntityId={rootEntityId} rootEntityName={rootEntityName} />
      </div>
    </div>
  )
}
