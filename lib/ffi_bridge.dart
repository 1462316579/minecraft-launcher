import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';
import 'package:ffi/ffi.dart';
import 'package:flutter/services.dart' show rootBundle;
import 'package:path_provider/path_provider.dart';

/// Minecraft 启动器 FFI 桥接：加载 Rust 动态库并声明 C ABI 函数
class McLauncherBridge {
  static final McLauncherBridge _instance = McLauncherBridge._internal();
  factory McLauncherBridge() => _instance;
  McLauncherBridge._internal();

  DynamicLibrary? _lib;
  bool _loaded = false;

  // FFI 函数指针（Dart 可调用类型）
  Pointer<Utf8> Function()? _mcVersion;
  void Function(Pointer<Utf8>)? _mcFreeString;
  Pointer<Utf8> Function()? _mcFetchVersionManifest;
  Pointer<Utf8> Function(int)? _mcFetchAnnouncement;
  int Function(Pointer<Utf8>, Pointer<Utf8>)? _mcDownloadFile;
  Pointer<Utf8> Function()? _mcLastError;
  int Function(
          Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>,
          Pointer<Utf8>)? _mcLaunch;

  // Vulkan 渲染 FFI 函数
  Pointer<Void> Function()? _vulkanRendererNew;
  int Function(Pointer<Void>, int, int)? _vulkanRendererInit;
  int Function(
          Pointer<Void>,
          Pointer<Uint8>,
          int,
          int,
          int,
          Pointer<Uint8>,
          int,
      )? _vulkanRendererRenderFrame;
  void Function(Pointer<Void>)? _vulkanRendererFree;
  Pointer<Utf8> Function()? _vulkanRendererVersion;

  /// 初始化并加载动态库（从 Flutter 资源提取到临时目录）
  Future<void> init() async {
    if (_loaded) return;

    final dir = await getApplicationDocumentsDirectory();
    final soFile = File('${dir.path}/libmc_launcher_core.so');

    // 每次启动都从资源重新写入，避免使用设备上缓存的旧 .so（可能带 libssl 依赖）
    final data = await rootBundle.load('assets/rust_lib/libmc_launcher_core.so');
    await soFile.writeAsBytes(data.buffer.asUint8List(), flush: true);

    if (Platform.isAndroid) {
      await Process.run('chmod', ['+x', soFile.path]);
    }

    _lib = DynamicLibrary.open(soFile.path);
    _bindFunctions();
    _loaded = true;
  }

  void _bindFunctions() {
    _mcVersion = _lib!
        .lookup<NativeFunction<Pointer<Utf8> Function()>>('mc_version')
        .asFunction<Pointer<Utf8> Function()>();
    _mcFreeString = _lib!
        .lookup<NativeFunction<Void Function(Pointer<Utf8>)>>('mc_free_string')
        .asFunction<void Function(Pointer<Utf8>)>();
    _mcFetchVersionManifest = _lib!
        .lookup<NativeFunction<Pointer<Utf8> Function()>>(
          'mc_fetch_version_manifest',
        )
        .asFunction<Pointer<Utf8> Function()>();
    _mcFetchAnnouncement = _lib!
        .lookup<NativeFunction<Pointer<Utf8> Function(Int32)>>(
          'mc_fetch_announcement',
        )
        .asFunction<Pointer<Utf8> Function(int)>();
    _mcDownloadFile = _lib!
        .lookup<NativeFunction<Int64 Function(Pointer<Utf8>, Pointer<Utf8>)>>(
          'mc_download_file',
        )
        .asFunction<int Function(Pointer<Utf8>, Pointer<Utf8>)>();
    _mcLastError = _lib!
        .lookup<NativeFunction<Pointer<Utf8> Function()>>('mc_last_error')
        .asFunction<Pointer<Utf8> Function()>();
    _mcLaunch = _lib!
        .lookup<NativeFunction<Int32 Function(
          Pointer<Utf8>,
          Pointer<Utf8>,
          Pointer<Utf8>,
          Pointer<Utf8>,
          Pointer<Utf8>,
        )>>('mc_launch')
        .asFunction<
          int Function(
            Pointer<Utf8>,
            Pointer<Utf8>,
            Pointer<Utf8>,
            Pointer<Utf8>,
            Pointer<Utf8>,
          )>();

    // Vulkan 渲染函数绑定
    _vulkanRendererNew = _lib!
        .lookup<NativeFunction<Pointer<Void> Function()>>(
          'vulkan_renderer_new',
        )
        .asFunction<Pointer<Void> Function()>();
    _vulkanRendererInit = _lib!
        .lookup<NativeFunction<Int32 Function(Pointer<Void>, Int32, Int32)>>(
          'vulkan_renderer_init',
        )
        .asFunction<int Function(Pointer<Void>, int, int)>();
    _vulkanRendererRenderFrame = _lib!
        .lookup<NativeFunction<Int32 Function(
          Pointer<Void>,
          Pointer<Uint8>,
          Int32,
          Int32,
          Int32,
          Pointer<Uint8>,
          Int32,
        )>>('vulkan_renderer_render_frame')
        .asFunction<
          int Function(
            Pointer<Void>,
            Pointer<Uint8>,
            int,
            int,
            int,
            Pointer<Uint8>,
            int,
          )>();
    _vulkanRendererFree = _lib!
        .lookup<NativeFunction<Void Function(Pointer<Void>)>>(
          'vulkan_renderer_free',
        )
        .asFunction<void Function(Pointer<Void>)>();
    _vulkanRendererVersion = _lib!
        .lookup<NativeFunction<Pointer<Utf8> Function()>>(
          'vulkan_renderer_version',
        )
        .asFunction<Pointer<Utf8> Function()>();
  }

  /// 获取 Rust 库版本
  String get version {
    if (!_loaded) return 'not loaded';
    final ptr = _mcVersion!();
    final str = ptr.toDartString();
    _mcFreeString!(ptr);
    return str;
  }

  /// 获取版本清单 JSON 字符串
  String fetchVersionManifest() {
    if (!_loaded) return 'not loaded';
    final ptr = _mcFetchVersionManifest!();
    final str = ptr.toDartString();
    _mcFreeString!(ptr);
    return str;
  }

  /// 获取公告 JSON 字符串
  /// cn: true 使用中文源（Gitee），false 使用国际源（GitHub raw）
  String fetchAnnouncement({required bool cn}) {
    if (!_loaded) return 'not loaded';
    final ptr = _mcFetchAnnouncement!(cn ? 1 : 0);
    final str = ptr.toDartString();
    _mcFreeString!(ptr);
    return str;
  }

  /// 下载文件，返回下载字节数；失败返回 -1
  int downloadFile(String url, String dest) {
    if (!_loaded) return -1;
    final urlPtr = url.toNativeUtf8();
    final destPtr = dest.toNativeUtf8();
    final result = _mcDownloadFile!(urlPtr, destPtr);
    malloc.free(urlPtr);
    malloc.free(destPtr);
    return result;
  }

  /// 获取上次错误信息
  String get lastError {
    if (!_loaded) return 'not loaded';
    final ptr = _mcLastError!();
    final str = ptr.toDartString();
    _mcFreeString!(ptr);
    return str;
  }

  /// 启动 Minecraft（阻塞调用，返回退出码）
  int launch({
    required String versionId,
    required String gameDir,
    required String authToken,
    required String authUuid,
    required String userName,
  }) {
    if (!_loaded) return -1;
    final vPtr = versionId.toNativeUtf8();
    final gPtr = gameDir.toNativeUtf8();
    final tPtr = authToken.toNativeUtf8();
    final uPtr = authUuid.toNativeUtf8();
    final nPtr = userName.toNativeUtf8();

    final result = _mcLaunch!(vPtr, gPtr, tPtr, uPtr, nPtr);

    malloc.free(vPtr);
    malloc.free(gPtr);
    malloc.free(tPtr);
    malloc.free(uPtr);
    malloc.free(nPtr);

    return result;
  }

  // ==================== Vulkan 渲染方法 ====================

  /// 创建 Vulkan 渲染器实例，返回指针
  Pointer<Void> vulkanRendererNew() {
    if (!_loaded) return Pointer<Void>.fromAddress(0);
    return _vulkanRendererNew!();
  }

  /// 初始化 Vulkan 渲染器
  /// 返回：0 成功，-1 失败
  int vulkanRendererInit(Pointer<Void> rendererPtr, int width, int height) {
    if (!_loaded) return -1;
    return _vulkanRendererInit!(rendererPtr, width, height);
  }

  /// 渲染一帧 3D 皮肤模型
  /// 返回：写入的字节数，失败返回 -1
  int vulkanRendererRenderFrame(
    Pointer<Void> rendererPtr,
    Uint8List skinData,
    int width,
    int height,
    Pointer<Uint8> outData,
    int outLen,
  ) {
    if (!_loaded) return -1;
    final skinPtr = malloc.allocate<Uint8>(skinData.length);
    final skinTyped = skinPtr.asTypedList(skinData.length);
    skinTyped.setAll(0, skinData);
    try {
      return _vulkanRendererRenderFrame!(
        rendererPtr,
        skinPtr,
        skinData.length,
        width,
        height,
        outData,
        outLen,
      );
    } finally {
      malloc.free(skinPtr);
    }
  }

  /// 销毁 Vulkan 渲染器
  void vulkanRendererFree(Pointer<Void> rendererPtr) {
    if (!_loaded) return;
    _vulkanRendererFree!(rendererPtr);
  }

  /// 获取 Vulkan 渲染器版本
  String vulkanRendererVersion() {
    if (!_loaded) return 'not loaded';
    final ptr = _vulkanRendererVersion!();
    final str = ptr.toDartString();
    _mcFreeString!(ptr);
    return str;
  }
}
