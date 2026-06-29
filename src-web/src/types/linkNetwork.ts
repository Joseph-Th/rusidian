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

/**
 * Human-readable labels for edge/link types, used in legends and tooltips.
 */
export const EDGE_LABELS: Record<string, string> = {
  supports: 'Supports',
  contradicts: 'Contradicts',
  cites: 'Cites',
  derived_from: 'Derived from',
  related_to: 'Related to',
  summarizes: 'Summarizes',
  mentions: 'Mentions',
  part_of: 'Part of',
  depends_on: 'Depends on',
  decided_in: 'Decided in',
  assigned_to: 'Assigned to',
  follows_up: 'Follows up',
}

export function getEdgeLabel(linkType: string): string {
  return EDGE_LABELS[linkType] || linkType.replace(/_/g, ' ')
}

/**
 * Accent color per entity kind, used to give graph nodes and preview cards a
 * consistent visual identity. `accent` drives the icon + left border, `tint`
 * is a very light fill suitable as a node background.
 */
export interface KindStyle {
  accent: string
  tint: string
}

export const KIND_STYLES: Record<string, KindStyle> = {
  person: { accent: '#6366f1', tint: '#eef2ff' }, // Indigo
  organization: { accent: '#d97706', tint: '#fffbeb' }, // Amber
  project: { accent: '#2563eb', tint: '#eff6ff' }, // Blue
  topic: { accent: '#7c3aed', tint: '#f5f3ff' }, // Violet
  book: { accent: '#0d9488', tint: '#f0fdfa' }, // Teal
  paper: { accent: '#0891b2', tint: '#ecfeff' }, // Cyan
  claim: { accent: '#059669', tint: '#ecfdf5' }, // Emerald
  decision: { accent: '#e11d48', tint: '#fff1f2' }, // Rose
  location: { accent: '#ea580c', tint: '#fff7ed' }, // Orange
  event: { accent: '#db2777', tint: '#fdf2f8' }, // Pink
}

export function getKindStyle(kind: string): KindStyle {
  return KIND_STYLES[kind?.toLowerCase()] || { accent: '#64748b', tint: '#f8fafc' }
}

export function getEdgeStyle(linkType: string) {
  const isContradiction = linkType === 'contradicts'
  return {
    stroke: getEdgeColor(linkType),
    strokeWidth: 2,
    strokeDasharray: isContradiction ? '5,5' : undefined,
  }
}
