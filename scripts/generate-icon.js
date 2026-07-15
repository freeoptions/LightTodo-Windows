import sharp from 'sharp';
import toIco from 'to-ico';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function createIconSvg(size) {
  return `
    <svg width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style="stop-color:#22c55e;stop-opacity:1" />
          <stop offset="100%" style="stop-color:#16a34a;stop-opacity:1" />
        </linearGradient>
      </defs>
      <rect x="0" y="0" width="${size}" height="${size}" rx="${size * 0.22}" fill="url(#grad)"/>
      <path d="M ${size * 0.28} ${size * 0.52}
               L ${size * 0.42} ${size * 0.66}
               L ${size * 0.72} ${size * 0.34}"
            fill="none"
            stroke="white"
            stroke-width="${size * 0.06}"
            stroke-linecap="round"
            stroke-linejoin="round"/>
    </svg>
  `;
}

const iconsDir = path.join(__dirname, '../src-tauri/icons');

async function generateIcons() {
  // 标准PNG图标
  const sizes = [
    { name: '32x32.png', size: 32 },
    { name: '128x128.png', size: 128 },
    { name: 'icon.png', size: 512 },
  ];

  for (const { name, size } of sizes) {
    const svg = createIconSvg(size);
    await sharp(Buffer.from(svg))
      .resize(size, size)
      .png()
      .toFile(path.join(iconsDir, name));
    console.log(`Generated ${name}`);
  }

  // 2x版本
  const svg2x = createIconSvg(256);
  await sharp(Buffer.from(svg2x))
    .resize(256, 256)
    .png()
    .toFile(path.join(iconsDir, '128x128@2x.png'));
  console.log('Generated 128x128@2x.png');

  // Square Logo
  const squareSizes = [30, 44, 71, 89, 107, 142, 150, 284, 310];
  for (const size of squareSizes) {
    const squareSvg = createIconSvg(size);
    await sharp(Buffer.from(squareSvg))
      .resize(size, size)
      .png()
      .toFile(path.join(iconsDir, `Square${size}x${size}Logo.png`));
    console.log(`Generated Square${size}x${size}Logo.png`);
  }

  // StoreLogo
  const storeSvg = createIconSvg(50);
  await sharp(Buffer.from(storeSvg))
    .resize(50, 50)
    .png()
    .toFile(path.join(iconsDir, 'StoreLogo.png'));
  console.log('Generated StoreLogo.png');

  // ICO文件 - 使用to-ico生成多尺寸ICO
  const icoSizes = [16, 32, 48, 64, 128, 256];
  const icoBuffers = await Promise.all(
    icoSizes.map(size =>
      sharp(Buffer.from(createIconSvg(size)))
        .resize(size, size)
        .png()
        .toBuffer()
    )
  );

  const icoBuffer = await toIco(icoBuffers);
  await fs.writeFile(path.join(iconsDir, 'icon.ico'), icoBuffer);
  console.log('Generated icon.ico');

  console.log('All icons generated successfully!');
}

generateIcons().catch(console.error);
