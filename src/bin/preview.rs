fn main() {
    let mut dom = dioxus::prelude::VirtualDom::new(whatoeat::ui::PreviewApp);
    dom.rebuild_in_place();
    println!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>今天吃什么 · 布局预览</title></head><body>{}</body></html>",
        dioxus::ssr::render(&dom)
    );
}
