use crate::{
    domain::*,
    platform,
    store::{Command, Store},
    theme::Theme,
};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use dioxus::prelude::*;

const CSS: &str = include_str!("../assets/app.css");
const INTERFACE_CSS: &str = include_str!("../assets/themes/interface.css");

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Eat,
    History,
    Foods,
    Backup,
}
impl Screen {
    fn key(self) -> &'static str {
        match self {
            Self::Eat => "eat",
            Self::History => "history",
            Self::Foods => "foods",
            Self::Backup => "backup",
        }
    }
}
#[derive(Clone)]
struct ViewModel {
    data: Option<Data>,
    busy: bool,
    notice: Option<Notice>,
    screen: Screen,
    theme: Theme,
    theme_saving: bool,
}
impl Default for ViewModel {
    fn default() -> Self {
        Self {
            data: None,
            busy: false,
            notice: None,
            screen: Screen::Eat,
            theme: Theme::default(),
            theme_saving: false,
        }
    }
}
#[derive(Clone, PartialEq)]
struct Notice {
    id: String,
    message: String,
    is_error: bool,
}
impl Notice {
    fn new(message: &str) -> Option<Self> {
        (!message.is_empty()).then(|| Self {
            id: id(),
            message: message.into(),
            is_error: false,
        })
    }
}

impl ViewModel {
    fn notify(&mut self, message: &str) {
        self.notice = Notice::new(message);
    }

    fn notify_error(&mut self, message: &str) {
        self.notify(message);
        if let Some(notice) = &mut self.notice {
            notice.is_error = true;
        }
    }
}

#[derive(Clone)]
struct AppContext {
    vm: Signal<ViewModel>,
    store: std::result::Result<Store, String>,
}

fn now() -> i64 {
    Utc::now().timestamp_millis()
}
fn date(time: i64) -> String {
    DateTime::from_timestamp_millis(time)
        .map(|d| d.with_timezone(&Local).format("%m.%d · %H:%M").to_string())
        .unwrap_or_default()
}
fn input_date(time: i64) -> String {
    DateTime::from_timestamp_millis(time)
        .map(|d| d.with_timezone(&Local).format("%Y-%m-%dT%H:%M").to_string())
        .unwrap_or_default()
}
fn last_label(last: Option<i64>, at: i64) -> String {
    match last {
        None => "还没记录过，尝尝看？".into(),
        Some(t) => {
            let days = ((at - t).max(0) as f64 / DAY).floor() as i64;
            if days == 0 {
                "今天已经吃过了".into()
            } else {
                format!("上次吃，是 {days} 天前。")
            }
        }
    }
}

fn dispatch(ctx: AppContext, command: Command, notice: &str) {
    dispatch_then(ctx, command, notice, || {});
}

fn dispatch_then(
    mut ctx: AppContext,
    command: Command,
    notice: &str,
    on_success: impl FnOnce() + 'static,
) {
    if ctx.vm.read().busy {
        return;
    }
    let Ok(store) = ctx.store.clone() else {
        return;
    };
    let request = id();
    let notice = notice.to_string();
    {
        let mut vm = ctx.vm.write();
        vm.busy = true;
        vm.notice = None;
    }
    spawn(async move {
        let recovery = store.clone();
        let result =
            tokio::task::spawn_blocking(move || store.execute(&request, command, now())).await;
        match result {
            Ok(Ok(data)) => {
                let mut vm = ctx.vm.write();
                vm.data = Some(data);
                vm.notify(&notice);
                vm.busy = false;
                drop(vm);
                on_success();
            }
            other => {
                let error = match other {
                    Ok(Err(e)) => e.to_string(),
                    Err(e) => format!("操作未完成：{e}"),
                    _ => unreachable!(),
                };
                // Refresh from disk after errors so another window cannot strand a stale card.
                let data = tokio::task::spawn_blocking(move || recovery.load())
                    .await
                    .ok()
                    .and_then(|r| r.ok());
                let mut vm = ctx.vm.write();
                if data.is_some() {
                    vm.data = data;
                }
                vm.notify_error(&error);
                vm.busy = false;
            }
        }
    });
}

fn change_theme(mut ctx: AppContext, theme: Theme) {
    if ctx.vm.peek().theme_saving || ctx.vm.peek().theme == theme {
        return;
    }
    let previous = ctx.vm.peek().theme;
    ctx.vm.write().theme = theme;
    let Ok(store) = ctx.store.clone() else {
        return;
    };
    ctx.vm.write().theme_saving = true;
    spawn(async move {
        let result = tokio::task::spawn_blocking(move || store.save_theme(theme)).await;
        let mut vm = ctx.vm.write();
        vm.theme_saving = false;
        let error = match result {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error.to_string()),
            Err(error) => Some(error.to_string()),
        };
        if let Some(error) = error {
            vm.theme = previous;
            vm.notify_error(&format!("主题未能保存：{error}"));
        }
    });
}

#[component]
pub fn App() -> Element {
    let store = use_hook(|| {
        platform::data_directory()
            .map(|p| Store::new(p.join("whatoeat.sqlite3")))
            .map_err(|e| e.to_string())
    });
    let vm = use_signal(ViewModel::default);
    let ctx = AppContext { vm, store };
    let provider = ctx.clone();
    use_context_provider(|| provider);
    use_future(move || {
        let mut ctx = ctx.clone();
        async move {
            match ctx.store.clone() {
                Ok(store) => match tokio::task::spawn_blocking(move || {
                    let data = store.load()?;
                    let theme = store.load_theme()?;
                    Ok::<_, AppError>((data, theme))
                })
                .await
                {
                    Ok(Ok((data, theme))) => {
                        let mut vm = ctx.vm.write();
                        vm.data = Some(data);
                        vm.theme = theme;
                    }
                    Ok(Err(e)) => ctx.vm.write().notify_error(&e.to_string()),
                    Err(e) => ctx.vm.write().notify_error(&e.to_string()),
                },
                Err(e) => ctx.vm.write().notify_error(&e),
            }
        }
    });
    rsx! { Shell {} }
}

/// Render the actual components with explicit sample state for layout review.
#[cfg(feature = "preview")]
#[component]
pub fn PreviewApp() -> Element {
    let vm = use_signal(|| {
        let mut data = Data::default();
        let key = data.add_food("牛肉面").unwrap();
        data.add_food("麻辣烫").unwrap();
        data.add_food("咖喱饭").unwrap();
        data.save_meal(None, &key, now() - 8 * 86_400_000, now())
            .unwrap();
        data.start(now(), 0.0, false).unwrap();
        if let Decision::Ready { candidate, .. } = data.decision.clone() {
            data.answer(&candidate.id, false, now(), 0.0).unwrap();
            data.start(now(), 0.0, true).unwrap();
        }
        let state = std::env::var("WHATOEAT_PREVIEW_STATE").unwrap_or_default();
        match state.as_str() {
            "empty" => data = Data::default(),
            "exhausted" => data.decision = Decision::Exhausted,
            "idle" => data.decision = Decision::Idle,
            "accepted" => {
                if let Decision::Ready { candidate, .. } = data.decision.clone() {
                    data.answer(&candidate.id, true, now(), 0.0).unwrap();
                }
            }
            "stale" => {
                if let Decision::Ready { candidate, .. } = &mut data.decision {
                    candidate.shown_at -= 3 * 60 * 60 * 1000;
                }
            }
            "long" => {
                for index in 0..12 {
                    let food_id = data.foods[index % 3].id.clone();
                    data.save_meal(
                        None,
                        &food_id,
                        now() - (index as i64 + 1) * 3_600_000,
                        now(),
                    )
                    .unwrap();
                }
                for (index, food) in data.foods.iter_mut().enumerate() {
                    food.name = format!("超长菜名测试·番茄牛腩土豆胡萝卜菌菇手工拉面套餐{index}");
                }
                data.start(now(), 0.0, true).unwrap();
            }
            _ => {}
        }
        let screen = match std::env::var("WHATOEAT_PREVIEW_SCREEN").as_deref() {
            Ok("foods") => Screen::Foods,
            Ok("history") => Screen::History,
            Ok("backup") => Screen::Backup,
            _ => Screen::Eat,
        };
        ViewModel {
            data: (state != "loading").then_some(data),
            busy: state == "busy",
            screen,
            theme: std::env::var("WHATOEAT_PREVIEW_THEME")
                .ok()
                .as_deref()
                .and_then(Theme::from_key)
                .unwrap_or_default(),
            notice: match std::env::var("WHATOEAT_PREVIEW_TOAST").as_deref() {
                Ok("1") => Notice::new("菜单已更新。"),
                Ok("error") => Some(Notice {
                    id: id(),
                    message: "记录未能保存，请稍后重试。".into(),
                    is_error: true,
                }),
                _ => None,
            },
            ..ViewModel::default()
        }
    });
    use_context_provider(|| AppContext {
        vm,
        store: Err("预览模式".into()),
    });
    rsx! { Shell {} }
}

#[component]
fn Shell() -> Element {
    let mut ctx = use_context::<AppContext>();
    let vm = ctx.vm.read().clone();
    let count = vm.data.as_ref().map(Data::eaten_count).unwrap_or(0);
    let theme_ctx = ctx.clone();
    let interface = vm.theme.is_interface();
    let scene = vm.theme.scene_svg();
    rsx! {
        style { {CSS} }
        style { {INTERFACE_CSS} }
        style { {vm.theme.stylesheet()} }
        div { class:"theme-root", "data-theme":vm.theme.key(), "data-layout":if interface {"interface"}else{"palette"}, "data-screen":vm.screen.key(),
        if interface {
            // These are bundled, original decorative SVGs, never imported user content.
            div { class:"scene-backdrop", "aria-hidden":"true", dangerous_inner_html:scene }
        }
        // All pages share this viewport-level notification outlet.
        if let Some(notice) = vm.notice.clone() { Toast { key:"{notice.id}", notice } }
        div { class:"app-shell",
            header { class:"masthead",
                div { class:"brand-block",
                    if interface { span { class:"brand-symbol", "aria-hidden":"true", "W" } }
                    div {
                        div { class:"wordmark", "WHAT", span { "/" }, "TO", span { "/" }, "EAT" }
                        if interface { div { class:"brand-subtitle", {vm.theme.description()} } }
                    }
                }
                if interface {
                    div { class:"shell-status", span { class:"connection-dot", "aria-hidden":"true" }, span { "本地记录" }, strong { "{count:02}" } }
                }
                label { class:"theme-picker", r#for:"theme-select",
                    span { class:"theme-picker-label", "主题" }
                    select { id:"theme-select", class:"theme-select", "aria-label":"选择主题", value:vm.theme.key(), disabled:vm.theme_saving || vm.data.is_none(),
                        onchange:move |event| {
                            if let Some(theme) = Theme::from_key(&event.value()) { change_theme(theme_ctx.clone(),theme); }
                        },
                        optgroup { label:"界面主题",
                            for theme in Theme::INTERFACES { option { value:theme.key(), selected:vm.theme == theme, {theme.label()} } }
                        }
                        optgroup { label:"配色样式",
                            for theme in Theme::PALETTES { option { value:theme.key(), selected:vm.theme == theme, {theme.label()} } }
                        }
                    }
                }
            }
            div { class:"workspace",
                nav { class:"nav", "aria-label":"主要页面",
                    for (screen, label, number) in [(Screen::Eat,"今天吃什么","01"),(Screen::History,"吃过的日子","02"),(Screen::Foods,"我的菜单","03"),(Screen::Backup,"复制与合并","04")] {
                        button { disabled:vm.busy, class:if vm.screen==screen {"nav-item active"}else{"nav-item"},
                            "aria-current":if vm.screen==screen {"page"}else{"false"},
                            onclick:move |_| { let mut state=ctx.vm.write(); state.screen=screen; state.notice = None; },
                            if interface { NavigationIcon { screen } }
                            small { {number} }, span { {label} }, span { class:"nav-arrow", "↗" }
                        }
                    }
                    div { class:"nav-note", p { "先吃饭。" }, p { "其他的，" }, p { "等会再说。" }, span { "已有 {count} 顿好好吃饭的记录" } }
                }
                main { class:"main", "aria-busy":vm.busy.to_string(),
                    if vm.data.is_none() {
                        section { class:"empty", h1 { "正在摆桌…" }, p { "读取这台设备的饮食记录。" } }
                    } else {
                        match vm.screen {
                            Screen::Eat => rsx!{ EatScreen {} },
                            Screen::History => rsx!{ HistoryScreen {} },
                            Screen::Foods => rsx!{ FoodsScreen {} },
                            Screen::Backup => rsx!{ BackupScreen {} },
                        }
                    }
                }
            }
            footer { class:"footer", span { "少点纠结，多吃两口。" }, span { "只存在这台设备 · 离线也开饭" } }
        }
        }
    }
}

#[component]
fn NavigationIcon(screen: Screen) -> Element {
    let path = match screen {
        Screen::Eat => "M5 3v6m3-6v6M3 3v4a3 3 0 0 0 6 0V3M6 10v11M17 3c-3 3-4 7-4 9h5m0-9v18",
        Screen::History => "M12 8v5l3 2M8 3H3v5M3.8 7a9 9 0 1 1-.4 9",
        Screen::Foods => "M8 5h12M8 12h12M8 19h12M3 5h1M3 12h1M3 19h1",
        Screen::Backup => "M4 14v6h16v-6M12 3v12m-5-7 5-5 5 5",
    };
    rsx! { svg { class:"nav-icon", width:"24", height:"24", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"1.6", stroke_linecap:"square", "aria-hidden":"true", path { d:path } } }
}

#[component]
fn InterfaceOverview(theme: Theme, data: Data) -> Element {
    let current = Local::now();
    let month = current.format("%Y / %m").to_string();
    let day = current.format("%d").to_string();
    let date_label = format!("今天 {month} / {day}");
    let active = data.menu().filter(|food| food.enabled).count();
    let count = data.eaten_count();
    let (kicker, title, second_line, copy, caption) = match theme {
        Theme::Automata => (
            "ARCHIVE : DAILY MEALS",
            "日常记录",
            " / 持续更新",
            "每一个普通的日子，都有值得记住的一餐。",
            "记录是为了生活，偶尔忘记也没有关系。",
        ),
        Theme::Reclamation => (
            "DAILY CAMP / 日常营地",
            "休整片刻，",
            "好好吃饭。",
            "今天的补给，从一顿喜欢的开始。",
            "停一停，也是一种前进。",
        ),
        Theme::Expedition => (
            "MEAL JOURNEY / 饮食旅程",
            "每一顿，",
            "都是新一站。",
            "不必计划很远，先决定这一餐。",
            "走过的日子，都在记录里。",
        ),
        _ => (
            "DAILY LIFE / 日常中枢",
            "日常，",
            "也值得认真。",
            "把选择交给直觉，把今天留给生活。",
            "用一顿好饭，为日常充能。",
        ),
    };
    rsx! {
        aside { class:"interface-overview", "aria-label":"今日概览",
            p { class:"overview-kicker", {kicker} }
            h2 { class:"overview-title", {title}, br {}, span { {second_line} } }
            p { class:"overview-copy", {copy} }
            div { class:"overview-visual", "aria-hidden":"true",
                svg { view_box:"0 0 240 240", fill:"none",
                    circle { cx:"120", cy:"120", r:"91", stroke:"currentColor", stroke_width:"1" }
                    circle { cx:"120", cy:"120", r:"75", stroke:"currentColor", stroke_width:".5", stroke_dasharray:"2 7" }
                    path { d:"M39 134h162c-9 36-37 58-81 58s-72-22-81-58ZM33 127h174M90 107c-17-20 16-29 0-49m30 49c-17-20 16-29 0-49m30 49c-17-20 16-29 0-49", stroke:"currentColor", stroke_width:"2.5" }
                    path { d:"M12 120h16m184 0h16M120 12v16m0 184v16", stroke:"currentColor", stroke_width:"1" }
                }
            }
            div { class:"day-dial", "aria-label":date_label,
                span { class:"day-month", {month} }
                strong { class:"day-number", {day} }
                span { class:"day-caption", "今天" }
            }
            div { class:"overview-stats",
                div { class:"overview-stat", strong { "{active:02}" }, span { "可选菜单" } }
                div { class:"overview-stat", strong { "{count:02}" }, span { "已记餐次" } }
            }
            p { class:"overview-caption", {caption} }
        }
    }
}

#[component]
fn EatScreen() -> Element {
    let ctx = use_context::<AppContext>();
    let vm = ctx.vm.read().clone();
    let data = vm.data.unwrap();
    let active = data.menu().filter(|f| f.enabled).count();
    let today = Local::now().format("%m / %d").to_string();
    rsx! {
        div { class:"eat-layout",
        if vm.theme.is_interface() { InterfaceOverview { theme:vm.theme, data:data.clone() } }
        div { class:"eat-console",
        div { class:"page-kicker", span { "THE DAILY DILEMMA" }, span { "{today} · 今天也要吃饱" } }
        div { class:"page-heading", h1 { "是啊，", em { "吃什么？" } }, span { class:"tiny-stamp", "听胃的！" } }
        match data.decision.clone() {
            Decision::Ready { candidate, .. } if now() < candidate.shown_at || now() - candidate.shown_at >= 2*60*60*1000 => {
                let restart_ctx=ctx.clone();
                rsx! { section { class:"empty illustrated", h2 { "又到饭点了？" }, p { "上次的候选已超过两小时，重新挑一个吧。" },
                    button { class:"button primary", disabled:vm.busy, onclick:move |_|dispatch(restart_ctx.clone(),Command::Start{new_round:true},""), "重新开始 ↗" }
                } }
            },
            Decision::Ready { candidate, seen, .. } => {
                let food=data.food(&candidate.food_id).unwrap();
                let name=food.name.clone();
                let name_class=if name.chars().count()>5 {"food-name long"} else {"food-name"};
                let label=last_label(candidate.last_eaten,candidate.shown_at);
                let round=seen.len()+1;
                let skip_ctx=ctx.clone(); let eat_ctx=ctx.clone();
                let skip_id=candidate.id.clone(); let eat_id=candidate.id.clone();
                rsx! {
                    article { class:"food-ticket", key:"{candidate.id}", "data-watermark":"TODAY", "aria-label":"当前推荐",
                        div { class:"ticket-top", span { "本轮第 {round:02} 位" }, span { "TODAY’S PICK ↙" } }
                        h2 { class:name_class, {name} }
                        div { class:"ticket-bottom", p { {label} }, span { class:"food-sticker", "就差你点头！" } }
                        span { class:"ticket-edge", "EAT WELL / FEEL GOOD / REPEAT" }
                    }
                    div { class:"decisions",
                        button { class:"decision skip", disabled:vm.busy, onclick:move |_| dispatch(skip_ctx.clone(),Command::Answer{recommendation_id:skip_id.clone(),eat:false},"这轮先跳过，看看下一位。"),
                            strong { "不吃 ↗" }, span { "换一个，继续挑" }
                        }
                        button { class:"decision eat", disabled:vm.busy, onclick:move |_| dispatch(eat_ctx.clone(),Command::Answer{recommendation_id:eat_id.clone(),eat:true},""),
                            strong { if vm.busy {"稍等…"}else{"吃！"} }, span { "就它了，记入这顿" }
                        }
                    }
                    div { class:"small-print", span { "一次一个，凭胃决定。" }, span { "{active} 个选项 · 吃过也能撤销" } }
                }
            },
            Decision::Accepted { meal_id,food_id } => {
                let name=data.food(&food_id).map(|f| f.name.clone()).unwrap_or_default();
                let undo_ctx=ctx.clone(); let next_ctx=ctx.clone();
                rsx! {
                    section { class:"celebration", div { class:"accepted-stamp", "开饭！" }, h2 { {name} }, p { "已记入这顿。趁热吃，慢慢来。" },
                        div { class:"inline-actions",
                            button { class:"button", disabled:vm.busy, onclick:move |_| dispatch(undo_ctx.clone(),Command::RemoveMeal{meal_id:meal_id.clone()},"这次记录和学习反馈已撤销。"), "点错了，撤销" }
                            button { class:"button primary", disabled:vm.busy, onclick:move |_| dispatch(next_ctx.clone(),Command::Start{new_round:true},""), "下一顿再挑 ↗" }
                        }
                    }
                }
            },
            Decision::Exhausted if active>0 => {
                let start_ctx=ctx.clone(); let mut nav_ctx=ctx.clone();
                rsx! { section { class:"empty illustrated", span { class:"empty-eyebrow", "NO RUSH, NO PRESSURE" }, h2 { "这轮都", br {}, "不想吃。" }, p { "那就先歇一下。也可以记下菜单之外的选择。" },
                    div { class:"inline-actions",
                        button { class:"button primary", disabled:vm.busy, onclick:move |_| dispatch(start_ctx.clone(),Command::Start{new_round:true},"新的一轮，重新开始。"), "再挑一轮 ↗" }
                        button { class:"button", onclick:move |_| nav_ctx.vm.write().screen=Screen::History, "手动记一顿" }
                    }
                } }
            },
            _ if active==0 => {
                let add_ctx=ctx.clone(); let mut nav_ctx=ctx.clone();
                rsx! { section { class:"empty illustrated", span { class:"empty-eyebrow", "YOUR MENU, YOUR RULES" }, h2 { "胃已就位。", br {}, "菜单呢？" }, p { "先放进几样你爱吃的，我们再来决定今天吃什么。" },
                    div { class:"inline-actions",
                        button { class:"button primary", disabled:vm.busy, onclick:move |_| dispatch(add_ctx.clone(),Command::AddStarters,"已加入六个常见选项，没有生成任何饮食历史。"), "加入六个常见选项 ↗" }
                        button { class:"button", onclick:move |_| nav_ctx.vm.write().screen=Screen::Foods, "自己写菜单" }
                    }
                } }
            },
            _ => {
                let start_ctx=ctx.clone();
                rsx! { section { class:"empty illustrated", span { class:"empty-eyebrow", "GOOD FOOD IS A GOOD IDEA" }, h2 { "纠结暂停。", br {}, "准备开饭！" }, p { "从你的 {active} 个选项里，挑一个现在想吃的。" },
                    button { class:"button primary big", disabled:vm.busy, onclick:move |_| dispatch(start_ctx.clone(),Command::Start{new_round:true},""), "今天就靠你了 ↗" }
                } }
            }
        }
        }
        }
    }
}

#[component]
fn Toast(notice: Notice) -> Element {
    let mut ctx = use_context::<AppContext>();
    let mut timer_ctx = ctx.clone();
    let notice_id = notice.id.clone();
    let is_error = notice.is_error;
    use_future(move || {
        let notice_id = notice_id.clone();
        async move {
            // Errors remain readable until dismissed or superseded.
            if is_error {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if timer_ctx
                .vm
                .peek()
                .notice
                .as_ref()
                .is_some_and(|n| n.id == notice_id)
            {
                timer_ctx.vm.write().notice = None;
            }
        }
    });
    rsx! {
        div { class:if is_error {"toast error"} else {"toast"}, role:if is_error {"alert"} else {"status"}, "aria-live":if is_error {"assertive"} else {"polite"}, "aria-atomic":"true",
            span { class:"toast-mark", if is_error {"!"} else {"✓"} }
            p { "{notice.message}" }
            button { class:"toast-close", "aria-label":"关闭提示", onclick:move |_|ctx.vm.write().notice=None, "×" }
        }
    }
}

#[component]
fn FoodsScreen() -> Element {
    let ctx = use_context::<AppContext>();
    let vm = ctx.vm.read().clone();
    let data = vm.data.unwrap();
    let mut name = use_signal(String::new);
    let add_ctx = ctx.clone();
    rsx! {
        div { class:"page-kicker", "THE GOOD FOOD LIST" }
        h1 { class:"section-title", "我的", em { "菜单。" } }
        p { class:"lede", "把想吃的放进来。不想被推荐时，先让它休息。" }
        form { class:"add-form", onsubmit:move |_| {
            let value=name();
            if !value.trim().is_empty() { dispatch_then(add_ctx.clone(),Command::AddFood{name:value},"菜单已更新。",move ||name.set(String::new())); }
        },
            label { class:"sr-only", r#for:"new-food", "食物名称" }
            input { id:"new-food", placeholder:"比如：番茄牛腩饭", maxlength:32, disabled:vm.busy, value:"{name}", oninput:move |e|name.set(e.value()) }
            button { class:"button primary", r#type:"submit", disabled:vm.busy, "加入菜单 +" }
        }
        div { class:"food-list",
            if data.menu().next().is_none() { p { class:"quiet", "还没有用餐选项。先写下一个吧。" } }
            for (index, food) in data.menu().enumerate() {
                FoodRow { key:"{food.id}", food:food.clone(), index }
            }
        }
        p { class:"footnote", "参考周期表示预测想吃概率达到 70% 的时间；反馈较少时，它仍主要来自初始设定。" }
    }
}

#[component]
fn FoodRow(food: Food, index: usize) -> Element {
    let ctx = use_context::<AppContext>();
    let busy = ctx.vm.read().busy;
    let mut editing = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut deleting = use_signal(|| false);
    let toggle_ctx = ctx.clone();
    let rename_ctx = ctx.clone();
    let delete_ctx = ctx.clone();
    let toggle_id = food.id.clone();
    let rename_id = food.id.clone();
    let delete_id = food.id.clone();
    let original_name = food.name.clone();
    let samples = food.model.samples;
    let cycle = food
        .model
        .interval()
        .map(|t| format!("约 {t:.1} 天"))
        .unwrap_or("超出 180 天".into());
    rsx! {
        article { class:if food.enabled {"food-row"}else{"food-row paused"},
            span { class:"row-number", "{index+1:02}" }
            div { class:"food-details",
                if editing() {
                    form { class:"rename-form", onsubmit:move |_|dispatch_then(rename_ctx.clone(),Command::RenameFood{food_id:rename_id.clone(),name:draft()},"名称已更新。",move ||editing.set(false)),
                        label { class:"sr-only", r#for:"rename-{food.id}", "新的食物名称" }
                        input { id:"rename-{food.id}", maxlength:32, required:true, disabled:busy, value:"{draft}", oninput:move |e|draft.set(e.value()) }
                        div { class:"food-actions",
                            button { class:"text-button", r#type:"submit", disabled:busy || draft().trim().is_empty(), "保存" }
                            button { class:"text-button", r#type:"button", disabled:busy, onclick:move |_|editing.set(false), "取消" }
                        }
                    }
                } else {
                    div { class:"row-main", h3 { "{food.name}" } }
                }
            }
            div { class:"food-metrics",
                if samples==0 { p { "还在认识你的口味" } }
                else { p { "参考周期 {cycle}" }, p { "{samples} 次记录" } }
            }
            div { class:"food-controls",
                button { class:"toggle", disabled:busy, "aria-pressed":food.enabled.to_string(), onclick:move |_|dispatch(toggle_ctx.clone(),Command::SetEnabled{food_id:toggle_id.clone(),enabled:!food.enabled},"推荐范围已更新。"),
                    if food.enabled {"推荐中"}else{"已休息"}
                }
                if !editing() && !deleting() {
                    div { class:"food-actions",
                        button { class:"text-button", disabled:busy, onclick:move |_|{draft.set(original_name.clone());editing.set(true);}, "改名" }
                        button { class:"text-button danger", disabled:busy, onclick:move |_|deleting.set(true), "删除" }
                    }
                }
            }
            if deleting() {
                div { class:"food-delete-confirm",
                    p { "从菜单删除？已有饮食记录会保留。" }
                    div { class:"food-actions",
                        button { class:"text-button danger", disabled:busy, onclick:move |_|dispatch(delete_ctx.clone(),Command::DeleteFood{food_id:delete_id.clone()},"已从菜单删除，饮食记录仍保留。"), "确认删除" }
                        button { class:"text-button", disabled:busy, onclick:move |_|deleting.set(false), "取消" }
                    }
                }
            }
        }
    }
}

#[component]
fn HistoryScreen() -> Element {
    let ctx = use_context::<AppContext>();
    let vm = ctx.vm.read().clone();
    let data = vm.data.unwrap();
    let initial = data.menu().next().map(|f| f.id.clone()).unwrap_or_default();
    let mut food = use_signal(|| initial.clone());
    let save_default = initial.clone();
    let cancel_default = initial;
    let mut at = use_signal(|| input_date(now()));
    let mut editing = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<String>);
    let mut meals: Vec<_> = data.meals.iter().filter(|m| !m.deleted).cloned().collect();
    meals.sort_by_key(|m| std::cmp::Reverse(m.eaten_at));
    let mut save_ctx = ctx.clone();
    rsx! {
        div { class:"page-kicker", "A LITTLE DIARY OF GOOD MEALS" }
        h1 { class:"section-title", "吃过的", em { "日子。" } }
        p { class:"lede", "每一顿都算数。忘了记也没关系，补上就好。" }
        if data.menu().next().is_none() && editing().is_none() { p { class:"message", "先去「我的菜单」添加一个选项，再来记录。" } }
        else {
            form { class:"meal-form", onsubmit:move |_| {
                let parsed=NaiveDateTime::parse_from_str(&at(),"%Y-%m-%dT%H:%M").ok().and_then(|d|Local.from_local_datetime(&d).single());
                if let Some(time)=parsed { if time.timestamp_millis()<=now() {
                    let next_food=save_default.clone();
                    dispatch_then(save_ctx.clone(),Command::SaveMeal{meal_id:editing(),food_id:food(),eaten_at:time.timestamp_millis()},"饮食记录已保存，相关模型已重新计算。",move ||{editing.set(None);food.set(next_food);});
                } else { save_ctx.vm.write().notify_error("不能记录未来的食用时间。"); } }
                else { save_ctx.vm.write().notify_error("请选择有效的本地时间。"); }
            },
                h2 { if editing().is_some() {"修改这一顿"}else{"补记一顿"} }
                div { class:"form-fields",
                    label { "吃了什么", select { value:"{food}", oninput:move |e|food.set(e.value()),
                        for option in data.foods.iter().filter(|f| !f.deleted || (editing().is_some() && f.id == food())) { option { value:"{option.id}", "{option.name}" } }
                    } }
                    label { "什么时候", input { r#type:"datetime-local", step:"60", value:"{at}", max:input_date(now()), oninput:move |e|at.set(e.value()) } }
                }
                div { class:"inline-actions",
                    button { class:"button primary", r#type:"submit", disabled:vm.busy, "保存这一顿 ↗" }
                    if editing().is_some() { button { class:"button", r#type:"button", disabled:vm.busy, onclick:move |_|{editing.set(None);food.set(cancel_default.clone());at.set(input_date(now()));}, "取消修改" } }
                }
            }
        }
        div { class:"history-list",
            if meals.is_empty() { p { class:"quiet", "第一顿记录，等你开饭。" } }
            for meal in meals {
                { let name=data.food(&meal.food_id).map(|f|f.name.clone()).unwrap_or_default(); let time=date(meal.eaten_at);
                  let edit_id=meal.id.clone(); let food_id=meal.food_id.clone(); let edit_at=meal.eaten_at;
                  let delete_id=meal.id.clone(); let confirmed=confirm_delete().as_deref()==Some(meal.id.as_str()); let delete_ctx=ctx.clone();
                  rsx! { article { class:"history-row", key:"{meal.id}",
                      if vm.theme.is_interface() { span { class:"history-node", "aria-hidden":"true" } }
                      div { class:"row-main", small { {time} }, h3 { {name} }, span { class:"source", if meal.feedback_id.is_some(){"选了就吃"}else{"手动记录"} } }
                      div { class:"row-actions",
                          button { class:"text-button", onclick:move |_|{editing.set(Some(edit_id.clone()));food.set(food_id.clone());at.set(input_date(edit_at));}, "修改" }
                          if confirmed {
                              button { class:"text-button danger", disabled:vm.busy, onclick:move |_|{dispatch(delete_ctx.clone(),Command::RemoveMeal{meal_id:delete_id.clone()},"记录已删除，相关学习已重建。");confirm_delete.set(None);}, "确认删除" }
                              button { class:"text-button", onclick:move |_|confirm_delete.set(None), "取消" }
                          } else { button { class:"text-button", onclick:move |_|confirm_delete.set(Some(delete_id.clone())), "删除" } }
                      }
                  } }
                }
            }
        }
    }
}

#[component]
fn BackupScreen() -> Element {
    let mut ctx = use_context::<AppContext>();
    let vm = ctx.vm.read().clone();
    let mut text = use_signal(String::new);
    let mut fallback = use_signal(String::new);
    let mut copying = use_signal(|| false);
    let merge_ctx = ctx.clone();
    let clear_ctx = ctx.clone();
    let mut confirm_clear = use_signal(|| false);
    rsx! {
        div { class:"page-kicker", "COPY / PASTE / KEEP EATING" }
        h1 { class:"section-title", "复制，", em { "就存好。" } }
        p { class:"lede", "把记录复制到你喜欢的地方。换台设备，粘贴回来就能合并。" }
        section { class:"backup-panel",
            h2 { "复制当前记录" }
            p { "包含菜单、饮食记录和学习反馈。" }
            button { class:"button primary", disabled:vm.busy || copying(), onclick:move |_| {
                let exported=ctx.vm.peek().data.as_ref().unwrap().export_text();
                match exported {
                    Ok(exported)=>{
                        copying.set(true);
                        spawn(async move {
                            match platform::copy_text(exported.clone()).await {
                                Ok(())=>{fallback.set(String::new());ctx.vm.write().notify("记录已复制，可以粘贴保存了。");}
                                Err(e)=>{fallback.set(exported);ctx.vm.write().notify_error(&e.to_string());}
                            }
                            copying.set(false);
                        });
                    }
                    Err(e)=>ctx.vm.write().notify_error(&e.to_string()),
                }
            }, if copying(){"正在复制…"}else{"复制当前记录 ↗"} }
            if !fallback().is_empty() {
                label { r#for:"export-text", "手动复制以下文本" }
                textarea { id:"export-text", rows:5, readonly:true, value:"{fallback}" }
            }
        }
        section { class:"backup-panel",
            h2 { "粘贴并合并" }
            p { "默认增量覆盖：同名菜单、同名且同一分钟的记录以导入内容为准，其他本地数据保留。" }
            label { class:"sr-only", r#for:"backup-text", "粘贴记录文本" }
            textarea { id:"backup-text", rows:6, placeholder:"在这里粘贴复制的记录…", disabled:vm.busy, value:"{text}", oninput:move |e|text.set(e.value()) }
            button { class:"button primary", disabled:vm.busy || text().trim().is_empty(), onclick:move |_| {
                dispatch_then(merge_ctx.clone(),Command::Merge{text:text()},"记录已增量合并，重复项已按导入内容更新。",move ||text.set(String::new()));
            }, "合并记录 ↗" }
        }
        section { class:"backup-panel",
            h2 { "清空全部数据" }
            p { "删除这台设备的菜单、饮食记录和学习反馈，重新开始。" }
            if confirm_clear() {
                div { class:"restore-confirm",
                    p { "确定清空？此操作无法撤销，需要保留的记录请先复制保存。" }
                    div { class:"inline-actions",
                        button { class:"button danger-button", disabled:vm.busy || copying(), onclick:move |_|dispatch_then(clear_ctx.clone(),Command::Clear,"本机数据已清空。",move ||{text.set(String::new());fallback.set(String::new());confirm_clear.set(false);}), "确认清空全部数据" }
                        button { class:"button", disabled:vm.busy, onclick:move |_|confirm_clear.set(false), "取消" }
                    }
                }
            } else {
                button { class:"button danger-button", disabled:vm.busy || copying(), onclick:move |_|confirm_clear.set(true), "清空全部数据" }
            }
        }
        p { class:"footnote", "名称忽略首尾空格和英文字母大小写。食用时间保存到分钟；同名、同一分钟算一条记录。支持旧版备份文本。" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc, time::Duration};

    type ViewHandle = Rc<RefCell<Option<Signal<ViewModel>>>>;

    fn toast_harness() -> Element {
        let handle = use_context::<ViewHandle>();
        let store = try_consume_context::<Store>();
        let vm = use_signal(|| ViewModel {
            data: Some(
                store
                    .as_ref()
                    .map(|s| s.load().unwrap())
                    .unwrap_or_default(),
            ),
            notice: Notice::new("菜单已更新。"),
            ..ViewModel::default()
        });
        use_hook(move || *handle.borrow_mut() = Some(vm));
        use_context_provider(|| AppContext {
            vm,
            store: store.ok_or_else(|| "test".into()),
        });
        rsx! { Shell {} }
    }

    async fn pump(dom: &mut VirtualDom, duration: Duration) {
        let _ = tokio::time::timeout(duration, async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate(&mut dioxus::core::NoOpMutations);
            }
        })
        .await;
    }

    #[test]
    fn theme_switch_is_immediate_persists_and_rolls_back_on_save_failure() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(directory.path().join("theme.sqlite3"));
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let handle: ViewHandle = Rc::new(RefCell::new(None));
                let mut dom = VirtualDom::new(toast_harness);
                dom.provide_root_context(handle.clone());
                dom.provide_root_context(store.clone());
                dom.rebuild_in_place();
                let vm = (*handle.borrow()).unwrap();
                dom.in_scope(ScopeId::ROOT, || {
                    change_theme(
                        AppContext {
                            vm,
                            store: Ok(store.clone()),
                        },
                        Theme::Rhodes,
                    )
                });
                assert_eq!(vm.peek().theme, Theme::Rhodes);
                assert!(vm.peek().theme_saving);
                tokio::time::timeout(Duration::from_secs(5), async {
                    while vm.peek().theme_saving {
                        dom.wait_for_work().await;
                        dom.render_immediate(&mut dioxus::core::NoOpMutations);
                    }
                })
                .await
                .unwrap();
                assert_eq!(store.load_theme().unwrap(), Theme::Rhodes);
                assert_eq!(vm.peek().theme, Theme::Rhodes);

                let blocker = directory.path().join("not-a-directory");
                std::fs::write(&blocker, "test").unwrap();
                let broken = Store::new(blocker.join("db.sqlite3"));
                dom.in_scope(ScopeId::ROOT, || {
                    change_theme(
                        AppContext {
                            vm,
                            store: Ok(broken),
                        },
                        Theme::Rhine,
                    )
                });
                assert_eq!(vm.peek().theme, Theme::Rhine);
                tokio::time::timeout(Duration::from_secs(5), async {
                    while vm.peek().theme_saving {
                        dom.wait_for_work().await;
                        dom.render_immediate(&mut dioxus::core::NoOpMutations);
                    }
                })
                .await
                .unwrap();
                assert_eq!(vm.peek().theme, Theme::Rhodes);
                assert!(vm.peek().notice.as_ref().unwrap().is_error);
                assert_eq!(store.load_theme().unwrap(), Theme::Rhodes);
            });
    }

    #[test]
    fn undo_uses_global_toast_and_expires_after_the_page_changes() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(directory.path().join("toast.sqlite3"));
        let mut data = Data::default();
        let food_id = data.add_food("饺子").unwrap();
        data.save_meal(None, &food_id, now() - 7 * 86_400_000, now())
            .unwrap();
        store
            .execute(
                &id(),
                Command::Restore {
                    backup: data.backup().unwrap(),
                },
                now(),
            )
            .unwrap();
        let ready = store
            .execute(&id(), Command::Start { new_round: true }, now())
            .unwrap();
        let Decision::Ready { candidate, .. } = ready.decision else {
            panic!("expected candidate")
        };
        let accepted = store
            .execute(
                &id(),
                Command::Answer {
                    recommendation_id: candidate.id,
                    eat: true,
                },
                now(),
            )
            .unwrap();
        assert_eq!(accepted.food(&food_id).unwrap().model.samples, 1);
        let Decision::Accepted { meal_id, .. } = accepted.decision else {
            panic!("expected accepted meal")
        };

        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let handle: ViewHandle = Rc::new(RefCell::new(None));
                let mut dom = VirtualDom::new(toast_harness);
                dom.provide_root_context(handle.clone());
                dom.provide_root_context(store.clone());
                dom.rebuild_in_place();
                let vm = (*handle.borrow()).unwrap();
                dom.in_scope(ScopeId::ROOT, || {
                    dispatch(
                        AppContext {
                            vm,
                            store: Ok(store.clone()),
                        },
                        Command::RemoveMeal { meal_id },
                        "这次记录和学习反馈已撤销。",
                    )
                });
                // Wait for the actual SQLite command, rendering the transition out
                // of the accepted screen, before checking the global notification.
                tokio::time::timeout(Duration::from_secs(5), async {
                    while vm.peek().busy {
                        dom.wait_for_work().await;
                        dom.render_immediate(&mut dioxus::core::NoOpMutations);
                    }
                })
                .await
                .unwrap();
                {
                    let state = vm.peek();
                    let notice = state.notice.as_ref().unwrap();
                    assert_eq!(notice.message, "这次记录和学习反馈已撤销。");
                    assert!(!notice.is_error);
                    let data = state.data.as_ref().unwrap();
                    assert_eq!(data.eaten_count(), 1);
                    assert_eq!(data.food(&food_id).unwrap().model.samples, 0);
                    assert!(!matches!(data.decision, Decision::Accepted { .. }));
                }
                pump(&mut dom, Duration::from_millis(3300)).await;
                assert!(vm.peek().notice.is_none());
            });
    }

    #[test]
    fn repeated_toast_gets_its_own_lifetime_and_then_disappears() {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let handle: ViewHandle = Rc::new(RefCell::new(None));
                let mut dom = VirtualDom::new(toast_harness);
                dom.provide_root_context(handle.clone());
                dom.rebuild_in_place();
                let mut vm = (*handle.borrow()).unwrap();
                let first_id = vm.peek().notice.as_ref().unwrap().id.clone();
                pump(&mut dom, Duration::from_secs(2)).await;
                assert!(vm.peek().notice.is_some());
                // The same text must start a fresh timer rather than inheriting
                // the previous operation's nearly-expired timer.
                dom.in_runtime(|| vm.write().notice = Notice::new("菜单已更新。"));
                pump(&mut dom, Duration::from_millis(1300)).await;
                assert_ne!(vm.peek().notice.as_ref().unwrap().id, first_id);
                pump(&mut dom, Duration::from_millis(2000)).await;
                assert!(vm.peek().notice.is_none());
            });
    }
}
