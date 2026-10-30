import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import {
  applyView,
  dragBox,
  drawMark,
  drawSelection,
  handlesOf,
  renderTextBitmap,
  toCanvas,
  toImage,
  type Handle,
} from '../../../edit/draw';
import {
  bounds,
  boxFrom,
  emptyLayer,
  fitMark,
  hitTest,
  isEmptyLayer,
  moveMark,
  newMarkId,
  snapEnd,
  squareEnd,
  type Area,
  type EditLayer,
  type Mark,
  type Point,
} from '../../../edit/layer';
import { t, type MessageKey } from '../../../i18n';
import { letterOf } from '../../../keyboard/shelfKeys';
import { icons } from './icons';
import styles from './Preview.module.css';
import { Stage, useFittedView, wheelZoom } from './Stage';

export type Tool = 'select' | 'pen' | 'highlighter' | 'rect' | 'ellipse' | 'arrow' | 'line' | 'text' | 'pixelate';

const TOOLS: { tool: Tool; key: string; label: MessageKey }[] = [
  { tool: 'select', key: 'v', label: 'markup.tool.select' },
  { tool: 'pen', key: 'b', label: 'markup.tool.pen' },
  { tool: 'highlighter', key: 'h', label: 'markup.tool.highlighter' },
  { tool: 'rect', key: 'r', label: 'markup.tool.rect' },
  { tool: 'ellipse', key: 'o', label: 'markup.tool.ellipse' },
  { tool: 'arrow', key: 'a', label: 'markup.tool.arrow' },
  { tool: 'line', key: 'd', label: 'markup.tool.line' },
  { tool: 'text', key: 't', label: 'markup.tool.text' },
  { tool: 'pixelate', key: 'x', label: 'markup.tool.pixelate' },
];

const COLORS = ['#e5322d', '#f28c28', '#ffd400', '#2fa84f', '#1e6fd9', '#1b1b1b', '#ffffff'];
// Sizes on screen; marks store them in image pixels, so a stroke looks the same at any zoom.
const WIDTHS = [3, 6, 12];
const HIGHLIGHT_WIDTHS = [14, 24, 38];
const TEXT_SIZES = [18, 28, 44];
const WIDTH_LABELS: MessageKey[] = ['markup.width.thin', 'markup.width.medium', 'markup.width.thick'];
const SELECTION = '#60cdff';

type Drag =
  | { kind: 'draw'; start: Point }
  | { kind: 'move'; start: Point; original: Mark }
  | { kind: 'resize'; handle: Handle; original: Mark }
  | { kind: 'pan'; start: Point; pan: [number, number] };

interface TextDraft {
  x: number;
  y: number;
  value: string;
  size: number;
  color: string;
  replaces: string | null;
}

export interface MarkupProps {
  image: ImageBitmap;
  layer: EditLayer;
  commit: (next: EditLayer) => void;
  undo: () => void;
  redo: () => void;
  canUndo: boolean;
  canRedo: boolean;
  /** The part of the picture to fit (the crop or all of it), with zoom and pan from the preview. */
  area: Area;
  clip: Area | null;
  zoom: number;
  pan: [number, number];
  onZoom: (zoom: number, pan: [number, number]) => void;
  saving: boolean;
  onSave: () => void;
  onDone: () => void;
  onCrop: () => void;
}

const isEditable = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || target.tagName === 'TEXTAREA' || target.tagName === 'INPUT');

/** Whether a finished shape is too small to keep (a click without a drag). */
function tooSmall(mark: Mark, scale: number): boolean {
  const min = 3 / scale;
  switch (mark.type) {
    case 'pen':
    case 'highlighter':
    case 'text':
      return false;
    case 'line':
    case 'arrow':
      return Math.hypot(mark.x2 - mark.x1, mark.y2 - mark.y1) < min;
    default:
      return mark.w < min || mark.h < min;
  }
}

function resized(original: Mark, handle: Handle, point: Point, shift: boolean): Mark {
  if (original.type === 'line' || original.type === 'arrow') {
    if (handle === 'p1') {
      const [x1, y1] = shift ? snapEnd([original.x2, original.y2], point) : point;
      return { ...original, x1, y1 };
    }
    const [x2, y2] = shift ? snapEnd([original.x1, original.y1], point) : point;
    return { ...original, x2, y2 };
  }
  const from = bounds(original);
  const to = dragBox(from, handle, point);
  return fitMark(original, from, { ...to, w: Math.max(1, to.w), h: Math.max(1, to.h) });
}

export function MarkupEditor(props: MarkupProps) {
  const { image, layer, commit, area, clip, zoom, pan, onZoom } = props;
  const { box, size, view } = useFittedView(area, zoom, pan);
  const [tool, setTool] = useState<Tool>('pen');
  const [color, setColor] = useState(COLORS[0]!);
  const [level, setLevel] = useState(1);
  const [selected, setSelected] = useState<string | null>(null);
  const [hidden, setHidden] = useState<string | null>(null);
  const [text, setText] = useState<TextDraft | null>(null);
  const [tick, setTick] = useState(0);
  const draft = useRef<Mark | null>(null);
  const drag = useRef<Drag | null>(null);
  const bump = useCallback(() => setTick((n) => n + 1), []);

  const selectedMark = layer.marks.find((m) => m.id === selected) ?? null;
  // A mark being moved is drawn from the draft only, so it does not appear twice.
  const shown = useMemo(
    () => (hidden ? { ...layer, marks: layer.marks.filter((m) => m.id !== hidden) } : layer),
    [layer, hidden],
  );

  const localPoint = (clientX: number, clientY: number): Point => {
    const rect = box.current?.getBoundingClientRect();
    return rect ? [clientX - rect.left, clientY - rect.top] : [clientX, clientY];
  };

  const replaceMark = (next: Mark) => commit({ ...layer, marks: layer.marks.map((m) => (m.id === next.id ? next : m)) });

  const applyStyle = (patch: { color?: string; level?: number }) => {
    if (patch.color) setColor(patch.color);
    if (patch.level !== undefined) setLevel(patch.level);
    // Changing colour or thickness restyles the selected mark.
    if (!selectedMark) return;
    const lvl = patch.level ?? level;
    if (selectedMark.type === 'text') {
      const next = { ...selectedMark, color: patch.color ?? selectedMark.color, size: patch.level === undefined ? selectedMark.size : TEXT_SIZES[lvl]! / view.scale };
      const r = renderTextBitmap(next.text, next.size, next.color);
      replaceMark({ ...next, bitmap: r.bitmap, w: r.w, h: r.h });
    } else if (selectedMark.type !== 'pixelate') {
      const widths = selectedMark.type === 'highlighter' ? HIGHLIGHT_WIDTHS : WIDTHS;
      replaceMark({
        ...selectedMark,
        color: patch.color ?? selectedMark.color,
        width: patch.level === undefined ? selectedMark.width : widths[lvl]! / view.scale,
      });
    }
  };

  const startText = (point: Point, existing?: Extract<Mark, { type: 'text' }>) => {
    const pad = existing ? Math.ceil(existing.size * 0.15) : 0;
    setText({
      x: existing ? existing.x + pad : point[0],
      y: existing ? existing.y + pad : point[1],
      value: existing?.text ?? '',
      size: existing?.size ?? TEXT_SIZES[level]! / view.scale,
      color: existing?.color ?? color,
      replaces: existing?.id ?? null,
    });
    setHidden(existing?.id ?? null);
  };

  const commitText = () => {
    if (!text) return;
    const value = text.value.replace(/\s+$/, '');
    let marks = layer.marks.filter((m) => m.id !== text.replaces);
    if (value) {
      const r = renderTextBitmap(value, text.size, text.color);
      const pad = Math.ceil(text.size * 0.15);
      const mark: Mark = {
        id: text.replaces ?? newMarkId(),
        type: 'text',
        color: text.color,
        size: text.size,
        text: value,
        bitmap: r.bitmap,
        x: Math.round(text.x - pad),
        y: Math.round(text.y - pad),
        w: r.w,
        h: r.h,
      };
      marks = text.replaces ? layer.marks.map((m) => (m.id === text.replaces ? mark : m)) : [...marks, mark];
    }
    if (value || text.replaces) commit({ ...layer, marks });
    setText(null);
    setHidden(null);
  };

  const newDraft = (point: Point): Mark | null => {
    const id = newMarkId();
    const [x, y] = point;
    const width = (tool === 'highlighter' ? HIGHLIGHT_WIDTHS : WIDTHS)[level]! / view.scale;
    switch (tool) {
      case 'pen':
      case 'highlighter':
        return { id, type: tool, color, width, points: [point] };
      case 'rect':
      case 'ellipse':
        return { id, type: tool, color, width, x, y, w: 0, h: 0 };
      case 'line':
      case 'arrow':
        return { id, type: tool, color, width, x1: x, y1: y, x2: x, y2: y };
      case 'pixelate':
        // Coarse blocks relative to the picture, so small text cannot be read back.
        return { id, type: 'pixelate', block: Math.max(10, Math.round(Math.max(image.width, image.height) * 0.012)), x, y, w: 0, h: 0 };
      default:
        return null;
    }
  };

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (text) {
      commitText();
      return;
    }
    const at = localPoint(e.clientX, e.clientY);
    const point = toImage(view, at[0], at[1]);
    if (e.button === 1) {
      e.currentTarget.setPointerCapture(e.pointerId);
      drag.current = { kind: 'pan', start: at, pan };
      return;
    }
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    if (tool === 'select') {
      if (selectedMark) {
        const handle = handlesOf(selectedMark).find(([, [hx, hy]]) => {
          const [cx, cy] = toCanvas(view, hx, hy);
          return Math.abs(cx - at[0]) <= 7 && Math.abs(cy - at[1]) <= 7;
        });
        if (handle) {
          drag.current = { kind: 'resize', handle: handle[0], original: selectedMark };
          setHidden(selectedMark.id);
          return;
        }
      }
      const hit = [...layer.marks].reverse().find((m) => hitTest(m, point, 6 / view.scale));
      setSelected(hit?.id ?? null);
      if (hit) {
        drag.current = { kind: 'move', start: point, original: hit };
        setHidden(hit.id);
      }
      return;
    }
    if (tool === 'text') {
      const hit = [...layer.marks].reverse().find((m) => m.type === 'text' && hitTest(m, point, 2 / view.scale));
      startText(point, hit?.type === 'text' ? hit : undefined);
      return;
    }
    drag.current = { kind: 'draw', start: point };
    draft.current = newDraft(point);
    setSelected(null);
    bump();
  };

  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    const native = e.nativeEvent;
    // Every coalesced sample is kept, so fast strokes lose no points.
    const samples = native.getCoalescedEvents?.() ?? [];
    const events = samples.length > 0 ? samples : [native];
    for (const ev of events) {
      const at = localPoint(ev.clientX, ev.clientY);
      const point = toImage(view, at[0], at[1]);
      const current = draft.current;
      if (d.kind === 'pan') {
        onZoom(zoom, [d.pan[0] + at[0] - d.start[0], d.pan[1] + at[1] - d.start[1]]);
      } else if (d.kind === 'move') {
        draft.current = moveMark(d.original, point[0] - d.start[0], point[1] - d.start[1]);
      } else if (d.kind === 'resize') {
        draft.current = resized(d.original, d.handle, point, ev.shiftKey);
      } else if (current?.type === 'pen' || current?.type === 'highlighter') {
        current.points.push(point);
      } else if (current?.type === 'line' || current?.type === 'arrow') {
        const [x2, y2] = ev.shiftKey ? snapEnd(d.start, point) : point;
        draft.current = { ...current, x2, y2 };
      } else if (current) {
        const end = ev.shiftKey && current.type !== 'pixelate' ? squareEnd(d.start, point) : point;
        draft.current = { ...current, ...boxFrom(d.start, end) } as Mark;
      }
    }
    bump();
  };

  const onPointerUp = () => {
    const d = drag.current;
    let mark = draft.current;
    drag.current = null;
    draft.current = null;
    setHidden(null);
    if (d && mark && (d.kind === 'move' || d.kind === 'resize')) {
      if (mark.type === 'text' && d.kind === 'resize') {
        const r = renderTextBitmap(mark.text, mark.size, mark.color);
        mark = { ...mark, bitmap: r.bitmap, w: r.w, h: r.h };
      }
      if (JSON.stringify(mark) !== JSON.stringify(d.original)) replaceMark(mark);
    } else if (d?.kind === 'draw' && mark && !tooSmall(mark, view.scale)) {
      commit({ ...layer, marks: [...layer.marks, mark] });
    }
    bump();
  };

  const remove = () => {
    if (!selectedMark) return;
    commit({ ...layer, marks: layer.marks.filter((m) => m.id !== selectedMark.id) });
    setSelected(null);
  };

  const overlay = useCallback(
    (ctx: CanvasRenderingContext2D, dpr: number) => {
      const current = draft.current;
      if (current) {
        ctx.save();
        applyView(ctx, view, dpr);
        if (clip) {
          ctx.beginPath();
          ctx.rect(clip.x, clip.y, clip.w, clip.h);
          ctx.clip();
        }
        if (current.type === 'pixelate') {
          ctx.setLineDash([6 / view.scale, 4 / view.scale]);
          ctx.lineWidth = 1.5 / view.scale;
          ctx.strokeStyle = SELECTION;
          ctx.strokeRect(current.x, current.y, current.w, current.h);
        } else {
          drawMark(ctx, current, bump);
        }
        ctx.restore();
      }
      const chosen = current && current.id === selected ? current : selectedMark;
      if (chosen && tool === 'select') drawSelection(ctx, chosen, view, dpr, SELECTION);
    },
    [view, clip, selected, selectedMark, tool, bump],
  );

  const { onSave, onDone, onCrop, undo, redo } = props;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isEditable(e.target)) return;
      const ctrl = e.ctrlKey || e.metaKey;
      const letter = letterOf(e);
      if (e.key === 'Escape') {
        e.preventDefault();
        onDone();
      } else if (ctrl && letter === 'z') {
        e.preventDefault();
        if (e.shiftKey) redo();
        else undo();
      } else if (ctrl && letter === 'y') {
        e.preventDefault();
        redo();
      } else if (ctrl && letter === 's') {
        e.preventDefault();
        onSave();
      } else if ((e.key === 'Delete' || e.key === 'Backspace') && selected) {
        e.preventDefault();
        commit({ ...layer, marks: layer.marks.filter((m) => m.id !== selected) });
        setSelected(null);
      } else if (!ctrl && !e.altKey && !e.shiftKey && letter) {
        if (letter === 'c') {
          e.preventDefault();
          onCrop();
          return;
        }
        const found = TOOLS.find((entry) => entry.key === letter);
        if (found) {
          e.preventDefault();
          setTool(found.tool);
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onDone, onSave, onCrop, undo, redo, commit, layer, selected]);

  const textStyle = text
    ? (() => {
        const [left, top] = toCanvas(view, text.x, text.y);
        const fontSize = text.size * view.scale;
        const lines = text.value.split('\n');
        const longest = Math.max(1, ...lines.map((line) => line.length));
        return {
          left,
          top,
          fontSize,
          color: text.color,
          width: Math.max(fontSize * 2, longest * fontSize * 0.62 + fontSize),
          height: lines.length * fontSize * 1.25 + 4,
        };
      })()
    : null;

  return (
    <>
      <div className={styles.bar} role="toolbar" aria-label={t('markup.label')}>
        {TOOLS.map((entry) => (
          <button
            key={entry.tool}
            type="button"
            className={styles.button}
            aria-pressed={tool === entry.tool}
            title={t(entry.label)}
            aria-label={t(entry.label)}
            onClick={() => setTool(entry.tool)}
          >
            {icons[entry.tool]}
          </button>
        ))}
        <span className={styles.sep} />
        {COLORS.map((c) => (
          <button
            key={c}
            type="button"
            className={styles.swatch}
            style={{ background: c }}
            aria-pressed={color === c}
            aria-label={`${t('markup.color')} ${c}`}
            onClick={() => applyStyle({ color: c })}
          />
        ))}
        <span className={styles.sep} />
        {WIDTHS.map((w, i) => (
          <button
            key={w}
            type="button"
            className={styles.button}
            aria-pressed={level === i}
            title={t(WIDTH_LABELS[i]!)}
            aria-label={t(WIDTH_LABELS[i]!)}
            onClick={() => applyStyle({ level: i })}
          >
            <svg viewBox="0 0 20 20" aria-hidden="true">
              <circle cx="10" cy="10" r={1.5 + i * 2.2} fill="currentColor" />
            </svg>
          </button>
        ))}
        <span className={styles.sep} />
        <button type="button" className={styles.button} title={t('markup.undo')} aria-label={t('markup.undo')} disabled={!props.canUndo} onClick={undo}>
          {icons.undo}
        </button>
        <button type="button" className={styles.button} title={t('markup.redo')} aria-label={t('markup.redo')} disabled={!props.canRedo} onClick={redo}>
          {icons.redo}
        </button>
        <button type="button" className={styles.button} title={t('markup.delete')} aria-label={t('markup.delete')} disabled={!selectedMark} onClick={remove}>
          {icons.close}
        </button>
        <span className={styles.sep} />
        <button type="button" className={styles.button} title={t('preview.crop')} aria-label={t('preview.crop')} onClick={onCrop}>
          {icons.crop}
        </button>
        <button
          type="button"
          className={styles.button}
          title={t('preview.revert')}
          aria-label={t('preview.revert')}
          disabled={isEmptyLayer(layer)}
          onClick={() => commit(emptyLayer())}
        >
          {icons.revert}
        </button>
        <span className={styles.spacer} />
        <button type="button" className={`${styles.button} ${styles.primary}`} onClick={onDone}>
          {icons.check}
          {t('markup.done')}
        </button>
      </div>
      <div className={styles.body} ref={box} onWheel={(e) => wheelZoom(e, size, area, view, zoom, onZoom)}>
        <Stage
          className={tool === 'select' ? '' : styles.draw}
          image={image}
          layer={shown}
          view={view}
          size={size}
          clip={clip}
          overlay={overlay}
          overlayKey={tick}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          onDoubleClick={(e) => {
            if (tool !== 'select') return;
            const at = localPoint(e.clientX, e.clientY);
            const hit = [...layer.marks].reverse().find((m) => m.type === 'text' && hitTest(m, toImage(view, at[0], at[1]), 2 / view.scale));
            if (hit?.type === 'text') startText([hit.x, hit.y], hit);
          }}
        />
        {text && textStyle && (
          <textarea
            className={styles.textBox}
            dir="auto"
            style={textStyle}
            value={text.value}
            autoFocus
            spellCheck={false}
            aria-label={t('markup.tool.text')}
            onChange={(e) => setText({ ...text, value: e.target.value })}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === 'Escape' || (e.key === 'Enter' && !e.shiftKey)) {
                e.preventDefault();
                commitText();
              }
            }}
          />
        )}
        {props.saving && <span className={styles.status}>{t('markup.saving')}</span>}
      </div>
    </>
  );
}
