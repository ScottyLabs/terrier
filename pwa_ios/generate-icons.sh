#!/bin/bash

# App Icon Generator for TerrierPWA
# This script generates all required app icon sizes from a source image
# 
# Usage: ./generate-icons.sh source-icon.png
#
# The source image should be at least 1024x1024 pixels

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <source-image.png>"
    echo "The source image should be at least 1024x1024 pixels"
    exit 1
fi

SOURCE_IMAGE="$1"
OUTPUT_DIR="TerrierPWA/Assets.xcassets/AppIcon.appiconset"

if [ ! -f "$SOURCE_IMAGE" ]; then
    echo "Error: Source image '$SOURCE_IMAGE' not found"
    exit 1
fi

# Check if sips is available (macOS built-in)
if ! command -v sips &> /dev/null; then
    echo "Error: sips command not found. This script requires macOS."
    exit 1
fi

echo "Generating app icons from $SOURCE_IMAGE..."

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Generate 1024x1024 app icon (required for App Store)
echo "Generating 1024x1024 icon..."
sips -z 1024 1024 "$SOURCE_IMAGE" --out "$OUTPUT_DIR/appicon-1024.png"

echo ""
echo "✅ App icon generated successfully!"
echo ""
echo "The following icons were created in $OUTPUT_DIR:"
echo "  - appicon-1024.png (1024x1024)"
echo ""
echo "Note: For dark mode and tinted variants, create separate source images and run:"
echo "  cp your-dark-icon.png $OUTPUT_DIR/appicon-1024-dark.png"
echo "  cp your-tinted-icon.png $OUTPUT_DIR/appicon-1024-tinted.png"
echo ""
echo "For the launch screen logo, add images to:"
echo "  TerrierPWA/Assets.xcassets/LaunchLogo.imageset/"
