//! yt-dlp stderr classification (ticket #187 split): turn a failed run's
//! stderr into a machine-readable kind + a summarized message.

use crate::resolver::{ResolveErrorKind, ResolveFailure};

/// Turn yt-dlp's stderr into an actionable failure.
pub fn ytdlp_failure_from_stderr(
    stderr: &str,
    status: std::process::ExitStatus,
) -> ResolveFailure {
    let detail = summarize_stderr(stderr);
    let kind = classify_ytdlp_stderr(stderr);

    let advice = match kind {
        ResolveErrorKind::YtDlpOutdated => {
            "\n\nyt-dlp looks out of date for this site. Update it:\n  \
             brew upgrade yt-dlp   (macOS)\n  \
             pip install -U yt-dlp (pip installs)\n  \
             yt-dlp -U             (standalone builds)"
        }
        ResolveErrorKind::Unavailable => {
            "\n\nThe video may be private, deleted, age-restricted, \
             members-only, or blocked in your region."
        }
        ResolveErrorKind::Network => {
            "\n\nCheck your network connection, proxy, or VPN and try again."
        }
        _ => "",
    };

    let message = if detail.is_empty() {
        format!(
            "yt-dlp exited with status {status} and no error output.{advice}"
        )
    } else {
        format!("yt-dlp failed: {detail}{advice}")
    };

    ResolveFailure::new(kind, message)
}

/// Map yt-dlp's stderr onto a failure kind.
pub fn classify_ytdlp_stderr(stderr: &str) -> ResolveErrorKind {
    let text = stderr.to_lowercase();

    let matches = |needles: &[&str]| needles.iter().any(|n| text.contains(n));

    // Check "update yt-dlp" style breakage before the generic buckets: those
    // messages often also mention "unable to download".
    if matches(&[
        "nsig extraction failed",
        "signature extraction failed",
        "please report this issue",
        "update to the latest version",
        "yt-dlp is out of date",
        "unable to extract player",
        "unsupported url",
    ]) {
        return ResolveErrorKind::YtDlpOutdated;
    }

    if matches(&[
        "confirm you're not a bot",
        "confirm you are not a bot",
        "private video",
        "video unavailable",
        "this video is unavailable",
        "removed by the uploader",
        "account associated with this video has been terminated",
        "members-only",
        "age-restricted",
        "sign in to view",
        // Covers both "not available in your country" and yt-dlp's
        // "The uploader has not made this video available in your country".
        "available in your country",
        "geo restricted",
        "geo-restricted",
        "blocked it in your country",
        "requires a subscription",
        "login required",
        "cookies",
    ]) {
        return ResolveErrorKind::Unavailable;
    }

    if matches(&[
        "unable to download webpage",
        "urlopen error",
        "temporary failure in name resolution",
        "name or service not known",
        "connection refused",
        "connection reset",
        "connection timed out",
        "network is unreachable",
        "read timed out",
        "ssl",
        "certificate verify failed",
        "proxy",
    ]) {
        return ResolveErrorKind::Network;
    }

    ResolveErrorKind::Internal
}

/// Keep the tail of yt-dlp's stderr — the last lines carry the actual error,
/// and the full text goes to the log file anyway.
pub fn summarize_stderr(stderr: &str) -> String {
    const MAX_LINES: usize = 4;
    const MAX_CHARS: usize = 600;

    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    let tail = if lines.len() > MAX_LINES {
        &lines[lines.len() - MAX_LINES..]
    } else {
        &lines[..]
    };

    let mut summary = tail.join("\n");
    if summary.chars().count() > MAX_CHARS {
        summary = summary.chars().take(MAX_CHARS).collect::<String>() + "…";
    }
    summary
}
