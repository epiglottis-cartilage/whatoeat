use crate::domain::{AppError, Result};
use std::path::PathBuf;

pub fn data_directory() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("WHATOEAT_DATA_DIR") {
        return Ok(PathBuf::from(path));
    }
    #[cfg(not(target_os = "android"))]
    {
        directories::ProjectDirs::from("", "whatoeat", "whatoeat")
            .map(|d| d.data_local_dir().to_path_buf())
            .ok_or_else(|| AppError::Invalid("无法确定应用数据目录。".into()))
    }
    #[cfg(target_os = "android")]
    {
        android::files_dir()
    }
}

/// Use the native Android clipboard, and the WebView clipboard on desktop.
/// Text is sent as data through the eval channel, never interpolated into JS.
pub async fn copy_text(text: String) -> Result<()> {
    #[cfg(target_os = "android")]
    {
        android::copy(text)
    }
    #[cfg(not(target_os = "android"))]
    {
        let mut eval = dioxus::document::eval(
            r#"
            const text = await dioxus.recv();
            let copied = false;
            try {
                if (navigator.clipboard) {
                    await navigator.clipboard.writeText(text);
                    copied = true;
                }
            } catch (_) {}
            if (!copied) {
                const previous = document.activeElement;
                const field = document.createElement('textarea');
                field.value = text;
                field.style.cssText = 'position:fixed;left:-9999px;top:0;';
                document.body.appendChild(field);
                field.select();
                try { copied = document.execCommand('copy'); } catch (_) {}
                field.remove();
                if (previous && previous.focus) previous.focus({preventScroll:true});
            }
            dioxus.send(copied);
        "#,
        );
        eval.send(text)
            .map_err(|e| AppError::Invalid(format!("无法访问剪贴板：{e}")))?;
        if eval
            .recv::<bool>()
            .await
            .map_err(|e| AppError::Invalid(format!("无法访问剪贴板：{e}")))?
        {
            Ok(())
        } else {
            Err(AppError::Invalid("自动复制未成功，请复制下方文本。".into()))
        }
    }
}

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use jni::{
        JavaVM,
        objects::{JObject, JString, JValue},
    };
    fn error(e: impl std::fmt::Display) -> AppError {
        AppError::Invalid(format!("安卓平台调用失败：{e}"))
    }
    pub fn files_dir() -> Result<PathBuf> {
        let context = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.map_err(error)?;
        let mut env = vm.attach_current_thread().map_err(error)?;
        let activity = unsafe { JObject::from_raw(context.context().cast()) };
        let file = env
            .call_method(&activity, "getFilesDir", "()Ljava/io/File;", &[])
            .map_err(error)?
            .l()
            .map_err(error)?;
        let path = env
            .call_method(file, "getAbsolutePath", "()Ljava/lang/String;", &[])
            .map_err(error)?
            .l()
            .map_err(error)?;
        let path: String = env.get_string(&JString::from(path)).map_err(error)?.into();
        Ok(PathBuf::from(path))
    }
    pub fn copy(text: String) -> Result<()> {
        let context = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.map_err(error)?;
        let mut env = vm.attach_current_thread().map_err(error)?;
        let activity = unsafe { JObject::from_raw(context.context().cast()) };
        let service_name = JObject::from(env.new_string("clipboard").map_err(error)?);
        let clipboard = env
            .call_method(
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_name)],
            )
            .map_err(error)?
            .l()
            .map_err(error)?;
        let label = JObject::from(env.new_string("今天吃什么").map_err(error)?);
        let text = JObject::from(env.new_string(text).map_err(error)?);
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&text)],
            )
            .map_err(error)?
            .l()
            .map_err(error)?;
        env.call_method(
            clipboard,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JValue::Object(&clip)],
        )
        .map_err(error)?;
        Ok(())
    }
}
