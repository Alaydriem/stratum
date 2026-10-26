#include "picker/StructureFilePicker.h"

#include <shobjidl.h>
#include <wrl/client.h>

#include <cwchar>

using Microsoft::WRL::ComPtr;

namespace stratum {

namespace {

// fixed shim text, outside levi-i18n; listed for the i18n audit
constexpr wchar_t const* DIALOG_TITLE = L"Import structures";
constexpr wchar_t const* FILTER_LABEL = L"Minecraft structure";
constexpr wchar_t const* FILTER_SPEC  = L"*.mcstructure";

} // namespace

PickResult StructureFilePicker::pick() {
    PickResult result;
    HRESULT    init = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    if (FAILED(init)) {
        result.status = init;
        return result;
    }
    {
        ComPtr<IFileOpenDialog> dialog;
        ComPtr<IShellItemArray> items;
        DWORD                   count = 0;
        HRESULT hr = CoCreateInstance(CLSID_FileOpenDialog, nullptr, CLSCTX_ALL, IID_PPV_ARGS(&dialog));
        if (SUCCEEDED(hr)) {
            COMDLG_FILTERSPEC filter[] = {
                {FILTER_LABEL, FILTER_SPEC}
            };
            FILEOPENDIALOGOPTIONS options = 0;
            hr                            = dialog->GetOptions(&options);
            if (SUCCEEDED(hr))
                hr = dialog->SetOptions(options | FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM);
            if (SUCCEEDED(hr)) hr = dialog->SetFileTypes(1, filter);
            if (SUCCEEDED(hr)) hr = dialog->SetTitle(DIALOG_TITLE);
        }
        if (SUCCEEDED(hr)) hr = dialog->Show(nullptr);
        if (SUCCEEDED(hr)) hr = dialog->GetResults(&items);
        if (SUCCEEDED(hr)) hr = items->GetCount(&count);
        result.status = hr;
        if (SUCCEEDED(hr)) {
            result.paths.reserve(count);
            for (DWORD index = 0; index < count; ++index) {
                ComPtr<IShellItem> item;
                PWSTR              path = nullptr;
                if (FAILED(items->GetItemAt(index, &item))
                    || FAILED(item->GetDisplayName(SIGDN_FILESYSPATH, &path))) {
                    ++result.dropped;
                    continue;
                }
                std::optional<std::string> utf8 = toUtf8(path);
                CoTaskMemFree(path);
                if (utf8) {
                    result.paths.push_back(std::move(*utf8));
                } else {
                    ++result.invalid;
                }
            }
        }
    }
    CoUninitialize();
    return result;
}

std::optional<std::string> StructureFilePicker::toUtf8(wchar_t const* wide) {
    int length = static_cast<int>(std::wcslen(wide));
    if (length == 0) return std::string();
    int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, wide, length, nullptr, 0, nullptr, nullptr);
    if (size <= 0) return std::nullopt;
    std::string out(static_cast<size_t>(size), '\0');
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, wide, length, out.data(), size, nullptr, nullptr) != size) {
        return std::nullopt;
    }
    return out;
}

} // namespace stratum
