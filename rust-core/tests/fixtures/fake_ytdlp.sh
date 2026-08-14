#!/bin/sh
# Fake yt-dlp for the resolver end-to-end tests (RS-01–23, manifest:
# docs/testing/behavior/resolver.md). Never touches the network: behavior is
# dispatched on the URL substring, and each non-version invocation is
# appended to $FAKE_YTDLP_CALL_LOG so tests can count subprocess calls.

if [ "$1" = "--version" ]; then
  echo "2024.01.01"
  exit 0
fi

if [ -n "$FAKE_YTDLP_CALL_LOG" ]; then
  echo "$*" >> "$FAKE_YTDLP_CALL_LOG"
fi

url=""
for arg in "$@"; do
  case "$arg" in
    http://*|https://*) url="$arg" ;;
  esac
done

case "$url" in
  *success-full*)
    printf '%s\n' '{"id":"v1","title":"Full Title","uploader":"Full Uploader","duration":125,"url":"https://cdn.example.com/full.m4a","http_headers":{"User-Agent":"Mozilla/5.0","Referer":"https://www.bilibili.com"},"thumbnail":"https://example.com/t.jpg"}'
    ;;
  *success-alt-title*)
    printf '%s\n' '{"id":"v2","fulltitle":"Alt Title","duration":60,"url":"https://cdn.example.com/a.m4a"}'
    ;;
  *success-no-title*)
    printf '%s\n' '{"id":"v3","duration":10,"url":"https://cdn.example.com/a.m4a"}'
    ;;
  *success-artist-channel*)
    printf '%s\n' '{"id":"v4","title":"T","channel":"Channel Name","duration":10,"url":"https://cdn.example.com/a.m4a"}'
    ;;
  *success-artist-creator*)
    printf '%s\n' '{"id":"v5","title":"T","creator":"Creator Name","duration":10,"url":"https://cdn.example.com/a.m4a"}'
    ;;
  *success-duration-string*)
    printf '%s\n' '{"id":"v6","title":"T","duration":"3:45","url":"https://cdn.example.com/a.m4a"}'
    ;;
  *success-duration-durationstring*)
    printf '%s\n' '{"id":"v7","title":"T","duration_string":"1:02:30","url":"https://cdn.example.com/a.m4a"}'
    ;;
  *success-duration-numericstring*)
    printf '%s\n' '{"id":"v7b","title":"T","duration":"125","url":"https://cdn.example.com/a.m4a"}'
    ;;
  *success-requested-formats*)
    printf '%s\n' '{"id":"v8","title":"T","requested_formats":[{"url":"https://cdn.example.com/first.m4a","http_headers":{"Referer":"https://example.com/page"}},{"url":"https://cdn.example.com/second.m4a"}]}'
    ;;
  *success-formats-audio*)
    printf '%s\n' '{"id":"v9","title":"T","formats":[{"ext":"mp4","url":"https://cdn.example.com/video.mp4","vcodec":"avc1"},{"url":"https://cdn.example.com/audio.m4a","vcodec":"none","acodec":"mp4a"}]}'
    ;;
  *success-formats-fallback*)
    printf '%s\n' '{"id":"v9b","title":"T","formats":[{"url":"https://cdn.example.com/video.mp4","vcodec":"avc1"}]}'
    ;;
  *success-manifest*)
    printf '%s\n' '{"id":"v10","title":"T","manifest_url":"https://cdn.example.com/index.m3u8"}'
    ;;
  *success-no-stream*)
    printf '%s\n' '{"id":"v11","title":"T","duration":30}'
    ;;
  *success-headers-format*)
    printf '%s\n' '{"id":"v12","title":"T","http_headers":{"Referer":"https://top.example.com"},"formats":[{"url":"https://cdn.example.com/f.m4a","http_headers":{"Referer":"https://format.example.com"}}]}'
    ;;
  *empty-output*)
    exit 0
    ;;
  *empty-with-stderr*)
    echo "ERROR: Private video. Sign in if you've been granted access to this video" >&2
    exit 0
    ;;
  *bad-json*)
    printf '%s\n' 'this is { not valid json'
    ;;
  *fail-outdated*)
    echo "ERROR: Unable to extract player function; please report this issue on https://github.com/yt-dlp/yt-dlp" >&2
    exit 1
    ;;
  *fail-unavailable*)
    echo "ERROR: Private video. Sign in if you've been granted access to this video" >&2
    exit 1
    ;;
  *fail-network*)
    echo "ERROR: unable to download webpage: connection timed out" >&2
    exit 1
    ;;
  *fail-unknown*)
    echo "ERROR: something odd happened" >&2
    exit 1
    ;;
  *)
    echo "fake yt-dlp: unexpected URL $url" >&2
    exit 1
    ;;
esac
exit 0
