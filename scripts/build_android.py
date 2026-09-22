#!/usr/bin/env python3
"""Build the Dioxus 0.7.10 Android prototype with the current JDK and SDK 37.

DX regenerates its older Gradle template on each build. Only a failure in the
APK assembly stage is recoverable here; Rust/asset failures always stop the run.
This adapter changes generated files under target/, never the installed CLI.
"""
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / 'target/dx/whatoeat/debug/android/app'
LOGS = ROOT / 'target/android-build'


def environment():
    env = os.environ.copy()
    java = shutil.which('java')
    if not java:
        raise RuntimeError('找不到 java；请配置现有 JDK 的 PATH。')
    env.setdefault('JAVA_HOME', str(Path(java).resolve().parent.parent))
    sdk = Path(env.get('ANDROID_HOME', Path.home() / 'Android/Sdk'))
    if not sdk.is_dir():
        raise RuntimeError('找不到 Android SDK；请设置 ANDROID_HOME。')
    env['ANDROID_HOME'] = str(sdk)
    ndk = env.get('NDK_HOME') or env.get('ANDROID_NDK_HOME')
    if not ndk:
        installed = [p for p in (sdk / 'ndk').glob('*') if (p / 'source.properties').is_file()]
        if not installed:
            raise RuntimeError('未安装 NDK (Side by side)。')
        ndk = str(max(installed, key=lambda p: tuple(int(n) for n in re.findall(r'\d+', p.name))))
    env['NDK_HOME'] = env['ANDROID_NDK_HOME'] = ndk
    env['PATH'] = os.pathsep.join([str(Path(env['JAVA_HOME']) / 'bin'), str(sdk / 'platform-tools'), env.get('PATH', '')])
    # Java does not read http(s)_proxy automatically. Preserve explicit JVM
    # settings and forward unauthenticated HTTP proxy endpoints without logging.
    options = shlex.split(env.get('JAVA_OPTS', ''))
    for scheme in ('http', 'https'):
        proxy = env.get(scheme + '_proxy') or env.get(scheme.upper() + '_PROXY')
        if proxy and not any(x.startswith(f'-D{scheme}.proxyHost=') for x in options):
            url = urlsplit(proxy)
            if url.scheme == 'http' and url.hostname and url.port and not url.username:
                options += [f'-D{scheme}.proxyHost={url.hostname}', f'-D{scheme}.proxyPort={url.port}']
    env['JAVA_OPTS'] = shlex.join(options)
    return env


def replace(path, before, after):
    text = path.read_text()
    if before not in text:
        if after and after in text:
            return
        raise RuntimeError(f'Dioxus 模板已变化，请检查：{path}')
    path.write_text(text.replace(before, after))


def adapt_gradle():
    replace(BUILD / 'gradle.properties', 'android.defaults.buildfeatures.buildconfig=true\n', '')
    replace(BUILD / 'gradle.properties', 'android.nonFinalResIds=false\n', '')
    replace(BUILD / 'gradle/wrapper/gradle-wrapper.properties', 'gradle-9.1.0-bin.zip', 'gradle-9.5.0-bin.zip')
    replace(BUILD / 'gradle/wrapper/gradle-wrapper.properties', 'networkTimeout=10000', 'networkTimeout=120000')
    replace(BUILD / 'build.gradle.kts', 'com.android.tools.build:gradle:8.7.0', 'com.android.tools.build:gradle:9.3.3')
    replace(BUILD / 'build.gradle.kts', '        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.0.20")\n', '')
    app = BUILD / 'app/build.gradle.kts'
    replace(app, 'android {\n', 'android {\n    packaging { jniLibs { useLegacyPackaging = true } }\n')
    replace(BUILD / 'app/src/main/AndroidManifest.xml', '        android:extractNativeLibs="true"\n', '')
    replace(app, '    id("org.jetbrains.kotlin.android")\n', '')
    replace(app, '    kotlinOptions {\n        jvmTarget = "17"\n    }\n', '')
    replace(app, '    compileSdk = 37', '    compileSdk { version = release(37) { minorApiLevel = 0 } }')
    # AGP 9 has built-in Kotlin. Explicitly register generated Kotlin sources.
    replace(app, '            java.srcDirs("src/main/kotlin", "src/main/java")', '            java.directories.add("src/main/java")\n            kotlin.directories.add("src/main/kotlin")')


def main():
    env = environment()
    version = subprocess.check_output(['dx', '--version'], text=True).strip()
    if not version.startswith('dioxus 0.7.10 '):
        raise RuntimeError(f'此适配器针对 Dioxus 0.7.10，当前为 {version}')
    LOGS.mkdir(parents=True, exist_ok=True)
    print(f'JDK: {env["JAVA_HOME"]}\nSDK: {env["ANDROID_HOME"]}\nNDK: {env["NDK_HOME"]}', flush=True)
    print('1/3 编译 Rust 并生成安卓工程…', flush=True)
    dx_log = LOGS / 'dioxus.log'
    with dx_log.open('w') as log:
        result = subprocess.run(['dx', 'build', '--platform', 'android', '--features', 'mobile', '--bin', 'whatoeat', '--target', 'aarch64-linux-android', '--locked'], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    output = dx_log.read_text()
    if result.returncode and 'Failed to assemble apk:' not in output:
        raise RuntimeError(f'Rust 或工程生成失败，未继续打包。日志：{dx_log}\n{output[-3000:]}')
    if not (BUILD / 'app/src/main/jniLibs/arm64-v8a/libmain.so').is_file():
        raise RuntimeError('未找到 Dioxus 生成的 ARM64 库。')
    print('2/3 适配 Gradle 9.5 / AGP 9.3.3 / SDK 37.0…', flush=True)
    adapt_gradle()
    print('3/3 打包 APK…', flush=True)
    gradle_log = LOGS / 'gradle.log'
    with gradle_log.open('w') as log:
        result = subprocess.run(['./gradlew', ':app:assembleDebug', '--no-daemon', '--console=plain', '--stacktrace'], cwd=BUILD, env=env, stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f'APK 打包失败。日志：{gradle_log}\n{gradle_log.read_text()[-3000:]}')
    apk = BUILD / 'app/build/outputs/apk/debug/app-debug.apk'
    destination = LOGS / 'whatoeat-debug.apk'
    shutil.copy2(apk, destination)
    print(f'构建成功：{destination}', flush=True)


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
