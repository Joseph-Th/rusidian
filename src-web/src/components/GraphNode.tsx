import { Handle, Position } from 'reactflow'
import {
  User,
  Briefcase,
  Folder,
  Tag,
  BookOpen,
  FileText,
  Lightbulb,
  CheckSquare2,
  MapPin,
  Calendar,
} from 'lucide-react'
import { getKindStyle } from '../types/linkNetwork'

function getIconForKind(kind: string) {
  const iconProps = { size: 15, className: 'flex-shrink-0' }
  switch (kind?.toLowerCase()) {
    case 'person':
      return <User {...iconProps} />
    case 'organization':
      return <Briefcase {...iconProps} />
    case 'project':
      return <Folder {...iconProps} />
    case 'topic':
      return <Tag {...iconProps} />
    case 'book':
      return <BookOpen {...iconProps} />
    case 'paper':
      return <FileText {...iconProps} />
    case 'claim':
      return <Lightbulb {...iconProps} />
    case 'decision':
      return <CheckSquare2 {...iconProps} />
    case 'location':
      return <MapPin {...iconProps} />
    case 'event':
      return <Calendar {...iconProps} />
    default:
      return <Tag {...iconProps} />
  }
}

/**
 * Shared ReactFlow node used by both the Argument Tree and Progressive Graph.
 * Color-codes by entity kind for an at-a-glance read of the graph.
 */
export default function GraphNode({ data }: { data: any }) {
  const { accent, tint } = getKindStyle(data.kind)

  return (
    <div
      className="rounded-xl border shadow-sm transition-all duration-150 hover:shadow-md hover:-translate-y-0.5 max-w-[15rem]"
      style={{
        background: tint,
        borderColor: accent,
        borderLeftWidth: 4,
      }}
    >
      <Handle type="target" position={Position.Top} style={{ background: accent, opacity: 0.5 }} />
      <div className="px-3 py-2">
        <div className="flex items-center gap-1.5 mb-1" style={{ color: accent }}>
          {getIconForKind(data.kind)}
          <span className="text-[10px] font-bold uppercase tracking-wider">{data.kind}</span>
        </div>
        <div className="text-sm font-semibold text-gray-900 leading-snug line-clamp-2">
          {data.title}
        </div>
        {data.isExpanding && (
          <div className="mt-2 text-xs text-gray-500 flex items-center gap-1.5">
            <span className="inline-block w-2 h-2 rounded-full animate-pulse" style={{ background: accent }} />
            Expanding…
          </div>
        )}
      </div>
      <Handle type="source" position={Position.Bottom} style={{ background: accent, opacity: 0.5 }} />
    </div>
  )
}
