#!/usr/bin/env python3
"""Build debug or compact release APKs with the current JDK and SDK 37.

DX regenerates its older Gradle template on each build. Only a failure in the
APK assembly stage is recoverable here; Rust/asset failures always stop the run.
This adapter changes generated files under target/, never the installed CLI.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import zipfile
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent.parent
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
        if not after or after in text:
            return
        raise RuntimeError(f'Dioxus 模板已变化，请检查：{path}')
    path.write_text(text.replace(before, after))


def adapt_gradle(build, release=False, unsigned=False):
    replace(build / 'gradle.properties', 'android.defaults.buildfeatures.buildconfig=true\n', '')
    replace(build / 'gradle.properties', 'android.nonFinalResIds=false\n', '')
    replace(build / 'gradle/wrapper/gradle-wrapper.properties', 'gradle-9.1.0-bin.zip', 'gradle-9.5.0-bin.zip')
    replace(build / 'gradle/wrapper/gradle-wrapper.properties', 'networkTimeout=10000', 'networkTimeout=120000')
    replace(build / 'build.gradle.kts', 'com.android.tools.build:gradle:8.7.0', 'com.android.tools.build:gradle:9.3.3')
    replace(build / 'build.gradle.kts', '        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.0.20")\n', '')
    app = build / 'app/build.gradle.kts'
    if 'packaging { jniLibs { useLegacyPackaging = true } }' not in app.read_text():
        replace(app, 'android {\n', 'android {\n    packaging { jniLibs { useLegacyPackaging = true } }\n')
    replace(build / 'app/src/main/AndroidManifest.xml', '        android:extractNativeLibs="true"\n', '')
    replace(app, '    id("org.jetbrains.kotlin.android")\n', '')
    replace(app, '    kotlinOptions {\n        jvmTarget = "17"\n    }\n', '')
    replace(app, '    compileSdk = 37', '    compileSdk { version = release(37) { minorApiLevel = 0 } }')
    # AGP 9 has built-in Kotlin. Explicitly register generated Kotlin sources.
    replace(app, '            java.srcDirs("src/main/kotlin", "src/main/java")', '            java.directories.add("src/main/java")\n            kotlin.directories.add("src/main/kotlin")')
    if release:
        # A local, installable optimized build uses the existing debug identity.
        # Formal distribution can request --unsigned and sign separately.
        signing = 'null' if unsigned else 'signingConfigs.getByName("debug")'
        marker = '            // whatoeat release settings\n'
        text = app.read_text()
        if marker not in text:
            replace(app, '        getByName("release") {\n',
                    '        getByName("release") {\n' + marker +
                    '            isDebuggable = false\n'
                    '            isJniDebuggable = false\n'
                    '            isShrinkResources = true\n' +
                    f'            signingConfig = {signing}\n')
        else:
            text = re.sub(r'(// whatoeat release settings\n.*?signingConfig = )[^\n]+',
                          lambda match: match.group(1) + signing, text, count=1, flags=re.S)
            app.write_text(text)
        # Retain DX's recursive *.pro collection, including generated Wry rules.
        shutil.copyfile(ROOT / 'scripts/android-release.pro', build / 'app/whatoeat-release.pro')


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', action='store_true', help='优化 Rust、裁剪符号及 Android 代码/资源，默认沿用本机调试证书签名')
    parser.add_argument('--unsigned', action='store_true', help='仅配合 --release：输出未签名 APK，供自行正式签名')
    parser.add_argument('--offline', action='store_true', help='Cargo 和最终 Gradle 打包仅用缓存；DX 内置 Gradle 尝试不受此选项控制')
    args = parser.parse_args(argv)
    if args.unsigned and not args.release:
        parser.error('--unsigned 需要同时使用 --release')
    return args


def verify_apk(apk, env, signed):
    with zipfile.ZipFile(apk) as archive:
        damaged = archive.testzip()
        if damaged:
            raise RuntimeError(f'APK ZIP 校验失败：{damaged}')
        files = archive.namelist()
        libraries = [name for name in files if name.startswith('lib/') and name.endswith('.so')]
        if libraries != ['lib/arm64-v8a/libmain.so'] or 'classes.dex' not in files:
            raise RuntimeError(f'APK 内容不符合预期：原生库 {libraries}')
        native = archive.getinfo(libraries[0])
        details = {'apk_bytes': apk.stat().st_size, 'native_bytes': native.file_size,
                   'native_compressed_bytes': native.compress_size}
    build_tools = Path(env['ANDROID_HOME']) / 'build-tools/36.0.0'
    with (LOGS / ('verify-' + apk.parent.name + '.log')).open('w') as log:
        subprocess.run([str(build_tools / 'zipalign'), '-c', '-P', '16', '4', str(apk)],
                       env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        if signed:
            subprocess.run([str(build_tools / 'apksigner'), 'verify', '--verbose', str(apk)],
                           env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    details['sha256'] = hashlib.sha256(apk.read_bytes()).hexdigest()
    return details


def main(argv=None):
    args = parse_args(argv)
    mode = 'release' if args.release else 'debug'
    build = ROOT / f'target/dx/whatoeat/{mode}/android/app'
    env = environment()
    version = subprocess.check_output(['dx', '--version'], text=True).strip()
    if not version.startswith('dioxus 0.7.10 '):
        raise RuntimeError(f'此适配器针对 Dioxus 0.7.10，当前为 {version}')
    LOGS.mkdir(parents=True, exist_ok=True)
    print(f'JDK: {env["JAVA_HOME"]}\nSDK: {env["ANDROID_HOME"]}\nNDK: {env["NDK_HOME"]}', flush=True)
    signing = 'unsigned' if args.unsigned else 'local-debug-certificate'
    print(f'模式：{mode}；签名：{signing}', flush=True)
    print('1/4 编译 Rust 并生成安卓工程…', flush=True)
    dx_log = LOGS / ('dioxus-release.log' if args.release else 'dioxus.log')
    command = ['dx', 'build', '--platform', 'android', '--features', 'mobile', '--bin', 'whatoeat', '--target', 'aarch64-linux-android', '--locked']
    if args.release:
        command.append('--release')
    if args.offline:
        command.append('--offline')
    native = build / 'app/src/main/jniLibs/arm64-v8a/libmain.so'
    # A failed Rust build must never be mistaken for a fresh generated library.
    native.unlink(missing_ok=True)
    with dx_log.open('w') as log:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    output = dx_log.read_text()
    if result.returncode and 'Failed to assemble apk:' not in output:
        raise RuntimeError(f'Rust 或工程生成失败，未继续打包。日志：{dx_log}\n{output[-3000:]}')
    if not native.is_file():
        raise RuntimeError('未找到 Dioxus 生成的 ARM64 库。')
    print('2/4 适配 Gradle 9.5 / AGP 9.3.3 / SDK 37.0…', flush=True)
    adapt_gradle(build, args.release, args.unsigned)
    print(f'3/4 打包 {mode} APK…', flush=True)
    gradle_log = LOGS / ('gradle-release.log' if args.release else 'gradle.log')
    apk_name = f'app-{mode}' + ('-unsigned' if args.unsigned else '') + '.apk'
    apk = build / 'app/build/outputs/apk' / mode / apk_name
    apk.unlink(missing_ok=True)
    gradle_command = ['./gradlew', ':app:assembleRelease' if args.release else ':app:assembleDebug', '--no-daemon', '--console=plain', '--stacktrace']
    if args.offline:
        gradle_command.append('--offline')
    with gradle_log.open('w') as log:
        result = subprocess.run(gradle_command, cwd=build, env=env, stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f'APK 打包失败。日志：{gradle_log}\n{gradle_log.read_text()[-3000:]}')
    print('4/4 校验 APK 内容、对齐与签名…', flush=True)
    details = verify_apk(apk, env, signed=not args.unsigned)
    destination = LOGS / f'whatoeat-{mode}{"-unsigned" if args.unsigned else ""}.apk'
    temporary = destination.with_suffix('.apk.tmp')
    shutil.copy2(apk, temporary)
    temporary.replace(destination)
    details.update(mode=mode, signing=signing, abi='arm64-v8a', apk=str(destination))
    destination.with_suffix('.json').write_text(json.dumps(details, indent=2) + '\n')
    print(f'构建成功：{destination}\nAPK：{details["apk_bytes"] / 1048576:.2f} MiB；原生库：{details["native_bytes"] / 1048576:.2f} MiB', flush=True)


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError, zipfile.BadZipFile, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
