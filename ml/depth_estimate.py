#!/usr/bin/env python3
"""Depth Anything V2 Small — monocular depth estimation.

Usage:
    python depth_estimate.py --input terrain.png --output depth.bin [--width 512] [--height 512]

Outputs raw float32 binary (row-major, little-endian) normalized to [0, 1].
Prints JSON status to stdout: {"success": true, "output": "path"} or {"success": false, "error": "msg"}.
"""

import argparse
import json
import sys
import numpy as np

def main():
    parser = argparse.ArgumentParser(description="Depth Anything V2 depth estimation")
    parser.add_argument("--input", required=True, help="Input image path (PNG)")
    parser.add_argument("--output", required=True, help="Output path for raw f32 binary")
    parser.add_argument("--width", type=int, default=512, help="Output width")
    parser.add_argument("--height", type=int, default=512, help="Output height")
    args = parser.parse_args()

    try:
        import torch
        from PIL import Image
        from transformers import AutoImageProcessor, AutoModelForDepthEstimation

        # Select device
        if torch.backends.mps.is_available():
            device = torch.device("mps")
        elif torch.cuda.is_available():
            device = torch.device("cuda")
        else:
            device = torch.device("cpu")

        # Load model
        model_name = "depth-anything/Depth-Anything-V2-Small-hf"
        processor = AutoImageProcessor.from_pretrained(model_name)
        model = AutoModelForDepthEstimation.from_pretrained(model_name)
        model.to(device).eval()

        # Load and process image
        image = Image.open(args.input).convert("RGB")
        inputs = processor(images=image, return_tensors="pt").to(device)

        # Inference
        with torch.no_grad():
            outputs = model(**inputs)
            predicted_depth = outputs.predicted_depth

        # Post-process: resize to target dimensions
        depth = torch.nn.functional.interpolate(
            predicted_depth.unsqueeze(1),
            size=(args.height, args.width),
            mode="bicubic",
            align_corners=False,
        ).squeeze()

        depth_np = depth.cpu().numpy().astype(np.float32)

        # Normalize to [0, 1]
        d_min = depth_np.min()
        d_max = depth_np.max()
        if d_max - d_min > 1e-6:
            depth_np = (depth_np - d_min) / (d_max - d_min)
        else:
            depth_np = np.zeros_like(depth_np)

        # Depth Anything: smaller values = closer to camera.
        # For top-down view: closer = higher terrain.
        # So we invert: high terrain should have large values in the heightmap.
        depth_np = 1.0 - depth_np

        # Write raw f32 binary (row-major, little-endian)
        with open(args.output, "wb") as f:
            f.write(depth_np.tobytes())

        print(json.dumps({"success": True, "output": args.output}))

    except Exception as e:
        print(json.dumps({"success": False, "error": str(e)}))
        sys.exit(1)


if __name__ == "__main__":
    main()
