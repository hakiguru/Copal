#!/bin/bash
# Takes a screenshot of a Copal app in a virtual X display (no real screen needed).
#
#   tools/ui-shot.sh <app binary> <out.png> [project.copal] [--interact]
#
# Needs: Xvfb, ImageMagick (import), Mesa (software OpenGL). --interact also needs xdotool:
# it clicks the first object, runs two console commands and hovers the primary button.
# SHOT_KEYS="2 ctrl+p" presses those keys first (xdotool names), with SHOT_KEY_WAIT seconds
# after each (default 1). SHOT_CLICK="x,y x,y" clicks those points after the keys.
set -euo pipefail
BIN=$1; OUT=$2; PROJ=${3:-}; MODE=${4:-}
export LIBGL_ALWAYS_SOFTWARE=1 WAYLAND_DISPLAY= DISPLAY=:${SHOT_DISPLAY:-99}
Xvfb "$DISPLAY" -screen 0 1600x960x24 >/dev/null 2>&1 & XPID=$!
trap 'kill $APID $XPID 2>/dev/null || true' EXIT
sleep 1.5
"$BIN" ${PROJ:+"$PROJ"} >/tmp/copal-shot.log 2>&1 & APID=$!
sleep "${WAIT:-6}"
if [ -n "${SHOT_KEYS:-}${SHOT_CLICK:-}" ]; then
  xdotool windowfocus "$(xdotool search --name Copal | head -1)" || true
  xdotool mousemove 700 400; sleep 0.3
  for k in ${SHOT_KEYS:-}; do xdotool key "$k"; sleep "${SHOT_KEY_WAIT:-1}"; done
  for c in ${SHOT_CLICK:-}; do xdotool mousemove "${c%,*}" "${c#*,}" click 1; sleep "${SHOT_KEY_WAIT:-1}"; done
fi
if [ "$MODE" = "--interact" ]; then
  xdotool windowfocus "$(xdotool search --name Copal | head -1)"
  xdotool mousemove 610 450 click 1; sleep 1
  xdotool mousemove 700 823 click 1; sleep 0.5
  xdotool type --delay 30 "object.rotate id=1 z=15"; xdotool key Return; sleep 1
  xdotool mousemove 117 263; sleep 1.2
fi
import -window root "$OUT"
