#!/bin/bash

set -e

VIDEO_DIR="website/static/video"
BACKUP_DIR="/tmp/backup_videos_$(date +%s)"

echo "=== Video Compression for Cloudflare Pages (25MB limit) ==="
echo "Backup directory: $BACKUP_DIR"

# Create backup
mkdir -p "$BACKUP_DIR"
cp "$VIDEO_DIR"/* "$BACKUP_DIR"/ 2>/dev/null || true

cd "$VIDEO_DIR"

compress_video() {
    local input="$1"
    local output="${input%.*}_compressed.mp4"

    if [[ ! -f "$input" ]]; then
        echo "File not found: $input"
        return 1
    fi

    local file_size=$(stat -f%z "$input")
    local file_size_mb=$((file_size / 1024 / 1024))

    echo ""
    echo "Processing: $input (${file_size_mb}MB)"

    if [[ $file_size_mb -le 25 ]]; then
        echo "✅ Already under 25MB limit, skipping"
        return 0
    fi

    echo "Compressing to: $output..."

    # Use H.265 for better compression, fallback to H.264
    if ffmpeg -y -i "$input" -c:v libx265 -preset medium -crf 28 -c:a aac -b:a 128k "$output" 2>/dev/null; then
        echo "✅ H.265 compression successful"
        # Test file size
        local compressed_size=$(stat -f%z "$output")
        local compressed_mb=$((compressed_size / 1024 / 1024))

        if [[ $compressed_mb -gt 25 ]]; then
            echo "⚠️  H.265 still too large (${compressed_mb}MB), trying H.264..."
            ffmpeg -y -i "$input" -c:v libx264 -preset medium -crf 28 -c:a aac -b:a 128k "${output%.*}_h264.mp4" 2>/dev/null
            local h264_size=$(stat -f%z "${output%.*}_h264.mp4")
            local h264_mb=$((h264_size / 1024 / 1024))

            if [[ $h264_mb -le 25 ]]; then
                echo "✅ H.264 compression successful (${h264_mb}MB)"
                mv "${output%.*}_h264.mp4" "$output"
            else
                echo "❌ Both compression methods failed, keeping larger version"
                rm "${output%.*}_h264.mp4" 2>/dev/null || true
            fi
        fi
    else
        echo "❌ H.265 compression failed, trying H.264..."
        if ffmpeg -y -i "$input" -c:v libx264 -preset medium -crf 28 -c:a aac -b:a 128k "${output%.*}_h264.mp4" 2>/dev/null; then
            echo "✅ H.264 compression successful"
            mv "${output%.*}_h264.mp4" "$output"
        else
            echo "❌ Both compression methods failed"
            return 1
        fi
    fi

    # Replace original with compressed if successful and smaller
    if [[ -f "$output" ]]; then
        local final_size=$(stat -f%z "$output")
        local final_mb=$((final_size / 1024 / 1024))

        if [[ $final_mb -lt $file_size_mb ]]; then
            echo "✅ Size reduction: ${file_size_mb}MB → ${final_mb}MB"
            mv "$output" "$input"
        else
            echo "❌ Compressed version is larger, keeping original"
            rm "$output" 2>/dev/null || true
        fi
    fi
}

# Process large files
echo "Starting compression of large video files..."

# Files over 25MB
compress_video "pitch_explainer1.mp4"          # 39MB
compress_video "pitch_explainer_0.1.mp4"        # 40MB
compress_video "demo_recording_project_manager.mov" # 24MB (close to limit)

echo ""
echo "=== Compression Summary ==="
echo "Final file sizes:"
ls -la | grep -E "\.(mov|mp4|gif)$" | while read -r line; do
    filename=$(echo "$line" | awk '{print $9}')
    if [[ -f "$filename" ]]; then
        size=$(echo "$line" | awk '{print $5}')
        size_mb=$((size / 1024 / 1024))
        printf "%-45s %6dMB\n" "$filename" "$size_mb"
    fi
done

echo ""
echo "=== Compliance Check ==="
for file in *.mov *.mp4 *.gif; do
    if [[ -f "$file" ]]; then
        size=$(stat -f%z "$file")
        size_mb=$((size / 1024 / 1024))
        if [[ $size_mb -le 25 ]]; then
            printf "✅ %-45s %6dMB (OK)\n" "$file" "$size_mb"
        else
            printf "❌ %-45s %6dMB (OVER LIMIT)\n" "$file" "$size_mb"
        fi
    fi
done

echo ""
echo "Backup saved to: $BACKUP_DIR"
echo "Compression completed!"