// zc_util.hpp - internal helpers shared by the adapter translation units.
#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

#include <stdexcept>
#include <string>

#include "zc_bridge.h"

namespace zc {

// Strict UTF-8 -> UTF-16. Throws std::invalid_argument on malformed input.
inline std::wstring Utf8ToWide(const char *s, size_t len) {
    if (len == 0) return std::wstring();
    if (s == nullptr || len > 0x7fffffff) throw std::invalid_argument("utf8 input");
    int n = ::MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, s, (int)len, nullptr, 0);
    if (n <= 0) throw std::invalid_argument("invalid utf8");
    std::wstring w((size_t)n, L'\0');
    ::MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, s, (int)len, &w[0], n);
    return w;
}

// UTF-16 -> UTF-8. Unpaired surrogates (which Windows allows in names) are
// replaced with U+FFFD rather than failing, so a weird display name can never
// abort an event.
inline std::string WideToUtf8(const wchar_t *w, size_t len) {
    if (w == nullptr || len == 0) return std::string();
    if (len > 0x7fffffff) throw std::invalid_argument("utf16 input");
    int n = ::WideCharToMultiByte(CP_UTF8, 0, w, (int)len, nullptr, 0, nullptr, nullptr);
    if (n <= 0) return std::string();
    std::string s((size_t)n, '\0');
    ::WideCharToMultiByte(CP_UTF8, 0, w, (int)len, &s[0], n, nullptr, nullptr);
    return s;
}
inline std::string WideToUtf8(const std::wstring &w) { return WideToUtf8(w.data(), w.size()); }
inline std::string WideToUtf8(const wchar_t *w) { return w ? WideToUtf8(w, wcslen(w)) : std::string(); }

// Overwrite a string's buffer before it is destroyed (for tokens).
template <class S> inline void Wipe(S &s) {
    if (!s.empty()) ::SecureZeroMemory(&s[0], s.size() * sizeof(s[0]));
    s.clear();
}

}  // namespace zc

// Boundary guard: no C++ exception may unwind into Rust.
#define ZC_GUARD_BEGIN try {
#define ZC_GUARD_END                                     \
    }                                                    \
    catch (const std::invalid_argument &) {              \
        return ZC_ERR_INVALID_ARG;                       \
    }                                                    \
    catch (...) {                                        \
        return ZC_ERR_EXCEPTION;                         \
    }
