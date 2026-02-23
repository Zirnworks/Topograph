#!/usr/bin/env python3
"""Stable Diffusion 1.5 inpainting via HuggingFace Diffusers.

Usage:
    python inpaint.py --image terrain.png --mask mask.png --prompt "volcanic crater" --output result.png

Mask: white (255) = regions to inpaint, black (0) = keep original.
Prints JSON status to stdout.
"""

import argparse
import json
import sys


def main():
    parser = argparse.ArgumentParser(description="SD 1.5 inpainting")
    parser.add_argument("--image", required=True, help="Input image path (PNG)")
    parser.add_argument("--mask", required=True, help="Mask image path (white = inpaint)")
    parser.add_argument("--prompt", required=True, help="Text prompt")
    parser.add_argument("--output", required=True, help="Output PNG path")
    parser.add_argument("--steps", type=int, default=30, help="Inference steps")
    parser.add_argument("--guidance", type=float, default=7.5, help="Guidance scale")
    parser.add_argument("--seed", type=int, default=-1, help="Random seed (-1 = random)")
    args = parser.parse_args()

    try:
        import torch
        from PIL import Image
        from diffusers import StableDiffusionInpaintPipeline

        # Device selection
        if torch.backends.mps.is_available():
            device = "mps"
        elif torch.cuda.is_available():
            device = "cuda"
        else:
            device = "cpu"

        # Load pipeline
        # MPS has multiple issues (float16 dtype mismatches, VAE black images,
        # device mismatches when VAE is on CPU). Use CPU for reliability on Mac.
        # ~60-90s on M4 Max in float32 — good enough for testing.
        model_id = "stable-diffusion-v1-5/stable-diffusion-inpainting"
        if device == "cuda":
            pipe = StableDiffusionInpaintPipeline.from_pretrained(
                model_id,
                torch_dtype=torch.float16,
                variant="fp16",
                use_safetensors=True,
            ).to("cuda")
        else:
            pipe = StableDiffusionInpaintPipeline.from_pretrained(
                model_id,
                torch_dtype=torch.float32,
                variant="fp16",
                use_safetensors=True,
            ).to("cpu")

        pipe.enable_attention_slicing()

        # Load images
        image = Image.open(args.image).convert("RGB").resize((512, 512))
        mask = Image.open(args.mask).convert("L").resize((512, 512))

        # Debug: save copies so we can inspect what arrived
        import os
        debug_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "debug")
        os.makedirs(debug_dir, exist_ok=True)
        image.save(os.path.join(debug_dir, "input_image.png"))
        mask.save(os.path.join(debug_dir, "input_mask.png"))
        # Log mask stats
        import numpy as np
        mask_arr = np.array(mask)
        print(f"MASK_DEBUG: min={mask_arr.min()}, max={mask_arr.max()}, white_pixels={np.sum(mask_arr > 128)}, total={mask_arr.size}", file=sys.stderr)

        # Seed
        generator = None
        if args.seed >= 0:
            generator = torch.Generator(device="cpu").manual_seed(args.seed)

        # Run inpainting
        with torch.no_grad():
            result = pipe(
                prompt=args.prompt,
                image=image,
                mask_image=mask,
                num_inference_steps=args.steps,
                guidance_scale=args.guidance,
                generator=generator,
            ).images[0]

        result.save(args.output)
        print(json.dumps({"success": True, "output": args.output}))

    except Exception as e:
        print(json.dumps({"success": False, "error": str(e)}))
        sys.exit(1)


if __name__ == "__main__":
    main()
