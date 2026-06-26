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

export default function PreviewCard({ data }: PreviewCardProps) {
  return (
    <div className="w-80 p-4 bg-white dark:bg-gray-800 shadow-xl rounded-lg border border-gray-200 dark:border-gray-700">
      <div className="flex justify-between items-start mb-2">
        <h3 className="font-bold text-lg text-gray-900 dark:text-white">
          {data.name}
        </h3>
        <span className="text-xs bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded text-gray-600 dark:text-gray-300 font-medium">
          {data.kind}
        </span>
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
