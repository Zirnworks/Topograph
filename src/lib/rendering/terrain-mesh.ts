import * as THREE from "three";
import type { HeightmapData, HeightmapRegion } from "../types";

export class TerrainRenderer {
  private mesh: THREE.Mesh | null = null;
  private geometry: THREE.BufferGeometry | null = null;
  private positionAttr: THREE.Float32BufferAttribute | null = null;
  private normalAttr: THREE.Float32BufferAttribute | null = null;
  private hmWidth = 0;
  private hmHeight = 0;
  private heightScale = 0.3;
  private texture: THREE.Texture | null = null;
  /** Persistent canvas that accumulates AI texture edits */
  private textureCanvas: OffscreenCanvas | null = null;
  private textureCtx: OffscreenCanvasRenderingContext2D | null = null;

  getMesh(): THREE.Mesh | null {
    return this.mesh;
  }

  getHeightScale(): number {
    return this.heightScale;
  }

  getDimensions(): { width: number; height: number } {
    return { width: this.hmWidth, height: this.hmHeight };
  }

  buildFull(data: HeightmapData, scene: THREE.Scene): void {
    this.dispose(scene);
    this.hmWidth = data.width;
    this.hmHeight = data.height;

    const count = data.width * data.height;
    const positions = new Float32Array(count * 3);
    const normals = new Float32Array(count * 3);
    const uvs = new Float32Array(count * 2);

    // Vertex positions: x in [-0.5, 0.5], z in [-0.5, 0.5], y = height * scale
    for (let gy = 0; gy < data.height; gy++) {
      for (let gx = 0; gx < data.width; gx++) {
        const i = gy * data.width + gx;
        const vi = i * 3;
        positions[vi] = gx / (data.width - 1) - 0.5;
        positions[vi + 1] = data.data[i] * this.heightScale;
        positions[vi + 2] = gy / (data.height - 1) - 0.5;
        uvs[i * 2] = gx / (data.width - 1);
        uvs[i * 2 + 1] = gy / (data.height - 1);
      }
    }

    // Index buffer: 2 triangles per quad
    const quads = (data.width - 1) * (data.height - 1);
    const indices = new Uint32Array(quads * 6);
    let idx = 0;
    for (let gy = 0; gy < data.height - 1; gy++) {
      for (let gx = 0; gx < data.width - 1; gx++) {
        const tl = gy * data.width + gx;
        const tr = tl + 1;
        const bl = (gy + 1) * data.width + gx;
        const br = bl + 1;
        indices[idx++] = tl;
        indices[idx++] = bl;
        indices[idx++] = tr;
        indices[idx++] = tr;
        indices[idx++] = bl;
        indices[idx++] = br;
      }
    }

    this.geometry = new THREE.BufferGeometry();
    this.positionAttr = new THREE.Float32BufferAttribute(positions, 3);
    this.normalAttr = new THREE.Float32BufferAttribute(normals, 3);
    this.geometry.setAttribute("position", this.positionAttr);
    this.geometry.setAttribute("normal", this.normalAttr);
    this.geometry.setAttribute("uv", new THREE.Float32BufferAttribute(uvs, 2));
    this.geometry.setIndex(new THREE.Uint32BufferAttribute(indices, 1));

    this.computeNormals(0, 0, data.width, data.height);

    const material = new THREE.MeshStandardMaterial({
      color: 0x8fbc8f,
      roughness: 0.85,
      metalness: 0.05,
      flatShading: false,
      side: THREE.DoubleSide,
    });

    this.mesh = new THREE.Mesh(this.geometry, material);
    scene.add(this.mesh);

    // Re-apply persistent texture canvas if it exists
    if (this.textureCanvas) {
      this.updateTextureFromCanvas();
    }
  }

  updateRegion(region: HeightmapRegion): void {
    if (!this.positionAttr || !this.normalAttr) return;
    const positions = this.positionAttr.array as Float32Array;

    for (let ry = 0; ry < region.h; ry++) {
      for (let rx = 0; rx < region.w; rx++) {
        const gx = region.x + rx;
        const gy = region.y + ry;
        const gi = gy * this.hmWidth + gx;
        const ri = ry * region.w + rx;
        positions[gi * 3 + 1] = region.data[ri] * this.heightScale;
      }
    }

    this.positionAttr.needsUpdate = true;

    // Recompute normals in expanded region (1-cell border for correct normals at edges)
    const nx = Math.max(0, region.x - 1);
    const ny = Math.max(0, region.y - 1);
    const nw = Math.min(this.hmWidth, region.x + region.w + 1) - nx;
    const nh = Math.min(this.hmHeight, region.y + region.h + 1) - ny;
    this.computeNormals(nx, ny, nw, nh);
    this.normalAttr.needsUpdate = true;
  }

  /**
   * Compute vertex normals for a rectangular sub-region using central differences.
   * Each vertex normal is computed from the gradient of its neighboring heights.
   */
  private computeNormals(
    rx: number,
    ry: number,
    rw: number,
    rh: number
  ): void {
    if (!this.positionAttr || !this.normalAttr) return;
    const positions = this.positionAttr.array as Float32Array;
    const normals = this.normalAttr.array as Float32Array;

    const cellW = 1.0 / (this.hmWidth - 1);
    const cellH = 1.0 / (this.hmHeight - 1);

    for (let ly = 0; ly < rh; ly++) {
      for (let lx = 0; lx < rw; lx++) {
        const gx = rx + lx;
        const gy = ry + ly;
        const gi = gy * this.hmWidth + gx;
        const vi = gi * 3;

        // Central differences for gradient
        const hL =
          gx > 0
            ? positions[(gi - 1) * 3 + 1]
            : positions[vi + 1];
        const hR =
          gx < this.hmWidth - 1
            ? positions[(gi + 1) * 3 + 1]
            : positions[vi + 1];
        const hD =
          gy > 0
            ? positions[(gi - this.hmWidth) * 3 + 1]
            : positions[vi + 1];
        const hU =
          gy < this.hmHeight - 1
            ? positions[(gi + this.hmWidth) * 3 + 1]
            : positions[vi + 1];

        // Normal from gradient: n = normalize(-dh/dx, 1, -dh/dz)
        const dhdx = (hR - hL) / (2 * cellW);
        const dhdz = (hU - hD) / (2 * cellH);

        const nx = -dhdx;
        const ny = 1.0;
        const nz = -dhdz;
        const len = Math.sqrt(nx * nx + ny * ny + nz * nz);

        normals[vi] = nx / len;
        normals[vi + 1] = ny / len;
        normals[vi + 2] = nz / len;
      }
    }
  }

  /**
   * Composite a PNG image onto the terrain's persistent texture canvas,
   * using a mask to control where new pixels appear.
   * Accumulates across multiple AI edits — previous edits are preserved
   * outside the current mask.
   *
   * On first call, the entire result image initializes the canvas (so
   * non-edited areas show the terrain capture, not black).
   * On subsequent calls, only the masked region is updated.
   */
  compositeTexture(pngBytes: Uint8Array, maskBytes: Uint8Array): Promise<void> {
    if (!this.mesh) return Promise.resolve();

    return new Promise((resolve) => {
      const resultBlob = new Blob([pngBytes], { type: "image/png" });
      const maskBlob = new Blob([maskBytes], { type: "image/png" });
      const resultUrl = URL.createObjectURL(resultBlob);
      const maskUrl = URL.createObjectURL(maskBlob);

      const resultImg = new Image();
      const maskImg = new Image();
      let loaded = 0;

      const onBothLoaded = () => {
        URL.revokeObjectURL(resultUrl);
        URL.revokeObjectURL(maskUrl);

        const size = 512;
        const isFirstEdit = !this.textureCanvas;

        // Initialize persistent canvas on first use
        if (!this.textureCanvas) {
          this.textureCanvas = new OffscreenCanvas(size, size);
          this.textureCtx = this.textureCanvas.getContext("2d")!;
        }
        const ctx = this.textureCtx!;

        if (isFirstEdit) {
          // First AI edit: paint the ENTIRE result image as the base.
          // The result already contains the terrain capture outside the mask
          // and the AI content inside, with feathered blending.
          ctx.drawImage(resultImg, 0, 0, size, size);
        } else {
          // Subsequent edits: composite using feathered mask

          // Get result and mask pixel data via temp canvas
          const tmpCanvas = new OffscreenCanvas(size, size);
          const tmpCtx = tmpCanvas.getContext("2d")!;
          tmpCtx.drawImage(resultImg, 0, 0, size, size);
          const resultData = tmpCtx.getImageData(0, 0, size, size);

          tmpCtx.clearRect(0, 0, size, size);
          tmpCtx.drawImage(maskImg, 0, 0, size, size);
          const rawMask = tmpCtx.getImageData(0, 0, size, size);

          // Feather the mask with a box blur to smooth transitions
          const feathered = this.featherMask(rawMask.data, size, size, 16);

          // Get existing canvas pixels
          const existingData = ctx.getImageData(0, 0, size, size);
          const existing = existingData.data;
          const result = resultData.data;

          for (let i = 0; i < size * size; i++) {
            const pi = i * 4;
            const alpha = feathered[i];

            if (alpha > 0.01) {
              existing[pi] = Math.round(existing[pi] * (1 - alpha) + result[pi] * alpha);
              existing[pi + 1] = Math.round(existing[pi + 1] * (1 - alpha) + result[pi + 1] * alpha);
              existing[pi + 2] = Math.round(existing[pi + 2] * (1 - alpha) + result[pi + 2] * alpha);
              existing[pi + 3] = 255;
            }
          }
          ctx.putImageData(existingData, 0, 0);
        }

        this.updateTextureFromCanvas();
        resolve();
      };

      resultImg.onload = () => { if (++loaded === 2) onBothLoaded(); };
      maskImg.onload = () => { if (++loaded === 2) onBothLoaded(); };
      resultImg.src = resultUrl;
      maskImg.src = maskUrl;
    });
  }

  /**
   * Box-blur a mask (from RGBA pixel data, using R channel) to feather edges.
   * Returns a float array [0..1] per pixel.
   */
  private featherMask(maskRGBA: Uint8ClampedArray, w: number, h: number, radius: number): Float32Array {
    // Extract R channel as float
    const src = new Float32Array(w * h);
    for (let i = 0; i < w * h; i++) {
      src[i] = maskRGBA[i * 4] / 255;
    }

    // Two-pass separable box blur
    const tmp = new Float32Array(w * h);
    const out = new Float32Array(w * h);

    // Horizontal pass
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        let sum = 0, count = 0;
        for (let dx = -radius; dx <= radius; dx++) {
          const nx = x + dx;
          if (nx >= 0 && nx < w) {
            sum += src[y * w + nx];
            count++;
          }
        }
        tmp[y * w + x] = sum / count;
      }
    }

    // Vertical pass
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        let sum = 0, count = 0;
        for (let dy = -radius; dy <= radius; dy++) {
          const ny = y + dy;
          if (ny >= 0 && ny < h) {
            sum += tmp[ny * w + x];
            count++;
          }
        }
        out[y * w + x] = sum / count;
      }
    }

    return out;
  }

  /** Update the Three.js texture from the persistent canvas. */
  private updateTextureFromCanvas(): void {
    if (!this.mesh || !this.textureCanvas) return;

    this.texture?.dispose();

    this.texture = new THREE.CanvasTexture(this.textureCanvas);
    this.texture.colorSpace = THREE.SRGBColorSpace;
    this.texture.minFilter = THREE.LinearMipmapLinearFilter;
    this.texture.magFilter = THREE.LinearFilter;
    this.texture.wrapS = THREE.ClampToEdgeWrapping;
    this.texture.wrapT = THREE.ClampToEdgeWrapping;
    this.texture.needsUpdate = true;

    const mat = this.mesh.material as THREE.MeshStandardMaterial;
    mat.map = this.texture;
    mat.color.set(0xffffff);
    mat.needsUpdate = true;
  }

  /**
   * Remove any applied texture and revert to the default green material.
   */
  clearTexture(): void {
    if (!this.mesh) return;
    this.texture?.dispose();
    this.texture = null;
    this.textureCanvas = null;
    this.textureCtx = null;

    const mat = this.mesh.material as THREE.MeshStandardMaterial;
    mat.map = null;
    mat.color.set(0x8fbc8f);
    mat.needsUpdate = true;
  }

  hasTexture(): boolean {
    return this.texture !== null;
  }

  dispose(scene: THREE.Scene): void {
    if (this.mesh) {
      scene.remove(this.mesh);
      this.geometry?.dispose();
      this.texture?.dispose();
      (this.mesh.material as THREE.Material).dispose();
      this.mesh = null;
      this.geometry = null;
      this.positionAttr = null;
      this.normalAttr = null;
      this.texture = null;
      // Note: textureCanvas is NOT cleared here — it persists across rebuilds
    }
  }
}
