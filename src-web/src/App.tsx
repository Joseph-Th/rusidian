import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  Home as HomeIcon,
  GitBranch,
  Workflow,
  Type,
  GitMerge,
  Grid3x3,
  CloudFog,
  History,
} from 'lucide-react'
import EntityLink from './components/EntityLink'
import ArgumentTree from './components/ArgumentTree'
import ProgressiveGraph from './components/ProgressiveGraph'
import ProvChain from './components/ProvChain'
import EntityMatrix from './components/EntityMatrix'
import FogOfWar from './components/FogOfWar'
import KnowledgeReplay from './components/KnowledgeReplay'
import MarkdownWithStatus, { Block } from './components/MarkdownWithStatus'
import EdgeLegend from './components/EdgeLegend'

type TabId =
  | 'home'
  | 'argument-tree'
  | 'progressive-graph'
  | 'micro-viz'
  | 'provenance'
  | 'entity-matrix'
  | 'fog-of-war'
  | 'knowledge-replay'

const TABS: {
  id: TabId
  label: string
  icon: typeof HomeIcon
  blurb: string
  accent: string
}[] = [
  { id: 'home', label: 'Home', icon: HomeIcon, blurb: 'Overview & quick access', accent: '#2563eb' },
  { id: 'argument-tree', label: 'Argument Tree', icon: GitBranch, blurb: 'Hierarchical top-down link layout', accent: '#059669' },
  { id: 'progressive-graph', label: 'Progressive Graph', icon: Workflow, blurb: 'Expand connections node by node', accent: '#7c3aed' },
  { id: 'micro-viz', label: 'Micro-Visualizations', icon: Type, blurb: 'Typography that encodes content status', accent: '#d97706' },
  { id: 'provenance', label: 'Provenance Maps', icon: GitMerge, blurb: 'Trace AI content back to its source', accent: '#0891b2' },
  { id: 'entity-matrix', label: 'Entity Matrix', icon: Grid3x3, blurb: 'Relationships in a confidence grid', accent: '#e11d48' },
  { id: 'fog-of-war', label: 'Fog of War', icon: CloudFog, blurb: 'Surface gaps in your knowledge', accent: '#64748b' },
  { id: 'knowledge-replay', label: 'Knowledge Replay', icon: History, blurb: 'Watch the graph evolve over time', accent: '#db2777' },
]

export default function App() {
  const [demoEntityId, setDemoEntityId] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [testResult, setTestResult] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<TabId>('home')
  const [idError, setIdError] = useState<string | null>(null)

  const isValidUUID = (id: string): boolean => {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
    return uuidRegex.test(id)
  }

  const handleTestConnection = async () => {
    try {
      setError(null)
      setTestResult(null)
      if (!demoEntityId.trim()) {
        setError('Please enter an entity ID')
        return
      }
      const result = await invoke('get_preview_card', { entityId: demoEntityId })
      setTestResult(JSON.stringify(result, null, 2))
    } catch (err) {
      setError(String(err))
    }
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 to-slate-100 flex flex-col">
      <div className="bg-white/90 backdrop-blur border-b border-slate-200 sticky top-0 z-20">
        <div className="container mx-auto px-4">
          <header className="pt-6 pb-4 flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-600 to-indigo-600 flex items-center justify-center text-white shadow-sm">
              <Workflow className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-slate-900 leading-tight">PKM Workbench</h1>
              <p className="text-sm text-slate-500">Personal Knowledge Management System</p>
            </div>
          </header>

          <nav className="flex gap-1.5 pb-3 overflow-x-auto -mx-1 px-1" aria-label="Visualizations">
            {TABS.map((tab) => {
              const Icon = tab.icon
              const active = activeTab === tab.id
              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  aria-current={active ? 'page' : undefined}
                  className={`flex items-center gap-2 px-3.5 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-150 ${
                    active
                      ? 'bg-blue-600 text-white shadow-sm shadow-blue-600/25'
                      : 'text-slate-600 hover:bg-slate-100 hover:text-slate-900'
                  }`}
                >
                  <Icon className="w-4 h-4 flex-shrink-0" />
                  {tab.label}
                </button>
              )
            })}
          </nav>
        </div>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'home' && (
          <div className="container mx-auto px-4 py-10 animate-fade-in-up">
            {/* Hero */}
            <div className="max-w-2xl mb-10">
              <h2 className="text-3xl font-bold text-slate-900 tracking-tight">
                Explore your knowledge, visually.
              </h2>
              <p className="text-slate-600 mt-3 leading-relaxed">
                Seven complementary lenses on the same knowledge graph — from hierarchical argument
                trees to time-lapse replays. Pick a visualization below to get started.
              </p>
            </div>

            {/* Feature grid */}
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
              {TABS.filter((t) => t.id !== 'home').map((tab) => {
                const Icon = tab.icon
                return (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id)}
                    className="group text-left bg-white rounded-2xl border border-slate-200 p-5 shadow-sm hover:shadow-md hover:-translate-y-0.5 hover:border-slate-300 transition-all duration-200"
                  >
                    <div
                      className="w-11 h-11 rounded-xl flex items-center justify-center mb-4 transition-transform duration-200 group-hover:scale-110"
                      style={{ background: `${tab.accent}1a`, color: tab.accent }}
                    >
                      <Icon className="w-5 h-5" />
                    </div>
                    <h3 className="font-semibold text-slate-900">{tab.label}</h3>
                    <p className="text-sm text-slate-500 mt-1 leading-snug">{tab.blurb}</p>
                    <span
                      className="inline-flex items-center gap-1 text-sm font-medium mt-3 opacity-0 group-hover:opacity-100 transition-opacity"
                      style={{ color: tab.accent }}
                    >
                      Open →
                    </span>
                  </button>
                )
              })}
            </div>

            {/* Hover Preview Demo */}
            <section className="bg-white rounded-2xl border border-slate-200 shadow-sm p-6 mt-8">
              <h3 className="text-lg font-semibold text-slate-900 mb-1">Hover Preview Demo</h3>
              <p className="text-sm text-slate-600 mb-4">
                Hover over the highlighted entities to fetch and preview their details from the backend.
              </p>
              <div className="bg-slate-50 border border-slate-200 p-4 rounded-xl text-slate-800 leading-relaxed">
                This is a sample note mentioning{' '}
                <EntityLink entityId={demoEntityId || 'sample-id-1'} entityName="Project Alpha" />
                {' '}and{' '}
                <EntityLink entityId={demoEntityId || 'sample-id-2'} entityName="John Doe" />.
              </div>
            </section>

            {/* Backend connection test */}
            <details className="bg-white rounded-2xl border border-slate-200 shadow-sm mt-6 group">
              <summary className="cursor-pointer list-none px-6 py-4 flex items-center justify-between text-slate-700 font-medium select-none">
                <span>Developer: test backend connection</span>
                <span className="text-slate-400 transition-transform group-open:rotate-90">›</span>
              </summary>
              <div className="px-6 pb-6 border-t border-slate-100 pt-4">
                <div className="flex flex-col sm:flex-row gap-2">
                  <input
                    type="text"
                    value={demoEntityId}
                    onChange={(e) => setDemoEntityId(e.target.value)}
                    placeholder="Paste an entity UUID…"
                    className="flex-1 px-4 py-2 border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                  <button
                    onClick={handleTestConnection}
                    className="px-5 py-2 bg-slate-800 text-white rounded-lg font-medium hover:bg-slate-900 transition-colors"
                  >
                    Fetch preview card
                  </button>
                </div>
                {error && (
                  <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mt-3 text-sm">
                    {error}
                  </div>
                )}
                {testResult && !error && (
                  <pre className="bg-slate-900 text-slate-100 text-xs rounded-lg mt-3 p-4 overflow-x-auto">
                    {testResult}
                  </pre>
                )}
              </div>
            </details>
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
