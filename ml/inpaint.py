#!/usr/bin/env python3
"""Terrain-aware image generation via Stable Diffusion.

Uses txt2img to generate prompt-following imagery, then composites it into
the masked region of the original terrain image with feathered blending.

Usage:
    python inpaint.py --image terrain.png --mask mask.png --prompt "volcanic crater" --output result.png

Mask: white (255) = regions to replace, black (0) = keep original.
Prints JSON status to stdout.
"""

import argparse
import json
import sys


def main():
    parser = argparse.ArgumentParser(description="SD terrain generation + compositing")
    parser.add_argument("--image", required=True, help="Input image path (PNG)")
    parser.add_argument("--mask", required=True, help="Mask image path (white = replace)")
    parser.add_argument("--prompt", required=True, help="Text prompt")
    parser.add_argument("--output", required=True, help="Output PNG path")
    parser.add_argument("--steps", type=int, default=30, help="Inference steps")
    parser.add_argument("--guidance", type=float, default=10.0, help="Guidance scale")
    parser.add_argument("--negative", type=str, default="", help="Negative prompt")
    parser.add_argument("--feather", type=int, default=12, help="Mask feather radius in pixels")
    parser.add_argument("--seed", type=int, default=-1, help="Random seed (-1 = random)")
    args = parser.parse_args()

    try:
        import os
        os.environ["PYTORCH_ENABLE_MPS_FALLBACK"] = "1"

        import numpy as np
        import torch
        from PIL import Image, ImageFilter
        from diffusers import StableDiffusionXLPipeline

        # Device selection: MPS float32, CUDA float16, CPU float32
        if torch.backends.mps.is_available():
            device = "mps"
        elif torch.cuda.is_available():
            device = "cuda"
        else:
            device = "cpu"

        print(f"Using device: {device}", file=sys.stderr)

        # SDXL: much better prompt adherence than SD 1.5
        model_id = "stabilityai/stable-diffusion-xl-base-1.0"
        if device == "cuda":
            pipe = StableDiffusionXLPipeline.from_pretrained(
                model_id,
                torch_dtype=torch.float16,
                variant="fp16",
                use_safetensors=True,
            ).to("cuda")
        else:
            # MPS + CPU: must use float32 (float16 has dtype mismatches on MPS)
            pipe = StableDiffusionXLPipeline.from_pretrained(
                model_id,
                torch_dtype=torch.float32,
                variant="fp16",
                use_safetensors=True,
            ).to(device)

        pipe.enable_attention_slicing()

        # Load images
        image = Image.open(args.image).convert("RGB").resize((512, 512))
        mask = Image.open(args.mask).convert("L").resize((512, 512))

        # Debug: save copies and log mask stats
        debug_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "debug")
        os.makedirs(debug_dir, exist_ok=True)
        image.save(os.path.join(debug_dir, "input_image.png"))
        mask.save(os.path.join(debug_dir, "input_mask.png"))
        mask_arr = np.array(mask)
        print(f"MASK_DEBUG: min={mask_arr.min()}, max={mask_arr.max()}, "
              f"white_pixels={np.sum(mask_arr > 128)}, total={mask_arr.size}", file=sys.stderr)

        # Seed
        generator = None
        if args.seed >= 0:
            generator = torch.Generator(device="cpu").manual_seed(args.seed)

        # Build prompt: flat top-down texture map — no 3D lighting or perspective
        prompt = f"{args.prompt}, flat top-down orthographic satellite view, terrain texture map, no shadows, no lighting, no depth, uniform flat illumination"
        negative = args.negative or "3d render, lighting, shadows, highlights, shading, depth, perspective, side view, horizon, volumetric, dramatic lighting, sun, cartoon, drawing, text, watermark"
        print(f"Prompt: {prompt}", file=sys.stderr)
        print(f"Negative: {negative}", file=sys.stderr)

        # Generate at SDXL native resolution (1024x1024), then downscale
        with torch.no_grad():
            generated = pipe(
                prompt=prompt,
                negative_prompt=negative,
                num_inference_steps=args.steps,
                guidance_scale=args.guidance,
                width=1024,
                height=1024,
                generator=generator,
            ).images[0]

        generated = generated.resize((512, 512), Image.LANCZOS)
        generated.save(os.path.join(debug_dir, "generated_raw.png"))

        # Composite: blend generated image into masked area with feathered edges
        gen_arr = np.array(generated).astype(np.float32)
        orig_arr = np.array(image).astype(np.float32)

        # Feather the mask for smooth blending
        mask_feathered = mask.filter(ImageFilter.GaussianBlur(radius=args.feather))
        alpha = np.array(mask_feathered).astype(np.float32) / 255.0

        composite = orig_arr * (1 - alpha[:, :, None]) + gen_arr * alpha[:, :, None]
        result = Image.fromarray(composite.clip(0, 255).astype(np.uint8))

        result.save(args.output)
        result.save(os.path.join(debug_dir, "output_result.png"))
        print(json.dumps({"success": True, "output": args.output}))

    except Exception as e:
        import traceback
        traceback.print_exc(file=sys.stderr)
        print(json.dumps({"success": False, "error": str(e)}))
        sys.exit(1)


if __name__ == "__main__":
    main()
