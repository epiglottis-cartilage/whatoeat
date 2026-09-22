# Wry 0.53.5 calls these from Rust through JNI. Its generated proguard-wry.pro
# already keeps the activity, WebView constructors, IPC and other JNI methods.
-keepclassmembers class dev.dioxus.main.RustWebView {
    void clearAllBrowsingData();
    java.lang.String getCookies(java.lang.String);
}
