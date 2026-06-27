export interface LinkNetworkNode {
  id: string
  title: string
  kind: string
}

export interface LinkNetworkEdge {
  id: string
  source: string
  target: string
  link_type: string
}

export interface LinkNetworkData {
  nodes: LinkNetworkNode[]
  edges: LinkNetworkEdge[]
}

export const EDGE_COLORS: Record<string, string> = {
  supports: '#10b981', // Green
  contradicts: '#ef4444', // Red
  cites: '#6b7280', // Gray
  derived_from: '#3b82f6', // Blue
  related_to: '#a78bfa', // Purple
  summarizes: '#f59e0b', // Amber
  mentions: '#6b7280', // Gray
  part_of: '#3b82f6', // Blue
  depends_on: '#f97316', // Orange
  decided_in: '#ec4899', // Pink
  assigned_to: '#14b8a6', // Teal
  follows_up: '#8b5cf6', // Violet
}

export function getEdgeColor(linkType: string): string {
  return EDGE_COLORS[linkType] || '#9ca3af'
}

export function getEdgeStyle(linkType: string) {
  const isContradiction = linkType === 'contradicts'
  return {
    stroke: getEdgeColor(linkType),
    strokeWidth: 2,
    strokeDasharray: isContradiction ? '5,5' : undefined,
  }
}
