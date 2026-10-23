# Cloudflare Pages File Size Limitation

## Issue Identified

Cloudflare Pages has a **25 MiB** file size limit per individual file. Some video files in `website/static/video/` exceed this limit:

- `pitch_explainer1.mp4`: 38.7 MiB ❌
- `pitch_explainer_0.1.mp4`: 39.6 MiB ❌
- `demo_recording_project_manager.mov`: 25.09 MiB ❌

## Solutions

### Option 1: Video Compression
- Convert videos to optimized formats
- Reduce resolution/bitrate for web
- Use modern codecs (H.265/AV1)

### Option 2: Video Hosting Platform
- Host large videos on dedicated video platform
- Embed videos from YouTube/Vimeo
- Use Cloudflare R2 for large media storage

### Option 3: External CDN
- Use Cloudflare R2 with Workers for video streaming
- Serve videos via CDN with proper streaming

## Current Status

For **Phase 1 testing**, videos were temporarily moved during deployment. Preview deployment successful at:
- **Preview**: https://preview.terraphim-ai.pages.dev

The website deployment infrastructure is working correctly. Video optimization will be addressed in Phase 2.