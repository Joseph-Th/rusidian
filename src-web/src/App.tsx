import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import EntityLink from './components/EntityLink'
import ArgumentTree from './components/ArgumentTree'
import ProgressiveGraph from './components/ProgressiveGraph'
import ProvChain from './components/ProvChain'
import EntityMatrix from './components/EntityMatrix'
import FogOfWar from './components/FogOfWar'
import KnowledgeReplay from './components/KnowledgeReplay'
import MarkdownWithStatus, { Block } from './components/MarkdownWithStatus'
import EdgeLegend from './components/EdgeLegend'
import { EDGE_COLORS } from './types/linkNetwork'

export default function App() {
  const [demoEntityId, setDemoEntityId] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<'home' | 'argument-tree' | 'progressive-graph' | 'micro-viz' | 'provenance' | 'entity-matrix' | 'fog-of-war' | 'knowledge-replay'>('home')
  const [idError, setIdError] = useState<string | null>(null)

  const isValidUUID = (id: string): boolean => {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
    return uuidRegex.test(id)
  }

  const handleTestConnection = async () => {
    try {
      setError(null)
      // You can paste a real entity ID here to test
      if (!demoEntityId.trim()) {
        setError('Please enter an entity ID')
        return
      }
      const result = await invoke('get_preview_card', { entityId: demoEntityId })
      console.log('Preview card result:', result)
    } catch (err) {
      setError(String(err))
    }
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 flex flex-col">
      <div className="bg-white border-b border-gray-200 sticky top-0 z-20">
        <div className="container mx-auto px-4">
          <header className="py-6">
            <h1 className="text-4xl font-bold text-gray-900 mb-2">PKM Workbench</h1>
            <p className="text-gray-600">Personal Knowledge Management System</p>
          </header>

          <nav className="flex gap-4 pb-4">
            <button
              onClick={() => setActiveTab('home')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'home'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Home
            </button>
            <button
              onClick={() => setActiveTab('argument-tree')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'argument-tree'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Argument Tree
            </button>
            <button
              onClick={() => setActiveTab('progressive-graph')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'progressive-graph'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Progressive Graph
            </button>
            <button
              onClick={() => setActiveTab('micro-viz')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'micro-viz'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Micro-Visualizations
            </button>
            <button
              onClick={() => setActiveTab('provenance')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'provenance'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Provenance Maps
            </button>
            <button
              onClick={() => setActiveTab('entity-matrix')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'entity-matrix'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Entity Matrix
            </button>
            <button
              onClick={() => setActiveTab('fog-of-war')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'fog-of-war'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Fog of War
            </button>
            <button
              onClick={() => setActiveTab('knowledge-replay')}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                activeTab === 'knowledge-replay'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Knowledge Replay
            </button>
          </nav>
        </div>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'home' && (
          <div className="container mx-auto px-4 py-8">
            <div className="space-y-8">
          {/* IPC Testing Section */}
          <section className="bg-white rounded-lg shadow-md p-6">
            <h2 className="text-2xl font-semibold text-gray-800 mb-4">
              Test Backend Connection
            </h2>
            <p className="text-gray-600 mb-4">
              Enter an entity ID to test the IPC bridge with the Rust backend.
            </p>
            <div className="flex gap-2 mb-4">
              <input
                type="text"
                value={demoEntityId}
                onChange={(e) => setDemoEntityId(e.target.value)}
                placeholder="Paste a UUID here (e.g., 550e8400-e29b-41d4-a716-446655440000)"
                className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <button
                onClick={handleTestConnection}
                className="px-6 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors"
              >
                Test Preview Card
              </button>
            </div>
            {error && (
              <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">
                {error}
              </div>
            )}
            <p className="text-sm text-gray-500 mt-4">
              Check the browser console (F12) for the result of the preview card query.
            </p>
          </section>

          {/* Hover Preview Demo */}
          <section className="bg-white rounded-lg shadow-md p-6">
            <h2 className="text-2xl font-semibold text-gray-800 mb-4">
              Hover Preview Demo
            </h2>
            <p className="text-gray-700 mb-6">
              Try hovering over the entity links below. When you hover, a preview card
              will appear showing entity details fetched from the backend.
            </p>
            <div className="bg-gray-50 p-4 rounded-lg">
              <p className="text-gray-800">
                This is a sample note mentioning{' '}
                <EntityLink
                  entityId={demoEntityId || 'sample-id-1'}
                  entityName="Project Alpha"
                />
                {' '}and{' '}
                <EntityLink
                  entityId={demoEntityId || 'sample-id-2'}
                  entityName="John Doe"
                />
                . Hover over the links to see entity information.
              </p>
            </div>
          </section>

          {/* Feature Roadmap */}
          <section className="bg-white rounded-lg shadow-md p-6">
            <h2 className="text-2xl font-semibold text-gray-800 mb-4">
              Feature Roadmap
            </h2>
            <ul className="space-y-2 text-gray-700">
              <li className="flex items-center gap-2">
                <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                <span>Priority 1: Hover Previews (in progress)</span>
              </li>
              <li className="flex items-center gap-2">
                <span className="w-2 h-2 bg-gray-300 rounded-full"></span>
                <span>Priority 2: Argument Trees with React Flow</span>
              </li>
              <li className="flex items-center gap-2">
                <span className="w-2 h-2 bg-gray-300 rounded-full"></span>
                <span>Priority 3: Progressive Disclosure Graphs</span>
              </li>
            </ul>
            </section>
            </div>
          </div>
        )}

        {activeTab === 'argument-tree' && (
          <div className="flex-1 flex flex-col">
            <div className="container mx-auto px-4 py-6">
              <h2 className="text-2xl font-semibold text-gray-800 mb-2">Argument Tree Visualization</h2>
              <p className="text-gray-600 mb-6">Visualize hierarchical relationships between entities using a top-down tree layout.</p>

              <EdgeLegend />

              <div className="bg-white rounded-lg shadow-md p-4 mb-6">
                <label className="block text-sm font-medium text-gray-700 mb-2">Entity ID (UUID)</label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={demoEntityId}
                    onChange={(e) => {
                      setDemoEntityId(e.target.value)
                      setIdError(null)
                    }}
                    placeholder="e.g., 550e8400-e29b-41d4-a716-446655440000"
                    className={`flex-1 px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 ${
                      idError ? 'border-red-500 focus:ring-red-500' : 'border-gray-300 focus:ring-blue-500'
                    }`}
                  />
                  <button
                    onClick={() => {
                      if (!demoEntityId.trim()) {
                        setIdError('Please enter an entity ID')
                      } else if (!isValidUUID(demoEntityId)) {
                        setIdError('Invalid UUID format')
                      } else {
                        setIdError(null)
                      }
                    }}
                    className="px-6 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors disabled:bg-gray-400"
                  >
                    Visualize
                  </button>
                </div>
                {idError && <p className="text-red-600 text-sm mt-2">{idError}</p>}
                <p className="text-xs text-gray-500 mt-2">Enter a valid UUID to visualize the argument tree</p>
              </div>
            </div>

            {demoEntityId && isValidUUID(demoEntityId) && (
              <div className="flex-1 border-t">
                <ArgumentTree rootEntityId={demoEntityId} rootEntityName={demoEntityId} />
              </div>
            )}

            {!demoEntityId && (
              <div className="flex-1 flex items-center justify-center">
                <div className="text-center text-gray-500">
                  <p className="mb-2">Enter a valid entity UUID above to visualize</p>
                  <p className="text-sm">The tree will show hierarchical links up to 2 levels deep</p>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'progressive-graph' && (
          <div className="flex-1 flex flex-col">
            <div className="container mx-auto px-4 py-6">
              <h2 className="text-2xl font-semibold text-gray-800 mb-2">Progressive Disclosure Graph</h2>
              <p className="text-gray-600 mb-6">Explore connections dynamically. Start with a root node and double-click to expand and discover relationships.</p>

              <EdgeLegend />

              <div className="bg-white rounded-lg shadow-md p-4 mb-6">
                <label className="block text-sm font-medium text-gray-700 mb-2">Entity ID (UUID)</label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={demoEntityId}
                    onChange={(e) => {
                      setDemoEntityId(e.target.value)
                      setIdError(null)
                    }}
                    placeholder="e.g., 550e8400-e29b-41d4-a716-446655440000"
                    className={`flex-1 px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 ${
                      idError ? 'border-red-500 focus:ring-red-500' : 'border-gray-300 focus:ring-blue-500'
                    }`}
                  />
                  <button
                    onClick={() => {
                      if (!demoEntityId.trim()) {
                        setIdError('Please enter an entity ID')
                      } else if (!isValidUUID(demoEntityId)) {
                        setIdError('Invalid UUID format')
                      } else {
                        setIdError(null)
                      }
                    }}
                    className="px-6 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors disabled:bg-gray-400"
                  >
                    Explore
                  </button>
                </div>
                {idError && <p className="text-red-600 text-sm mt-2">{idError}</p>}
                <p className="text-xs text-gray-500 mt-2">Enter a valid UUID to start exploring the graph</p>
              </div>
            </div>

            {demoEntityId && isValidUUID(demoEntityId) && (
              <div className="flex-1 border-t">
                <ProgressiveGraph initialNodeId={demoEntityId} initialNodeName={demoEntityId} />
              </div>
            )}

            {!demoEntityId && (
              <div className="flex-1 flex items-center justify-center">
                <div className="text-center text-gray-500">
                  <p className="mb-2">Enter a valid entity UUID above to start exploring</p>
                  <p className="text-sm">Double-click any node to expand and discover its connections</p>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'micro-viz' && (
          <div className="flex-1 flex flex-col">
            <div className="container mx-auto px-4 py-6 flex-1 flex flex-col">
              <h2 className="text-2xl font-semibold text-gray-800 mb-2">
                Micro-Visualizations: Typography as Data
              </h2>
              <p className="text-gray-600 mb-6">
                Content status is indicated through visual styling. AI-generated content gets a wavy purple underline (like a spellchecker),
                AI summaries are highlighted, and each block type has a distinct visual identity. Click any block to see details.
              </p>

              <MicroVizDemo />
            </div>
          </div>
        )}

        {activeTab === 'provenance' && (
          <div className="flex-1 flex flex-col">
            <div className="bg-white border-b border-gray-200 px-6 py-4">
              <h2 className="text-2xl font-semibold text-gray-800 mb-2">
                Supply Chain of Truth (Provenance Maps)
              </h2>
              <p className="text-gray-600 mb-4">
                Trace any AI-generated block back to its original source. Click on a block in the chain to see
                where the AI extracted information from, with precise byte ranges highlighted in the source document.
              </p>
              <div className="flex gap-2 items-center">
                <input
                  type="text"
                  value={demoEntityId}
                  onChange={(e) => {
                    setDemoEntityId(e.target.value)
                    setIdError(null)
                  }}
                  placeholder="Enter a block ID (e.g., 550e8400-e29b-41d4-a716-446655440000)"
                  className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>
            {demoEntityId && (
              <div className="flex-1 overflow-hidden">
                <ProvChain blockId={demoEntityId} />
              </div>
            )}
            {!demoEntityId && (
              <div className="flex-1 flex items-center justify-center bg-gray-50">
                <div className="text-center text-gray-500">
                  <p className="mb-2">Enter a block ID above to see its provenance chain</p>
                  <p className="text-sm">The chain will show the complete derivation history from source to final content</p>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'entity-matrix' && (
          <div className="flex-1 flex flex-col">
            <div className="bg-white border-b border-gray-200 px-6 py-4">
              <h2 className="text-2xl font-semibold text-gray-800 mb-2">Dynamic Entity Matrices (AI Spreadsheet)</h2>
              <p className="text-gray-600 mb-4">
                Visualize multi-dimensional entity relationships in a matrix format. Select row and column entity
                types to see all connections between them. Colored dots show link types, circle size indicates confidence.
              </p>
              <div className="flex gap-4 items-center">
                <div className="flex-1">
                  <label className="block text-sm font-medium text-gray-700 mb-2">Row Entity Type</label>
                  <select className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500">
                    <option value="">Select entity type (Person, Organization, Product, ...)</option>
                    <option value="person">Person</option>
                    <option value="organization">Organization</option>
                    <option value="product">Product</option>
                    <option value="project">Project</option>
                    <option value="topic">Topic</option>
                  </select>
                </div>
                <div className="flex-1">
                  <label className="block text-sm font-medium text-gray-700 mb-2">Column Entity Type</label>
                  <select className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500">
                    <option value="">Select entity type (Person, Organization, Product, ...)</option>
                    <option value="person">Person</option>
                    <option value="organization">Organization</option>
                    <option value="product">Product</option>
                    <option value="project">Project</option>
                    <option value="topic">Topic</option>
                  </select>
                </div>
              </div>
            </div>
            <div className="flex-1 overflow-hidden">
              <EntityMatrix rowKind="person" colKind="product" />
            </div>
          </div>
        )}

        {activeTab === 'fog-of-war' && (
          <div className="flex-1 flex flex-col">
            <div className="container mx-auto px-4 py-6 flex-1 flex flex-col">
              <div className="bg-white rounded-lg shadow-md p-6 flex-1 flex flex-col">
                <h2 className="text-2xl font-semibold text-gray-800 mb-2">Fog of War Maps (Missing Knowledge)</h2>
                <p className="text-gray-600 mb-6">
                  Visualize knowledge gaps and missing information. Solid circles represent known entities,
                  while dashed pulsing circles show open questions waiting for answers.
                </p>

                <div className="bg-white rounded-lg shadow-md p-4 flex-1 mb-6">
                  <label className="block text-sm font-medium text-gray-700 mb-2">Entity ID (UUID)</label>
                  <div className="flex gap-2 mb-6">
                    <input
                      type="text"
                      value={demoEntityId}
                      onChange={(e) => {
                        setDemoEntityId(e.target.value)
                        setIdError(null)
                      }}
                      placeholder="e.g., 550e8400-e29b-41d4-a716-446655440000"
                      className={`flex-1 px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 ${
                        idError ? 'border-red-500 focus:ring-red-500' : 'border-gray-300 focus:ring-blue-500'
                      }`}
                    />
                    <button
                      onClick={() => {
                        if (!demoEntityId.trim()) {
                          setIdError('Please enter an entity ID')
                        } else if (!isValidUUID(demoEntityId)) {
                          setIdError('Invalid UUID format')
                        } else {
                          setIdError(null)
                        }
                      }}
                      className="px-6 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors disabled:bg-gray-400"
                    >
                      Visualize
                    </button>
                  </div>
                  {idError && <p className="text-red-600 text-sm mt-2">{idError}</p>}

                  {demoEntityId && isValidUUID(demoEntityId) ? (
                    <div className="flex-1">
                      <FogOfWar initialNodeId={demoEntityId} initialNodeName={demoEntityId} />
                    </div>
                  ) : (
                    <div className="flex items-center justify-center h-96">
                      <div className="text-center text-gray-500">
                        <p className="mb-2">Enter a valid entity UUID above</p>
                        <p className="text-sm">The visualization will show known entities and knowledge gaps</p>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'knowledge-replay' && (
          <div className="flex-1 flex flex-col">
            <div className="container mx-auto px-4 py-6 flex-1 flex flex-col">
              <div className="flex-1 flex flex-col">
                <h2 className="text-2xl font-semibold text-gray-800 mb-2">Knowledge Replay (Time-Lapse Animation)</h2>
                <p className="text-gray-600 mb-6">
                  Watch your knowledge base evolve over time. Use the timeline slider to see which entities
                  and connections existed at any point in the past, then hit Play for a time-lapse animation.
                </p>

                <div className="bg-white rounded-lg shadow-md p-4 flex-1 flex flex-col">
                  <label className="block text-sm font-medium text-gray-700 mb-2">Entity ID (UUID)</label>
                  <div className="flex gap-2 mb-6">
                    <input
                      type="text"
                      value={demoEntityId}
                      onChange={(e) => {
                        setDemoEntityId(e.target.value)
                        setIdError(null)
                      }}
                      placeholder="e.g., 550e8400-e29b-41d4-a716-446655440000"
                      className={`flex-1 px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 ${
                        idError ? 'border-red-500 focus:ring-red-500' : 'border-gray-300 focus:ring-blue-500'
                      }`}
                    />
                    <button
                      onClick={() => {
                        if (!demoEntityId.trim()) {
                          setIdError('Please enter an entity ID')
                        } else if (!isValidUUID(demoEntityId)) {
                          setIdError('Invalid UUID format')
                        } else {
                          setIdError(null)
                        }
                      }}
                      className="px-6 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors disabled:bg-gray-400"
                    >
                      Start Timeline
                    </button>
                  </div>
                  {idError && <p className="text-red-600 text-sm mt-2">{idError}</p>}

                  {demoEntityId && isValidUUID(demoEntityId) ? (
                    <div className="flex-1">
                      <KnowledgeReplay rootEntityId={demoEntityId} rootEntityName={demoEntityId} />
                    </div>
                  ) : (
                    <div className="flex items-center justify-center h-96">
                      <div className="text-center text-gray-500">
                        <p className="mb-2">Enter a valid entity UUID above</p>
                        <p className="text-sm">The timeline will show the knowledge base state at any point in time</p>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

function MicroVizDemo() {
  const [blocks, setBlocks] = useState<Block[]>([
    {
      id: 'block-1',
      content: 'This is a user-authored note with original content written by the knowledge worker.',
      status: 'UserAuthored',
      created_by: 'joseph@example.com',
    },
    {
      id: 'block-2',
      content: 'This content was extracted from a PDF document without any processing or interpretation.',
      status: 'RawSource',
      created_by: 'System',
    },
    {
      id: 'block-3',
      content: 'This paragraph was automatically generated as a summary of the source material using an AI model.',
      status: 'AiSummary',
      created_by: 'claude-opus',
    },
    {
      id: 'block-4',
      content:
        'This block was generated by an AI agent based on analysis of linked documents and entities in the vault. It has not been reviewed yet and awaits human approval.',
      status: 'UnreviewedSuggestion',
      created_by: 'claude-opus',
    },
    {
      id: 'block-5',
      content: 'This is inferred metadata extracted from structured data or relationships within the knowledge base.',
      status: 'ExtractedMetadata',
      created_by: 'System',
    },
    {
      id: 'block-6',
      content: 'This link between two entities was inferred by the AI system based on similarity analysis.',
      status: 'InferredLink',
      created_by: 'claude-opus',
    },
    {
      id: 'block-7',
      content: 'This content has been reviewed and approved by the user. It is now trusted and ready for use.',
      status: 'Reviewed',
      created_by: 'joseph@example.com',
    },
  ])

  const handleReviewBlock = (blockId: string, accepted: boolean) => {
    setBlocks((prev) =>
      prev.map((block) =>
        block.id === blockId
          ? { ...block, status: accepted ? 'Reviewed' : 'RawSource' }
          : block
      )
    )
  }

  return (
    <div className="bg-white rounded-lg shadow-md p-6">
      <div className="mb-6 p-4 bg-blue-50 rounded border border-blue-200">
        <p className="text-sm text-blue-800">
          <strong>Tip:</strong> Click on any block to expand and see details. Unreviewed AI-generated blocks
          can be accepted or dismissed.
        </p>
      </div>

      <MarkdownWithStatus blocks={blocks} onReviewBlock={handleReviewBlock} />

      <div className="mt-8 pt-6 border-t border-gray-200">
        <h3 className="text-lg font-semibold text-gray-800 mb-4">Status Indicators</h3>
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-3">
            <div className="flex gap-2">
              <span className="text-blue-600">✍️</span>
              <div>
                <p className="font-medium text-gray-800">User Authored</p>
                <p className="text-xs text-gray-600">Content written by the user</p>
              </div>
            </div>
            <div className="flex gap-2">
              <span className="text-gray-600">📄</span>
              <div>
                <p className="font-medium text-gray-800">Raw Source</p>
                <p className="text-xs text-gray-600">Unprocessed captured content</p>
              </div>
            </div>
            <div className="flex gap-2">
              <span className="text-amber-600">🤖</span>
              <div>
                <p className="font-medium text-gray-800">AI Summary</p>
                <p className="text-xs text-gray-600">AI-generated summary with highlight</p>
              </div>
            </div>
          </div>
          <div className="space-y-3">
            <div className="flex gap-2">
              <span className="text-yellow-600">⚠️</span>
              <div>
                <p className="font-medium text-gray-800">Unreviewed Suggestion</p>
                <p className="text-xs text-gray-600">Purple wavy underline • needs approval</p>
              </div>
            </div>
            <div className="flex gap-2">
              <span className="text-purple-600">🏷️</span>
              <div>
                <p className="font-medium text-gray-800">Extracted Metadata</p>
                <p className="text-xs text-gray-600">System-extracted structured data</p>
              </div>
            </div>
            <div className="flex gap-2">
              <span className="text-green-600">✓</span>
              <div>
                <p className="font-medium text-gray-800">Reviewed</p>
                <p className="text-xs text-gray-600">Approved and trusted content</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
