// Shared fixtures for the Wave 4a behavior suites (WA/WB).
#pragma once

#include "pch.h"
#include "Bridge/RhythmCore.h"

#include <filesystem>
#include <fstream>

namespace fs = std::filesystem;
using namespace rhythm;

/// UTF-8 conversion for test fixtures (the bridge keeps its own private
/// converters).
inline std::string WideToUtf8ForTest(const std::wstring& ws) {
    if (ws.empty()) return {};
    int len = WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), nullptr, 0, nullptr, nullptr);
    std::string result(len, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), result.data(), len, nullptr, nullptr);
    return result;
}

/// Pins the L10n language for the scope of a test (registry override),
/// restoring the previous value on destruction (fixed-locale assertions,
/// #142 parity).
struct LocaleOverride {
    std::wstring previous;
    LocaleOverride(const std::wstring& code) {
        previous = L10n::OverrideLanguage();
        L10n::SetOverrideLanguage(code);
    }
    ~LocaleOverride() {
        L10n::SetOverrideLanguage(previous);
    }
};

/// Call-recording coordinator spy (ticket #173): mirrors the coordinator
/// contract with a small sequential queue model, so AppState tests run
/// playback paths with no audio device. The real rules live and are tested
/// in rust-core (coordinator_behavior.rs).
class SpyCoordinator : public ICoordinator {
public:
    struct StartCall {
        Track track;
        std::vector<Track> queueTracks;
        int32_t mode;
    };

    std::vector<StartCall> startCalls;
    std::vector<std::vector<Track>> syncQueueCalls;
    std::vector<int32_t> setPlayModeCalls;
    int nextCalls = 0;
    int previousCalls = 0;
    int toggleCalls = 0;
    int stopCalls = 0;
    float lastVolume = 0.0f;

    // Engine mirror (what the UI renders between events).
    int32_t engineState = 0;  // 0 Stopped, 1 Playing, 2 Paused, 3 Buffering
    double position = 0.0;
    double duration = 0.0;
    std::wstring errorMessage;
    std::wstring errorKind;

    // Mini queue model (sequential cursor), mirroring the coordinator.
    std::vector<Track> queueTracks;
    std::vector<Track> libraryTracks;
    int32_t cursor = 0;
    int32_t mode = 0;
    std::optional<Track> currentTrack;
    std::function<void(const std::wstring&)> eventHandler;

    CoordinatorResult Start(const Track& track,
                            const std::vector<Track>& queueTracks,
                            int32_t mode) override {
        startCalls.push_back({track, queueTracks, mode});
        CoordinatorResult result;
        // Mirror of the core's no-playable-location guard (#81).
        if (!Playable(track)) {
            result.ok = false;
            result.errorKind = L"no_playable_location";
            return result;
        }
        this->queueTracks = queueTracks;
        this->mode = mode;
        cursor = IndexOf(track.id);
        currentTrack = track;
        engineState = 1;
        result.ok = true;
        result.currentTrack = track;
        result.playbackActive = true;
        return result;
    }

    CoordinatorResult Next() override {
        ++nextCalls;
        return Advance(false);
    }

    CoordinatorResult Previous() override {
        ++previousCalls;
        return Advance(true);
    }

    CoordinatorResult TogglePlayPause() override {
        ++toggleCalls;
        CoordinatorResult result;
        result.ok = true;
        if (engineState == 1 || engineState == 3) {
            engineState = 2;  // pause
            result.currentTrack = currentTrack;
            result.playbackActive = false;
        } else if (engineState == 2) {
            engineState = 1;  // resume
            result.currentTrack = currentTrack;
            result.playbackActive = true;
        } else {
            if (!currentTrack.has_value() && !libraryTracks.empty()) {
                // Idle start: first playable library track.
                for (const auto& t : libraryTracks) {
                    if (Playable(t)) {
                        return Start(t, libraryTracks, mode);
                    }
                }
            }
            result.currentTrack = currentTrack;
            result.playbackActive = false;
        }
        return result;
    }

    void SyncQueue(const std::vector<Track>& tracks) override {
        syncQueueCalls.push_back(tracks);
        libraryTracks = tracks;
        queueTracks = tracks;
        if (currentTrack.has_value() && currentTrack->id >= 0) {
            int32_t pos = IndexOf(currentTrack->id);
            if (pos >= 0) cursor = pos;
            else cursor = 0;
        } else {
            cursor = 0;
        }
    }

    void Stop() override {
        ++stopCalls;
        engineState = 0;
        currentTrack.reset();
        queueTracks.clear();
        cursor = 0;
    }

    void SetVolume(float volume) override { lastVolume = volume; }
    void SetPlayMode(int32_t m) override { mode = m; setPlayModeCalls.push_back(m); }
    bool HasNext() const override {
        if (!currentTrack.has_value() || queueTracks.empty()) return false;
        return mode == 0 ? cursor + 1 < static_cast<int32_t>(queueTracks.size()) : true;
    }
    bool HasPrevious() const override {
        if (!currentTrack.has_value() || queueTracks.empty()) return false;
        return mode == 0 ? cursor > 0 : true;
    }
    bool CanTogglePlayback() const override {
        return currentTrack.has_value() || !libraryTracks.empty();
    }
    bool CanStop() const override { return engineState == 1 || engineState == 2 || engineState == 3; }
    double Position() const override { return position; }
    double Duration() const override { return duration; }
    int32_t State() const override { return engineState; }
    std::wstring ErrorMessage() const override { return errorMessage; }
    std::wstring ErrorKind() const override { return errorKind; }
    std::optional<Track> CurrentTrack() const override { return currentTrack; }
    void SetLibrary(Library*) override {}
    void SetEventHandler(std::function<void(const std::wstring&)> handler) override {
        eventHandler = std::move(handler);
    }

    /// Fire a coordinator event JSON through the registered handler.
    void FireEvent(const std::wstring& json) {
        if (eventHandler) eventHandler(json);
    }

private:
    static bool Playable(const Track& t) {
        if (t.sourceType == L"local") {
            return t.filePath.has_value() && !t.filePath->empty();
        }
        return t.sourceUrl.has_value() && !t.sourceUrl->empty();
    }

    int32_t IndexOf(int64_t id) const {
        for (size_t i = 0; i < queueTracks.size(); ++i) {
            if (queueTracks[i].id == id) return static_cast<int32_t>(i);
        }
        return 0;
    }

    CoordinatorResult Advance(bool backwards) {
        CoordinatorResult result;
        result.ok = true;
        if (!currentTrack.has_value() || queueTracks.empty()) {
            result.currentTrack = currentTrack;
            return result;
        }
        if (mode == 2) {  // SingleLoop: repeat the current track.
            result.currentTrack = currentTrack;
            result.playbackActive = true;
            return result;
        }
        int32_t bound = static_cast<int32_t>(queueTracks.size());
        for (int32_t i = 0; i < bound; ++i) {
            int32_t nextIndex = backwards ? cursor - 1 : cursor + 1;
            if (nextIndex < 0 || nextIndex >= bound) break;
            cursor = nextIndex;
            if (Playable(queueTracks[nextIndex])) {
                currentTrack = queueTracks[nextIndex];
                engineState = 1;
                result.currentTrack = currentTrack;
                result.playbackActive = true;
                return result;
            }
        }
        result.currentTrack = currentTrack;
        result.playbackActive = (engineState == 1 || engineState == 3);
        return result;
    }
};

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
