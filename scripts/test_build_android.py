"""Exercise Android build orchestration without invoking Rust, Java, or Gradle."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location(
    'build_android', Path(__file__).with_name('build_android.py'))
build_android = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_android)

# The Dioxus 0.7.10 template sections consumed by the adapter. In particular,
# preserve the recursive rule collection used for generated Wry JNI keep rules.
DEBUG_BLOCK = '''        getByName("debug") {
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
            }
        }
'''
PROGUARD_BLOCK = '''            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
'''
APP_TEMPLATE = '''plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    compileSdk = 37
    buildTypes {
''' + DEBUG_BLOCK + '''        getByName("release") {
            isMinifyEnabled = true
''' + PROGUARD_BLOCK + '''        }
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    sourceSets {
        getByName("main") {
            java.srcDirs("src/main/kotlin", "src/main/java")
        }
    }
}
'''


class BuildAndroidTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='whatoeat-android-test-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.logs = self.root / 'target/android-build'
        (self.root / 'scripts').mkdir()
        (self.root / 'scripts/android-release.pro').write_text(
            '# Test keep-rule fixture\n-keep class example.JniBridge { *; }\n')
        patcher = mock.patch.multiple(build_android, ROOT=self.root, LOGS=self.logs)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.env = {
            'JAVA_HOME': str(self.root / 'existing-jdk'),
            'ANDROID_HOME': str(self.root / 'existing-sdk'),
            'NDK_HOME': str(self.root / 'existing-ndk'),
        }

    def project(self, mode='release'):
        project = self.root / f'target/dx/whatoeat/{mode}/android/app'
        files = {
            'gradle.properties': (
                'org.gradle.jvmargs=-Xmx2048m\n'
                'android.defaults.buildfeatures.buildconfig=true\n'
                'android.nonFinalResIds=false\n'),
            'gradle/wrapper/gradle-wrapper.properties': (
                'distributionUrl=https\\://services.gradle.org/distributions/'
                'gradle-9.1.0-bin.zip\nnetworkTimeout=10000\n'),
            'build.gradle.kts': (
                'buildscript {\n    dependencies {\n'
                '        classpath("com.android.tools.build:gradle:8.7.0")\n'
                '        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.0.20")\n'
                '    }\n}\n'),
            'app/build.gradle.kts': APP_TEMPLATE,
            'app/src/main/AndroidManifest.xml': (
                '<manifest><application\n'
                '        android:extractNativeLibs="true"\n'
                '        android:label="Test" /></manifest>\n'),
            'app/src/main/generated/wry.pro': '-keep class example.Wry { *; }\n',
        }
        for name, contents in files.items():
            path = project / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
        return project

    @staticmethod
    def native(project):
        return project / 'app/src/main/jniLibs/arm64-v8a/libmain.so'

    def run_main(self, args, command_runner, verifier):
        with (
            mock.patch.object(build_android, 'environment', return_value=self.env),
            mock.patch.object(build_android.subprocess, 'check_output',
                              return_value='dioxus 0.7.10 (test build)'),
            mock.patch.object(build_android.subprocess, 'run',
                              side_effect=command_runner) as run,
            mock.patch.object(build_android, 'verify_apk', verifier),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            build_android.main(args)
        return run

    def test_debug_adaptation_preserves_debug_and_recursive_keep_rules(self):
        project = self.project('debug')
        build_android.adapt_gradle(project)
        app = (project / 'app/build.gradle.kts').read_text()
        self.assertIn(DEBUG_BLOCK, app)
        self.assertIn(PROGUARD_BLOCK, app)
        self.assertNotIn('// whatoeat release settings', app)
        self.assertNotIn('isShrinkResources', app)
        self.assertFalse((project / 'app/whatoeat-release.pro').exists())
        self.assertIn('compileSdk { version = release(37)', app)
        self.assertIn('kotlin.directories.add("src/main/kotlin")', app)

    def test_release_adaptation_preserves_jni_rules_and_disables_debugging(self):
        project = self.project()
        build_android.adapt_gradle(project, release=True)
        app = (project / 'app/build.gradle.kts').read_text()
        release = app.split('getByName("release") {', 1)[1]
        self.assertIn(DEBUG_BLOCK, app)
        self.assertIn(PROGUARD_BLOCK, release)
        for setting in (
            'isMinifyEnabled = true', 'isShrinkResources = true',
            'isDebuggable = false', 'isJniDebuggable = false',
            'signingConfig = signingConfigs.getByName("debug")',
        ):
            self.assertIn(setting, release)
        self.assertEqual(
            (project / 'app/whatoeat-release.pro').read_bytes(),
            (self.root / 'scripts/android-release.pro').read_bytes())
        self.assertEqual(
            (project / 'app/src/main/generated/wry.pro').read_text(),
            '-keep class example.Wry { *; }\n')

    def test_repeated_adaptation_is_idempotent_and_signing_can_be_toggled(self):
        project = self.project()
        build_android.adapt_gradle(project, release=True)
        app_path = project / 'app/build.gradle.kts'
        signed = app_path.read_text()
        build_android.adapt_gradle(project, release=True)
        self.assertEqual(app_path.read_text(), signed)
        build_android.adapt_gradle(project, release=True, unsigned=True)
        unsigned = app_path.read_text()
        self.assertEqual(unsigned.count('// whatoeat release settings'), 1)
        self.assertEqual(unsigned.count('packaging { jniLibs { useLegacyPackaging'), 1)
        self.assertIn('signingConfig = null', unsigned)
        self.assertNotIn('signingConfig = signingConfigs.getByName("debug")', unsigned)
        build_android.adapt_gradle(project, release=True, unsigned=True)
        self.assertEqual(app_path.read_text(), unsigned)
        build_android.adapt_gradle(project, release=True)
        self.assertEqual(app_path.read_text(), signed)

    def test_release_adaptation_rejects_missing_release_template(self):
        project = self.project()
        app = project / 'app/build.gradle.kts'
        app.write_text(APP_TEMPLATE.replace('getByName("release")', 'named("release")'))
        with self.assertRaisesRegex(RuntimeError, 'Dioxus 模板已变化'):
            build_android.adapt_gradle(project, release=True)

    def test_unsigned_requires_release(self):
        with contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as error:
                build_android.parse_args(['--unsigned'])
        self.assertEqual(error.exception.code, 2)

    def test_build_modes_select_matching_project_commands_and_apk(self):
        for mode, unsigned in [('debug', False), ('release', False), ('release', True)]:
            with self.subTest(mode=mode, unsigned=unsigned):
                project = self.project(mode)
                native = self.native(project)
                native.parent.mkdir(parents=True, exist_ok=True)
                native.write_bytes(b'stale native binary')
                name = f'app-{mode}{"-unsigned" if unsigned else ""}.apk'
                apk = project / 'app/build/outputs/apk' / mode / name
                apk.parent.mkdir(parents=True, exist_ok=True)
                apk.write_bytes(b'stale generated APK')
                calls = []

                def run(command, **kwargs):
                    calls.append(command)
                    if command[0] == 'dx':
                        self.assertFalse(native.exists())
                        self.assertEqual(kwargs['cwd'], self.root)
                        native.write_bytes(b'fresh native binary')
                        # The adapter may recover only the DX assembly stage.
                        kwargs['stdout'].write('Failed to assemble apk: old Gradle template\n')
                        return subprocess.CompletedProcess(command, 1)
                    self.assertEqual(kwargs['cwd'], project)
                    self.assertFalse(apk.exists())
                    apk.write_bytes(b'fresh verified APK')
                    return subprocess.CompletedProcess(command, 0)

                verifier = mock.Mock(return_value={
                    'apk_bytes': 18, 'native_bytes': 19,
                    'native_compressed_bytes': 11, 'sha256': 'verified-sha256',
                })
                args = ['--offline']
                if mode == 'release':
                    args.append('--release')
                if unsigned:
                    args.append('--unsigned')
                self.run_main(args, run, verifier)
                self.assertEqual(len(calls), 2)
                self.assertEqual('--release' in calls[0], mode == 'release')
                self.assertIn('--locked', calls[0])
                self.assertIn('--offline', calls[0])
                self.assertEqual(calls[1][1], ':app:assemble' + mode.title())
                self.assertIn('--offline', calls[1])
                verifier.assert_called_once_with(apk, self.env, signed=not unsigned)
                destination = self.logs / f'whatoeat-{mode}{"-unsigned" if unsigned else ""}.apk'
                self.assertEqual(destination.read_bytes(), b'fresh verified APK')
                metadata = json.loads(destination.with_suffix('.json').read_text())
                self.assertEqual(metadata['mode'], mode)
                self.assertEqual(metadata['signing'], 'unsigned' if unsigned else 'local-debug-certificate')
                self.assertEqual(metadata['abi'], 'arm64-v8a')
                self.assertFalse(destination.with_suffix('.apk.tmp').exists())

    def test_rust_failure_stops_before_gradle_even_with_stale_native_and_apk(self):
        project = self.project()
        native = self.native(project)
        native.parent.mkdir(parents=True)
        native.write_bytes(b'stale native binary')
        self.logs.mkdir(parents=True)
        destination = self.logs / 'whatoeat-release.apk'
        destination.write_bytes(b'previous successful APK')
        calls = []

        def run(command, **kwargs):
            calls.append(command)
            self.assertFalse(native.exists())
            kwargs['stdout'].write('error: Rust compilation failed\n')
            return subprocess.CompletedProcess(command, 1)

        verifier = mock.Mock()
        with mock.patch.object(build_android, 'adapt_gradle') as adapt:
            with self.assertRaisesRegex(RuntimeError, 'Rust 或工程生成失败'):
                self.run_main(['--release'], run, verifier)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0], 'dx')
        adapt.assert_not_called()
        verifier.assert_not_called()
        self.assertEqual(destination.read_bytes(), b'previous successful APK')

    def test_assembly_error_without_fresh_native_cannot_use_stale_library(self):
        project = self.project()
        native = self.native(project)
        native.parent.mkdir(parents=True)
        native.write_bytes(b'stale native binary')
        calls = []

        def run(command, **kwargs):
            calls.append(command)
            kwargs['stdout'].write('Failed to assemble apk: missing compilation output\n')
            return subprocess.CompletedProcess(command, 1)

        with mock.patch.object(build_android, 'adapt_gradle') as adapt:
            with self.assertRaisesRegex(RuntimeError, '未找到 Dioxus 生成的 ARM64 库'):
                self.run_main(['--release'], run, mock.Mock())
        self.assertEqual(len(calls), 1)
        self.assertFalse(native.exists())
        adapt.assert_not_called()

    def test_gradle_failure_does_not_publish_old_generated_or_previous_apk(self):
        project = self.project()
        native = self.native(project)
        native.parent.mkdir(parents=True)
        generated = project / 'app/build/outputs/apk/release/app-release.apk'
        generated.parent.mkdir(parents=True)
        generated.write_bytes(b'stale generated APK')
        self.logs.mkdir(parents=True)
        destination = self.logs / 'whatoeat-release.apk'
        destination.write_bytes(b'previous successful APK')

        def run(command, **kwargs):
            if command[0] == 'dx':
                native.write_bytes(b'fresh native binary')
                return subprocess.CompletedProcess(command, 0)
            self.assertFalse(generated.exists())
            kwargs['stdout'].write('Gradle build failed\n')
            return subprocess.CompletedProcess(command, 1)

        verifier = mock.Mock()
        with self.assertRaisesRegex(RuntimeError, 'APK 打包失败'):
            self.run_main(['--release'], run, verifier)
        verifier.assert_not_called()
        self.assertEqual(destination.read_bytes(), b'previous successful APK')

    def test_verification_failure_does_not_overwrite_previous_apk(self):
        project = self.project()
        native = self.native(project)
        native.parent.mkdir(parents=True)
        generated = project / 'app/build/outputs/apk/release/app-release.apk'
        generated.parent.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        destination = self.logs / 'whatoeat-release.apk'
        destination.write_bytes(b'previous successful APK')

        def run(command, **kwargs):
            if command[0] == 'dx':
                native.write_bytes(b'fresh native binary')
            else:
                generated.write_bytes(b'unverified APK')
            return subprocess.CompletedProcess(command, 0)

        verifier = mock.Mock(side_effect=RuntimeError('APK verification failed'))
        with self.assertRaisesRegex(RuntimeError, 'APK verification failed'):
            self.run_main(['--release'], run, verifier)
        self.assertEqual(destination.read_bytes(), b'previous successful APK')


if __name__ == '__main__':
    unittest.main()
