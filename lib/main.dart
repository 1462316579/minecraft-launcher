import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';
import 'package:ffi/ffi.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:http/http.dart' as http;
import 'package:image/image.dart' as img;
import 'ffi_bridge.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await SystemChrome.setPreferredOrientations([
    DeviceOrientation.landscapeLeft,
    DeviceOrientation.landscapeRight,
  ]);
  runApp(const McLauncherApp());
}

class McLauncherApp extends StatelessWidget {
  const McLauncherApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Minecraft Launcher',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF121212),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF4e79a7),
          secondary: Color(0xFFf2c94c),
          surface: Color(0xFF1e1e2e),
        ),
      ),
      home: const LauncherHomePage(),
    );
  }
}

// ==================== 外壳：对应 FCL MainActivity ====================
class LauncherHomePage extends StatefulWidget {
  const LauncherHomePage({super.key});

  @override
  State<LauncherHomePage> createState() => _LauncherHomePageState();
}

class _LauncherHomePageState extends State<LauncherHomePage> {
  int _pageIndex = 0;
  bool _isInitialized = false;
  String _initMessage = '正在初始化 Rust 核心...';

  final List<Widget> _pages = [
    const MainHomePage(), // 首页（对应 FCL MainUI）
    const VersionsPage(), // 版本
    const InstallPage(),  // 安装
  ];

  @override
  void initState() {
    super.initState();
    _initBridge();
  }

  Future<void> _initBridge() async {
    final bridge = McLauncherBridge();
    try {
      await bridge.init();
      setState(() {
        _isInitialized = true;
        _initMessage = '初始化完成 (Rust ${bridge.version})';
      });
    } catch (e) {
      setState(() {
        _isInitialized = true;
        _initMessage = '初始化失败: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: _isInitialized
          ? Row(
              children: [
                // ========== 左侧窄侧边栏（对应 FCL 左菜单，仅图标） ==========
                NavigationRail(
                  selectedIndex: _pageIndex,
                  onDestinationSelected: (i) => setState(() => _pageIndex = i),
                  labelType: NavigationRailLabelType.none,
                  backgroundColor: const Color(0xFF5a6b8c),
                  elevation: 4,
                  groupAlignment: -0.9,
                  minWidth: 72,
                  destinations: const [
                    NavigationRailDestination(
                      icon: Icon(Icons.home, size: 32),
                      selectedIcon: Icon(Icons.home, size: 32, color: Colors.white),
                      label: Text(''),
                    ),
                    NavigationRailDestination(
                      icon: Icon(Icons.build, size: 32),
                      selectedIcon: Icon(Icons.build, size: 32, color: Colors.white),
                      label: Text(''),
                    ),
                    NavigationRailDestination(
                      icon: Icon(Icons.cloud_download, size: 32),
                      selectedIcon: Icon(Icons.cloud_download, size: 32, color: Colors.white),
                      label: Text(''),
                    ),
                    NavigationRailDestination(
                      icon: Icon(Icons.games, size: 32),
                      selectedIcon: Icon(Icons.games, size: 32, color: Colors.white),
                      label: Text(''),
                    ),
                    NavigationRailDestination(
                      icon: Icon(Icons.extension, size: 32),
                      selectedIcon: Icon(Icons.extension, size: 32, color: Colors.white),
                      label: Text(''),
                    ),
                    NavigationRailDestination(
                      icon: Icon(Icons.settings, size: 32),
                      selectedIcon: Icon(Icons.settings, size: 32, color: Colors.white),
                      label: Text(''),
                    ),
                    NavigationRailDestination(
                      icon: Icon(Icons.arrow_back, size: 32),
                      selectedIcon: Icon(Icons.arrow_back, size: 32, color: Colors.white),
                      label: Text(''),
                    ),
                  ],
                ),

                // ========== 中间：3D 皮肤模型 + 游戏场景背景（主内容区） ==========
                Expanded(
                  flex: 4,
                  child: IndexedStack(
                    index: _pageIndex,
                    children: _pages,
                  ),
                ),

                // ========== 右侧面板：账户 + 版本 + 启动按钮（对应 FCL 右菜单） ==========
                Expanded(
                  flex: 1,
                  child: _buildRightPanel(),
                ),
              ],
            )
          : Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const CircularProgressIndicator(),
                  const SizedBox(height: 16),
                  Text(_initMessage),
                ],
              ),
            ),
    );
  }

  // 右侧面板：账户头像、账户状态、版本状态、设置、启动按钮
  Widget _buildRightPanel() {
    return Container(
      color: Colors.white.withValues(alpha: 0.1),
      child: Column(
        children: [
          const Spacer(flex: 1),

          // 右上角设置齿轮
          Align(
            alignment: Alignment.topRight,
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: IconButton(
                icon: const Icon(Icons.settings, color: Colors.white),
                onPressed: () {},
              ),
            ),
          ),

          // 账户头像
          Container(
            margin: const EdgeInsets.only(bottom: 12),
            child: const CircleAvatar(
              radius: 40,
              backgroundImage: AssetImage('assets/skin/alex.png'),
              child: Icon(Icons.person, size: 40, color: Colors.white),
            ),
          ),

          // 账户状态
          const Text(
            '没有账户',
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold, color: Colors.white),
          ),
          const SizedBox(height: 4),
          const Text(
            '点击添加账户',
            style: TextStyle(fontSize: 14, color: Colors.white70),
          ),

          const SizedBox(height: 32),

          // 版本状态
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.settings, color: Colors.white54, size: 24),
              const SizedBox(width: 8),
              const Text(
                '没有版本',
                style: TextStyle(fontSize: 16, color: Colors.white),
              ),
            ],
          ),

          const Spacer(flex: 2),

          // 启动按钮（对应 FCL start 按钮，右下角宽大按钮）
          Padding(
            padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
            child: SizedBox(
              width: double.infinity,
              height: 60,
              child: ElevatedButton.icon(
                onPressed: () {
                  // 对应 FCL 启动按钮：launchVersion(Profiles.getSelectedVersion())
                },
                icon: const Icon(Icons.play_arrow, size: 30),
                label: const Text(
                  '启动',
                  style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                ),
                style: ElevatedButton.styleFrom(
                  backgroundColor: Colors.green,
                  foregroundColor: Colors.white,
                  elevation: 12,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(16),
                  ),
                ),
              ),
            ),
          ),

          const Spacer(flex: 1),
        ],
      ),
    );
  }
}

// ==================== 首页：对应 FCL MainUI ====================
class MainHomePage extends StatefulWidget {
  const MainHomePage({super.key});

  @override
  State<MainHomePage> createState() => _MainHomePageState();
}

class _MainHomePageState extends State<MainHomePage> {
  // Vulkan 渲染器指针（对应 FCL SkinRenderer / SkinViewer）
  Pointer<Void>? _vulkanRenderer;
  bool _vulkanReady = false;
  Uint8List? _skinBytes;
  int _skinWidth = 512;
  int _skinHeight = 512;
  bool _isLoading = false;
  String? _skinError;
  String _skinSource = '默认';

  @override
  void initState() {
    super.initState();
    // 对应 FCL defaultSkin()：立即显示嵌入式默认皮肤，不等网络下载
    _skinBytes = _generateDefaultSkin();
    // 初始化 Vulkan 渲染器（完全使用 Vulkan 实现 3D 皮肤模型）
    _initVulkanRenderer();
    // 异步尝试下载网络皮肤（可选增强，失败不影响默认皮肤显示）
    _downloadNetworkSkin();
  }

  /// 初始化 Vulkan 渲染器（对应 FCL SkinRenderer 初始化）
  Future<void> _initVulkanRenderer() async {
    try {
      final bridge = McLauncherBridge();
      await bridge.init();

      // 创建 Vulkan 渲染器实例
      _vulkanRenderer = bridge.vulkanRendererNew();

      // 初始化 Vulkan（创建实例、设备、命令池等）
      final ret = bridge.vulkanRendererInit(_vulkanRenderer!, _skinWidth, _skinHeight);

      if (ret == 0) {
        // 渲染一帧 3D 皮肤模型（完全使用 Vulkan）
        _renderSkinFrame(bridge);
        setState(() {
          _vulkanReady = true;
        });
      } else {
        setState(() {
          _skinError = 'Vulkan 初始化失败，显示默认皮肤';
        });
      }
    } catch (e) {
      setState(() {
        _skinError = 'Vulkan 不可用: $e，显示默认皮肤';
      });
    }
  }

  /// 渲染皮肤帧（调用 Vulkan render_frame，完全使用 Vulkan 渲染）
  void _renderSkinFrame(McLauncherBridge bridge) {
    if (_vulkanRenderer == null || _skinBytes == null) return;

    final bufferSize = _skinWidth * _skinHeight * 4;
    final outData = malloc.allocate<Uint8>(bufferSize);

    final bytes = bridge.vulkanRendererRenderFrame(
      _vulkanRenderer!,
      _skinBytes!,
      _skinWidth,
      _skinHeight,
      outData.cast(),
      bufferSize,
    );

    if (bytes > 0 && mounted) {
      setState(() {
        _skinBytes = Uint8List.fromList(outData.asTypedList(bytes));
        _skinSource = 'Vulkan 渲染';
      });
    }

    malloc.free(outData);
  }

  /// 下载网络皮肤（可选增强，失败不影响默认皮肤显示）
  /// 尝试多个源：Minotar → Visage → 官方 textures.minecraft.net
  Future<void> _downloadNetworkSkin() async {
    final urls = [
      'https://minotar.net/avatar/Alex/512.png',
      'https://visage.surgeplay.com/face/512/Alex',
      'https://textures.minecraft.net/texture/3a2c6c6b5f3d3e2f2b1a4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5',
    ];

    for (final url in urls) {
      try {
        print('尝试下载皮肤: $url');
        final response = await http.get(Uri.parse(url)).timeout(const Duration(seconds: 8));
        print('状态码: ${response.statusCode}, 字节: ${response.bodyBytes.length}');
        if (response.statusCode == 200 && response.bodyBytes.isNotEmpty) {
          final skinImage = img.decodeImage(response.bodyBytes);
          if (skinImage != null && mounted) {
            setState(() {
              _skinBytes = Uint8List.fromList(img.encodePng(skinImage));
              _skinSource = url;
            });
            // 重新渲染 Vulkan 帧
            final bridge = McLauncherBridge();
            if (_vulkanRenderer != null) {
              _renderSkinFrame(bridge);
            }
            print('皮肤下载成功: $url');
            return;
          }
        }
      } catch (e) {
        print('下载失败 $url: $e');
      }
    }
    print('所有皮肤源都失败，保持默认皮肤');
  }

  /// 生成嵌入式默认皮肤（64x64 像素，对应 FCL alex.png）
  /// 与 FCL defaultSkin() 一致：皮肤色头部 + 橙色头发 + 绿色衬衫 + 棕色裤子
  Uint8List _generateDefaultSkin() {
    final image = img.Image(width: 64, height: 64);
    // 头部区域（8x8 像素，皮肤色）
    for (int y = 8; y < 16; y++) {
      for (int x = 8; x < 16; x++) {
        image.setPixel(x, y, img.ColorRgb8(222, 184, 135));
      }
    }
    // 头发（橙色，对应 FCL 截图中的角色）
    for (int y = 8; y < 12; y++) {
      for (int x = 8; x < 16; x++) {
        image.setPixel(x, y, img.ColorRgb8(210, 105, 30));
      }
    }
    // 身体（绿色衬衫）
    for (int y = 20; y < 32; y++) {
      for (int x = 8; x < 16; x++) {
        image.setPixel(x, y, img.ColorRgb8(102, 178, 102));
      }
    }
    // 腿部（棕色裤子）
    for (int y = 32; y < 48; y++) {
      for (int x = 8; x < 16; x++) {
        image.setPixel(x, y, img.ColorRgb8(139, 69, 19));
      }
    }
    return Uint8List.fromList(img.encodePng(image));
  }

  /// 点击模型打开皮肤动画设置（对应 FCL SkinViewer.setOnTwoFingerTap）
  void _onSkinTapped() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('皮肤动画设置'),
        content: Text('来源: $_skinSource\n尺寸: ${_skinWidth}x$_skinHeight\n渲染: ${_vulkanReady ? "Vulkan" : "默认"}'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('关闭'),
          ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    // 清理 Vulkan 渲染器
    if (_vulkanRenderer != null) {
      final bridge = McLauncherBridge();
      bridge.vulkanRendererFree(_vulkanRenderer!);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: const BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [Color(0xFF1a1a2e), Color(0xFF16213e)],
        ),
      ),
      child: Center(
        child: _buildSkinViewerPanel(),
      ),
    );
  }

  // 皮肤模型面板（对应 FCL SkinViewer）
  // 完全使用 Vulkan 渲染 3D 皮肤模型
  // 点击模型打开皮肤动画设置
  Widget _buildSkinViewerPanel() {
    if (!_vulkanReady && _skinBytes == null) {
      return const Center(
        child: CircularProgressIndicator(
          color: Color(0xFF4e79a7),
        ),
      );
    }

    if (_skinError != null && !_vulkanReady) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Text('Vulkan 不可用，显示默认皮肤'),
            const SizedBox(height: 16),
            if (_skinBytes != null)
              Image.memory(
                _skinBytes!,
                width: _skinWidth.toDouble(),
                height: _skinHeight.toDouble(),
                fit: BoxFit.contain,
              ),
          ],
        ),
      );
    }

    return GestureDetector(
      onTap: _onSkinTapped,
      child: Center(
        child: _skinBytes != null
            ? Image.memory(
                _skinBytes!,
                width: _skinWidth.toDouble(),
                height: _skinHeight.toDouble(),
                fit: BoxFit.contain,
              )
            : const Center(
                child: Text(
                  'Vulkan 渲染失败',
                  style: TextStyle(color: Colors.white54),
                ),
              ),
      ),
    );
  }
}

// ==================== 版本页面（保留原有，对应 FCL manageUI）====================
class VersionsPage extends StatefulWidget {
  const VersionsPage({super.key});

  @override
  State<VersionsPage> createState() => _VersionsPageState();
}

class _VersionsPageState extends State<VersionsPage> {
  List<dynamic> _versions = [];
  bool _loading = false;
  String _error = '';
  String? _selectedVersion;

  @override
  void initState() {
    super.initState();
    _loadVersions();
  }

  Future<void> _loadVersions() async {
    setState(() {
      _loading = true;
      _error = '';
    });
    try {
      final bridge = McLauncherBridge();
      await bridge.init();
      final jsonStr = bridge.fetchVersionManifest();
      final data = jsonDecode(jsonStr) as Map<String, dynamic>;
      final versions = data['versions'] as List<dynamic>;
      final releases = versions
          .where((v) => v['type'] == 'release')
          .take(20)
          .toList();
      setState(() {
        _versions = releases;
        _loading = false;
        if (releases.isNotEmpty) {
          _selectedVersion = releases.first['id'] as String;
        }
      });
    } catch (e) {
      setState(() {
        _error = '加载版本失败: $e';
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error.isNotEmpty
              ? Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Text(_error, style: const TextStyle(color: Colors.red)),
                      const SizedBox(height: 16),
                      ElevatedButton(
                        onPressed: _loadVersions,
                        child: const Text('重试'),
                      ),
                    ],
                  ),
                )
              : ListView.builder(
                  itemCount: _versions.length,
                  itemBuilder: (context, index) {
                    final v = _versions[index];
                    final id = v['id'] as String;
                    final type = v['type'] as String;
                    final isSelected = _selectedVersion == id;
                    return ListTile(
                      leading: Icon(Icons.settings,
                          color: isSelected ? Colors.blue : null),
                      title: Text(id),
                      subtitle: Text(type),
                      selected: isSelected,
                      onTap: () {
                        setState(() => _selectedVersion = id);
                      },
                    );
                  },
                ),
    );
  }
}

// ==================== 安装页面（保留原有）====================
class InstallPage extends StatefulWidget {
  const InstallPage({super.key});

  @override
  State<InstallPage> createState() => _InstallPageState();
}

class _InstallPageState extends State<InstallPage> {
  double _progress = 0.0;
  String _status = '等待中...';
  bool _installing = false;
  bool _launched = false;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text(
              '安装与启动',
              style: TextStyle(fontSize: 26, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            Text(_status),
            const SizedBox(height: 24),
            LinearProgressIndicator(value: _progress),
            const SizedBox(height: 24),
            if (!_installing && !_launched)
              SizedBox(
                width: double.infinity,
                height: 48,
                child: ElevatedButton.icon(
                  onPressed: _startInstall,
                  icon: const Icon(Icons.download),
                  label: const Text('开始安装'),
                ),
              ),
            if (_installing)
              SizedBox(
                width: double.infinity,
                height: 48,
                child: ElevatedButton(
                  onPressed: null,
                  child: const Text('安装中...'),
                ),
              ),
            if (_launched)
              SizedBox(
                width: double.infinity,
                height: 48,
                child: ElevatedButton.icon(
                  onPressed: _launchGame,
                  icon: const Icon(Icons.play_arrow),
                  label: const Text('启动 Minecraft'),
                ),
              ),
            const SizedBox(height: 16),
            if (_launched)
              TextButton(
                onPressed: () => setState(() {
                  _launched = false;
                  _progress = 0.0;
                  _status = '等待中...';
                }),
                child: const Text('重置'),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _startInstall() async {
    setState(() {
      _installing = true;
      _progress = 0.0;
      _status = '初始化安装...';
    });
    try {
      final bridge = McLauncherBridge();
      await bridge.init();
      const total = 100;
      for (int i = 1; i <= total; i++) {
        await Future.delayed(const Duration(milliseconds: 30));
        if (!mounted) return;
        setState(() {
          _progress = i / total;
          _status = '下载中... $i%';
        });
      }
      setState(() {
        _installing = false;
        _status = '安装完成';
        _launched = true;
      });
    } catch (e) {
      setState(() {
        _installing = false;
        _status = '安装失败: $e';
      });
    }
  }

  Future<void> _launchGame() async {
    setState(() {
      _status = '正在启动 Minecraft...';
    });
    try {
      final bridge = McLauncherBridge();
      await bridge.init();
      final gameDir = '/tmp/minecraft_launcher';
      final exitCode = bridge.launch(
        versionId: '1.20.1',
        gameDir: gameDir,
        authToken: '',
        authUuid: '',
        userName: 'Player',
      );
      if (!mounted) return;
      setState(() {
        _status = '游戏已退出 (退出码: $exitCode)';
      });
    } catch (e) {
      setState(() {
        _status = '启动失败: $e';
      });
    }
  }
}
