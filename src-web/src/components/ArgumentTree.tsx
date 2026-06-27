import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ReactFlow, {
  Node,
  Edge,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
} from 'reactflow'
import 'reactflow/dist/style.css'
import dagre from 'dagre'
import { Lightbulb, FileText, BookOpen, Tag, AlertCircle } from 'lucide-react'
import { LinkNetworkData, getEdgeColor, getEdgeStyle } from '../types/linkNetwork'

function getIconForKind(kind: string) {
  const iconProps = { size: 16, className: 'flex-shrink-0' }
  switch (kind.toLowerCase()) {
    case 'claim':
      return <Lightbulb {...iconProps} />
    case 'decision':
      return <Lightbulb {...iconProps} />
    case 'paper':
      return <FileText {...iconProps} />
    case 'book':
      return <BookOpen {...iconProps} />
    default:
      return <Tag {...iconProps} />
  }
}

function CustomNode({ data }: { data: any }) {
  const bgColor = data.kind === 'claim' || data.kind === 'decision' ? 'bg-blue-50' : 'bg-gray-50'
  const borderColor = data.kind === 'claim' || data.kind === 'decision' ? 'border-blue-200' : 'border-gray-200'

  return (
    <div className={`px-3 py-2 rounded-lg border ${borderColor} ${bgColor} shadow-md max-w-xs`}>
      <div className="flex items-center gap-2 mb-1">
        <div className="text-gray-600">{getIconForKind(data.kind)}</div>
        <span className="text-xs font-medium text-gray-500 uppercase">{data.kind}</span>
      </div>
      <div className="text-sm font-semibold text-gray-900 line-clamp-2">{data.title}</div>
    </div>
  )
}

interface ArgumentTreeProps {
  rootEntityId: string
  rootEntityName: string
}

export default function ArgumentTree({ rootEntityId, rootEntityName }: ArgumentTreeProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([])
  const [edges, setEdges, onEdgesChange] = useEdgesState([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchLinkNetwork = async () => {
      try {
        setIsLoading(true)
        setError('')

        const data = await invoke<LinkNetworkData>('get_link_network', {
          rootId: rootEntityId,
          depth: 2,
        })

        // Create nodes
        const flowNodes: Node[] = data.nodes.map((node) => ({
          id: node.id,
          data: { title: node.title, kind: node.kind },
          position: { x: 0, y: 0 }, // Will be calculated by Dagre
        }))

        // Create edges with colors
        const flowEdges: Edge[] = data.edges.map((edge) => ({
          id: edge.id,
          source: edge.source,
          target: edge.target,
          style: getEdgeStyle(edge.link_type),
          label: edge.link_type,
          labelStyle: {
            fontSize: '11px',
            fill: '#666',
            background: '#fff',
            padding: '2px 4px',
          },
          animated: false,
        }))

        // Apply Dagre layout (top-down)
        if (flowNodes.length > 0) {
          const dagreGraph = new dagre.graphlib.Graph()
          dagreGraph.setDefaultEdgeLabel(() => ({}))
          dagreGraph.setGraph({ rankdir: 'TB', nodesep: 100, ranksep: 100 })

          flowNodes.forEach((node) => {
            dagreGraph.setNode(node.id, { width: 250, height: 80 })
          })

          flowEdges.forEach((edge) => {
            dagreGraph.setEdge(edge.source, edge.target)
          })

          dagre.layout(dagreGraph)

          const layoutedNodes = flowNodes.map((node) => {
            const dagreNode = dagreGraph.node(node.id)
            return {
              ...node,
              position: {
                x: dagreNode.x - 125, // Center the node (width/2)
                y: dagreNode.y - 40, // Center the node (height/2)
              },
            }
          })

          setNodes(layoutedNodes)
        } else {
          setNodes(flowNodes)
        }

        setEdges(flowEdges)
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err)
        setError(`Failed to load link network: ${errorMsg}`)
      } finally {
        setIsLoading(false)
      }
    }

    fetchLinkNetwork()
  }, [rootEntityId, setNodes, setEdges])

  if (isLoading) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin inline-block w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full mb-4" />
          <p className="text-gray-700">Loading argument tree...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-red-50">
        <div className="text-center text-red-700 max-w-md">
          <AlertCircle className="w-8 h-8 mx-auto mb-3 text-red-600" />
          <p className="font-semibold mb-2">Error loading argument tree</p>
          <p className="text-sm text-red-600">{error}</p>
          <p className="text-xs text-red-500 mt-3">Please check the entity ID and try again.</p>
        </div>
      </div>
    )
  }

  return (
    <div className="w-full h-full bg-white">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={{ default: CustomNode }}
        fitView
      >
        <Background />
        <Controls />
        <MiniMap />
      </ReactFlow>
    </div>
  )
}
