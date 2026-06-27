import { useState, useEffect, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { AlertCircle } from 'lucide-react'
import { EDGE_COLORS } from '../types/linkNetwork'

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
    if (!data) return { rows: 0, cols: 0, links: 0 }
    const linkCount = data.matrix.flat().filter((cell) => cell !== null).length
    return { rows: data.row_entities.length, cols: data.col_entities.length, links: linkCount }
  }, [data])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading entity matrix...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50 p-4">
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 max-w-md">
          <div className="flex gap-3">
            <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-medium text-red-900">Failed to load matrix</p>
              <p className="text-red-800 text-sm mt-1">{error}</p>
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (!data || data.row_entities.length === 0 || data.col_entities.length === 0) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50">
        <div className="text-center text-gray-600">
          <p className="font-medium mb-1">No entities found</p>
          <p className="text-sm">
            No {rowKind} or {colKind} entities in the knowledge base
          </p>
        </div>
      </div>
    )
  }

  const cellLink = (row: number, col: number) => data?.matrix[row]?.[col] ?? null
  const getCellColor = (link: MatrixCellLink | null) => {
    if (!link) return 'bg-white'
    const color = EDGE_COLORS[link.link_type as keyof typeof EDGE_COLORS]
    return color ? `hover:bg-opacity-80 cursor-pointer relative` : 'bg-gray-100'
  }

  return (
    <div className="flex flex-col h-full bg-white">
      {/* Header with stats */}
      <div className="border-b border-gray-200 p-4 sticky top-0 bg-white z-10">
        <div className="flex justify-between items-center">
          <div>
            <h3 className="text-lg font-semibold text-gray-900">
              {rowKind} vs {colKind} Matrix
            </h3>
            <p className="text-sm text-gray-600 mt-1">
              {stats.rows} rows × {stats.cols} columns, {stats.links} connections
            </p>
          </div>
          <div className="flex gap-2">
            <button className="px-3 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
              Export CSV
            </button>
          </div>
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto relative">
        <table className="border-collapse w-full">
          <thead className="sticky top-0 bg-gray-50 border-b border-gray-200 z-20">
            <tr>
              <td className="sticky left-0 z-20 bg-gray-50 w-40 px-4 py-3 border-r border-gray-200"></td>
              {data.col_entities.map((col, idx) => (
                <th
                  key={col[0]}
                  className="px-3 py-2 text-xs font-medium text-gray-700 border-r border-gray-200 bg-gray-50 whitespace-nowrap"
                  title={col[1]}
                >
                  <div className="max-w-32 truncate">{col[1].slice(0, 20)}</div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.row_entities.map((row, rowIdx) => (
              <tr key={row[0]} className="border-b border-gray-200 hover:bg-blue-50">
                <td
                  className="sticky left-0 z-10 bg-white px-4 py-2 text-sm font-medium text-gray-900 border-r border-gray-200 max-w-40 truncate"
                  title={row[1]}
                >
                  {row[1].slice(0, 30)}
                </td>
                {data.col_entities.map((col, colIdx) => {
                  const link = cellLink(rowIdx, colIdx)
                  const isSelected = selectedCell?.[0] === rowIdx && selectedCell?.[1] === colIdx
                  const isHovered = hoveredCell?.[0] === rowIdx && hoveredCell?.[1] === colIdx

                  return (
                    <td
                      key={`${row[0]}-${col[0]}`}
                      className={`px-3 py-2 border-r border-gray-100 text-center ${getCellColor(link)} ${
                        isSelected ? 'bg-blue-100 ring-2 ring-blue-400' : ''
                      }`}
                      onMouseEnter={() => setHoveredCell([rowIdx, colIdx])}
                      onMouseLeave={() => setHoveredCell(null)}
                      onClick={() =>
                        setSelectedCell(isSelected ? null : [rowIdx, colIdx])
                      }
                    >
                      {link && (
                        <div className="relative inline-flex items-center justify-center">
                          <div
                            className={`w-6 h-6 rounded-full border-2 transition-all ${
                              EDGE_COLORS[link.link_type as keyof typeof EDGE_COLORS] ||
                              'border-gray-300 bg-gray-100'
                            } ${isHovered ? 'scale-125' : 'scale-100'}`}
                            title={`${link.link_type} (${(link.confidence * 100).toFixed(0)}%)`}
                          >
                            <span className="text-xs font-bold text-center w-full">
                              {(link.confidence * 100).toFixed(0)}%
                            </span>
                          </div>
                        </div>
                      )}
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
        <div className="border-t border-gray-200 bg-gray-50 p-4">
          <div className="flex justify-between items-start mb-3">
            <div>
              <h4 className="font-semibold text-gray-900">
                {data.row_entities[selectedCell[0]][1]} ←→{' '}
                {data.col_entities[selectedCell[1]][1]}
              </h4>
              <p className="text-sm text-gray-600 mt-1">
                Link type: <span className="font-mono font-medium">{cellLink(selectedCell[0], selectedCell[1])?.link_type}</span>
              </p>
            </div>
            <button
              onClick={() => setSelectedCell(null)}
              className="text-gray-500 hover:text-gray-700 text-xl"
            >
              ✕
            </button>
          </div>
          <div className="grid grid-cols-3 gap-3">
            <div>
              <p className="text-xs font-medium text-gray-600 uppercase">Confidence</p>
              <p className="text-lg font-bold text-gray-900">
                {((cellLink(selectedCell[0], selectedCell[1])?.confidence || 0) * 100).toFixed(0)}%
              </p>
            </div>
            <div>
              <p className="text-xs font-medium text-gray-600 uppercase">Link ID</p>
              <p className="text-sm font-mono text-gray-700 truncate">
                {cellLink(selectedCell[0], selectedCell[1])?.link_id.slice(0, 12)}...
              </p>
            </div>
            <div className="text-right">
              <button className="px-3 py-1 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 transition-colors">
                View Link
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
