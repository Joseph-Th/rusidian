import { EDGE_COLORS } from '../types/linkNetwork'

export default function EdgeLegend() {
  const legendItems = [
    { type: 'supports', label: 'Supports', color: EDGE_COLORS.supports },
    { type: 'contradicts', label: 'Contradicts (dashed)', color: EDGE_COLORS.contradicts },
    { type: 'derived_from', label: 'Derived From', color: EDGE_COLORS.derived_from },
    { type: 'cites', label: 'Cites', color: EDGE_COLORS.cites },
    { type: 'related_to', label: 'Related To', color: EDGE_COLORS.related_to },
  ]

  return (
    <div className="bg-white rounded-lg shadow-md p-4 mb-6">
      <h3 className="font-semibold text-gray-800 mb-3">Edge Types Legend</h3>
      <div className="grid grid-cols-2 gap-3">
        {legendItems.map((item) => (
          <div key={item.type} className="flex items-center gap-2">
            <div className="flex-1 flex items-center gap-2">
              <div
                className="w-6 flex-shrink-0"
                style={
                  item.type === 'contradicts'
                    ? { borderTop: `2px dashed ${item.color}` }
                    : { height: 2, backgroundColor: item.color, borderRadius: 2 }
                }
              />
              <span className="text-xs text-gray-600">{item.label}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
