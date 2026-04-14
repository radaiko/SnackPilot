import * as fs from 'fs';
import * as path from 'path';

const ACCENTS: Record<string, { color: string; gradientEnd: string }> = {
  orange: { color: '#D4501A', gradientEnd: '#B84415' },
  emerald: { color: '#2E7D4F', gradientEnd: '#236B3F' },
  berry: { color: '#A62547', gradientEnd: '#8C1E3B' },
  golden: { color: '#C08B1A', gradientEnd: '#A07415' },
  ocean: { color: '#2563A8', gradientEnd: '#1E528C' },
};

function generateSvg(id: string, color: string, gradientEnd: string): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg-${id}" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#FFFFFF"/>
      <stop offset="100%" style="stop-color:#F0F0F2"/>
    </linearGradient>
    <linearGradient id="accent-${id}" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:${color}"/>
      <stop offset="100%" style="stop-color:${gradientEnd}"/>
    </linearGradient>
  </defs>

  <!-- Background -->
  <rect width="1024" height="1024" rx="224" fill="url(#bg-${id})"/>

  <!-- Subtle plate circle -->
  <circle cx="512" cy="512" r="320" fill="none" stroke="${color}" stroke-opacity="0.08" stroke-width="6"/>
  <circle cx="512" cy="512" r="260" fill="none" stroke="${color}" stroke-opacity="0.05" stroke-width="3"/>

  <!-- Crossed Fork and Knife -->
  <g transform="translate(512, 512) rotate(-30) translate(-512, -512)">
    <!-- Fork -->
    <g transform="translate(400, 220)" fill="url(#accent-${id})">
      <rect x="10" y="0" width="14" height="130" rx="7"/>
      <rect x="38" y="0" width="14" height="130" rx="7"/>
      <rect x="66" y="0" width="14" height="130" rx="7"/>
      <rect x="94" y="0" width="14" height="130" rx="7"/>
      <rect x="10" y="120" width="98" height="34" rx="6"/>
      <rect x="40" y="144" width="38" height="440" rx="19"/>
    </g>
  </g>

  <g transform="translate(512, 512) rotate(30) translate(-512, -512)">
    <!-- Knife -->
    <g transform="translate(510, 220)" fill="url(#accent-${id})">
      <path d="M 20 0 C 20 0, 90 12, 90 70 L 90 155 L 20 155 Z"/>
      <rect x="20" y="145" width="70" height="34" rx="6"/>
      <rect x="36" y="169" width="38" height="420" rx="19"/>
    </g>
  </g>
</svg>`;
}

function generateAdaptiveSvg(id: string, color: string, gradientEnd: string): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="accent-a-${id}" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:${color}"/>
      <stop offset="100%" style="stop-color:${gradientEnd}"/>
    </linearGradient>
  </defs>

  <!-- Crossed Fork and Knife -->
  <g transform="translate(512, 512) rotate(-30) translate(-512, -512)">
    <g transform="translate(400, 220)" fill="url(#accent-a-${id})">
      <rect x="10" y="0" width="14" height="130" rx="7"/>
      <rect x="38" y="0" width="14" height="130" rx="7"/>
      <rect x="66" y="0" width="14" height="130" rx="7"/>
      <rect x="94" y="0" width="14" height="130" rx="7"/>
      <rect x="10" y="120" width="98" height="34" rx="6"/>
      <rect x="40" y="144" width="38" height="440" rx="19"/>
    </g>
  </g>

  <g transform="translate(512, 512) rotate(30) translate(-512, -512)">
    <g transform="translate(510, 220)" fill="url(#accent-a-${id})">
      <path d="M 20 0 C 20 0, 90 12, 90 70 L 90 155 L 20 155 Z"/>
      <rect x="20" y="145" width="70" height="34" rx="6"/>
      <rect x="36" y="169" width="38" height="420" rx="19"/>
    </g>
  </g>
</svg>`;
}

const outDir = path.join(__dirname, '..', '..', 'src', 'app', 'assets', 'icons');
fs.mkdirSync(outDir, { recursive: true });

for (const [id, { color, gradientEnd }] of Object.entries(ACCENTS)) {
  const svg = generateSvg(id, color, gradientEnd);
  fs.writeFileSync(path.join(outDir, `icon-${id}.svg`), svg);
  console.log(`Generated icon-${id}.svg`);

  const adaptiveSvg = generateAdaptiveSvg(id, color, gradientEnd);
  fs.writeFileSync(path.join(outDir, `adaptive-icon-${id}.svg`), adaptiveSvg);
  console.log(`Generated adaptive-icon-${id}.svg`);
}

console.log('\nDone! Now convert SVGs to PNGs:');
console.log('  npm run render');
