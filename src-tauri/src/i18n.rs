use std::collections::HashMap;

#[allow(dead_code)]
static RU_MAP: once_cell::sync::Lazy<HashMap<&'static str, &'static str>> =
    once_cell::sync::Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert("app_name", "DeepSeek Extractor");
        m.insert("paste_from_clipboard", "📋 Вставить из буфера");
        m.insert("extract", "🔍 Извлечь");
        m.insert("extract_from_url", "Извлечь из URL");
        m.insert("save_to_folder", "💾 Сохранить в папку");
        m.insert("create_archive", "📦 Создать архив");
        m.insert("extract_archive", "📂 Извлечь архив");
        m.insert("files_found", "Найдено файлов");
        m.insert("total_size", "Общий размер");
        m.insert("preview", "Предпросмотр");
        m.insert("file_name", "Имя файла");
        m.insert("path", "Путь");
        m.insert("language", "Язык");
        m.insert("size", "Размер");
        m.insert("actions", "Действия");
        m.insert("select_all", "Выбрать все");
        m.insert("url_placeholder", "Вставьте URL с диалогом ИИ...");
        m.insert("archive_path", "Путь к архиву");
        m.insert("status_ready", "Готов к работе");
        m.insert("status_extracting", "Извлечение файлов...");
        m.insert("status_saving", "Сохранение файлов...");
        m.insert("status_archiving", "Создание архива...");
        m.insert("status_extracting_archive", "Извлечение архива...");
        m.insert("drag_drop_hint", "Перетащите текстовый файл сюда");
        m.insert("reset_learning", "🔄 Сбросить обучение");
        m.insert("lang_ru", "RU");
        m.insert("lang_en", "EN");
        m.insert("confirm_overwrite", "Некоторые файлы уже существуют. Перезаписать?");
        m.insert("learning_model_updated", "Модель обучения обновлена");
        m.insert("error_no_files", "Нет извлечённых файлов");
        m.insert("rename", "✏️");
        m.insert("delete", "❌");
        m.insert("version", "Версия 1.0.0");
        m.insert("save", "Сохранить");
        m.insert("cancel", "Отмена");
        m.insert("rename_title", "Переименовать файл");
        m.insert("enter_new_name", "Введите новое имя файла");
        m.insert("confirm", "Подтвердить");
        m.insert("no_files_in_archive", "В архиве нет файлов");
        m.insert("archive_created", "Архив создан");
        m.insert("archives", "Архивы CodePack (*.cpk)");
        m.insert("all_files", "Все файлы");
        m
    });

#[allow(dead_code)]
static EN_MAP: once_cell::sync::Lazy<HashMap<&'static str, &'static str>> =
    once_cell::sync::Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert("app_name", "DeepSeek Extractor");
        m.insert("paste_from_clipboard", "📋 Paste from Clipboard");
        m.insert("extract", "🔍 Extract");
        m.insert("extract_from_url", "Extract from URL");
        m.insert("save_to_folder", "💾 Save to Folder");
        m.insert("create_archive", "📦 Create Archive");
        m.insert("extract_archive", "📂 Extract Archive");
        m.insert("files_found", "Files found");
        m.insert("total_size", "Total size");
        m.insert("preview", "Preview");
        m.insert("file_name", "File name");
        m.insert("path", "Path");
        m.insert("language", "Language");
        m.insert("size", "Size");
        m.insert("actions", "Actions");
        m.insert("select_all", "Select All");
        m.insert("url_placeholder", "Paste AI conversation URL...");
        m.insert("archive_path", "Archive path");
        m.insert("status_ready", "Ready");
        m.insert("status_extracting", "Extracting files...");
        m.insert("status_saving", "Saving files...");
        m.insert("status_archiving", "Creating archive...");
        m.insert("status_extracting_archive", "Extracting archive...");
        m.insert("drag_drop_hint", "Drop a text file here");
        m.insert("reset_learning", "🔄 Reset Learning");
        m.insert("lang_ru", "RU");
        m.insert("lang_en", "EN");
        m.insert("confirm_overwrite", "Some files already exist. Overwrite?");
        m.insert("learning_model_updated", "Learning model updated");
        m.insert("error_no_files", "No extracted files");
        m.insert("rename", "✏️");
        m.insert("delete", "❌");
        m.insert("version", "Version 1.0.0");
        m.insert("save", "Save");
        m.insert("cancel", "Cancel");
        m.insert("rename_title", "Rename File");
        m.insert("enter_new_name", "Enter new file name");
        m.insert("confirm", "Confirm");
        m.insert("no_files_in_archive", "No files in archive");
        m.insert("archive_created", "Archive created");
        m.insert("archives", "CodePack Archives (*.cpk)");
        m.insert("all_files", "All Files");
        m
    });

/// Supported language codes
#[allow(dead_code)]
pub(crate) enum Lang {
    Ru,
    En,
}

impl Lang {
    #[allow(dead_code)]
    pub(crate) fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "ru" => Lang::Ru,
            _ => Lang::En,
        }
    }
}

/// Translate a key. Supports `{n}` placeholders (up to 3 args).
/// Falls back to the key itself if not found.
#[allow(dead_code)]
pub(crate) fn t(key: &str, lang: &Lang) -> String {
    let map = match lang {
        Lang::Ru => &*RU_MAP,
        Lang::En => &*EN_MAP,
    };
    map.get(key).copied().unwrap_or(key).to_string()
}

/// Translate with positional arguments.
/// Replace `{0}`, `{1}`, etc. in the translated string.
#[allow(dead_code)]
pub(crate) fn tf(key: &str, lang: &Lang, args: &[&str]) -> String {
    let mut s = t(key, lang);
    for (i, arg) in args.iter().enumerate() {
        s = s.replace(&format!("{{{}}}", i), arg);
    }
    s
}
