import { asArray, asRecord, asString } from '#lib/utils/config.ts';

export type NodeKind = 'person' | 'app' | 'model' | 'data' | 'tool';

export interface DiagramNode {
	id: string;
	label: string | null;
	detail: string | null;
	kind: NodeKind | null;
	slot: boolean;
	row: number;
	col: number;
}

export interface DiagramEdge {
	from: string;
	to: string;
	label: string | null;
	dashed: boolean;
}

export interface PlacedNode extends DiagramNode {
	x: number;
	y: number;
}

export interface PlacedEdge extends DiagramEdge {
	x1: number;
	y1: number;
	x2: number;
	y2: number;
}

export const NODE_WIDTH = 148;
export const NODE_HEIGHT = 56;
const GAP_X = 52;
const GAP_Y = 44;
const PADDING = 8;

const kinds: NodeKind[] = ['person', 'app', 'model', 'data', 'tool'];

/** Reads the diagram part of a step config (see `lp_core::diagram`). */
export function parseDiagram(config: Record<string, unknown>): { nodes: DiagramNode[]; edges: DiagramEdge[] } {
	const nodes = asArray(config.nodes).map((value, index): DiagramNode => {
		const node = asRecord(value);
		const kind = asString(node.kind);
		return {
			id: asString(node.id) ?? String(index),
			label: asString(node.label) ?? null,
			detail: asString(node.detail) ?? null,
			kind: kinds.includes(kind as NodeKind) ? (kind as NodeKind) : null,
			slot: node.slot === true,
			row: typeof node.row === 'number' ? node.row : 0,
			col: typeof node.col === 'number' ? node.col : index
		};
	});
	const edges = asArray(config.edges).map((value): DiagramEdge => {
		const edge = asRecord(value);
		return {
			from: asString(edge.from) ?? '',
			to: asString(edge.to) ?? '',
			label: asString(edge.label) ?? null,
			dashed: edge.dashed === true
		};
	});
	return { nodes, edges };
}

/** Pixel positions on the grid, plus arrows clipped to the box borders. */
export function layoutDiagram(nodes: DiagramNode[], edges: DiagramEdge[]) {
	const placed: PlacedNode[] = nodes.map((node) => ({
		...node,
		x: PADDING + node.col * (NODE_WIDTH + GAP_X),
		y: PADDING + node.row * (NODE_HEIGHT + GAP_Y)
	}));
	const byId = new Map(placed.map((node) => [node.id, node]));
	const arrows: PlacedEdge[] = edges.flatMap((edge) => {
		const from = byId.get(edge.from);
		const to = byId.get(edge.to);
		if (!from || !to) return [];
		const start = center(from);
		const end = center(to);
		const [x1, y1] = clip(start, end);
		const [x2, y2] = clip(end, start);
		return [{ ...edge, x1, y1, x2, y2 }];
	});
	const width = Math.max(...placed.map((node) => node.x + NODE_WIDTH), 0) + PADDING;
	const height = Math.max(...placed.map((node) => node.y + NODE_HEIGHT), 0) + PADDING;
	return { nodes: placed, edges: arrows, width, height };
}

function center(node: PlacedNode): [number, number] {
	return [node.x + NODE_WIDTH / 2, node.y + NODE_HEIGHT / 2];
}

/** Where the line from `inside` towards `outside` leaves the box centred on `inside`. */
function clip([cx, cy]: [number, number], [tx, ty]: [number, number]): [number, number] {
	const dx = tx - cx;
	const dy = ty - cy;
	if (!dx && !dy) return [cx, cy];
	const margin = 4;
	const scaleX = dx ? (NODE_WIDTH / 2 + margin) / Math.abs(dx) : Infinity;
	const scaleY = dy ? (NODE_HEIGHT / 2 + margin) / Math.abs(dy) : Infinity;
	const scale = Math.min(scaleX, scaleY);
	return [cx + dx * scale, cy + dy * scale];
}
