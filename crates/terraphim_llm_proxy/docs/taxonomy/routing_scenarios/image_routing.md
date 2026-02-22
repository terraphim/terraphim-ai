# Image Routing

Image routing (beta feature) handles requests involving image analysis, generation, or vision-related tasks. This routing scenario directs image-centric requests to models with visual understanding capabilities.

In claude-code-router, image routing configuration:
```json
"Router": {
  "image": "provider_name,vision_model_name"
}
```

route:: openrouter, anthropic/claude-sonnet-4.5

For models without native tool calling support, enable force mode:
```json
"config": {
  "forceUseImageAgent": true
}
```

Capabilities:
- Image analysis and description
- Visual debugging (screenshot analysis)
- UI/UX design review
- Diagram and flowchart interpretation
- Code-to-visual and visual-to-code conversions

Image routing is powered by CCR's built-in image agent for models lacking native vision support.

Use cases:
- Analyzing error screenshots
- Reviewing UI implementations
- Understanding architectural diagrams
- Processing visual documentation
- Screenshot-based debugging

synonyms:: image model, vision routing, visual analysis, multimodal routing, screenshot analysis, image agent
