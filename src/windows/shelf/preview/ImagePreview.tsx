import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { commit as record, redo as redoStep, startHistory, undo as undoStep, type History } from '../../../edit/history';
import { cropRect, emptyLayer, isEmptyLayer, visibleArea, type Area, type EditLayer, type Point } from '../../../edit/layer';
import { ltr, t } from '../../../i18n';
import { getEdit, mediaUrl, revertEdit, saveEdit, type ShelfItem } from '../../../ipc';
import { CropEditor } from './CropEditor';
import { icons } from './icons';
import { MarkupEditor } from './MarkupEditor';
import type { PreviewMode } from './Preview';
import styles from './Preview.module.css';
import { trackSave } from './saves';
import { MIN_ZOOM, Stage, useFittedView, wheelZoom } from './Stage';

export interface ImagePreviewProps {
  item: ShelfItem;
  mode: PreviewMode;
  onMode: (mode: PreviewMode) => void;
  onClose: () => void;
}

const send = (id: number, layer: EditLayer) => trackSave(isEmptyLayer(layer) ? revertEdit(id) : saveEdit(id, layer));

/** The picture fitted to the panel; wheel zoom 100–400 %, drag to pan, double click resets. The
 * crop and marks are edited here (M, C) and saved when leaving the editor or the picture. */
export function ImagePreview({ item, mode, onMode, onClose }: ImagePreviewProps) {
  const [image, setImage] = useState<ImageBitmap | null>(null);
  const [failed, setFailed] = useState(false);
  const [history, setHistory] = useState<History | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [zoom, setZoom] = useState(MIN_ZOOM);
  const [pan, setPan] = useState<[number, number]>([0, 0]);
  const [panning, setPanning] = useState(false);
  const saved = useRef<EditLayer | null>(null);
  const latest = useRef<EditLayer | null>(null);
  const grab = useRef<{ start: Point; pan: [number, number] } | null>(null);
  const id = item.id;

  // The original is decoded once per opening; every view and edit draws from this bitmap.
  useEffect(() => {
    let alive = true;
    let bitmap: ImageBitmap | null = null;
    const img = new Image();
    const fail = () => {
      if (alive) setFailed(true);
    };
    // `onload` and `createImageBitmap` rather than `img.decode()`, which can wait for a frame that
    // a page without rendering never draws.
    img.onload = () => {
      createImageBitmap(img).then((decoded) => {
        bitmap = decoded;
        if (alive) setImage(decoded);
        else decoded.close();
      }, fail);
    };
    img.onerror = fail;
    img.src = mediaUrl('original', id);
    return () => {
      alive = false;
      bitmap?.close();
    };
  }, [id]);

  useEffect(() => {
    let alive = true;
    getEdit(id)
      .then((stored) => {
        if (!alive) return;
        const layer = stored ?? emptyLayer();
        saved.current = layer;
        latest.current = layer;
        setHistory(startHistory(layer));
      })
      .catch((err: unknown) => {
        if (alive) setError(String(err));
      });
    return () => {
      alive = false;
    };
  }, [id]);

  const layer = useMemo(() => history?.present ?? emptyLayer(), [history]);
  useEffect(() => {
    latest.current = layer;
  }, [layer]);

  const save = useCallback(
    async (next: EditLayer) => {
      if (next === saved.current) return;
      saved.current = next;
      setSaving(true);
      setError(null);
      try {
        await send(id, next);
      } catch (err) {
        setError(String(err));
      } finally {
        setSaving(false);
      }
    },
    [id],
  );

  // Leaving the picture (another card, closing the preview or the shelf) keeps the edits.
  useEffect(
    () => () => {
      const pending = latest.current;
      if (pending && pending !== saved.current) void send(id, pending);
    },
    [id],
  );

  const commit = useCallback((next: EditLayer) => setHistory((h) => (h ? record(h, next) : h)), []);
  const undo = useCallback(() => setHistory((h) => (h ? undoStep(h) : h)), []);
  const redo = useCallback(() => setHistory((h) => (h ? redoStep(h) : h)), []);
  const done = useCallback(() => {
    void save(latest.current ?? emptyLayer());
    onMode('view');
  }, [save, onMode]);
  const saveNow = useCallback(() => void save(latest.current ?? emptyLayer()), [save]);
  // Cropping returns to where it was opened from: the markup editor, or the plain preview.
  const cropReturn = useRef<PreviewMode>('view');
  useEffect(() => {
    if (mode !== 'crop') cropReturn.current = mode === 'markup' ? 'markup' : 'view';
  }, [mode]);
  const toCrop = useCallback(() => onMode('crop'), [onMode]);
  const backFromCrop = useCallback(() => onMode(cropReturn.current), [onMode]);
  const applyCrop = useCallback(
    (crop: Area | null) => {
      const next = { ...(latest.current ?? emptyLayer()), crop };
      commit(next);
      const back = cropReturn.current;
      onMode(back);
      // Without the markup editor in between, nothing else would save the crop.
      if (back === 'view') void save(next);
    },
    [commit, onMode, save],
  );
  const onZoom = useCallback((nextZoom: number, nextPan: [number, number]) => {
    setZoom(nextZoom);
    setPan(nextPan);
  }, []);

  const width = image?.width ?? item.width ?? 1;
  const height = image?.height ?? item.height ?? 1;
  const area = useMemo(() => visibleArea(layer, width, height), [layer, width, height]);
  const clip = useMemo(() => (layer.crop ? cropRect(layer.crop, width, height) : null), [layer.crop, width, height]);
  const { box, size, view } = useFittedView(area, zoom, pan);

  // A new picture, mode or crop starts fitted.
  useEffect(() => {
    setZoom(MIN_ZOOM);
    setPan([0, 0]);
  }, [id, mode, layer.crop]);

  if (failed) return <div className={styles.empty}>{t('preview.loadFailed')}</div>;

  if (image && history && mode === 'markup') {
    return (
      <MarkupEditor
        image={image}
        layer={layer}
        commit={commit}
        undo={undo}
        redo={redo}
        canUndo={history.past.length > 0}
        canRedo={history.future.length > 0}
        area={area}
        clip={clip}
        zoom={zoom}
        pan={pan}
        onZoom={onZoom}
        saving={saving}
        onSave={saveNow}
        onDone={done}
        onCrop={toCrop}
      />
    );
  }

  if (image && history && mode === 'crop') {
    return <CropEditor image={image} layer={layer} onApply={applyCrop} onCancel={backFromCrop} />;
  }

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || zoom <= MIN_ZOOM) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    grab.current = { start: [e.clientX, e.clientY], pan };
    setPanning(true);
  };
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const g = grab.current;
    if (g) setPan([g.pan[0] + e.clientX - g.start[0], g.pan[1] + e.clientY - g.start[1]]);
  };
  const endPan = () => {
    grab.current = null;
    setPanning(false);
  };

  return (
    <>
      <div className={styles.bar}>
        <span className={styles.title}>
          {t('preview.image')} · {ltr(`${Math.round(area.w)}×${Math.round(area.h)}`)}
        </span>
        <button type="button" className={styles.button} title={`${t('preview.markup')} (M)`} onClick={() => onMode('markup')}>
          {icons.pen}
          {t('preview.markup')}
        </button>
        <button type="button" className={styles.button} title={`${t('preview.crop')} (C)`} onClick={() => onMode('crop')}>
          {icons.crop}
          {t('preview.crop')}
        </button>
        {!isEmptyLayer(layer) && (
          <button
            type="button"
            className={styles.button}
            onClick={() => {
              commit(emptyLayer());
              void save(emptyLayer());
            }}
          >
            {icons.revert}
            {t('preview.revert')}
          </button>
        )}
        <span className={styles.sep} />
        <button type="button" className={styles.button} title={t('preview.close')} aria-label={t('preview.close')} onClick={onClose}>
          {icons.close}
        </button>
      </div>
      <div className={styles.body} ref={box} onWheel={(e) => wheelZoom(e, size, area, view, zoom, onZoom)}>
        {image && history && size.w > 0 && (
          <Stage
            className={zoom > MIN_ZOOM ? (panning ? styles.grabbing : styles.grab) : ''}
            image={image}
            layer={layer}
            view={view}
            size={size}
            clip={clip}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={endPan}
            onPointerCancel={endPan}
            onDoubleClick={() => onZoom(MIN_ZOOM, [0, 0])}
          />
        )}
        {image && (
          <span className={styles.status}>
            {Math.round(zoom * 100)} %{saving ? ` · ${t('markup.saving')}` : ''}
          </span>
        )}
        {error && (
          <span className={styles.status} role="alert">
            {t('markup.saveFailed', { error })}
          </span>
        )}
      </div>
    </>
  );
}
