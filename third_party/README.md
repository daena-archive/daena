# Third-party checkouts

## Fantasy Map Generator

Pinned upstream source for the Maps provider lives at
`third_party/fantasy-map-generator` (gitignored).

```sh
mkdir -p third_party/fantasy-map-generator
cd third_party/fantasy-map-generator
git init
git remote add origin https://github.com/Azgaar/Fantasy-Map-Generator.git
git fetch --depth 1 origin tag v1.119
git checkout FETCH_HEAD
# HEAD must be 3430c22204f60baa412d4657ca2f9d00c270eda9
npm ci --no-audit --no-fund
npm run build
cd ../..
node scripts/fmg-phase0-check.mjs third_party/fantasy-map-generator
npm run prepare:maps
```

`npm run prepare:maps` uses this directory by default when regenerating
`src-tauri/plugin-assets/maps/fmg-v1.119.zip`.
