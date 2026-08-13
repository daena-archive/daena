import Konva from "konva";

export type ImageMapTool = "pan" | "brush" | "eraser";

export type ImageMapPin = {
  id: string;
  entityId: string;
  label: string;
  x: number;
  y: number;
  focused?: boolean;
};

const MIN_ZOOM = 0.2;
const MAX_ZOOM = 16;

export class ImageMapStage {
  private readonly stage: Konva.Stage;
  private readonly world: Konva.Group;
  private readonly mapLayer: Konva.Layer;
  private readonly overlayLayer: Konva.Layer;
  private readonly pinsGroup: Konva.Group;
  private baseNode: Konva.Image | null = null;
  private layerNodes = new Map<string, Konva.Image>();
  private mapWidth = 1;
  private mapHeight = 1;
  private tool: ImageMapTool = "pan";
  private brushColor = "#d5ab6c";
  private brushSize = 16;
  private activeCanvas: HTMLCanvasElement | null = null;
  private painting = false;
  private lastPoint: { x: number; y: number } | null = null;
  private picking = false;
  private spacePan = false;
  private dragged = false;

  onPaint: (() => void) | null = null;
  onPick: ((point: [number, number]) => void) | null = null;
  onOpenPin: ((entityId: string) => void) | null = null;
  onStrokeStart: (() => void) | null = null;

  constructor(host: HTMLDivElement) {
    const bounds = host.getBoundingClientRect();
    this.stage = new Konva.Stage({
      container: host,
      width: Math.max(1, bounds.width),
      height: Math.max(1, bounds.height),
    });
    this.mapLayer = new Konva.Layer({ listening: true });
    this.overlayLayer = new Konva.Layer();
    this.world = new Konva.Group({ draggable: true });
    this.pinsGroup = new Konva.Group();
    this.mapLayer.add(this.world);
    this.overlayLayer.add(this.pinsGroup);
    this.stage.add(this.mapLayer);
    this.stage.add(this.overlayLayer);
    this.bind();
  }

  destroy() {
    this.disposeListeners();
    this.stage.destroy();
  }

  resize() {
    const container = this.stage.container();
    const bounds = container.getBoundingClientRect();
    this.stage.size({ width: Math.max(1, bounds.width), height: Math.max(1, bounds.height) });
    this.mapLayer.batchDraw();
    this.overlayLayer.batchDraw();
  }

  setPicking(picking: boolean) {
    this.picking = picking;
    this.syncCursor();
  }

  setTool(tool: ImageMapTool) {
    this.tool = tool;
    this.world.draggable(this.canPan());
    this.syncCursor();
  }

  setBrush(color: string, size: number) {
    this.brushColor = color;
    this.brushSize = Math.max(1, Math.min(128, size));
  }

  setActiveCanvas(canvas: HTMLCanvasElement | null) {
    this.activeCanvas = canvas;
  }

  setBase(image: HTMLImageElement) {
    this.mapWidth = Math.max(1, image.naturalWidth);
    this.mapHeight = Math.max(1, image.naturalHeight);
    this.baseNode?.destroy();
    this.baseNode = new Konva.Image({
      image,
      x: 0,
      y: 0,
      width: this.mapWidth,
      height: this.mapHeight,
      listening: false,
    });
    this.world.add(this.baseNode);
    this.baseNode.moveToBottom();
    this.fit();
  }

  setRasterLayer(id: string, canvas: HTMLCanvasElement, options: { visible: boolean; opacity: number; order: number }) {
    let node = this.layerNodes.get(id);
    if (!node) {
      node = new Konva.Image({
        image: canvas,
        x: 0,
        y: 0,
        width: this.mapWidth,
        height: this.mapHeight,
        listening: false,
      });
      this.world.add(node);
      this.layerNodes.set(id, node);
    } else {
      node.image(canvas);
    }
    node.visible(options.visible);
    node.opacity(options.opacity);
    node.zIndex(options.order + 1);
    this.baseNode?.zIndex(0);
    this.mapLayer.batchDraw();
  }

  removeRasterLayer(id: string) {
    this.layerNodes.get(id)?.destroy();
    this.layerNodes.delete(id);
    this.mapLayer.batchDraw();
  }

  refreshLayer(id: string) {
    this.layerNodes.get(id)?.getLayer()?.batchDraw();
  }

  setPins(pins: ImageMapPin[]) {
    this.pinsGroup.destroyChildren();
    for (const pin of pins) {
      const x = pin.x * this.mapWidth;
      const y = pin.y * this.mapHeight;
      const group = new Konva.Group({ x, y, listening: true });
      const marker = new Konva.Circle({
        radius: pin.focused ? 9 : 7,
        fill: pin.focused ? "#f3d39a" : "#d5ab6c",
        stroke: "#17211d",
        strokeWidth: 2,
      });
      const label = new Konva.Label({ y: 12 });
      label.add(new Konva.Tag({ fill: "#202c27", cornerRadius: 4 }));
      label.add(
        new Konva.Text({
          text: pin.label,
          fontSize: 11,
          fontFamily: "system-ui",
          fill: "#edf2ec",
          padding: 4,
        }),
      );
      group.add(marker);
      group.add(label);
      group.on("click tap", (event) => {
        event.cancelBubble = true;
        if (this.picking) return;
        this.onOpenPin?.(pin.entityId);
      });
      this.pinsGroup.add(group);
    }
    this.syncPinsToWorld();
    this.overlayLayer.batchDraw();
  }

  fit() {
    const scale = Math.min(this.stage.width() / this.mapWidth, this.stage.height() / this.mapHeight);
    const next = Number.isFinite(scale) && scale > 0 ? scale : 1;
    this.world.scale({ x: next, y: next });
    this.world.position({
      x: (this.stage.width() - this.mapWidth * next) / 2,
      y: (this.stage.height() - this.mapHeight * next) / 2,
    });
    this.syncPinsToWorld();
    this.stage.batchDraw();
  }

  applyView(center: [number, number], zoom: number) {
    this.fit();
    const fitScale = this.world.scaleX();
    const scale = clamp(fitScale * Math.max(zoom, 0.01), fitScale * MIN_ZOOM, fitScale * MAX_ZOOM);
    this.world.scale({ x: scale, y: scale });
    this.world.position({
      x: this.stage.width() / 2 - center[0] * this.mapWidth * scale,
      y: this.stage.height() / 2 - center[1] * this.mapHeight * scale,
    });
    this.syncPinsToWorld();
    this.stage.batchDraw();
  }

  currentView(): { center: [number, number]; zoom: number } {
    const scale = this.world.scaleX();
    const fitScale = Math.min(this.stage.width() / this.mapWidth, this.stage.height() / this.mapHeight) || 1;
    return {
      center: [
        (this.stage.width() / 2 - this.world.x()) / (this.mapWidth * scale),
        (this.stage.height() / 2 - this.world.y()) / (this.mapHeight * scale),
      ],
      zoom: scale / fitScale,
    };
  }

  focusNormalized(point: [number, number]) {
    const scale = this.world.scaleX();
    this.world.position({
      x: this.stage.width() / 2 - point[0] * this.mapWidth * scale,
      y: this.stage.height() / 2 - point[1] * this.mapHeight * scale,
    });
    this.syncPinsToWorld();
    this.stage.batchDraw();
  }

  panBy(dx: number, dy: number) {
    this.world.position({ x: this.world.x() + dx, y: this.world.y() + dy });
    this.syncPinsToWorld();
    this.stage.batchDraw();
  }

  zoomAtCenter(factor: number) {
    const pointer = { x: this.stage.width() / 2, y: this.stage.height() / 2 };
    const oldScale = this.world.scaleX();
    const mouse = {
      x: (pointer.x - this.world.x()) / oldScale,
      y: (pointer.y - this.world.y()) / oldScale,
    };
    const next = clamp(oldScale * factor, MIN_ZOOM * 0.25, MAX_ZOOM * 4);
    this.world.scale({ x: next, y: next });
    this.world.position({
      x: pointer.x - mouse.x * next,
      y: pointer.y - mouse.y * next,
    });
    this.syncPinsToWorld();
    this.stage.batchDraw();
  }

  private canPan() {
    return this.tool === "pan" || this.spacePan || this.picking;
  }

  private syncCursor() {
    const container = this.stage.container();
    if (this.picking) container.style.cursor = "crosshair";
    else if (this.canPan()) container.style.cursor = "grab";
    else container.style.cursor = "crosshair";
  }

  private bind() {
    this.world.dragBoundFunc((pos) => pos);
    this.world.on("dragmove", () => this.syncPinsToWorld());
    this.world.on("dragstart", () => {
      this.dragged = true;
    });
    this.stage.on("wheel", (event) => {
      event.evt.preventDefault();
      const pointer = this.stage.getPointerPosition();
      if (!pointer) return;
      const oldScale = this.world.scaleX();
      const mouse = {
        x: (pointer.x - this.world.x()) / oldScale,
        y: (pointer.y - this.world.y()) / oldScale,
      };
      const factor = event.evt.deltaY > 0 ? 0.9 : 1.1;
      const next = clamp(oldScale * factor, MIN_ZOOM * 0.25, MAX_ZOOM * 4);
      this.world.scale({ x: next, y: next });
      this.world.position({
        x: pointer.x - mouse.x * next,
        y: pointer.y - mouse.y * next,
      });
      this.syncPinsToWorld();
      this.stage.batchDraw();
    });
    this.stage.on("mousedown touchstart", (event) => {
      if (this.picking) return;
      if (this.canPan() || event.evt.button === 1) return;
      this.beginStroke();
    });
    this.stage.on("mousemove touchmove", () => this.continueStroke());
    this.stage.on("mouseup mouseleave touchend", () => this.endStroke());
    this.stage.on("click tap", () => {
      if (!this.picking) return;
      if (this.dragged) {
        this.dragged = false;
        return;
      }
      const local = this.pointerOnMap();
      if (!local) return;
      this.onPick?.([clamp(local.x / this.mapWidth, 0, 1), clamp(local.y / this.mapHeight, 0, 1)]);
    });
    window.addEventListener("keydown", this.onKeyDown);
    window.addEventListener("keyup", this.onKeyUp);
  }

  private onKeyDown = (event: KeyboardEvent) => {
    if (isEditableTarget(event.target) || event.code !== "Space" || this.spacePan) return;
    event.preventDefault();
    this.spacePan = true;
    this.world.draggable(this.canPan());
    this.syncCursor();
  };

  private onKeyUp = (event: KeyboardEvent) => {
    if (event.code === "Space") {
      this.spacePan = false;
      this.world.draggable(this.canPan());
      this.syncCursor();
    }
  };

  private pointerOnMap() {
    const pointer = this.stage.getPointerPosition();
    if (!pointer) return null;
    const transform = this.world.getAbsoluteTransform().copy().invert();
    const local = transform.point(pointer);
    if (local.x < 0 || local.y < 0 || local.x > this.mapWidth || local.y > this.mapHeight) return null;
    return local;
  }

  private beginStroke() {
    if (this.tool === "pan" || !this.activeCanvas) return;
    const local = this.pointerOnMap();
    if (!local) return;
    this.painting = true;
    this.lastPoint = local;
    this.onStrokeStart?.();
    this.paintTo(local);
  }

  private continueStroke() {
    if (!this.painting) return;
    const local = this.pointerOnMap();
    if (!local) return;
    this.paintTo(local);
    this.lastPoint = local;
  }

  private endStroke() {
    if (!this.painting) return;
    this.painting = false;
    this.lastPoint = null;
    this.onPaint?.();
  }

  private paintTo(point: { x: number; y: number }) {
    const canvas = this.activeCanvas;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const from = this.lastPoint ?? point;
    ctx.save();
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.lineWidth = this.brushSize;
    ctx.strokeStyle = this.brushColor;
    ctx.globalCompositeOperation = this.tool === "eraser" ? "destination-out" : "source-over";
    ctx.beginPath();
    ctx.moveTo(from.x, from.y);
    ctx.lineTo(point.x, point.y);
    ctx.stroke();
    ctx.restore();
    this.mapLayer.batchDraw();
  }

  private syncPinsToWorld() {
    this.pinsGroup.position(this.world.position());
    this.pinsGroup.scale(this.world.scale());
  }

  disposeListeners() {
    window.removeEventListener("keydown", this.onKeyDown);
    window.removeEventListener("keyup", this.onKeyUp);
  }
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return Boolean(target.closest("input, textarea, select, [contenteditable=true]"));
}
