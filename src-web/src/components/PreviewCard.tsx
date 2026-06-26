import { User, Briefcase, Folder, Tag, BookOpen, FileText, Lightbulb, CheckSquare2, MapPin, Calendar } from 'lucide-react'

interface PreviewData {
  id: string
  name: string
  kind: string
  aliases: string[]
  summary: string
}

interface PreviewCardProps {
  data: PreviewData
}

function getIconForKind(kind: string) {
  const iconProps = { size: 16, className: 'flex-shrink-0' }
  switch (kind.toLowerCase()) {
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

export default function PreviewCard({ data }: PreviewCardProps) {
  return (
    <div className="w-80 p-4 bg-white dark:bg-gray-800 shadow-xl rounded-lg border border-gray-200 dark:border-gray-700">
      <div className="flex justify-between items-start gap-2 mb-2">
        <h3 className="font-bold text-lg text-gray-900 dark:text-white flex-1 line-clamp-2">
          {data.name}
        </h3>
        <div className="flex items-center gap-2 flex-shrink-0">
          <div className="text-gray-600 dark:text-gray-400">
            {getIconForKind(data.kind)}
          </div>
          <span className="text-xs bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded text-gray-600 dark:text-gray-300 font-medium whitespace-nowrap">
            {data.kind}
          </span>
        </div>
      </div>

      {data.aliases.length > 0 && (
        <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
          Also known as: {data.aliases.join(', ')}
        </p>
      )}

      <p className="text-sm text-gray-700 dark:text-gray-300 line-clamp-4">
        {data.summary}
      </p>
    </div>
  )
}
