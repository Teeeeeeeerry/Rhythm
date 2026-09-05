#!/usr/bin/env python3
"""Fake yt-dlp for the resolver end-to-end tests (RS-01-23, manifest:
docs/testing/behavior/resolver.md). Never touches the network: behavior is
dispatched on the URL substring, and each non-version invocation is
appended to $FAKE_YTDLP_CALL_LOG so tests can count subprocess calls.
"""

import os
import sys

# Dispatch table: URL substring -> (stdout line, stderr line, exit code).
# Order matters: the first substring that matches wins, so more specific
# tags must come before the prefixes they extend.
CASES = [
    ("success-full",
     '{"id":"v1","title":"Full Title","uploader":"Full Uploader","duration":125,'
     '"url":"https://cdn.example.com/full.m4a","http_headers":{"User-Agent":"Mozilla/5.0",'
     '"Referer":"https://www.bilibili.com"},"thumbnail":"https://example.com/t.jpg"}',
     "", 0),
    ("success-alt-title",
     '{"id":"v2","fulltitle":"Alt Title","duration":60,"url":"https://cdn.example.com/a.m4a"}',
     "", 0),
    ("success-no-title",
     '{"id":"v3","duration":10,"url":"https://cdn.example.com/a.m4a"}', "", 0),
    ("success-artist-channel",
     '{"id":"v4","title":"T","channel":"Channel Name","duration":10,'
     '"url":"https://cdn.example.com/a.m4a"}', "", 0),
    ("success-artist-creator",
     '{"id":"v5","title":"T","creator":"Creator Name","duration":10,'
     '"url":"https://cdn.example.com/a.m4a"}', "", 0),
    ("success-duration-durationstring",
     '{"id":"v7","title":"T","duration_string":"1:02:30",'
     '"url":"https://cdn.example.com/a.m4a"}', "", 0),
    ("success-duration-numericstring",
     '{"id":"v7b","title":"T","duration":"125","url":"https://cdn.example.com/a.m4a"}',
     "", 0),
    ("success-duration-string",
     '{"id":"v6","title":"T","duration":"3:45","url":"https://cdn.example.com/a.m4a"}',
     "", 0),
    ("success-requested-formats",
     '{"id":"v8","title":"T","requested_formats":[{"url":"https://cdn.example.com/first.m4a",'
     '"http_headers":{"Referer":"https://example.com/page"}},'
     '{"url":"https://cdn.example.com/second.m4a"}]}', "", 0),
    ("success-formats-audio",
     '{"id":"v9","title":"T","formats":[{"ext":"mp4","url":"https://cdn.example.com/video.mp4",'
     '"vcodec":"avc1"},{"url":"https://cdn.example.com/audio.m4a","vcodec":"none",'
     '"acodec":"mp4a"}]}', "", 0),
    ("success-formats-fallback",
     '{"id":"v9b","title":"T","formats":[{"url":"https://cdn.example.com/video.mp4",'
     '"vcodec":"avc1"}]}', "", 0),
    ("success-manifest",
     '{"id":"v10","title":"T","manifest_url":"https://cdn.example.com/index.m3u8"}', "", 0),
    ("success-no-stream", '{"id":"v11","title":"T","duration":30}', "", 0),
    ("success-headers-format",
     '{"id":"v12","title":"T","http_headers":{"Referer":"https://top.example.com"},'
     '"formats":[{"url":"https://cdn.example.com/f.m4a",'
     '"http_headers":{"Referer":"https://format.example.com"}}]}', "", 0),
    ("empty-with-stderr", "",
     "ERROR: Private video. Sign in if you've been granted access to this video", 0),
    ("empty-output", "", "", 0),
    ("bad-json", "this is { not valid json", "", 0),
    ("fail-outdated", "",
     "ERROR: Unable to extract player function; please report this issue on "
     "https://github.com/yt-dlp/yt-dlp", 1),
    ("fail-unavailable", "",
     "ERROR: Private video. Sign in if you've been granted access to this video", 1),
    ("fail-network", "", "ERROR: unable to download webpage: connection timed out", 1),
    ("fail-unknown", "", "ERROR: something odd happened", 1),
]


def main(argv: list[str]) -> int:
    if argv[:1] == ["--version"]:
        print("2024.01.01")
        return 0

    log = os.environ.get("FAKE_YTDLP_CALL_LOG")
    if log:
        with open(log, "a", encoding="utf-8") as fh:
            fh.write(" ".join(argv) + "\n")

    url = ""
    for arg in argv:
        if arg.startswith(("http://", "https://")):
            url = arg

    for needle, out, err, code in CASES:
        if needle in url:
            if out:
                print(out)
            if err:
                print(err, file=sys.stderr)
            return code

    print(f"fake yt-dlp: unexpected URL {url}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
