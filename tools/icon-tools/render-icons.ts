import * as fs from 'fs';
import * as path from 'path';
import sharp from 'sharp';

const assetsDir = path.join(__dirname, '..', '..', 'src', 'app', 'assets');
const iconsDir = path.join(assetsDir, 'icons');

async function main() {
  const svgFiles = fs.readdirSync(iconsDir).filter(f => f.endsWith('.svg'));

  for (const svgFile of svgFiles) {
    const svgPath = path.join(iconsDir, svgFile);
    const pngFile = svgFile.replace('.svg', '.png');
    const pngPath = path.join(iconsDir, pngFile);

    await sharp(svgPath)
      .resize(1024, 1024)
      .png()
      .toFile(pngPath);

    console.log(`Rendered ${pngFile}`);
  }

  // Also update the default icon.png and adaptive-icon.png from orange variant
  await sharp(path.join(iconsDir, 'icon-orange.svg'))
    .resize(1024, 1024)
    .png()
    .toFile(path.join(assetsDir, 'icon.png'));
  console.log('Updated assets/icon.png from orange variant');

  await sharp(path.join(iconsDir, 'adaptive-icon-orange.svg'))
    .resize(1024, 1024)
    .png()
    .toFile(path.join(assetsDir, 'adaptive-icon.png'));
  console.log('Updated assets/adaptive-icon.png from orange variant');
}

main().catch(console.error);
