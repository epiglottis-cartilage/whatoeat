fn main() {
    let language =
        whatoeat::i18n::Language::from_preferences([std::env::var("WHATOEAT_PREVIEW_LANG")
            .unwrap_or_else(|_| "zh-CN".into())
            .as_str()]);
    let mut dom = dioxus::prelude::VirtualDom::new(whatoeat::ui::PreviewApp);
    dom.rebuild_in_place();
    println!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>今天吃什么 · 布局预览</title></head><body>{}</body></html>",
        language.tag(),
        dioxus::ssr::render(&dom)
    );
}
