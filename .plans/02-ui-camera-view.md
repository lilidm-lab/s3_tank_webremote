# Plan: UI camera view (render `{"cam": b64}` telemetry as image)

## Goal
Browser UI renders the tank's OV3660 jpeg stream instead of dumping base64 as
raw JSON text (root README promises "camera frames stream from the tank").

## Tasks
- [x] 0. index.html: `<img id="cam">` between pad and telemetry pre
- [x] 1. main.ts: type-guard `cam` frames -> `img.src = data:image/jpeg;base64,...`;
      other telemetry still rendered as JSON text
- [x] 2. style.css: 4:3 contained frame styling
- [x] 3. `yarn build` (tsc strict + vite): green

## Notes
- Firmware already streams `{"cam":"<b64 jpeg>"}` ~10 fps (QVGA 4:3, plan 01)
- Server wraps device JSON as `{"evt":"telemetry","data":{...}}` unchanged (plan 00)
- Deployed nginx container must be rebuilt to pick up the new dist:
  `docker compose build web && docker compose up -d web`

## Result
- `yarn --cwd remote/vite build`: clean
