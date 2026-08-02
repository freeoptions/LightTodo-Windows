import sharp from 'sharp';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');
const artworkSource = path.join(projectRoot, 'src-tauri', 'icon-artwork.png');
const canonicalSource = path.join(projectRoot, 'src-tauri', 'icon-source.png');
const requestedSource = process.argv[2]
  ? path.resolve(process.argv[2])
  : artworkSource;
const iconsDir = path.join(projectRoot, 'src-tauri', 'icons');
const publicIcon = path.join(projectRoot, 'public', 'icon.png');
const transparent = { r: 0, g: 0, b: 0, alpha: 0 };
const iconGreen = '#18c892';

async function createArtwork() {
  const artwork = await sharp(requestedSource)
    .ensureAlpha()
    .trim({ background: transparent, threshold: 10 })
    .resize(1024, 1024, {
      fit: 'contain',
      background: transparent,
      kernel: sharp.kernel.lanczos3,
    })
    .png()
    .toBuffer();

  await fs.writeFile(artworkSource, artwork);
  return artwork;
}

async function extractWhiteMark(artwork) {
  const { data, info } = await sharp(artwork)
    .ensureAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  const mark = Buffer.alloc(data.length);

  for (let index = 0; index < data.length; index += 4) {
    const pixel = index / 4;
    const x = pixel % info.width;
    const y = Math.floor(pixel / info.width);
    const normalizedX = (x + 0.5) / info.width - 0.5;
    const normalizedY = (y + 0.5) / info.height - 0.5;
    const insideMarkArea = Math.hypot(normalizedX, normalizedY) < 0.43;
    const sourceAlpha = data[index + 3] / 255;
    // The original K artwork uses a white check over a green base. Red is the
    // most reliable separator because the green base has a very low red value.
    const whiteness = insideMarkArea
      ? Math.max(0, Math.min(1, (data[index] - 72) / 150))
      : 0;
    mark[index] = 255;
    mark[index + 1] = 255;
    mark[index + 2] = 255;
    mark[index + 3] = Math.round(255 * sourceAlpha * whiteness);
  }

  return sharp(mark, {
    raw: { width: info.width, height: info.height, channels: 4 },
  }).png().toBuffer();
}

function roundedSquareSvg(size) {
  const radius = Math.round(size * 0.22);
  return Buffer.from(`
    <svg width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" xmlns="http://www.w3.org/2000/svg">
      <rect width="${size}" height="${size}" rx="${radius}" fill="${iconGreen}"/>
    </svg>
  `);
}

function smallIconSvg(size) {
  const fragments = size <= 20
    ? '<rect x="7" y="15.3" width="1.8" height="1.8" rx="0.25"/>'
    : `
      <rect x="5.7" y="14.9" width="1.7" height="1.7" rx="0.25"/>
      <rect x="7.7" y="12.9" width="1.35" height="1.35" rx="0.2"/>
      <rect x="7.8" y="17.4" width="2" height="2" rx="0.25"/>
      <rect x="9.4" y="14.8" width="1.5" height="1.5" rx="0.2"/>
    `;

  return Buffer.from(`
    <svg width="${size}" height="${size}" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
      <rect width="32" height="32" rx="7" fill="${iconGreen}"/>
      <g fill="#fff">${fragments}</g>
      <path d="M9.7 15.9 14.7 21 25.1 10.4" fill="none" stroke="#fff" stroke-width="4.6" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  `);
}

async function createRoundedMaster(artwork) {
  const mark = await extractWhiteMark(artwork);
  const master = await sharp(roundedSquareSvg(1024))
    .composite([{ input: mark }])
    .png()
    .toBuffer();
  await fs.writeFile(canonicalSource, master);
  return master;
}

async function renderIcon(master, size) {
  if (size <= 48) {
    return sharp(smallIconSvg(size)).png().toBuffer();
  }

  return sharp(master)
    .resize(size, size, { kernel: sharp.kernel.lanczos3 })
    .png()
    .toBuffer();
}

async function createDibFrame(png, size) {
  const { data } = await sharp(png)
    .ensureAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  const headerSize = 40;
  const xorSize = size * size * 4;
  const maskStride = Math.ceil(size / 32) * 4;
  const maskSize = maskStride * size;
  const dib = Buffer.alloc(headerSize + xorSize + maskSize);

  dib.writeUInt32LE(headerSize, 0);
  dib.writeInt32LE(size, 4);
  dib.writeInt32LE(size * 2, 8);
  dib.writeUInt16LE(1, 12);
  dib.writeUInt16LE(32, 14);
  dib.writeUInt32LE(0, 16);
  dib.writeUInt32LE(xorSize, 20);

  for (let y = 0; y < size; y += 1) {
    const sourceY = size - 1 - y;
    for (let x = 0; x < size; x += 1) {
      const source = (sourceY * size + x) * 4;
      const target = headerSize + (y * size + x) * 4;
      dib[target] = data[source + 2];
      dib[target + 1] = data[source + 1];
      dib[target + 2] = data[source];
      dib[target + 3] = data[source + 3];

      if (data[source + 3] === 0) {
        const maskByte = headerSize + xorSize + y * maskStride + Math.floor(x / 8);
        dib[maskByte] |= 1 << (7 - (x % 8));
      }
    }
  }

  return dib;
}

function createIco(entries) {
  const headerSize = 6;
  const directorySize = 16;
  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(entries.length, 4);

  let offset = headerSize + directorySize * entries.length;
  const directories = entries.map(({ size, data }) => {
    const entry = Buffer.alloc(directorySize);
    entry.writeUInt8(size === 256 ? 0 : size, 0);
    entry.writeUInt8(size === 256 ? 0 : size, 1);
    entry.writeUInt8(0, 2);
    entry.writeUInt8(0, 3);
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(data.length, 8);
    entry.writeUInt32LE(offset, 12);
    offset += data.length;
    return entry;
  });

  return Buffer.concat([header, ...directories, ...entries.map(({ data }) => data)]);
}

async function generateAllIcons() {
  await fs.mkdir(iconsDir, { recursive: true });
  const artwork = await createArtwork();
  const master = await createRoundedMaster(artwork);
  const pngSizes = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256, 512];
  const rendered = new Map();

  for (const size of pngSizes) {
    rendered.set(size, await renderIcon(master, size));
  }

  const pngIcons = [
    ['16x16.png', 16],
    ['20x20.png', 20],
    ['24x24.png', 24],
    ['32x32.png', 32],
    ['40x40.png', 40],
    ['48x48.png', 48],
    ['64x64.png', 64],
    ['96x96.png', 96],
    ['128x128.png', 128],
    ['128x128@2x.png', 256],
    ['icon.png', 512],
    ['Square30x30Logo.png', 30],
    ['Square44x44Logo.png', 44],
    ['Square71x71Logo.png', 71],
    ['Square89x89Logo.png', 89],
    ['Square107x107Logo.png', 107],
    ['Square142x142Logo.png', 142],
    ['Square150x150Logo.png', 150],
    ['Square284x284Logo.png', 284],
    ['Square310x310Logo.png', 310],
    ['StoreLogo.png', 50],
  ];

  for (const [name, size] of pngIcons) {
    const png = rendered.get(size) ?? await renderIcon(master, size);
    await fs.writeFile(path.join(iconsDir, name), png);
  }

  await fs.writeFile(publicIcon, rendered.get(512));
  const icoSizes = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];
  const icoEntries = [];
  for (const size of icoSizes) {
    const png = rendered.get(size);
    icoEntries.push({
      size,
      data: size === 256 ? png : await createDibFrame(png, size),
    });
  }
  await fs.writeFile(path.join(iconsDir, 'icon.ico'), createIco(icoEntries));

  console.log(`Original K artwork: ${requestedSource}`);
  console.log(`Generated a full-canvas rounded-square master, ${pngIcons.length + 2} PNG assets, and a ${icoSizes.length}-size Windows-compatible ICO.`);
}

generateAllIcons().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
