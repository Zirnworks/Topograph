import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

export class SceneManager {
  public scene: THREE.Scene;
  public camera: THREE.PerspectiveCamera;
  public renderer: THREE.WebGLRenderer;
  public controls: OrbitControls;
  private animationId = 0;

  constructor(container: HTMLElement) {
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x1a1a2e);

    const rect = container.getBoundingClientRect();
    this.camera = new THREE.PerspectiveCamera(
      50,
      rect.width / rect.height,
      0.001,
      10
    );
    this.camera.position.set(0.6, 0.5, 0.6);
    this.camera.lookAt(0, 0, 0);

    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setSize(rect.width, rect.height);
    this.renderer.setPixelRatio(window.devicePixelRatio);
    container.appendChild(this.renderer.domElement);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.1;
    this.controls.target.set(0, 0, 0);

    // Right-click = orbit, middle-click = pan, left-click = none (reserved for sculpting)
    this.controls.mouseButtons = {
      LEFT: null as unknown as THREE.MOUSE,
      MIDDLE: THREE.MOUSE.PAN,
      RIGHT: THREE.MOUSE.ROTATE,
    };

    // Lighting
    const ambientLight = new THREE.AmbientLight(0x404040, 0.6);
    this.scene.add(ambientLight);

    const dirLight = new THREE.DirectionalLight(0xffffff, 1.0);
    dirLight.position.set(0.5, 1.0, 0.3);
    this.scene.add(dirLight);
  }

  resize(width: number, height: number) {
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height);
  }

  start() {
    const animate = () => {
      this.animationId = requestAnimationFrame(animate);
      this.controls.update();
      this.renderer.render(this.scene, this.camera);
    };
    animate();
  }

  stop() {
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = 0;
    }
  }

  /**
   * Render the scene from a top-down orthographic camera and return PNG bytes.
   * The ortho camera spans [-0.5, 0.5] in x/z so each pixel maps 1:1 to a heightmap cell.
   */
  captureOrthographic(size: number = 512): Uint8Array {
    const orthoCamera = new THREE.OrthographicCamera(
      -0.5, 0.5,   // left, right
      0.5, -0.5,    // top, bottom (flipped so +z = down in image)
      0.001, 10
    );
    orthoCamera.position.set(0, 5, 0);
    orthoCamera.lookAt(0, 0, 0);

    // Add bright overhead lighting for the capture (so the image isn't dark)
    const captureLight = new THREE.DirectionalLight(0xffffff, 2.5);
    captureLight.position.set(0, 10, 0); // straight down
    this.scene.add(captureLight);

    const captureFill = new THREE.AmbientLight(0xffffff, 1.0);
    this.scene.add(captureFill);

    // Temporarily change background to a neutral tone for better AI input
    const origBackground = this.scene.background;
    this.scene.background = new THREE.Color(0x404040);

    const renderTarget = new THREE.WebGLRenderTarget(size, size, {
      format: THREE.RGBAFormat,
      type: THREE.UnsignedByteType,
    });

    // Render to offscreen target
    this.renderer.setRenderTarget(renderTarget);
    this.renderer.render(this.scene, orthoCamera);
    this.renderer.setRenderTarget(null);

    // Clean up temporary lights and restore background
    this.scene.remove(captureLight);
    this.scene.remove(captureFill);
    captureLight.dispose();
    captureFill.dispose();
    this.scene.background = origBackground;

    // Read pixels
    const pixels = new Uint8Array(size * size * 4);
    this.renderer.readRenderTargetPixels(renderTarget, 0, 0, size, size, pixels);
    renderTarget.dispose();

    // WebGL reads bottom-to-top — flip vertically
    const flipped = new Uint8Array(size * size * 4);
    const rowBytes = size * 4;
    for (let y = 0; y < size; y++) {
      const srcOffset = (size - 1 - y) * rowBytes;
      const dstOffset = y * rowBytes;
      flipped.set(pixels.subarray(srcOffset, srcOffset + rowBytes), dstOffset);
    }

    // Encode as PNG via offscreen canvas
    const canvas = document.createElement("canvas");
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext("2d")!;
    const imageData = new ImageData(new Uint8ClampedArray(flipped.buffer), size, size);
    ctx.putImageData(imageData, 0, 0);

    // Convert to PNG blob synchronously via toDataURL
    const dataUrl = canvas.toDataURL("image/png");
    const base64 = dataUrl.split(",")[1];
    const binaryString = atob(base64);
    const pngBytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      pngBytes[i] = binaryString.charCodeAt(i);
    }
    return pngBytes;
  }

  dispose() {
    this.stop();
    this.renderer.dispose();
    this.controls.dispose();
  }
}
