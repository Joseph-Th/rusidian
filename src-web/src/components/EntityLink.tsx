import { useState, useRef, useEffect } from 'react'
import { useFloating, offset, flip, shift } from '@floating-ui/react'
import { invoke } from '@tauri-apps/api/core'
import PreviewCard from './PreviewCard'

interface PreviewData {
  id: string
  name: string
  kind: string
  aliases: string[]
  summary: string
}

interface EntityLinkProps {
  entityId: string
  entityName: string
}

export default function EntityLink({ entityId, entityName }: EntityLinkProps) {
  const [isOpen, setIsOpen] = useState(false)
  const [previewData, setPreviewData] = useState<PreviewData | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const { refs, floatingStyles } = useFloating({
    open: isOpen,
    onOpenChange: setIsOpen,
    middleware: [offset(10), flip(), shift()],
  })

  const hoverTimeoutRef = useRef<ReturnType<typeof setTimeout>>()

  useEffect(() => {
    return () => {
      if (hoverTimeoutRef.current) {
        clearTimeout(hoverTimeoutRef.current)
      }
    }
  }, [])

  const handleMouseEnter = async () => {
    // Debounce hover to avoid fetching on quick mouse passes
    hoverTimeoutRef.current = setTimeout(async () => {
      setIsOpen(true)
      if (!previewData && !isLoading) {
        setIsLoading(true)
        setError(null)
        try {
          const data = await invoke<PreviewData>('get_preview_card', {
            entityId,
          })
          setPreviewData(data)
        } catch (err) {
          setError(String(err))
        } finally {
          setIsLoading(false)
        }
      }
    }, 300)
  }

  const handleMouseLeave = () => {
    if (hoverTimeoutRef.current) {
      clearTimeout(hoverTimeoutRef.current)
    }
    setIsOpen(false)
  }

  return (
    <>
      <span
        ref={refs.setReference}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        className="text-blue-600 underline decoration-dashed cursor-pointer hover:text-blue-700 transition-colors"
      >
        {entityName}
      </span>

      {isOpen && (
        <div
          ref={refs.setFloating}
          style={floatingStyles}
          className="z-50"
          onMouseEnter={() => {
            if (hoverTimeoutRef.current) clearTimeout(hoverTimeoutRef.current)
          }}
          onMouseLeave={handleMouseLeave}
        >
          {isLoading ? (
            <div className="w-80 p-4 bg-white dark:bg-gray-800 shadow-xl rounded-lg border border-gray-200 flex items-center justify-center">
              <div className="animate-spin inline-block w-5 h-5 border-2 border-blue-500 border-t-transparent rounded-full" />
            </div>
          ) : error ? (
            <div className="w-80 p-4 bg-red-50 dark:bg-red-900/30 shadow-xl rounded-lg border border-red-200 text-red-700 text-sm">
              Failed to load: {error}
            </div>
          ) : previewData ? (
            <PreviewCard data={previewData} />
          ) : null}
        </div>
      )}
    </>
  )
}
