pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "ru" => Lang::Ru,
            _ => Lang::En,
        }
    }
}

pub fn t(key: &str, lang: &Lang) -> &str {
    match lang {
        Lang::Ru => t_ru(key),
        Lang::En => t_en(key),
    }
}

fn t_ru(key: &str) -> &str {
    match key {
        "app_name" => "DeepSeek Extractor",
        "paste_from_clipboard" => "📋 Вставить из буфера",
        "extract" => "🔍 Извлечь",
        "extract_from_url" => "Извлечь из URL",
        "save_to_folder" => "💾 Сохранить в папку",
        "create_archive" => "📦 Создать архив",
        "extract_archive" => "📂 Извлечь архив",
        "files_found" => "Найдено файлов",
        "total_size" => "Общий размер",
        "preview" => "Предпросмотр",
        "file_name" => "Имя файла",
        "path" => "Путь",
        "language" => "Язык",
        "size" => "Размер",
        "actions" => "Действия",
        "select_all" => "Выбрать все",
        "url_placeholder" => "Вставьте URL с диалогом ИИ...",
        "archive_path" => "Путь к архиву",
        "status_ready" => "Готов к работе",
        "status_extracting" => "Извлечение файлов...",
        "status_saving" => "Сохранение файлов...",
        "status_archiving" => "Создание архива...",
        "status_extracting_archive" => "Извлечение архива...",
        "drag_drop_hint" => "Перетащите текстовый файл сюда",
        "reset_learning" => "🔄 Сбросить обучение",
        "lang_ru" => "RU",
        "lang_en" => "EN",
        "confirm_overwrite" => "Некоторые файлы уже существуют. Перезаписать?",
        "learning_model_updated" => "Модель обучения обновлена",
        "error_no_files" => "Нет извлечённых файлов",
        "rename" => "✏️",
        "delete" => "❌",
        "version" => "Версия 1.0.0",
        "save" => "Сохранить",
        "cancel" => "Отмена",
        "rename_title" => "Переименовать файл",
        "enter_new_name" => "Введите новое имя файла",
        "overwrite_confirm" => "Подтверждение перезаписи",
        "confirm" => "Подтвердить",
        "no_files_in_archive" => "В архиве нет файлов",
        "archive_created" => "Архив создан",
        "archives" => "Архивы CodePack (*.cpk)",
        "all_files" => "Все файлы",
        _ => key,
    }
}

fn t_en(key: &str) -> &str {
    match key {
        "app_name" => "DeepSeek Extractor",
        "paste_from_clipboard" => "📋 Paste from Clipboard",
        "extract" => "🔍 Extract",
        "extract_from_url" => "Extract from URL",
        "save_to_folder" => "💾 Save to Folder",
        "create_archive" => "📦 Create Archive",
        "extract_archive" => "📂 Extract Archive",
        "files_found" => "Files found",
        "total_size" => "Total size",
        "preview" => "Preview",
        "file_name" => "File name",
        "path" => "Path",
        "language" => "Language",
        "size" => "Size",
        "actions" => "Actions",
        "select_all" => "Select All",
        "url_placeholder" => "Paste AI conversation URL...",
        "archive_path" => "Archive path",
        "status_ready" => "Ready",
        "status_extracting" => "Extracting files...",
        "status_saving" => "Saving files...",
        "status_archiving" => "Creating archive...",
        "status_extracting_archive" => "Extracting archive...",
        "drag_drop_hint" => "Drop a text file here",
        "reset_learning" => "🔄 Reset Learning",
        "lang_ru" => "RU",
        "lang_en" => "EN",
        "confirm_overwrite" => "Some files already exist. Overwrite?",
        "learning_model_updated" => "Learning model updated",
        "error_no_files" => "No extracted files",
        "rename" => "✏️",
        "delete" => "❌",
        "version" => "Version 1.0.0",
        "save" => "Save",
        "cancel" => "Cancel",
        "rename_title" => "Rename File",
        "enter_new_name" => "Enter new file name",
        "overwrite_confirm" => "Overwrite Confirmation",
        "confirm" => "Confirm",
        "no_files_in_archive" => "No files in archive",
        "archive_created" => "Archive created",
        "archives" => "CodePack Archives (*.cpk)",
        "all_files" => "All Files",
        _ => key,
    }
}
