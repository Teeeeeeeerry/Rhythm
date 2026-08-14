// Shared fixtures for the Wave 4a behavior suites (WA/WB).
#pragma once

#include "pch.h"
#include "Bridge/RhythmCore.h"

#include <filesystem>
#include <fstream>

namespace fs = std::filesystem;
using namespace rhythm;

namespace rhythm_tests {

/// A fresh temp directory, cleaned up when the guard goes out of scope.
struct TempDir {
    fs::path path;

    TempDir() {
        auto base = fs::temp_directory_path();
        path = base / (L"rhythm-tests-" +
            std::to_wstring(::GetCurrentProcessId()) + L"-" +
            std::to_wstring(::GetTickCount64()));
        fs::create_directories(path);
    }

    ~TempDir() {
        std::error_code ec;
        fs::remove_all(path, ec);
    }

    std::wstring dbPath() const {
        return (path / L"test.db").wstring();
    }
};

/// Write a minimal valid stereo 16-bit WAV (44100 Hz, 440 Hz sine on the
/// left) — the same fixture shape the rust-core suites use.
inline fs::path writeWavAt(const fs::path& dir, const std::wstring& name, double seconds = 1.0) {
    const uint32_t sampleRate = 44100;
    const uint16_t channels = 2;
    const uint16_t bits = 16;
    const size_t frames = static_cast<size_t>(sampleRate * seconds);
    const uint32_t dataLen = static_cast<uint32_t>(frames * channels * 2);

    std::vector<uint8_t> wav;
    auto put32 = [&](uint32_t v) {
        wav.push_back(v & 0xFF);
        wav.push_back((v >> 8) & 0xFF);
        wav.push_back((v >> 16) & 0xFF);
        wav.push_back((v >> 24) & 0xFF);
    };
    auto put16 = [&](uint16_t v) {
        wav.push_back(v & 0xFF);
        wav.push_back((v >> 8) & 0xFF);
    };

    wav.insert(wav.end(), {'R', 'I', 'F', 'F'});
    put32(36 + dataLen);
    wav.insert(wav.end(), {'W', 'A', 'V', 'E'});
    wav.insert(wav.end(), {'f', 'm', 't', ' '});
    put32(16);
    put16(1); // PCM
    put16(channels);
    put32(sampleRate);
    put32(sampleRate * channels * 2);
    put16(channels * bits / 8);
    put16(bits);
    wav.insert(wav.end(), {'d', 'a', 't', 'a'});
    put32(dataLen);
    for (size_t i = 0; i < frames; ++i) {
        double v = sin(440.0 * 2.0 * 3.141592653589793 * i / sampleRate);
        int16_t sample = static_cast<int16_t>(v * 32767.0);
        put16(sample);
        put16(0);
    }

    auto p = dir / name;
    std::ofstream out(p, std::ios::binary);
    out.write(reinterpret_cast<const char*>(wav.data()), wav.size());
    return p;
}

inline Track makeLocalTrack(const std::wstring& path, const std::wstring& title) {
    Track t;
    t.id = -1;
    t.filePath = path;
    t.sourceType = L"local";
    t.title = title;
    t.duration = 1.0;
    return t;
}

inline Track makeUrlTrack(const std::wstring& url, const std::wstring& title) {
    Track t;
    t.id = -1;
    t.sourceType = L"direct_url";
    t.sourceUrl = url;
    t.title = title;
    t.duration = 0.0;
    return t;
}

/// Poll `condition` until it holds or `timeoutMs` elapses.
template <typename F>
bool waitFor(F condition, int timeoutMs = 5000) {
    auto deadline = ::GetTickCount64() + timeoutMs;
    while (!condition() && ::GetTickCount64() < deadline) {
        ::Sleep(10);
    }
    return condition();
}

} // namespace rhythm_tests
