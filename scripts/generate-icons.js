import sharp from 'sharp';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const sourceIcon = path.join(__dirname, '../src-tauri/icon-source.png');
const iconsDir = path.join(__dirname, '../src-tauri/icons');

// Create a square 1024x1024 icon with padding
async function generateSquareIcon() {
  // Read the source image metadata
  const metadata = await sharp(sourceIcon).metadata();

  // Calculate size to make it square (use the larger dimension)
  const size = Math.max(metadata.width, metadata.height);

  // Create a square image with the original image centered
  await sharp(sourceIcon)
    .resize(size, size, { fit: 'contain', background: { r: 0, g: 122, b: 255, alpha: 0 } })
    .resize(1024, 1024, { fit: 'contain', background: { r: 59, g: 130, b: 246, alpha: 1 } })
    .toFile(path.join(__dirname, '../src-tauri/icon-1024.png'));

  console.log('Generated 1024x1024 square icon');
  return path.join(__dirname, '../src-tauri/icon-1024.png');
}

// Generate all required icon sizes
async function generateAllIcons() {
  const squareIcon = await generateSquareIcon();

  const sizes = [
    { name: '32x32.png', size: 32 },
    { name: '128x128.png', size: 128 },
    { name: '128x128@2x.png', size: 256 },
    { name: 'icon.png', size: 512 },
  ];

  for (const { name, size } of sizes) {
    await sharp(squareIcon)
      .resize(size, size)
      .toFile(path.join(iconsDir, name));
    console.log(`Generated ${name}`);
  }

  // Generate Windows Store logos
  const storeSizes = [
    { name: 'Square30x30Logo.png', size: 30 },
    { name: 'Square44x44Logo.png', size: 44 },
    { name: 'Square71x71Logo.png', size: 71 },
    { name: 'Square89x89Logo.png', size: 89 },
    { name: 'Square107x107Logo.png', size: 107 },
    { name: 'Square142x142Logo.png', size: 142 },
    { name: 'Square150x150Logo.png', size: 150 },
    { name: 'Square284x284Logo.png', size: 284 },
    { name: 'Square310x310Logo.png', size: 310 },
    { name: 'StoreLogo.png', size: 50 },
  ];

  for (const { name, size } of storeSizes) {
    await sharp(squareIcon)
      .resize(size, size)
      .toFile(path.join(iconsDir, name));
    console.log(`Generated ${name}`);
  }

  // Generate .ico file (Windows)
  await sharp(squareIcon)
    .resize(256, 256)
    .toFile(path.join(iconsDir, 'icon-256.png'));

  // Use ImageMagick or another tool to create .ico if available
  // For now, we'll copy the existing .ico structure
  console.log('Note: .ico and .icns files need additional tools (icoutils/png2ico for .ico, iconutil for .icns)');
  console.log('The PNG icons have been generated. You can use online converters or the Tauri CLI to finalize.');

  console.log('All PNG icons generated successfully!');
}

generateAllIcons().catch(console.error);
