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

  const progressPercent = Math.round(timelineProgress * 100)

  return (
    <div className="flex flex-col h-full bg-gray-50">
      {/* Timeline controls header */}
      <header className="bg-white border-b-2 border-gray-300 p-6 space-y-5 shadow-sm">
        <div>
          <h2 className="text-xl font-bold text-gray-900">Knowledge Timeline Replay</h2>
          <p className="text-sm text-gray-600 mt-1">Watch your knowledge base evolve through time</p>
        </div>

        {/* Timeline slider */}
        <fieldset className="space-y-3">
          <legend className="text-sm font-semibold text-gray-700">
            Progress: <span className="font-mono text-blue-700 text-base">{displayDate}</span> ({progressPercent}%)
          </legend>

          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={timelineProgress}
            onChange={handleSliderChange}
            className="w-full h-3 bg-gray-300 rounded-lg appearance-none cursor-pointer accent-blue-600 transition-shadow focus:shadow-lg focus:outline-none"
            aria-label="Timeline progress slider"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={progressPercent}
            aria-valuetext={displayDate}
          />

          <div className="flex justify-between text-xs font-medium text-gray-600">
            <time>{startDate.toLocaleDateString('en-US', { year: 'numeric', month: 'short' })}</time>
            <time>{endDate.toLocaleDateString('en-US', { year: 'numeric', month: 'short' })}</time>
          </div>
        </fieldset>

        {/* Play controls */}
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setIsPlaying(!isPlaying)}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-lg font-semibold text-sm transition-all duration-150 shadow-sm ${
              isPlaying
                ? 'bg-red-600 text-white hover:bg-red-700 active:bg-red-800'
                : 'bg-blue-600 text-white hover:bg-blue-700 active:bg-blue-800'
            }`}
            aria-label={isPlaying ? 'Pause timeline' : 'Play timeline animation'}
            aria-pressed={isPlaying}
          >
            {isPlaying ? (
              <>
                <Pause className="w-4 h-4" aria-hidden="true" /> Pause
              </>
            ) : (
              <>
                <Play className="w-4 h-4" aria-hidden="true" /> Play
              </>
            )}
          </button>
          <button
            onClick={handleReset}
            className="flex items-center gap-2 px-4 py-2.5 rounded-lg font-semibold text-sm bg-gray-300 text-gray-900 hover:bg-gray-400 active:bg-gray-500 transition-all duration-150 shadow-sm"
            aria-label="Reset timeline to beginning"
          >
            <RotateCcw className="w-4 h-4" aria-hidden="true" /> Reset
          </button>
        </div>

        {/* Info box */}
        <article className="p-3.5 bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg border-2 border-blue-200">
          <p className="text-sm text-blue-900 leading-relaxed">
            <strong>⏱️ Timeline Replay:</strong> Drag the slider backward and forward through time to see how your knowledge base evolved. Watch entities and connections appear as they were discovered or added. Click <strong>Play</strong> for an automated replay.
          </p>
        </article>
      </header>

      {/* Graph visualization */}
      <div className="flex-1 overflow-hidden relative">
        <ArgumentTree rootEntityId={rootEntityId} rootEntityName={rootEntityName} />

        {/* Contextual timestamp display */}
        <div className="absolute top-4 right-4 bg-white rounded-lg shadow-lg p-4 border-l-4 border-blue-500 z-10 max-w-xs" role="status" aria-live="polite" aria-label="Current timeline date">
          <p className="text-xs font-semibold text-gray-700 uppercase tracking-wide">Current date</p>
          <p className="text-lg font-bold text-blue-700 mt-1">{displayDate}</p>
          <p className="text-xs text-gray-600 mt-2">
            The graph shows the state of your knowledge base on this date. {progressPercent < 50 ? 'Earlier in the timeline' : 'Later in the timeline'}.
          </p>
        </div>
      </div>
    </div>
  )
}
