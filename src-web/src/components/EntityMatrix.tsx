import { useState, useEffect, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { AlertCircle } from 'lucide-react'
import { getEdgeColor, getEdgeLabel } from '../types/linkNetwork'

interface MatrixCellLink {
  link_type: string
  confidence: number
  link_id: string
}

interface EntityMatrixData {
  row_entities: Array<[string, string]>
  col_entities: Array<[string, string]>
  matrix: Array<Array<MatrixCellLink | null>>
}

interface EntityMatrixProps {
  rowKind: string
  colKind: string
  minConfidence?: number
}

export default function EntityMatrix({
  rowKind,
  colKind,
  minConfidence,
}: EntityMatrixProps) {
  const [data, setData] = useState<EntityMatrixData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [hoveredCell, setHoveredCell] = useState<[number, number] | null>(null)
  const [selectedCell, setSelectedCell] = useState<[number, number] | null>(null)

  useEffect(() => {
    const loadMatrix = async () => {
      try {
        setLoading(true)
        setError(null)
        const result = await invoke<EntityMatrixData>('get_entity_matrix', {
          row_kind: rowKind,
          col_kind: colKind,
          min_confidence: minConfidence,
        })
        setData(result)
      } catch (err) {
        setError(String(err))
      } finally {
        setLoading(false)
      }
    }

    if (rowKind && colKind) {
      loadMatrix()
    }
  }, [rowKind, colKind, minConfidence])

  const stats = useMemo(() => {
    if (!data) return { rows: 0, cols: 0, links: 0, linkTypes: [] as string[] }
    const links = data.matrix.flat().filter((cell): cell is MatrixCellLink => cell !== null)
    const linkTypes = Array.from(new Set(links.map((l) => l.link_type))).sort()
    return { rows: data.row_entities.length, cols: data.col_entities.length, links: links.length, linkTypes }
  }, [data])

  const exportCsv = () => {
    if (!data) return
    const esc = (s: string) => `"${s.replace(/"/g, '""')}"`
    const header = ['', ...data.col_entities.map((c) => esc(c[1]))].join(',')
    const rows = data.row_entities.map((row, rowIdx) =>
      [
        esc(row[1]),
        ...data.col_entities.map((_, colIdx) => {
          const link = data.matrix[rowIdx]?.[colIdx]
          return link ? esc(`${link.link_type} (${(link.confidence * 100).toFixed(0)}%)`) : ''
        }),
      ].join(',')
    )
    const csv = [header, ...rows].join('\n')
    const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv;charset=utf-8' }))
    const a = document.createElement('a')
    a.href = url
    a.download = `${rowKind}-x-${colKind}-matrix.csv`
    a.click()
    URL.revokeObjectURL(url)
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50" role="status" aria-live="polite" aria-label="Loading entity matrix">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p className="text-gray-700 font-medium">Loading entity matrix...</p>
          <p className="text-xs text-gray-500 mt-2">Computing {rowKind} × {colKind} relationships</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50 p-4" role="alert">
        <div className="bg-red-50 border-2 border-red-200 rounded-lg p-4 max-w-md shadow-sm">
          <div className="flex gap-3">
            <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" aria-hidden="true" />
            <div>
              <p className="font-semibold text-red-900">Failed to Load Matrix</p>
              <p className="text-red-800 text-sm mt-2">{error}</p>
              <p className="text-red-700 text-xs mt-3">Try selecting different entity types.</p>
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (!data || data.row_entities.length === 0 || data.col_entities.length === 0) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50" role="status">
        <div className="text-center text-gray-600 max-w-md">
          <p className="font-medium text-lg">No entities found</p>
          <p className="text-sm mt-2">
            The knowledge base doesn't contain any <strong>{rowKind}</strong> {!data || data.row_entities.length === 0 ? 'or' : 'and'} <strong>{colKind}</strong> entities yet.
          </p>
          <p className="text-xs text-gray-500 mt-3">Create some entities first, then the matrix will populate automatically.</p>
        </div>
      </div>
    )
  }

  const cellLink = (row: number, col: number) => data?.matrix[row]?.[col] ?? null

  return (
    <div className="flex flex-col h-full bg-white">
      {/* Header with stats */}
      <header className="border-b-2 border-gray-300 p-4 sticky top-0 bg-gradient-to-r from-white to-gray-50 z-30">
        <div className="flex justify-between items-start gap-4">
          <div>
            <h1 className="text-xl font-bold text-gray-900">
              {rowKind.charAt(0).toUpperCase() + rowKind.slice(1)} × {colKind.charAt(0).toUpperCase() + colKind.slice(1)}
            </h1>
            <div className="mt-2 flex flex-wrap gap-3 text-sm">
              <div className="text-gray-700">
                <span className="font-semibold text-blue-600">{stats.rows}</span> rows
              </div>
              <div className="text-gray-400">•</div>
              <div className="text-gray-700">
                <span className="font-semibold text-blue-600">{stats.cols}</span> columns
              </div>
              <div className="text-gray-400">•</div>
              <div className="text-gray-700">
                <span className="font-semibold text-green-600">{stats.links}</span> connections
              </div>
            </div>
          </div>
          <button
            onClick={exportCsv}
            className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-lg hover:bg-blue-700 active:bg-blue-800 transition-colors duration-150 shadow-sm flex-shrink-0"
            aria-label="Export matrix as CSV file"
          >
            ⬇ Export CSV
          </button>
        </div>

        {/* Link-type legend */}
        {stats.linkTypes.length > 0 && (
          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1.5">
            {stats.linkTypes.map((type) => (
              <span key={type} className="inline-flex items-center gap-1.5 text-xs text-gray-600">
                <span className="w-2.5 h-2.5 rounded-full" style={{ background: getEdgeColor(type) }} />
                {getEdgeLabel(type)}
              </span>
            ))}
            <span className="text-xs text-gray-400 ml-auto">Circle size & number = confidence</span>
          </div>
        )}
      </header>

      {/* Table */}
      <div className="flex-1 overflow-auto relative bg-gray-50">
        <table className="border-collapse w-full bg-white" role="grid" aria-label={`${rowKind} vs ${colKind} relationships matrix`}>
          <thead className="sticky top-0 bg-gray-100 border-b-2 border-gray-300 z-20">
            <tr>
              <th
                scope="col"
                className="sticky left-0 z-20 bg-gray-100 w-40 px-4 py-3 border-r-2 border-gray-300 text-left"
                aria-label={rowKind}
              >
                <span className="text-xs font-bold text-gray-700 uppercase tracking-wide">{rowKind}</span>
              </th>
              {data.col_entities.map((col) => (
                <th
                  key={col[0]}
                  scope="col"
                  className="px-3 py-3 text-xs font-bold text-gray-700 border-r border-gray-300 bg-gray-100 whitespace-nowrap"
                  title={col[1]}
                >
                  <div className="max-w-32 truncate">{col[1]}</div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.row_entities.map((row, rowIdx) => (
              <tr key={row[0]} className="border-b border-gray-200 hover:bg-blue-50 transition-colors duration-75">
                <th
                  scope="row"
                  className="sticky left-0 z-10 bg-white px-4 py-3 text-sm font-medium text-gray-900 border-r-2 border-gray-300 max-w-40 truncate"
                  title={row[1]}
                >
                  {row[1]}
                </th>
                {data.col_entities.map((col, colIdx) => {
                  const link = cellLink(rowIdx, colIdx)
                  const isSelected = selectedCell?.[0] === rowIdx && selectedCell?.[1] === colIdx
                  const isHovered = hoveredCell?.[0] === rowIdx && hoveredCell?.[1] === colIdx

                  return (
                    <td
                      key={`${row[0]}-${col[0]}`}
                      className={`px-3 py-3 border-r border-gray-200 text-center transition-all duration-150 ${
                        isSelected ? 'bg-blue-200 ring-2 ring-inset ring-blue-500' : isHovered ? 'bg-blue-100' : ''
                      } ${link ? 'cursor-pointer' : ''}`}
                      onMouseEnter={() => link && setHoveredCell([rowIdx, colIdx])}
                      onMouseLeave={() => setHoveredCell(null)}
                      onClick={() => link && setSelectedCell(isSelected ? null : [rowIdx, colIdx])}
                      role="button"
                      tabIndex={link ? 0 : -1}
                      aria-pressed={isSelected}
                      aria-label={link ? `${row[1]} → ${col[1]}: ${link.link_type} (${(link.confidence * 100).toFixed(0)}% confidence)` : 'No connection'}
                    >
                      {link && (() => {
                        const color = getEdgeColor(link.link_type)
                        const pct = Math.round(link.confidence * 100)
                        // Confidence drives diameter (20–34px) so stronger links read as larger.
                        const size = 20 + Math.round(link.confidence * 14)
                        return (
                          <div className="relative inline-flex items-center justify-center">
                            <div
                              className={`rounded-full border-2 font-bold text-[11px] text-white flex items-center justify-center transition-transform duration-150 ${
                                isHovered ? 'scale-125 shadow-lg' : 'scale-100'
                              }`}
                              style={{
                                width: size,
                                height: size,
                                background: color,
                                borderColor: color,
                                // Lower-confidence links are slightly translucent.
                                opacity: 0.55 + link.confidence * 0.45,
                              }}
                              title={`${getEdgeLabel(link.link_type)} · ${pct}% confidence`}
                            >
                              {pct === 100 ? '✓' : pct}
                            </div>
                          </div>
                        )
                      })()}
                    </td>
                  )
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Details pane for selected cell */}
      {selectedCell && cellLink(selectedCell[0], selectedCell[1]) && (
        <footer className="border-t-2 border-gray-300 bg-gradient-to-r from-blue-50 to-blue-100 p-4 shadow-lg">
          <div className="flex justify-between items-start gap-4">
            <div>
              <h2 className="text-lg font-bold text-gray-900">
                {data.row_entities[selectedCell[0]][1]} <span className="text-gray-500">↔</span> {data.col_entities[selectedCell[1]][1]}
              </h2>
              <p className="text-sm text-gray-700 mt-1">
                <strong>Link type:</strong>{' '}
                <code className="bg-white px-2 py-0.5 rounded text-xs font-semibold text-blue-700 border border-blue-300">
                  {cellLink(selectedCell[0], selectedCell[1])?.link_type}
                </code>
              </p>
            </div>
            <button
              onClick={() => setSelectedCell(null)}
              className="text-gray-500 hover:text-gray-700 transition-colors text-2xl leading-none"
              aria-label="Close details panel"
            >
              ✕
            </button>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 mt-3">
            <div className="bg-white rounded p-3 border border-gray-300">
              <p className="text-xs font-bold text-gray-600 uppercase tracking-wide">Confidence</p>
              <p className="text-2xl font-bold text-blue-700 mt-1">
                {((cellLink(selectedCell[0], selectedCell[1])?.confidence || 0) * 100).toFixed(0)}%
              </p>
            </div>
            <div className="bg-white rounded p-3 border border-gray-300">
              <p className="text-xs font-bold text-gray-600 uppercase tracking-wide">Link ID</p>
              <code className="text-sm text-gray-700 font-mono mt-1 break-all">
                {cellLink(selectedCell[0], selectedCell[1])?.link_id.slice(0, 16)}...
              </code>
            </div>
            <div className="flex items-end">
              <button
                onClick={() => console.log('View link details')}
                className="w-full px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded hover:bg-blue-700 active:bg-blue-800 transition-colors duration-150"
                aria-label="View full link details"
              >
                View Link Details
              </button>
            </div>
          </div>
        </footer>
      )}
    </div>
  )
}
