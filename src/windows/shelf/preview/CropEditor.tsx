import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { boxHandles, dragBox, toCanvas, toImage, type Handle } from '../../../edit/draw';
import { cropRect, type Area, type EditLayer, type Point } from '../../../edit/layer';
import { ltr, t } from '../../../i18n';
import { icons } from './icons';
import styles from './Preview.module.css';
import { Stage, useFittedView } from './Stage';

type Drag =
  | { kind: 'resize'; handle: Handle }
  | { kind: 'move'; start: Point; from: Area }
  | { kind: 'new'; start: Point };

const MIN_SIDE = 8;
const NO_PAN: [number, number] = [0, 0];

/** Keeps a crop inside the picture and at least a few pixels big. */
function contain(area: Area, width: number, height: number): Area {
  const w = Math.min(width, Math.max(MIN_SIDE, area.w));
  const h = Math.min(height, Math.max(MIN_SIDE, area.h));
  return {
    x: Math.min(width - w, Math.max(0, area.x)),
    y: Math.min(height - h, Math.max(0, area.y)),
    w,
    h,
  };
}

export interface CropProps {
  image: ImageBitmap;
  layer: EditLayer;
  onApply: (crop: Area | null) => void;
  onCancel: () => void;
}

/** Crop (C): eight handles; Enter applies, Esc cancels. The pixels themselves never change. */
export function CropEditor({ image, layer, onApply, onCancel }: CropProps) {
  const { width, height } = image;
  const whole = useMemo(() => ({ x: 0, y: 0, w: width, h: height }), [width, height]);
  const { box, size, view } = useFittedView(whole, 1, NO_PAN);
  const [rect, setRect] = useState<Area>(() => (layer.crop && cropRect(layer.crop, width, height)) || { x: 0, y: 0, w: width, h: height });
  const drag = useRef<Drag | null>(null);

  const apply = useCallback(() => {
    const whole = rect.x <= 0 && rect.y <= 0 && rect.w >= width && rect.h >= height;
    onApply(whole ? null : cropRect(rect, width, height));
  }, [rect, width, height, onApply]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        apply();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onCancel();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [apply, onCancel]);

  const local = (e: { clientX: number; clientY: number }): Point => {
    const rect = box.current?.getBoundingClientRect();
    return rect ? [e.clientX - rect.left, e.clientY - rect.top] : [e.clientX, e.clientY];
  };

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const at = local(e);
    const point = toImage(view, at[0], at[1]);
    const handle = boxHandles(rect).find(([, [hx, hy]]) => {
      const [cx, cy] = toCanvas(view, hx, hy);
      return Math.abs(cx - at[0]) <= 9 && Math.abs(cy - at[1]) <= 9;
    });
    if (handle) drag.current = { kind: 'resize', handle: handle[0] };
    else if (point[0] >= rect.x && point[0] <= rect.x + rect.w && point[1] >= rect.y && point[1] <= rect.y + rect.h)
      drag.current = { kind: 'move', start: point, from: rect };
    else drag.current = { kind: 'new', start: point };
  };

  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    const at = local(e);
    const [px, py] = toImage(view, at[0], at[1]);
    const point: Point = [Math.max(0, Math.min(width, px)), Math.max(0, Math.min(height, py))];
    if (d.kind === 'resize') setRect((r) => contain(dragBox(r, d.handle, point), width, height));
    else if (d.kind === 'move') setRect(contain({ ...d.from, x: d.from.x + point[0] - d.start[0], y: d.from.y + point[1] - d.start[1] }, width, height));
    else {
      const [sx, sy] = d.start;
      setRect(contain({ x: Math.min(sx, point[0]), y: Math.min(sy, point[1]), w: Math.abs(point[0] - sx), h: Math.abs(point[1] - sy) }, width, height));
    }
  };

  const overlay = useCallback(
    (ctx: CanvasRenderingContext2D, dpr: number) => {
      const [x1, y1] = toCanvas(view, rect.x, rect.y);
      const [x2, y2] = toCanvas(view, rect.x + rect.w, rect.y + rect.h);
      const [ix1, iy1] = toCanvas(view, 0, 0);
      const [ix2, iy2] = toCanvas(view, width, height);
      ctx.save();
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      // Dim what will be cut away.
      ctx.beginPath();
      ctx.rect(ix1, iy1, ix2 - ix1, iy2 - iy1);
      ctx.rect(x1, y1, x2 - x1, y2 - y1);
      ctx.fillStyle = 'rgba(0, 0, 0, 0.55)';
      ctx.fill('evenodd');
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.strokeRect(x1, y1, x2 - x1, y2 - y1);
      // Thirds help with framing.
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.35)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      for (const f of [1 / 3, 2 / 3]) {
        ctx.moveTo(x1 + (x2 - x1) * f, y1);
        ctx.lineTo(x1 + (x2 - x1) * f, y2);
        ctx.moveTo(x1, y1 + (y2 - y1) * f);
        ctx.lineTo(x2, y1 + (y2 - y1) * f);
      }
      ctx.stroke();
      ctx.fillStyle = '#ffffff';
      for (const [, [hx, hy]] of boxHandles(rect)) {
        const [cx, cy] = toCanvas(view, hx, hy);
        ctx.fillRect(cx - 5, cy - 5, 10, 10);
      }
      ctx.restore();
    },
    [view, rect, width, height],
  );

  const endDrag = () => {
    drag.current = null;
  };
  const shown = cropRect(rect, width, height);

  return (
    <>
      <div className={styles.bar} role="toolbar" aria-label={t('preview.crop')}>
        <span className={styles.title}>
          {t('crop.hint')} · {shown ? ltr(`${shown.w}×${shown.h}`) : ''}
        </span>
        <button type="button" className={styles.button} onClick={onCancel}>
          {icons.close}
          {t('crop.cancel')}
        </button>
        <button type="button" className={`${styles.button} ${styles.primary}`} onClick={apply}>
          {icons.check}
          {t('crop.apply')}
        </button>
      </div>
      <div className={styles.body} ref={box}>
        <Stage
          image={image}
          layer={layer}
          view={view}
          size={size}
          clip={null}
          overlay={overlay}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
        />
      </div>
    </>
  );
}
