#!/bin/bash

create_bundle() {
    local binary="$1"
    local app_name="$2"

    echo "Creating app bundle for $app_name..."

    # Create app bundle structure
    mkdir -p "$app_name.app/Contents/MacOS"
    mkdir -p "$app_name.app/Contents/Resources"

    # Copy binary
    cp "$binary" "$app_name.app/Contents/MacOS/$app_name"

    # Create Info.plist
    cat > "$app_name.app/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"\>
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$app_name</string>
    <key>CFBundleIdentifier</key>
    <string>ai.terraphim.$app_name</string>
    <key>CFBundleName</key>
    <string>$app_name</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

    echo "Created $app_name.app"
}

# Create app bundles
create_bundle "terraphim_server" "TerraphimServer"
create_bundle "terraphim-agent" "TerraphimTUI"

echo "App bundles created successfully!"
