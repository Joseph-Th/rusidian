import { useState, useEffect, useCallback, useRef } from 'react'
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
import * as d3Force from 'd3-force'
import { Lightbulb, FileText, BookOpen, Tag, AlertCircle, Zap } from 'lucide-react'
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
    <div className={`px-3 py-2 rounded-lg border ${borderColor} ${bgColor} shadow-md max-w-xs cursor-pointer hover:shadow-lg transition-shadow`}>
      <div className="flex items-center gap-2 mb-1">
        <div className="text-gray-600">{getIconForKind(data.kind)}</div>
        <span className="text-xs font-medium text-gray-500 uppercase">{data.kind}</span>
      </div>
      <div className="text-sm font-semibold text-gray-900 line-clamp-2">{data.title}</div>
      {data.isExpanding && (
        <div className="mt-2 text-xs text-gray-500 flex items-center gap-1">
          <span className="inline-block w-2 h-2 bg-blue-400 rounded-full animate-pulse" />
          Expanding...
        </div>
      )}
    </div>
  )
}

interface ProgressiveGraphProps {
  initialNodeId: string
  initialNodeName: string
}

export default function ProgressiveGraph({ initialNodeId, initialNodeName }: ProgressiveGraphProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([])
  const [edges, setEdges, onEdgesChange] = useEdgesState([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const expandingNodesRef = useRef(new Set<string>())
  const simulationRef = useRef<d3Force.Simulation<d3Force.SimulationNodeDatum, undefined> | null>(null)

  const applyForceDirectedLayout = useCallback(
    (flowNodes: Node[], flowEdges: Edge[]) => {
      if (flowNodes.length === 0) return

      // Kill previous simulation
      if (simulationRef.current) {
        simulationRef.current.stop()
      }

      const width = 800
      const height = 600

      const d3Nodes: (d3Force.SimulationNodeDatum & { id: string })[] = flowNodes.map((node) => ({
        ...node.position,
        id: node.id,
        vx: 0,
        vy: 0,
        fx: null,
        fy: null,
      }))

      const d3Links = flowEdges.map((edge) => ({
        source: edge.source,
        target: edge.target,
      }))

      const simulation = d3Force
        .forceSimulation(d3Nodes as any)
        .force('link', d3Force.forceLink(d3Links).id((d: any) => d.id).distance(150).strength(0.5))
        .force('charge', d3Force.forceManyBody().strength(-400))
        .force('collide', d3Force.forceCollide(80))
        .force('center', d3Force.forceCenter(width / 2, height / 2))

      let tickCount = 0
      simulation.on('tick', () => {
        tickCount++
        if (tickCount % 2 === 0) {
          setNodes((prevNodes) =>
            prevNodes.map((node) => {
              const d3Node = d3Nodes.find((d) => d.id === node.id)
              if (d3Node) {
                return {
                  ...node,
                  position: { x: d3Node.x || 0, y: d3Node.y || 0 },
                }
              }
              return node
            })
          )
        }
      })

      simulationRef.current = simulation

      // Stop after some ticks
      setTimeout(() => {
        if (simulationRef.current) {
          simulationRef.current.stop()
        }
      }, 3000)
    },
    [setNodes]
  )

  const handleNodeDoubleClick = useCallback(
    async (event: any, node: Node) => {
      // Prevent expansion if already expanding
      if (expandingNodesRef.current.has(node.id)) {
        return
      }

      expandingNodesRef.current.add(node.id)
      setNodes((prevNodes) =>
        prevNodes.map((n) => (n.id === node.id ? { ...n, data: { ...n.data, isExpanding: true } } : n))
      )

      try {
        const data = await invoke<LinkNetworkData>('get_neighbors', {
          targetId: node.id,
          depth: 1,
        })

        // Add new nodes, positioning them at the parent node location
        const newNodeIds = new Set(nodes.map((n) => n.id))
        const newNodes = data.nodes
          .filter((n) => !newNodeIds.has(n.id))
          .map((n) => ({
            id: n.id,
            data: { title: n.title, kind: n.kind },
            position: { x: node.position.x, y: node.position.y }, // Start at parent position
          }))

        // Add new edges
        const newEdgeIds = new Set(edges.map((e) => e.id))
        const newEdges = data.edges
          .filter((e) => !newEdgeIds.has(e.id))
          .map((edge) => ({
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

        // Update nodes and edges
        const updatedNodes = [...nodes, ...newNodes]
        const updatedEdges = [...edges, ...newEdges]

        setNodes(updatedNodes)
        setEdges(updatedEdges)

        // Apply force-directed layout to new graph
        applyForceDirectedLayout(updatedNodes, updatedEdges)
      } catch (err) {
        console.error('Failed to expand node:', err)
      } finally {
        expandingNodesRef.current.delete(node.id)
        setNodes((prevNodes) =>
          prevNodes.map((n) => (n.id === node.id ? { ...n, data: { ...n.data, isExpanding: false } } : n))
        )
      }
    },
    [nodes, edges, setNodes, setEdges, applyForceDirectedLayout]
  )

  useEffect(() => {
    const initializeGraph = async () => {
      try {
        setIsLoading(true)
        setError('')

        const data = await invoke<LinkNetworkData>('get_neighbors', {
          targetId: initialNodeId,
          depth: 1,
        })

        const flowNodes: Node[] = data.nodes.map((node) => ({
          id: node.id,
          data: { title: node.title, kind: node.kind },
          position: { x: Math.random() * 400 - 200, y: Math.random() * 400 - 200 },
        }))

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

        setNodes(flowNodes)
        setEdges(flowEdges)
        applyForceDirectedLayout(flowNodes, flowEdges)
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err)
        setError(`Failed to load initial graph: ${errorMsg}`)
      } finally {
        setIsLoading(false)
      }
    }

    initializeGraph()
  }, [initialNodeId, setNodes, setEdges, applyForceDirectedLayout])

  if (isLoading) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin inline-block w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full mb-4" />
          <p className="text-gray-700">Loading graph...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="w-full h-full flex items-center justify-center bg-red-50">
        <div className="text-center text-red-700 max-w-md">
          <AlertCircle className="w-8 h-8 mx-auto mb-3 text-red-600" />
          <p className="font-semibold mb-2">Error loading graph</p>
          <p className="text-sm text-red-600">{error}</p>
          <p className="text-xs text-red-500 mt-3">Please check the entity ID and try again.</p>
        </div>
      </div>
    )
  }

  return (
    <div className="w-full h-full bg-white relative">
      <div className="absolute top-4 left-4 z-10 bg-white px-4 py-3 rounded-lg shadow-md text-sm text-gray-600">
        <div className="flex items-start gap-2 mb-2">
          <Zap className="w-4 h-4 text-blue-600 flex-shrink-0 mt-0.5" />
          <div>
            <p className="font-semibold text-gray-900">Progressive Graph</p>
            <p className="text-xs text-gray-500 mt-0.5">Double-click nodes to expand</p>
          </div>
        </div>
        <p className="text-xs text-gray-500 bg-gray-50 px-2 py-1 rounded mt-2">
          {nodes.length} node{nodes.length !== 1 ? 's' : ''} · {edges.length} connection{edges.length !== 1 ? 's' : ''}
        </p>
      </div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeDoubleClick={handleNodeDoubleClick}
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
