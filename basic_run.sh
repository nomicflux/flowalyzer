#!/bin/bash

set -e

source .env

echo "Whisper model: $WHISPER_MODEL_PATH"

clear=0

while [ "$1" != "" ]; do 
  case $1 in
    -f | --file ) shift
                  file="$1"
                  ;;
    -o | --out-dir ) shift
                     outdir="$1"
                     ;;
    -c | --clear ) clear=1
                   ;;
    -s | --start ) shift
                   start="$1"
                   ;;
    -e | --end ) shift
                 end="$1"
                 ;;
    -l | --language ) shift
                      language="$1"
                      ;;
    -d | --duration ) shift
                      duration="$1"
                      ;;
    * ) exit 1
  esac
  shift
done

if [ -z $file ]; then 
  read -r -p "Filename: " filename
fi  

if [ -z $outdir ]; then 
  read -r -p "Out Directory: " outdir
fi  

if [ -z $start ]; then
  read -r -p "Start time (00:00:00): " start
fi

if [ -z $end ]; then
  read -r -p "End time (00:00:00): " end
fi

if [ -z $duration ]; then
  read -r -p "Target duration (2.0): " duration
fi

echo "Creating $outdir"
mkdir -p "$outdir"

if [ $clear == 1 ]; then
  echo "Clearing $outdir/*"
  rm -rf "$outdir/*"
fi

language_text=""
if [ -n $language ]; then
  language_text="--whisper-language $language"
fi

echo "Flowing with $file"
cargo run -- \
  "$file" \
  "$outdir" \
  --recipe-json '{
    "name": "language-learning",
    "steps": [
      {"repeat_count": 7, "speed_factor": 0.75, "silent": false},
      {"repeat_count": 1, "speed_factor": 0.75, "silent": true},
      {"repeat_count": 7, "speed_factor": 0.9, "silent": false},
      {"repeat_count": 1, "speed_factor": 0.9, "silent": true},
      {"repeat_count": 7, "speed_factor": 1.0, "silent": false},
      {"repeat_count": 1, "speed_factor": 1.0, "silent": true},
      {"repeat_count": 7, "speed_factor": 1.1, "silent": false},
      {"repeat_count": 1, "speed_factor": 1.1, "silent": true},
      {"repeat_count": 7, "speed_factor": 1.25, "silent": false},
      {"repeat_count": 1, "speed_factor": 1.25, "silent": true}
    ]
  }' \
  --target-duration $duration \
  --start $start \
  --end $end \
  $language_text
