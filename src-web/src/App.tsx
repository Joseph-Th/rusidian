import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import EntityLink from './components/EntityLink'
import ArgumentTree from './components/ArgumentTree'
import ProgressiveGraph from './components/ProgressiveGraph'
import EdgeLegend from './components/EdgeLegend'
import { EDGE_COLORS } from './types/linkNetwork'

export default function App() {
  const [demoEntityId, setDemoEntityId] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<'home' | 'argument-tree' | 'progressive-graph'>('home')
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
      </div>
    </div>
  )
}
