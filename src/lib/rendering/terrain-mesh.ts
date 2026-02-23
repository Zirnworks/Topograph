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
   * Apply a PNG image as the terrain's diffuse texture.
   * The image should be in top-down orthographic projection (matching UV space).
   */
  applyTexture(pngBytes: Uint8Array): void {
    if (!this.mesh) return;

    const blob = new Blob([pngBytes], { type: "image/png" });
    const url = URL.createObjectURL(blob);

    const loader = new THREE.TextureLoader();
    loader.load(url, (tex) => {
      URL.revokeObjectURL(url);
      tex.colorSpace = THREE.SRGBColorSpace;
      tex.minFilter = THREE.LinearMipmapLinearFilter;
      tex.magFilter = THREE.LinearFilter;
      tex.wrapS = THREE.ClampToEdgeWrapping;
      tex.wrapT = THREE.ClampToEdgeWrapping;

      // Dispose old texture if any
      this.texture?.dispose();
      this.texture = tex;

      const mat = this.mesh!.material as THREE.MeshStandardMaterial;
      mat.map = tex;
      mat.color.set(0xffffff); // neutral tint so texture colors show accurately
      mat.needsUpdate = true;
    });
  }

  /**
   * Remove any applied texture and revert to the default green material.
   */
  clearTexture(): void {
    if (!this.mesh) return;
    this.texture?.dispose();
    this.texture = null;

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
    }
  }
}
