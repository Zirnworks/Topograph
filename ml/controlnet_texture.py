#!/usr/bin/env python3
"""ControlNet depth-conditioned texture generation via img2img.

Uses SD 1.5 + ControlNet depth with the captured terrain render as the
init image, so the generated texture follows the terrain's spatial layout
(valleys, ridges, channels) precisely.

Usage:
    python controlnet_texture.py --image terrain.png --depth heightmap.png \
        --mask mask.png --prompt "lush green forest" --output result.png

Mask: white (255) = regions to replace, black (0) = keep original.
Depth: grayscale heightmap used as ControlNet conditioning (white = high).
Prints JSON status to stdout.
"""

import argparse
import json
import sys


def main():
    parser = argparse.ArgumentParser(description="ControlNet texture generation")
    parser.add_argument("--image", required=True, help="Captured terrain PNG (init image + compositing)")
    parser.add_argument("--depth", required=True, help="Heightmap grayscale PNG (ControlNet conditioning)")
    parser.add_argument("--mask", required=True, help="Mask image (white = replace)")
    parser.add_argument("--prompt", required=True, help="Text prompt")
    parser.add_argument("--output", required=True, help="Output PNG path")
    parser.add_argument("--steps", type=int, default=30, help="Inference steps")
    parser.add_argument("--guidance", type=float, default=7.5, help="Guidance scale")
    parser.add_argument("--strength", type=float, default=0.65,
                        help="img2img denoising strength (0=keep init, 1=full generation)")
    parser.add_argument("--controlnet_scale", type=float, default=1.2,
                        help="ControlNet conditioning scale")
    parser.add_argument("--negative", type=str, default="", help="Negative prompt")
    parser.add_argument("--feather", type=int, default=12, help="Mask feather radius")
    parser.add_argument("--seed", type=int, default=-1, help="Random seed (-1 = random)")
    args = parser.parse_args()

    try:
        import os
        os.environ["PYTORCH_ENABLE_MPS_FALLBACK"] = "1"

        import numpy as np
        import torch
        from PIL import Image, ImageFilter
        from diffusers import ControlNetModel, StableDiffusionControlNetImg2ImgPipeline

        # Device selection (same pattern as inpaint.py)
        if torch.backends.mps.is_available():
            device = "mps"
        elif torch.cuda.is_available():
            device = "cuda"
        else:
            device = "cpu"

        print(f"Using device: {device}", file=sys.stderr)

        # Load ControlNet depth model
        controlnet = ControlNetModel.from_pretrained(
            "lllyasviel/control_v11f1p_sd15_depth",
            torch_dtype=torch.float16 if device == "cuda" else torch.float32,
        )

        # Load SD 1.5 img2img pipeline with ControlNet
        if device == "cuda":
            pipe = StableDiffusionControlNetImg2ImgPipeline.from_pretrained(
                "stable-diffusion-v1-5/stable-diffusion-v1-5",
                controlnet=controlnet,
                torch_dtype=torch.float16,
                variant="fp16",
                use_safetensors=True,
            ).to("cuda")
        else:
            pipe = StableDiffusionControlNetImg2ImgPipeline.from_pretrained(
                "stable-diffusion-v1-5/stable-diffusion-v1-5",
                controlnet=controlnet,
                torch_dtype=torch.float32,
                use_safetensors=True,
            ).to(device)

        pipe.enable_attention_slicing()

        # Load images
        image = Image.open(args.image).convert("RGB").resize((512, 512))
        depth = Image.open(args.depth).convert("RGB").resize((512, 512))
        mask = Image.open(args.mask).convert("L").resize((512, 512))

        # Debug output
        debug_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "debug")
        os.makedirs(debug_dir, exist_ok=True)
        image.save(os.path.join(debug_dir, "cn_input_image.png"))
        depth.save(os.path.join(debug_dir, "cn_depth_image.png"))
        mask.save(os.path.join(debug_dir, "cn_input_mask.png"))

        mask_arr = np.array(mask)
        print(f"MASK_DEBUG: min={mask_arr.min()}, max={mask_arr.max()}, "
              f"white_pixels={np.sum(mask_arr > 128)}, total={mask_arr.size}", file=sys.stderr)

        # Seed
        generator = None
        if args.seed >= 0:
            generator = torch.Generator(device="cpu").manual_seed(args.seed)

        # Build prompt for terrain texture
        prompt = (f"{args.prompt}, flat top-down orthographic satellite view, "
                  "terrain texture map, no shadows, no lighting, no depth, "
                  "uniform flat illumination")
        negative = (args.negative or
                    "3d render, lighting, shadows, highlights, shading, depth, "
                    "perspective, side view, horizon, volumetric, dramatic lighting, "
                    "sun, cartoon, drawing, text, watermark")

        print(f"Prompt: {prompt}", file=sys.stderr)
        print(f"Negative: {negative}", file=sys.stderr)
        print(f"ControlNet scale: {args.controlnet_scale}", file=sys.stderr)
        print(f"img2img strength: {args.strength}", file=sys.stderr)

        # Generate texture using img2img (terrain render as init) + ControlNet depth
        # The init image provides spatial structure (valleys, ridges in correct positions)
        # ControlNet depth reinforces the topology
        # The prompt guides the style/theme
        with torch.no_grad():
            generated = pipe(
                prompt=prompt,
                negative_prompt=negative,
                image=image,             # Init image: captured terrain render
                control_image=depth,     # ControlNet conditioning: heightmap as depth
                strength=args.strength,  # How much to deviate from init image
                num_inference_steps=args.steps,
                guidance_scale=args.guidance,
                controlnet_conditioning_scale=args.controlnet_scale,
                generator=generator,
            ).images[0]

        generated.save(os.path.join(debug_dir, "cn_generated_raw.png"))

        # Composite: blend generated texture into captured image using feathered mask
        gen_arr = np.array(generated).astype(np.float32)
        orig_arr = np.array(image).astype(np.float32)

        mask_feathered = mask.filter(ImageFilter.GaussianBlur(radius=args.feather))
        alpha = np.array(mask_feathered).astype(np.float32) / 255.0

        composite = orig_arr * (1 - alpha[:, :, None]) + gen_arr * alpha[:, :, None]
        result = Image.fromarray(composite.clip(0, 255).astype(np.uint8))

        result.save(args.output)
        result.save(os.path.join(debug_dir, "cn_output_result.png"))
        print(json.dumps({"success": True, "output": args.output}))

    except Exception as e:
        import traceback
        traceback.print_exc(file=sys.stderr)
        print(json.dumps({"success": False, "error": str(e)}))
        sys.exit(1)


if __name__ == "__main__":
    main()
