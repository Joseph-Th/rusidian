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
import { AlertCircle } from 'lucide-react'
import { LinkNetworkData, getEdgeStyle, getEdgeLabel } from '../types/linkNetwork'
import GraphNode from './GraphNode'

const nodeTypes = { default: GraphNode }

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
          label: getEdgeLabel(edge.link_type),
          labelStyle: { fontSize: 10, fontWeight: 600, fill: '#475569' },
          labelBgStyle: { fill: '#ffffff', fillOpacity: 0.9 },
          labelBgPadding: [4, 2] as [number, number],
          labelBgBorderRadius: 4,
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
        nodeTypes={nodeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={20} size={1} color="#e2e8f0" />
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable nodeColor="#cbd5e1" maskColor="rgba(241,245,249,0.7)" />
      </ReactFlow>
    </div>
  )
}
